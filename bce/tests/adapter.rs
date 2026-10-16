// Copyright (c) 2026 Hirofumi Iwasaki
// SPDX-License-Identifier: BSD-2-Clause
use musha_bce::{
    Error,
    adapter::*,
    wire::{Message, PROTOCOL_VERSION, SET_PROTOCOL},
};
use std::collections::VecDeque;
#[derive(Debug, Clone, PartialEq, Eq)]
enum Op {
    Read(u32),
    Write(u32, u32),
    Fence,
    Submission(u16),
    ToDevice,
    FromDevice,
    Doorbell(u32, u16),
    Completion(u16),
    Copy(u16, usize),
    Clear(u16),
    Retain,
    Command(u16),
}
#[derive(Default)]
struct Peer {
    trace: Vec<Op>,
    fail_at: Option<usize>,
    replies: VecDeque<u64>,
    payload: Vec<u8>,
    cq: [u8; 24],
    descriptors: Vec<[u8; 32]>,
    retained: bool,
}
impl Peer {
    fn op(&mut self, op: Op) -> Result<(), Error> {
        self.trace.push(op);
        if self.fail_at == Some(self.trace.len()) {
            Err(Error::Io)
        } else {
            Ok(())
        }
    }
    fn reply(&mut self) {
        self.replies
            .push_back(Message::new(SET_PROTOCOL, PROTOCOL_VERSION).unwrap().raw());
    }
    fn complete(&mut self, index: u16) {
        self.cq = [0; 24];
        self.cq[8..16].copy_from_slice(&(self.payload.len() as u64).to_le_bytes());
        self.cq[16..18].copy_from_slice(&1u16.to_le_bytes());
        self.cq[18..20].copy_from_slice(&index.to_le_bytes());
        self.cq[22..24].copy_from_slice(&0x8000u16.to_le_bytes());
    }
}
impl Registers for Peer {
    fn read(&mut self, offset: u32) -> Result<u32, Error> {
        self.op(Op::Read(offset))?;
        Ok(match offset {
            REPLY_COUNT => (self.replies.len() as u32) << 20,
            REPLY => *self.replies.front().unwrap() as u32,
            x if x == REPLY + 4 => (*self.replies.front().unwrap() >> 32) as u32,
            x if x == REPLY + 12 => {
                self.replies.pop_front();
                0
            }
            _ => 0,
        })
    }
    fn write(&mut self, o: u32, v: u32) -> Result<(), Error> {
        self.op(Op::Write(o, v))
    }
    fn fence(&mut self) -> Result<(), Error> {
        self.op(Op::Fence)
    }
}
impl Memory for Peer {
    fn write_submission(&mut self, slot: u16, b: [u8; 32]) -> Result<(), Error> {
        self.op(Op::Submission(slot))?;
        self.descriptors.push(b);
        Ok(())
    }
    fn sync_to_device(&mut self) -> Result<(), Error> {
        self.op(Op::ToDevice)
    }
    fn doorbell(&mut self, o: u32, i: u16) -> Result<(), Error> {
        self.op(Op::Doorbell(o, i))
    }
    fn sync_from_device(&mut self) -> Result<(), Error> {
        self.op(Op::FromDevice)
    }
    fn completion(&mut self, i: u16) -> Result<[u8; 24], Error> {
        self.op(Op::Completion(i))?;
        Ok(self.cq)
    }
    fn copy_payload(&mut self, i: u16, out: &mut [u8]) -> Result<(), Error> {
        self.op(Op::Copy(i, out.len()))?;
        out.copy_from_slice(&self.payload[..out.len()]);
        Ok(())
    }
    fn clear_completion(&mut self, i: u16) -> Result<(), Error> {
        self.op(Op::Clear(i))?;
        self.cq[22..24].fill(0);
        Ok(())
    }
    fn retain(&mut self) {
        self.trace.push(Op::Retain);
        self.retained = true;
    }
}
#[test]
fn mailbox_register_order_periodic_time_during_wait_and_stop() {
    let mut peer = Peer::default();
    let mut m = Mailbox::default();
    m.start(&mut peer, 0, 1000, 100).unwrap();
    let msg = Message::new(SET_PROTOCOL, PROTOCOL_VERSION)
        .unwrap()
        .words();
    assert_eq!(
        peer.trace,
        [
            Op::Read(REPLY_COUNT),
            Op::Read(TIME),
            Op::Fence,
            Op::Write(TIME + 8, 0xffff_fffc),
            Op::Write(TIME, u32::MAX),
            Op::Write(OUT, msg[0]),
            Op::Write(OUT + 4, msg[1]),
            Op::Write(OUT + 8, 0),
            Op::Write(OUT + 12, 0)
        ]
    );
    peer.trace.clear();
    assert!(!m.poll(&mut peer, 149).unwrap());
    assert_eq!(peer.trace, [Op::Read(REPLY_COUNT)]);
    peer.trace.clear();
    assert!(!m.poll(&mut peer, 150).unwrap());
    assert_eq!(
        peer.trace,
        [
            Op::Read(TIME + 8),
            Op::Fence,
            Op::Write(TIME + 8, 150_000_000),
            Op::Write(TIME, 0),
            Op::Read(REPLY_COUNT)
        ]
    );
    peer.reply();
    assert!(m.poll(&mut peer, 151).unwrap());
    assert!(m.ready());
    peer.trace.clear();
    assert!(m.poll(&mut peer, 4500).unwrap()); // late caller: one update, no catch-up loop
    assert_eq!(
        peer.trace,
        [
            Op::Read(TIME + 8),
            Op::Fence,
            Op::Write(TIME + 8, 4_500_000_000u64 as u32),
            Op::Write(TIME, 1),
            Op::Read(REPLY_COUNT)
        ]
    );
    peer.trace.clear();
    m.stop(&mut peer);
    m.stop(&mut peer);
    assert_eq!(
        peer.trace,
        [Op::Write(TIME + 8, 0xffff_fffe), Op::Write(TIME, u32::MAX)]
    );
    assert_eq!(m.poll(&mut peer, 4501), Err(Error::Stopped));
    assert!(!m.ready());
}
#[test]
fn every_initialization_io_failure_is_terminal_and_cleanup_is_bounded() {
    for fail in 1..=9 {
        let mut peer = Peer {
            fail_at: Some(fail),
            ..Peer::default()
        };
        let mut m = Mailbox::default();
        assert_eq!(m.start(&mut peer, 0, 1000, 10), Err(Error::Io));
        assert!(peer.trace.len() <= 11);
        let n = peer.trace.len();
        assert_eq!(m.poll(&mut peer, 1), Err(Error::Io));
        assert_eq!(m.start(&mut peer, 2, 100, 10), Err(Error::Io));
        m.stop(&mut peer);
        assert_eq!(peer.trace.len(), n);
    }
}
#[test]
fn polling_io_failures_and_stop_write_failure_are_recorded() {
    // One timestamp update (4 operations) followed by counter and 4 reply reads.
    for fail in 1..=9 {
        let mut peer = Peer::default();
        let mut m = Mailbox::default();
        m.start(&mut peer, 0, 1000, 10).unwrap();
        peer.trace.clear();
        peer.reply();
        peer.fail_at = Some(fail);
        assert_eq!(m.poll(&mut peer, 150), Err(Error::Io));
        assert!(!m.ready());
        let n = peer.trace.len();
        assert_eq!(m.poll(&mut peer, 151), Err(Error::Io));
        assert_eq!(peer.trace.len(), n);
    }
    let mut p = Peer::default();
    let mut m = Mailbox::default();
    m.start(&mut p, 0, 100, 10).unwrap();
    p.trace.clear();
    p.fail_at = Some(1);
    m.stop(&mut p);
    assert!(m.cleanup_failed());
    assert_eq!(p.trace.len(), 1);
}
#[test]
fn stale_multiple_wrong_late_and_duplicate_replies_never_recover() {
    let mut p = Peer::default();
    p.reply();
    let mut m = Mailbox::default();
    assert_eq!(m.start(&mut p, 0, 1000, 10), Err(Error::UnexpectedReply));
    assert_eq!(p.trace, [Op::Read(REPLY_COUNT)]);
    for case in 0..4 {
        let mut p = Peer::default();
        let mut m = Mailbox::default();
        m.start(&mut p, 0, 100, 10).unwrap();
        let (now, error) = match case {
            0 => {
                p.reply();
                p.reply();
                (1, Error::UnexpectedReply)
            }
            1 => {
                p.replies.push_back(0);
                (1, Error::UnexpectedReply)
            }
            2 => {
                p.reply();
                (100, Error::Timeout)
            }
            _ => {
                p.reply();
                assert!(m.poll(&mut p, 1).unwrap());
                p.reply();
                (2, Error::UnexpectedReply)
            }
        };
        assert_eq!(m.poll(&mut p, now), Err(error));
        assert!(!m.ready());
    }
}
#[test]
fn mailbox_clock_failures_stop_without_unbounded_wait() {
    let mut p = Peer::default();
    let mut m = Mailbox::default();
    m.start(&mut p, 10, 100, 1).unwrap();
    assert!(!m.poll(&mut p, 10).unwrap());
    assert_eq!(m.poll(&mut p, 10), Err(Error::PollLimit));
    let mut p = Peer::default();
    let mut m = Mailbox::default();
    m.start(&mut p, 10, 100, 10).unwrap();
    assert_eq!(m.poll(&mut p, 9), Err(Error::ClockBackwards));
    let mut p = Peer::default();
    let mut m = Mailbox::default();
    assert_eq!(m.start(&mut p, u64::MAX, 100, 10), Err(Error::Invalid));
    assert!(p.trace.is_empty());
}
#[test]
fn transfer_publication_copy_ack_retirement_order_and_ring_wrap() {
    let mut t = Transfer::<4>::new(1, 0, 0, 100).unwrap();
    let mut p = Peer::default();
    let mut out = [0; 8];
    for i in 0..100 {
        p.trace.clear();
        p.payload = vec![i as u8; 8];
        let index = (i % 4) as u16;
        t.submit(&mut p, 0x1000, 8, i * 2, 100).unwrap();
        assert_eq!(
            p.trace,
            [
                Op::Submission(index),
                Op::ToDevice,
                Op::Doorbell(DOORBELL + 4, (index + 1) % 4)
            ]
        );
        let descriptor = p.descriptors.last().unwrap();
        assert_eq!(&descriptor[..8], &8u64.to_le_bytes());
        assert_eq!(&descriptor[8..16], &0x1000u64.to_le_bytes());
        p.trace.clear();
        p.complete(index);
        assert_eq!(t.poll(&mut p, i * 2 + 1, &mut out), Ok(Some(8)));
        assert_eq!(
            p.trace,
            [
                Op::FromDevice,
                Op::Completion(index),
                Op::Copy(index, 8),
                Op::Clear(index),
                Op::ToDevice,
                Op::Doorbell(DOORBELL, (index + 1) % 4)
            ]
        );
        assert_eq!(out, [i as u8; 8]);
        assert_eq!(t.retained(), 0);
        assert!(!p.retained);
    }
}
#[test]
fn every_publication_and_completion_io_failure_retains_buffers() {
    for fail in 1..=3 {
        let mut t = Transfer::<4>::new(1, 0, 0, 100).unwrap();
        let mut p = Peer {
            fail_at: Some(fail),
            ..Peer::default()
        };
        assert_eq!(t.submit(&mut p, 0x1000, 8, 0, 100), Err(Error::Io));
        assert_eq!(t.retained(), 1);
        assert!(p.retained);
        let n = p.trace.len();
        assert_eq!(t.poll(&mut p, 1, &mut [0; 8]), Err(Error::Io));
        assert_eq!(p.trace.len(), n);
    }
    for fail in 1..=6 {
        let mut t = Transfer::<4>::new(1, 0, 0, 100).unwrap();
        let mut p = Peer::default();
        t.submit(&mut p, 0x1000, 8, 0, 100).unwrap();
        p.payload = vec![1; 8];
        p.complete(0);
        p.trace.clear();
        p.fail_at = Some(fail);
        assert_eq!(t.poll(&mut p, 1, &mut [0; 8]), Err(Error::Io));
        assert_eq!(t.retained(), 1);
        assert!(p.retained);
        let n = p.trace.len();
        t.stop(&mut p);
        assert_eq!(t.submit(&mut p, 0x2000, 8, 2, 100), Err(Error::Io));
        assert_eq!(p.trace.len(), n);
    }
}
#[test]
fn timeout_short_buffer_invalid_completion_and_shutdown_keep_pending_memory() {
    for case in 0..5 {
        let mut t = Transfer::<4>::new(1, 0, 0, 100).unwrap();
        let mut p = Peer::default();
        t.submit(&mut p, 0x1000, 8, 0, 100).unwrap();
        p.payload = vec![1; 8];
        p.complete(0);
        p.trace.clear();
        let error = match case {
            0 => t.poll(&mut p, 100, &mut [0; 8]),
            1 => t.poll(&mut p, 1, &mut [0; 7]),
            2 => {
                p.cq[16] = 2;
                t.poll(&mut p, 1, &mut [0; 8])
            }
            3 => {
                t.stop(&mut p);
                t.poll(&mut p, 1, &mut [0; 8])
            }
            _ => {
                p.cq[20] = 1;
                t.poll(&mut p, 1, &mut [0; 8])
            }
        };
        assert!(error.is_err());
        assert_eq!(t.retained(), 1);
        assert!(p.retained);
        assert!(
            !p.trace
                .iter()
                .any(|op| matches!(op, Op::Copy(..) | Op::Clear(..) | Op::Doorbell(..)))
        );
    }
}
#[test]
fn delayed_transfer_allows_periodic_mailbox_service_and_unrelated_progress() {
    let mut p = Peer::default();
    let mut mailbox = Mailbox::default();
    mailbox.start(&mut p, 0, 1000, 1000).unwrap();
    p.reply();
    assert!(mailbox.poll(&mut p, 1).unwrap());
    let mut transfer = Transfer::<4>::new(1, 0, 1, 2000).unwrap();
    transfer.submit(&mut p, 0x1000, 4, 1, 900).unwrap();
    let mut unrelated = 0;
    for now in 2..=600 {
        assert!(mailbox.poll(&mut p, now).unwrap());
        assert_eq!(transfer.poll(&mut p, now, &mut [0; 4]), Ok(None));
        unrelated += 1;
    }
    assert_eq!(unrelated, 599);
    for ms in [150, 300, 450, 600] {
        assert!(p.trace.contains(&Op::Write(TIME + 8, ms * 1_000_000)));
    }
    p.payload = vec![1, 2, 3, 4];
    p.complete(0);
    let mut out = [0; 4];
    assert_eq!(transfer.poll(&mut p, 601, &mut out), Ok(Some(4)));
    assert_eq!(out, [1, 2, 3, 4]);
    transfer.stop(&mut p);
    mailbox.stop(&mut p);
    assert!(p.retained);
}

#[test]
fn simulated_payload_to_hid_decoder_to_shared_input() {
    use musha_input::{
        hid::{Decoder, Layout},
        sources::Sources,
    };
    // Synthetic descriptor, not captured from an Apple device.
    let descriptor = vec![
        0x05, 1, 0x09, 6, 0xa1, 1, 0x85, 5, 0x05, 7, 0x15, 0, 0x25, 1, 0x75, 1, 0x95, 8, 0x19,
        0xe0, 0x29, 0xe7, 0x81, 2, 0x19, 4, 0x29, 11, 0x81, 2, 0xc0,
    ];
    let mut peer = Peer::default();
    let mut transfer = Transfer::<4>::new(1, 0, 0, 100).unwrap();
    transfer.submit(&mut peer, 0x1000, 64, 0, 100).unwrap();
    peer.payload = descriptor;
    peer.complete(0);
    let mut bytes = [0; 64];
    let length = transfer.poll(&mut peer, 1, &mut bytes).unwrap().unwrap();
    let mut decoder = Decoder::new(Layout::parse(&bytes[..length]).unwrap());
    let mut sources = Sources::<2>::default();
    let source = sources.connect(1, |_, _| {}).unwrap();
    transfer.submit(&mut peer, 0x1000, 3, 2, 100).unwrap();
    peer.payload = vec![5, 2, 1];
    peer.complete(1);
    let length = transfer.poll(&mut peer, 3, &mut bytes).unwrap().unwrap();
    let mut events = Vec::new();
    decoder
        .update(&bytes[..length], |k, d| {
            sources
                .key(source, k, d, |k, d| events.push((k, d)))
                .unwrap()
        })
        .unwrap();
    assert_eq!(events, [(0xe1, true), (4, true)]);
    transfer.stop(&mut peer);
    events.clear();
    sources
        .disconnect(source, |k, d| events.push((k, d)))
        .unwrap();
    assert_eq!(events, [(0xe1, false), (4, false)]);
}

#[test]
fn registration_gate_then_transfer_ack_flush_and_unregister() {
    use musha_bce::{
        registration::{Operation, Registration, State},
        wire::RegistrationConfig,
    };
    let mut life = Registration::new(2).unwrap();
    let mut transfer = Transfer::<4>::new(2, 0, 0, 100).unwrap();
    let mut peer = Peer::default();
    // The integrating caller must gate before touching the transfer adapter.
    assert_eq!(life.begin_transfer(), Err(Error::Busy));
    assert!(peer.trace.is_empty());
    let request = life
        .register(
            RegistrationConfig {
                qid: 2,
                count: 4,
                vector_or_cq: 0,
                address: 0x1000,
                stride: 32,
                name: Some(b"kbd"),
                out: false,
            },
            0,
            10,
            100,
        )
        .unwrap();
    assert_eq!(life.begin_transfer(), Err(Error::Busy));
    // Scripted command transport completion, not firmware emulation.
    life.poll(1, Some((request.token, 0))).unwrap();
    let ticket = life.begin_transfer().unwrap();
    peer.payload = vec![4, 0, 0, 0];
    transfer.submit(&mut peer, 0x2000, 4, 2, 10).unwrap();
    assert!(life.command(Operation::Flush, 2, 10, 10).is_err());
    peer.complete(0);
    peer.cq[16..18].copy_from_slice(&2u16.to_le_bytes());
    let mut output = [0; 4];
    assert_eq!(transfer.poll(&mut peer, 3, &mut output), Ok(Some(4)));
    life.finish_transfer(ticket).unwrap();
    let flush = life.command(Operation::Flush, 4, 10, 10).unwrap();
    life.poll(5, Some((flush.token, 0))).unwrap();
    let unregister = life.command(Operation::Unregister, 6, 10, 10).unwrap();
    life.poll(7, Some((unregister.token, 0))).unwrap();
    assert_eq!(life.state(), State::Unregistered);
    assert_eq!(output, [4, 0, 0, 0]);
    // Transfer's metadata and backing storage still exist; no deallocation API.
    assert_eq!(life.begin_transfer(), Err(Error::Busy));
}

#[test]
fn declared_extent_rejection_has_no_io_or_ownership_side_effects() {
    let mut transfer = Transfer::<4>::new(2, 0, 0, 100).unwrap();
    let mut peer = Peer::default();
    let region = musha_bce::wire::ContiguousRegion::new(0x1000, 4096, 64).unwrap();
    for (address, length) in [(0x1004, 8), (0x1fc0, 65), (0x2000, 8)] {
        assert_eq!(
            transfer.submit_in_region(&mut peer, &region, address, length, 0, 10),
            Err(Error::Invalid)
        );
        assert!(peer.trace.is_empty());
        assert_eq!(transfer.retained(), 0);
    }
    transfer
        .submit_in_region(&mut peer, &region, 0x1fc0, 64, 0, 10)
        .unwrap();
    transfer.stop(&mut peer);
    let trace_length = peer.trace.len();
    assert_eq!(
        transfer.submit_in_region(&mut peer, &region, 0x1000, 8, 1, 10),
        Err(Error::Stopped)
    );
    assert_eq!(peer.trace.len(), trace_length);
    assert_eq!(transfer.retained(), 1);
}

impl musha_bce::registered::Transport for Peer {
    fn publish_command(&mut self, request: &musha_bce::registration::Request) -> Result<(), Error> {
        self.op(Op::Command(u16::from_le_bytes(
            request.bytes[4..6].try_into().unwrap(),
        )))
    }
}
fn integrated() -> musha_bce::registered::RegisteredTransfer<4> {
    musha_bce::registered::RegisteredTransfer::new(
        2,
        3,
        musha_bce::wire::ContiguousRegion::new(0x2000, 0x2000, 4).unwrap(),
        0,
        100,
    )
    .unwrap()
}
fn queue_config(cq: bool) -> musha_bce::wire::RegistrationConfig<'static> {
    musha_bce::wire::RegistrationConfig {
        qid: if cq { 3 } else { 2 },
        count: 4,
        vector_or_cq: if cq { 0 } else { 3 },
        address: if cq { 0x2000 } else { 0x2100 },
        stride: if cq { 24 } else { 32 },
        name: None,
        out: !cq,
    }
}
fn integrated_ready(peer: &mut Peer) -> musha_bce::registered::RegisteredTransfer<4> {
    use musha_bce::registered::QueueKind::*;
    let mut r = integrated();
    let cq = r
        .register(peer, Completion, queue_config(true), 0, 100, 100)
        .unwrap();
    r.poll_registration(peer, Completion, 1, Some((cq, 0)))
        .unwrap();
    let sq = r
        .register(peer, Submission, queue_config(false), 2, 100, 100)
        .unwrap();
    r.poll_registration(peer, Submission, 3, Some((sq, 0)))
        .unwrap();
    r
}
#[test]
fn integrated_cq_then_sq_ack_and_extent_gates_precede_all_transfer_io() {
    use musha_bce::registered::QueueKind::*;
    let mut p = Peer::default();
    let mut r = integrated();
    assert_eq!(r.submit(&mut p, 0x3000, 4, 0, 10), Err(Error::Busy));
    assert_eq!(
        r.register(&mut p, Submission, queue_config(false), 0, 10, 10),
        Err(Error::Busy)
    );
    assert!(p.trace.is_empty());
    let cq = r
        .register(&mut p, Completion, queue_config(true), 0, 100, 100)
        .unwrap();
    assert_eq!(
        r.register(&mut p, Submission, queue_config(false), 0, 10, 10),
        Err(Error::Busy)
    );
    r.poll_registration(&mut p, Completion, 1, Some((cq, 0)))
        .unwrap();
    let sq = r
        .register(&mut p, Submission, queue_config(false), 2, 100, 100)
        .unwrap();
    let old = p.trace.clone();
    assert_eq!(r.submit(&mut p, 0x3000, 4, 3, 10), Err(Error::Busy));
    assert_eq!(p.trace, old);
    r.poll_registration(&mut p, Submission, 4, Some((sq, 0)))
        .unwrap();
    for (address, length) in [
        (0x2000, 4),
        (0x2100, 4),
        (0x1ffc, 8),
        (0x3ffc, 8),
        (0x3001, 4),
        (0x3000, 0),
    ] {
        assert_eq!(
            r.submit(&mut p, address, length, 5, 10),
            Err(Error::Invalid)
        );
        assert_eq!(p.trace, old);
    }
    r.submit(&mut p, 0x3000, 4, 5, 10).unwrap();
    assert_eq!(r.retained(), 1);
}
#[test]
fn integrated_wrong_queue_layout_never_publishes_or_consumes_registration() {
    use musha_bce::registered::QueueKind::*;
    let mut p = Peer::default();
    let mut r = integrated();
    for (count, stride, out, address) in [
        (3, 24, false, 0x2000),
        (4, 32, false, 0x2000),
        (4, 24, true, 0x2000),
        (4, 24, false, 0x3ffc),
    ] {
        let mut c = queue_config(true);
        c.count = count;
        c.stride = stride;
        c.out = out;
        c.address = address;
        assert!(r.register(&mut p, Completion, c, 0, 10, 10).is_err());
        assert!(p.trace.is_empty());
    }
    let cq = r
        .register(&mut p, Completion, queue_config(true), 0, 100, 100)
        .unwrap();
    r.poll_registration(&mut p, Completion, 1, Some((cq, 0)))
        .unwrap();
    let before = p.trace.clone();
    let mut c = queue_config(false);
    c.address = 0x2000;
    assert_eq!(
        r.register(&mut p, Submission, c, 2, 10, 10),
        Err(Error::Invalid)
    );
    assert_eq!(p.trace, before);
    c = queue_config(false);
    c.vector_or_cq = 7;
    assert_eq!(
        r.register(&mut p, Submission, c, 2, 10, 10),
        Err(Error::Invalid)
    );
    assert_eq!(p.trace, before);
}
#[test]
fn integrated_publication_and_registration_ack_failure_quarantine_both_queues() {
    use musha_bce::{registered::QueueKind::*, registration::State};
    for sq_failure in [false, true] {
        for publication in [false, true] {
            let mut p = Peer::default();
            let mut r = integrated();
            if sq_failure {
                let token = r
                    .register(&mut p, Completion, queue_config(true), 0, 100, 100)
                    .unwrap();
                r.poll_registration(&mut p, Completion, 1, Some((token, 0)))
                    .unwrap();
            }
            if publication {
                p.fail_at = Some(p.trace.len() + 1);
            }
            let kind = if sq_failure { Submission } else { Completion };
            let expected = if publication {
                Error::Io
            } else {
                Error::Status
            };
            let token = r.register(&mut p, kind, queue_config(!sq_failure), 2, 100, 100);
            if publication {
                assert_eq!(token, Err(expected));
            } else {
                assert_eq!(
                    r.poll_registration(&mut p, kind, 3, Some((token.unwrap(), 7))),
                    Err(expected)
                );
            }
            assert_eq!(
                r.states(),
                (State::Quarantined(expected), State::Quarantined(expected))
            );
            assert!(p.retained);
            let before = p.trace.clone();
            assert_eq!(r.submit(&mut p, 0x3000, 4, 4, 10), Err(expected));
            r.stop(&mut p);
            assert_eq!(p.trace, before);
        }
    }
}
#[test]
fn integrated_failed_cq_ack_retains_ownership_and_rejects_all_later_activity() {
    use musha_bce::{registered::QueueKind::*, registration::State};
    let mut p = Peer::default();
    let mut r = integrated_ready(&mut p);
    p.payload = vec![9, 8, 7, 6];
    r.submit(&mut p, 0x3000, 4, 4, 100).unwrap();
    p.complete(0);
    p.cq[16..18].copy_from_slice(&2u16.to_le_bytes());
    p.fail_at = Some(p.trace.len() + 6);
    let mut out = [0; 4];
    assert_eq!(r.poll(&mut p, 5, &mut out), Err(Error::Io));
    assert_eq!(out, [9, 8, 7, 6]);
    assert_eq!(r.retained(), 1);
    assert_eq!(
        r.states(),
        (State::Quarantined(Error::Io), State::Quarantined(Error::Io))
    );
    let before = p.trace.clone();
    assert_eq!(
        r.poll_registration(&mut p, Submission, 6, None),
        Err(Error::Io)
    );
    assert_eq!(r.poll(&mut p, 6, &mut out), Err(Error::Io));
    assert_eq!(r.submit(&mut p, 0x3000, 4, 6, 100), Err(Error::Io));
    r.stop(&mut p);
    assert_eq!(p.trace, before);
}
#[test]
fn integrated_delayed_completion_and_ring_wrap_retire_only_after_cq_ack() {
    let mut p = Peer::default();
    let mut r = integrated_ready(&mut p);
    p.payload = vec![1, 2, 3, 4];
    let mut out = [0; 4];
    for n in 0..13 {
        let now = 4 + n * 3;
        r.submit(&mut p, 0x3000, 4, now, 100).unwrap();
        assert_eq!(r.poll(&mut p, now + 1, &mut out), Ok(None));
        assert_eq!(r.retained(), 1);
        p.complete((n % 4) as u16);
        p.cq[16..18].copy_from_slice(&2u16.to_le_bytes());
        assert_eq!(r.poll(&mut p, now + 2, &mut out), Ok(Some(4)));
        assert_eq!(r.retained(), 0);
        assert_eq!(out, [1, 2, 3, 4]);
    }
}
#[test]
fn integrated_timeout_and_shutdown_keep_pending_allocation_terminal() {
    for stop in [false, true] {
        let mut p = Peer::default();
        let mut r = integrated_ready(&mut p);
        r.submit(&mut p, 0x3000, 4, 4, 10).unwrap();
        let expected = if stop {
            r.stop(&mut p);
            Error::Stopped
        } else {
            Error::Timeout
        };
        let mut out = [0; 4];
        assert_eq!(r.poll(&mut p, 14, &mut out), Err(expected));
        assert_eq!(r.retained(), 1);
        assert!(p.retained);
        let before = p.trace.clone();
        p.complete(0);
        p.cq[16..18].copy_from_slice(&2u16.to_le_bytes());
        assert_eq!(r.poll(&mut p, 15, &mut out), Err(expected));
        assert_eq!(r.submit(&mut p, 0x3000, 4, 15, 10), Err(expected));
        assert_eq!(p.trace, before);
    }
}
#[test]
fn integrated_reconstruction_still_does_not_prove_hardware_reset() {
    use musha_bce::registered::QueueKind::*;
    let mut p = Peer::default();
    let mut old = integrated();
    let a = old
        .register(&mut p, Completion, queue_config(true), 0, 100, 100)
        .unwrap();
    old.stop(&mut p);
    assert_eq!(
        old.poll_registration(&mut p, Completion, 1, Some((a, 0))),
        Err(Error::Stopped)
    );
    let mut fresh = integrated();
    let b = fresh
        .register(&mut p, Completion, queue_config(true), 0, 100, 100)
        .unwrap();
    assert_eq!(a, b);
    // Never inject a stale command reply into fresh metadata: no hardware epoch
    // exists on wire. No reset/reclaim API is offered by this wrapper either.
}

#[test]
fn integrated_registration_deadline_poll_budget_and_cross_queue_clock_are_terminal() {
    use musha_bce::{registered::QueueKind::*, registration::State};
    for case in 0..3 {
        let mut p = Peer::default();
        let mut r = integrated();
        let token = r
            .register(&mut p, Completion, queue_config(true), 2, 10, 1)
            .unwrap();
        let expected = match case {
            0 => {
                assert_eq!(
                    r.poll_registration(&mut p, Completion, 12, Some((token, 0))),
                    Err(Error::Timeout)
                );
                Error::Timeout
            }
            1 => {
                assert_eq!(r.poll_registration(&mut p, Completion, 3, None), Ok(false));
                assert_eq!(
                    r.poll_registration(&mut p, Completion, 4, Some((token, 0))),
                    Err(Error::PollLimit)
                );
                Error::PollLimit
            }
            _ => {
                r.poll_registration(&mut p, Completion, 3, Some((token, 0)))
                    .unwrap();
                assert_eq!(
                    r.register(&mut p, Submission, queue_config(false), 2, 10, 10),
                    Err(Error::ClockBackwards)
                );
                Error::ClockBackwards
            }
        };
        assert_eq!(
            r.states(),
            (State::Quarantined(expected), State::Quarantined(expected))
        );
        assert!(p.retained);
        let before = p.trace.clone();
        assert_eq!(r.submit(&mut p, 0x3000, 4, 20, 10), Err(expected));
        assert_eq!(p.trace, before);
    }
}
