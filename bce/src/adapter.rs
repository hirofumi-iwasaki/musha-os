// Copyright (c) 2026 Abdelkader Boudih <freebsd@seuros.com>
// Copyright (c) 2026 Hirofumi Iwasaki (Rust model adaptation)
// SPDX-License-Identifier: BSD-2-Clause
//! I/O orchestration for scripted peers only. No real MMIO/DMA implementation.
//! A real adapter requires separate resource, coherency and quiescence evidence.
use crate::{
    Error,
    mailbox::{Handshake, State},
    queue::{CompletionCursor, Queue, Ticket},
    wire::{self, Message},
};
pub const REPLY_COUNT: u32 = 0x108;
pub const REPLY: u32 = 0x810;
pub const OUT: u32 = 0x820;
pub const TIME: u32 = 0xc000;
pub const DOORBELL: u32 = 0x44000;
/// Offsets are BAR4-relative. Each method must be bounded/nonblocking.
pub trait Registers {
    fn read(&mut self, offset: u32) -> Result<u32, Error>;
    fn write(&mut self, offset: u32, value: u32) -> Result<(), Error>;
    fn fence(&mut self) -> Result<(), Error>;
}
/// Timestamp and one-shot protocol handshake. A fault prevents all future polls.
/// Stop is best-effort notification, never proof of device/DMA quiescence.
pub struct Mailbox {
    handshake: Handshake,
    started: bool,
    last: u64,
    next_time: u64,
    fault: Option<Error>,
    cleanup_failed: bool,
}
impl Default for Mailbox {
    fn default() -> Self {
        Self {
            handshake: Handshake::default(),
            started: false,
            last: 0,
            next_time: 0,
            fault: None,
            cleanup_failed: false,
        }
    }
}
impl Mailbox {
    pub fn ready(&self) -> bool {
        self.fault.is_none() && self.handshake.state() == State::Complete
    }
    pub fn cleanup_failed(&self) -> bool {
        self.cleanup_failed
    }
    fn stop_time(&mut self, io: &mut impl Registers) {
        if self.started {
            self.started = false;
            // Mark stopped before I/O, including when either write fails.
            self.cleanup_failed = io
                .write(TIME + 8, 0xffff_fffe)
                .and_then(|_| io.write(TIME, u32::MAX))
                .is_err();
        }
    }
    fn fail<T>(&mut self, io: &mut impl Registers, e: Error) -> Result<T, Error> {
        if self.fault.is_none() {
            self.fault = Some(e);
            self.handshake.stop();
            self.stop_time(io);
        }
        Err(self.fault.unwrap())
    }
    pub fn start(
        &mut self,
        io: &mut impl Registers,
        now_ms: u64,
        timeout: u64,
        polls: u32,
    ) -> Result<(), Error> {
        if let Some(e) = self.fault {
            return Err(e);
        }
        if self.handshake.state() != State::Idle {
            return Err(Error::Busy);
        }
        let next = now_ms.checked_add(150).ok_or(Error::Invalid)?;
        now_ms.checked_mul(1_000_000).ok_or(Error::Invalid)?;
        let message = self.handshake.start(now_ms, timeout, polls)?;
        self.last = now_ms;
        self.next_time = next;
        let result = (|| {
            // Never consume an old response as the answer to this new request.
            if (io.read(REPLY_COUNT)? >> 20) & 15 != 0 {
                return Err(Error::UnexpectedReply);
            }
            io.read(TIME)?;
            io.fence()?;
            self.started = true; // partial publication also needs stop notification
            io.write(TIME + 8, 0xffff_fffc)?;
            io.write(TIME, u32::MAX)?;
            for (i, w) in message.words().iter().enumerate() {
                io.write(OUT + i as u32 * 4, *w)?;
            }
            Ok(())
        })();
        match result {
            Ok(()) => Ok(()),
            Err(e) => self.fail(io, e),
        }
    }
    /// At most one reply and one timestamp update per call, including while waiting.
    pub fn poll(&mut self, io: &mut impl Registers, now_ms: u64) -> Result<bool, Error> {
        if let Some(e) = self.fault {
            return Err(e);
        }
        if !self.started {
            return Err(Error::Invalid);
        }
        let result = (|| {
            if now_ms < self.last {
                return Err(Error::ClockBackwards);
            }
            self.last = now_ms;
            // Do not write a time value if conversion or rescheduling would wrap.
            let ns = now_ms.checked_mul(1_000_000).ok_or(Error::Invalid)?;
            if now_ms >= self.next_time {
                let next = now_ms.checked_add(150).ok_or(Error::Invalid)?;
                io.read(TIME + 8)?;
                io.fence()?;
                io.write(TIME + 8, ns as u32)?;
                io.write(TIME, (ns >> 32) as u32)?;
                self.next_time = next;
            }
            let count = (io.read(REPLY_COUNT)? >> 20) & 15;
            if count > 1 {
                return Err(Error::UnexpectedReply);
            }
            let reply = if count == 1 {
                let lo = io.read(REPLY)?;
                let hi = io.read(REPLY + 4)?;
                io.read(REPLY + 8)?;
                io.read(REPLY + 12)?;
                Some(Message::from_raw(lo as u64 | ((hi as u64) << 32)))
            } else {
                None
            };
            if self.handshake.state() == State::Complete {
                if reply.is_some() {
                    return Err(Error::UnexpectedReply);
                }
                return Ok(true);
            }
            self.handshake.poll(now_ms, reply)
        })();
        match result {
            Ok(v) => Ok(v),
            Err(e) => self.fail(io, e),
        }
    }
    pub fn stop(&mut self, io: &mut impl Registers) {
        let _: Result<(), _> = self.fail(io, Error::Stopped);
    }
}

/// CPU-owned simulated DMA memory. A future real implementation must validate
/// addresses and retain actual allocations on fault/drop; this trait cannot do so.
pub trait Memory {
    fn write_submission(&mut self, slot: u16, bytes: [u8; 32]) -> Result<(), Error>;
    fn sync_to_device(&mut self) -> Result<(), Error>;
    fn doorbell(&mut self, offset: u32, index: u16) -> Result<(), Error>;
    fn sync_from_device(&mut self) -> Result<(), Error>;
    fn completion(&mut self, slot: u16) -> Result<[u8; 24], Error>;
    fn copy_payload(&mut self, slot: u16, out: &mut [u8]) -> Result<(), Error>;
    fn clear_completion(&mut self, slot: u16) -> Result<(), Error>;
    /// Terminal quarantine: retain all buffers; never interpret as DMA shutdown.
    fn retain(&mut self);
}
/// Simulated already-registered SQ/CQ, limited to one request at a time.
/// Does NOT register firmware queues; independent from mailbox readiness.
pub struct Transfer<const N: usize> {
    sq: Queue<N>,
    cq: CompletionCursor<N>,
    sqid: u16,
    cqid: u16,
    pending: Option<Ticket>,
    fault: Option<Error>,
}
impl<const N: usize> Transfer<N> {
    pub fn new(sqid: u16, cqid: u16, now: u64, polls: u32) -> Result<Self, Error> {
        if cqid >= 256 || cqid == sqid {
            return Err(Error::Invalid);
        }
        Ok(Self {
            sq: Queue::new(sqid, now, polls)?,
            cq: CompletionCursor::new()?,
            sqid,
            cqid,
            pending: None,
            fault: None,
        })
    }
    pub fn retained(&self) -> usize {
        self.sq.retained()
    }
    fn fail<T>(&mut self, io: &mut impl Memory, e: Error) -> Result<T, Error> {
        if self.fault.is_none() {
            self.fault = Some(e);
            self.sq.stop();
            self.cq.stop();
            io.retain();
        }
        Err(self.fault.unwrap())
    }
    pub fn submit(
        &mut self,
        io: &mut impl Memory,
        address: u64,
        length: u64,
        now: u64,
        timeout: u64,
    ) -> Result<(), Error> {
        if let Some(e) = self.fault {
            return Err(e);
        }
        if self.pending.is_some() {
            return Err(Error::Busy);
        }
        let descriptor = wire::submission(address, length)?;
        let ticket = match self.sq.submit(length, now, timeout) {
            Ok(t) => t,
            Err(e) => return self.fail(io, e),
        };
        self.pending = Some(ticket);
        let result = (|| {
            io.write_submission(ticket.index, descriptor)?;
            io.sync_to_device()?;
            io.doorbell(
                DOORBELL + self.sqid as u32 * 4,
                ((ticket.index as usize + 1) % N) as u16,
            )
        })();
        match result {
            Ok(()) => Ok(()),
            Err(e) => self.fail(io, e),
        }
    }
    /// Caller may use `out` only on Ok(Some(length)); error can leave copied bytes.
    pub fn poll(
        &mut self,
        io: &mut impl Memory,
        now: u64,
        out: &mut [u8],
    ) -> Result<Option<usize>, Error> {
        if let Some(e) = self.fault {
            return Err(e);
        }
        let result = (|| {
            self.sq.poll(now)?;
            io.sync_from_device()?;
            let index = self.cq.index();
            let bytes = io.completion(index)?;
            let Some((completion, ack)) = self.cq.offer(index, &bytes)? else {
                return Ok(None);
            };
            let ticket = self.sq.complete(completion, now)?.ok_or(Error::Invalid)?;
            if self.pending != Some(ticket) {
                return Err(Error::Ticket);
            }
            let len = usize::try_from(completion.data_size).map_err(|_| Error::Length)?;
            if len > out.len() {
                return Err(Error::Length);
            }
            io.copy_payload(ticket.index, &mut out[..len])?;
            io.clear_completion(ack.slot)?;
            io.sync_to_device()?;
            io.doorbell(DOORBELL + self.cqid as u32 * 4, ack.next)?;
            self.cq.acknowledged(ack)?;
            self.sq.retire(ticket)?;
            self.pending = None;
            Ok(Some(len))
        })();
        match result {
            Ok(v) => Ok(v),
            Err(e) => self.fail(io, e),
        }
    }
    pub fn stop(&mut self, io: &mut impl Memory) {
        let _: Result<(), _> = self.fail(io, Error::Stopped);
    }
}
