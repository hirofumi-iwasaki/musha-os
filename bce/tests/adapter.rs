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
