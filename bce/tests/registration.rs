// SPDX-License-Identifier: BSD-2-Clause
use musha_bce::{
    Error,
    registration::{Operation, Registration, State},
    wire::{self, RegistrationConfig},
};
fn config() -> RegistrationConfig<'static> {
    RegistrationConfig {
        qid: 2,
        count: 4,
        vector_or_cq: 1,
        address: 0x12345000,
        stride: 32,
        name: Some(b"kbd"),
        out: true,
    }
}
fn ready() -> Registration {
    let mut r = Registration::new(2).unwrap();
    let cmd = r.register(config(), 0, 10, 10).unwrap();
    assert!(r.poll(1, Some((cmd.token, 0))).unwrap());
    r
}
#[test]
fn golden_commands_match_upstream_offsets_and_zero_padding() {
    let b = wire::register_queue(config()).unwrap();
    assert_eq!(
        b,
        [
            0x20, 0, 3, 0, 2, 0, 0, 0, 4, 0, 1, 0, 0, 0, 3, 0, b'k', b'b', b'd', 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x50, 0x34,
            0x12, 0, 0, 0, 0, 128, 0, 0, 0, 0, 0, 0, 0
        ]
    );
    for (flush, opcode) in [(true, 0x40), (false, 0x30)] {
        let b = wire::simple_queue_command(255, flush).unwrap();
        assert_eq!(&b[..6], &[opcode, 0, 0, 0, 255, 0]);
        assert!(b[6..].iter().all(|b| *b == 0));
    }
    let mut c = config();
    c.name = None;
    c.out = false;
    let b = wire::register_queue(c).unwrap();
    assert_eq!(b[2], 0);
    assert!(b[12..48].iter().all(|b| *b == 0));
    c.name = Some(&[b'x'; 32]);
    assert!(wire::register_queue(c).is_ok());
}
#[test]
fn invalid_configuration_never_starts_registration() {
    for name in [&b""[..], &b"a\0b"[..], &[b'x'; 33][..]] {
        let mut c = config();
        c.name = Some(name);
        let mut r = Registration::new(2).unwrap();
        assert!(matches!(r.register(c, 0, 10, 3), Err(Error::Length)));
        assert_eq!(r.state(), State::New);
    }
    for id in [0, 1, 256, u16::MAX] {
        assert!(Registration::new(id).is_err());
        assert!(wire::simple_queue_command(id, true).is_err());
    }
    for (addr, count, stride) in [
        (0, 4, 32),
        (3, 4, 32),
        (u64::MAX - 3, 4, 32),
        (4096, 1, 32),
        (4096, 257, 32),
        (4096, 4, 31),
    ] {
        let mut c = config();
        c.address = addr;
        c.count = count;
        c.stride = stride;
        assert!(wire::register_queue(c).is_err());
    }
    let mut r = Registration::new(3).unwrap();
    assert!(matches!(
        r.register(config(), 0, 10, 3),
        Err(Error::QueueId)
    ));
    let mut r = Registration::new(2).unwrap();
    for (now, timeout, polls) in [(0, 0, 1), (0, 1, 0), (u64::MAX, 1, 1)] {
        assert!(matches!(
            r.register(config(), now, timeout, polls),
            Err(Error::Invalid)
        ));
        assert_eq!(r.state(), State::New);
    }
}
#[test]
fn registration_gates_transfers_flush_and_unregister() {
    let mut r = Registration::new(2).unwrap();
    assert_eq!(r.begin_transfer(), Err(Error::Busy));
    assert!(r.command(Operation::Flush, 0, 10, 4).is_err());
    let cmd = r.register(config(), 0, 10, 4).unwrap();
    assert_eq!(r.begin_transfer(), Err(Error::Busy));
    assert!(r.register(config(), 0, 10, 4).is_err());
    assert!(!r.poll(1, None).unwrap());
    r.poll(2, Some((cmd.token, 0))).unwrap();
    let work = r.begin_transfer().unwrap();
    assert_eq!(r.begin_transfer(), Err(Error::Busy));
    assert!(r.command(Operation::Flush, 3, 10, 4).is_err());
    r.finish_transfer(work).unwrap();
    assert!(r.command(Operation::Unregister, 3, 10, 4).is_err());
    let flush = r.command(Operation::Flush, 3, 10, 4).unwrap();
    assert_eq!(r.begin_transfer(), Err(Error::Busy));
    r.poll(4, Some((flush.token, 0))).unwrap();
    assert_eq!(r.state(), State::Flushed);
    let unreg = r.command(Operation::Unregister, 5, 10, 4).unwrap();
    r.poll(6, Some((unreg.token, 0))).unwrap();
    assert_eq!(r.state(), State::Unregistered);
    assert_eq!(r.begin_transfer(), Err(Error::Busy));
    assert!(r.register(config(), 7, 10, 4).is_err());
}
#[test]
fn new_work_invalidates_previous_flush() {
    let mut r = ready();
    let f = r.command(Operation::Flush, 2, 10, 4).unwrap();
    r.poll(3, Some((f.token, 0))).unwrap();
    let t = r.begin_transfer().unwrap();
    r.finish_transfer(t).unwrap();
    assert!(r.command(Operation::Unregister, 4, 10, 4).is_err());
}
#[test]
fn timeout_clock_and_poll_budget_are_terminal_even_with_reply() {
    for (start, now, budget, expected) in [
        (0, 10, 2, Error::Timeout),
        (2, 1, 2, Error::ClockBackwards),
        (0, 1, 1, Error::PollLimit),
    ] {
        let mut r = Registration::new(2).unwrap();
        let c = r.register(config(), start, 10, budget).unwrap();
        if expected == Error::PollLimit {
            assert!(!r.poll(0, None).unwrap());
        }
        assert_eq!(r.poll(now, Some((c.token, 0))), Err(expected));
        assert_eq!(r.poll(0, Some((c.token, 0))), Err(expected));
        assert_eq!(r.begin_transfer(), Err(expected));
        r.quarantine(Error::Stopped);
        assert_eq!(r.state(), State::Quarantined(expected));
    }
}
#[test]
fn stale_duplicate_and_foreign_queue_replies_are_rejected() {
    let mut r = Registration::new(2).unwrap();
    let old = r.register(config(), 0, 10, 10).unwrap();
    r.poll(1, Some((old.token, 0))).unwrap();
    r.command(Operation::Flush, 2, 10, 10).unwrap();
    assert_eq!(r.poll(3, Some((old.token, 0))), Err(Error::Ticket));
    let mut r = ready();
    assert_eq!(r.poll(2, None), Err(Error::UnexpectedReply));
    let mut other = Registration::new(3).unwrap();
    let mut c = config();
    c.qid = 3;
    let foreign = other.register(c, 0, 10, 10).unwrap();
    let mut r = Registration::new(2).unwrap();
    r.register(config(), 0, 10, 10).unwrap();
    assert_eq!(r.poll(1, Some((foreign.token, 0))), Err(Error::Ticket));
}
#[test]
fn errors_at_each_lifecycle_stage_block_further_operations() {
    for stage in 0..3 {
        for publication_error in [false, true] {
            let mut r = Registration::new(2).unwrap();
            let mut c = r.register(config(), 0, 10, 10).unwrap();
            if stage > 0 {
                r.poll(1, Some((c.token, 0))).unwrap();
                c = r.command(Operation::Flush, 2, 10, 10).unwrap();
            }
            if stage > 1 {
                r.poll(3, Some((c.token, 0))).unwrap();
                c = r.command(Operation::Unregister, 4, 10, 10).unwrap();
            }
            let expected = if publication_error {
                r.quarantine(Error::Io);
                Error::Io
            } else {
                assert_eq!(r.poll(5, Some((c.token, 7))), Err(Error::Status));
                Error::Status
            };
            assert_eq!(r.begin_transfer(), Err(expected));
            assert_eq!(r.poll(6, Some((c.token, 0))), Err(expected));
        }
    }
}
#[test]
fn transfer_failure_and_duplicate_retirement_quarantine() {
    let mut r = ready();
    let t = r.begin_transfer().unwrap();
    r.quarantine(Error::Io);
    assert_eq!(r.finish_transfer(t), Err(Error::Io));
    let mut r = ready();
    let t = r.begin_transfer().unwrap();
    r.finish_transfer(t).unwrap();
    assert_eq!(r.finish_transfer(t), Err(Error::Ticket));
}
