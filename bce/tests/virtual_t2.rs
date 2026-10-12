// SPDX-License-Identifier: BSD-2-Clause
//! Scripted peer, not a hardware emulator: supplies bytes and logical time only.
use musha_bce::{
    Error,
    mailbox::{Handshake, State},
    queue::{Queue, Ticket},
    wire::{Completion, Message, PENDING},
};
struct FakeT2 {
    events: Vec<(u64, [u8; 24])>,
}
impl FakeT2 {
    fn new() -> Self {
        Self { events: Vec::new() }
    }
    // Independent peer encoding, not a round-trip through production serializer.
    fn schedule(&mut self, at: u64, qid: u16, index: u16, bytes: u64, status: u16) {
        let mut b = [0; 24];
        b[8..16].copy_from_slice(&bytes.to_le_bytes());
        b[16..18].copy_from_slice(&qid.to_le_bytes());
        b[18..20].copy_from_slice(&index.to_le_bytes());
        b[20..22].copy_from_slice(&status.to_le_bytes());
        b[23] = 128;
        self.events.push((at, b));
    }
    fn next(&mut self, now: u64) -> Option<Completion> {
        let i = self.events.iter().position(|e| e.0 <= now)?;
        Some(Completion::decode(&self.events.remove(i).1).unwrap())
    }
}
fn good(t: Ticket) -> Completion {
    Completion {
        result: 0,
        data_size: 8,
        qid: 2,
        index: t.index,
        status: 0,
        flags: PENDING,
    }
}
#[test]
fn delayed_handshake_matches_version_and_cannot_be_reused() {
    let mut h = Handshake::default();
    let request = h.start(10, 100, 20).unwrap();
    assert!(!h.poll(11, None).unwrap());
    assert!(!h.poll(20, None).unwrap());
    assert_eq!(h.start(21, 100, 20), Err(Error::Busy));
    assert!(h.poll(30, Some(Message::from_raw(request.raw()))).unwrap());
    assert_eq!(h.state(), State::Complete);
    assert_eq!(h.start(31, 100, 20), Err(Error::Busy));
}
#[test]
fn mailbox_absent_late_wrong_and_repeated_replies_are_terminal() {
    let mut h = Handshake::default();
    let reply = h.start(0, 10, 50).unwrap();
    assert_eq!(h.poll(10, Some(reply)), Err(Error::Timeout));
    assert_eq!(h.poll(11, Some(reply)), Err(Error::Timeout));
    assert_eq!(h.start(12, 10, 50), Err(Error::Busy));
    for raw in [0x3000000000020000, 0x2c00000000020001] {
        let mut h = Handshake::default();
        h.start(0, 10, 50).unwrap();
        assert_eq!(
            h.poll(1, Some(Message::from_raw(raw))),
            Err(Error::UnexpectedReply)
        );
    }
    let mut h = Handshake::default();
    let r = h.start(0, 10, 5).unwrap();
    h.poll(1, Some(r)).unwrap();
    assert_eq!(h.poll(2, Some(r)), Err(Error::UnexpectedReply));
}
#[test]
fn stalled_and_backwards_clocks_cannot_leave_mailbox_pending_forever() {
    let mut h = Handshake::default();
    h.start(10, 10, 2).unwrap();
    h.poll(10, None).unwrap();
    h.poll(10, None).unwrap();
    assert_eq!(h.poll(10, None), Err(Error::PollLimit));
    let mut h = Handshake::default();
    h.start(10, 10, 2).unwrap();
    assert_eq!(h.poll(9, None), Err(Error::ClockBackwards));
    let mut h = Handshake::default();
    assert_eq!(h.start(u64::MAX, 1, 2), Err(Error::Invalid));
    assert_eq!(h.state(), State::Idle);
    let mut h = Handshake::default();
    h.start(0, 10, 2).unwrap();
    h.stop();
    assert_eq!(h.poll(1, None), Err(Error::Stopped));
}
#[test]
fn delayed_out_of_order_completions_retire_in_order() {
    let mut q = Queue::<4>::new(2, 0, 100).unwrap();
    let mut peer = FakeT2::new();
    let a = q.submit(8, 0, 100).unwrap();
    let b = q.submit(8, 0, 100).unwrap();
    let c = q.submit(8, 0, 100).unwrap();
    assert_eq!(q.submit(8, 0, 100), Err(Error::QueueFull));
    peer.schedule(5, 2, b.index, 8, 0);
    peer.schedule(8, 2, a.index, 8, 0);
    peer.schedule(9, 2, c.index, 0, 0);
    for now in 1..=9 {
        q.poll(now).unwrap();
        if let Some(reply) = peer.next(now) {
            let ticket = q.complete(reply, now).unwrap().unwrap();
            if now == 5 {
                assert_eq!(q.retire(ticket), Err(Error::Ticket));
                assert_eq!(q.retained(), 3);
            }
            if now == 8 {
                q.retire(a).unwrap();
                q.retire(b).unwrap();
            }
            if now == 9 {
                q.retire(c).unwrap();
            }
        }
    }
    assert_eq!(q.retained(), 0);
}
#[test]
fn repeated_ring_wrap_rejects_stale_cpu_tickets() {
    let mut q = Queue::<3>::new(2, 0, 50).unwrap();
    let first = q.submit(8, 0, 10).unwrap();
    q.complete(good(first), 0).unwrap();
    q.retire(first).unwrap();
    for now in 1..1000 {
        let t = q.submit(8, now, 10).unwrap();
        q.complete(good(t), now).unwrap();
        assert_eq!(q.retire(first), Err(Error::Ticket));
        q.retire(t).unwrap();
    }
    assert_eq!(q.retained(), 0);
}
#[test]
fn late_completion_after_timeout_does_not_free_or_reuse_slot() {
    let mut q = Queue::<4>::new(2, 0, 50).unwrap();
    let mut peer = FakeT2::new();
    let t = q.submit(8, 0, 5).unwrap();
    peer.schedule(6, 2, t.index, 8, 0);
    assert_eq!(q.poll(5), Err(Error::Timeout));
    assert_eq!(q.complete(peer.next(6).unwrap(), 6), Err(Error::Timeout));
    assert_eq!(q.retire(t), Err(Error::Timeout));
    assert_eq!(q.submit(8, 7, 5), Err(Error::Timeout));
    assert_eq!(q.retained(), 1);
}
#[test]
fn invalid_completions_quarantine_all_outstanding_buffers() {
    for expected in [
        Error::QueueId,
        Error::Slot,
        Error::Length,
        Error::Status,
        Error::Flags,
    ] {
        let mut q = Queue::<4>::new(2, 0, 50).unwrap();
        let t = q.submit(8, 0, 10).unwrap();
        q.submit(8, 0, 10).unwrap();
        let mut c = good(t);
        match expected {
            Error::QueueId => c.qid = 3,
            Error::Slot => c.index = 4,
            Error::Length => c.data_size = 9,
            Error::Status => c.status = 2,
            Error::Flags => c.flags |= 1,
            _ => unreachable!(),
        }
        assert_eq!(q.complete(c, 1), Err(expected));
        assert_eq!(q.retained(), 2);
        assert_eq!(q.retire(t), Err(expected));
        assert_eq!(q.submit(8, 1, 10), Err(expected));
    }
}
#[test]
fn duplicates_before_and_after_retirement_are_rejected() {
    for retired in [false, true] {
        let mut q = Queue::<4>::new(2, 0, 50).unwrap();
        let t = q.submit(8, 0, 10).unwrap();
        q.complete(good(t), 1).unwrap();
        if retired {
            q.retire(t).unwrap();
        }
        assert_eq!(q.complete(good(t), 2), Err(Error::Duplicate));
        assert!(q.submit(8, 2, 10).is_err());
    }
}
#[test]
fn unpublished_completion_is_not_consumed_and_early_retire_is_denied() {
    let mut q = Queue::<4>::new(2, 0, 50).unwrap();
    let t = q.submit(8, 0, 10).unwrap();
    assert_eq!(q.retire(t), Err(Error::NotComplete));
    let mut c = good(t);
    c.flags = 0;
    c.index = u16::MAX;
    c.qid = u16::MAX;
    assert_eq!(q.complete(c, 1), Ok(None));
    assert_eq!(q.retained(), 1);
    q.complete(good(t), 2).unwrap();
    q.retire(t).unwrap();
    assert_eq!(q.retained(), 0);
}
#[test]
fn shutdown_and_stalled_clock_pin_pending_ownership() {
    let mut q = Queue::<4>::new(2, 0, 2).unwrap();
    let t = q.submit(8, 0, 10).unwrap();
    q.poll(0).unwrap();
    q.poll(0).unwrap();
    assert_eq!(q.poll(0), Err(Error::PollLimit));
    assert_eq!(q.retained(), 1);
    assert_eq!(q.retire(t), Err(Error::PollLimit));
    let mut q = Queue::<4>::new(2, 0, 10).unwrap();
    let t = q.submit(8, 0, 10).unwrap();
    q.stop();
    assert_eq!(q.complete(good(t), 1), Err(Error::Stopped));
    assert_eq!(q.retained(), 1);
    let mut q = Queue::<4>::new(2, 10, 10).unwrap();
    q.submit(8, 10, 10).unwrap();
    assert_eq!(q.poll(9), Err(Error::ClockBackwards));
    assert_eq!(q.retained(), 1);
}
#[test]
fn invalid_configuration_and_overflow_never_reserve() {
    assert!(Queue::<1>::new(2, 0, 10).is_err());
    assert!(Queue::<257>::new(2, 0, 10).is_err());
    assert!(Queue::<4>::new(0, 0, 10).is_err());
    assert!(Queue::<4>::new(256, 0, 10).is_err());
    let mut q = Queue::<4>::new(2, u64::MAX - 1, 10).unwrap();
    assert_eq!(q.submit(8, u64::MAX - 1, 2), Err(Error::Invalid));
    assert_eq!(q.retained(), 0);
}

#[test]
fn cq_waits_for_ack_before_advancing_or_reusing_and_wraps() {
    use musha_bce::queue::CompletionCursor;
    let mut cq = CompletionCursor::<3>::new().unwrap();
    let mut sq = Queue::<4>::new(2, 0, 100).unwrap();
    let mut peer = FakeT2::new();
    for now in 0..100 {
        let ticket = sq.submit(8, now, 10).unwrap();
        peer.schedule(now, 2, ticket.index, 8, 0);
        let (_, bytes) = peer.events.remove(0);
        let old_index = cq.index();
        let (completion, ack) = cq.offer(old_index, &bytes).unwrap().unwrap();
        assert_eq!(cq.offer(old_index, &bytes), Err(Error::Busy));
        assert_eq!(cq.index(), old_index);
        assert_eq!(sq.complete(completion, now).unwrap(), Some(ticket));
        // Peer acknowledgement: clear ownership before publishing consumer index.
        let mut cleared = bytes;
        cleared[22..].fill(0);
        cq.acknowledged(ack).unwrap();
        sq.retire(ticket).unwrap();
        assert_eq!(cq.index(), ((now + 1) % 3) as u16);
        assert_eq!(cq.offer(cq.index(), &cleared), Ok(None));
    }
    assert_eq!(sq.retained(), 0);
}
#[test]
fn cq_malformed_or_failed_ack_cannot_advance() {
    use musha_bce::queue::{Ack, CompletionCursor};
    let mut cq = CompletionCursor::<4>::new().unwrap();
    assert_eq!(cq.offer(0, &[0; 23]), Err(Error::Length));
    assert_eq!(cq.offer(0, &[0; 24]), Err(Error::Length));
    let mut cq = CompletionCursor::<4>::new().unwrap();
    let mut b = [0; 24];
    b[23] = 128;
    let (_, ack) = cq.offer(0, &b).unwrap().unwrap();
    assert_eq!(
        cq.acknowledged(Ack { slot: 0, next: 2 }),
        Err(Error::Ticket)
    );
    assert_eq!(cq.acknowledged(ack), Err(Error::Ticket));
    assert_eq!(cq.index(), 0);
    let mut cq = CompletionCursor::<4>::new().unwrap();
    let (_, ack) = cq.offer(0, &b).unwrap().unwrap();
    cq.stop();
    assert_eq!(cq.acknowledged(ack), Err(Error::Stopped));
    assert_eq!(cq.index(), 0);
}
