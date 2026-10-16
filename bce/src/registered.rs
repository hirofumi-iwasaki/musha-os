// Copyright (c) 2026 Hirofumi Iwasaki
// SPDX-License-Identifier: BSD-2-Clause
//! Host-only composition of existing queue/registration/extent models.
//! No MMIO, physical allocation/coherency proof or hardware reset authority.
//! A command transport must already be established. Reply tokens are assigned
//! after its validated/acknowledged command completion, never decoded from wire.
use crate::{
    Error,
    adapter::{Memory, Transfer},
    registration::{Registration, Request, State, Token},
    wire,
};

pub trait Transport: Memory {
    /// May partially publish before failure; caller will retain all backing.
    fn publish_command(&mut self, request: &Request) -> Result<(), Error>;
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QueueKind {
    Completion,
    Submission,
}
#[derive(Clone, Copy)]
struct Extent {
    start: u64,
    end: u64,
}
impl Extent {
    fn new(start: u64, length: u64) -> Result<Self, Error> {
        Ok(Self {
            start,
            end: start.checked_add(length).ok_or(Error::Invalid)?,
        })
    }
    fn overlaps(self, other: Self) -> bool {
        self.start < other.end && other.start < self.end
    }
}
/// Owns CQ-before-SQ admission, declared extents, transfer tokens and sticky
/// quarantine together. Low-level `Transfer` remains a separate model API.
///
/// Fresh construction is NOT device reset: same QIDs restart local tokens.
/// A future hardware caller must prove old device ownership ended and drain all
/// previous replies before any reconstruction. This API grants no reclaim/reuse.
pub struct RegisteredTransfer<const N: usize> {
    sq: Registration,
    cq: Registration,
    transfer: Transfer<N>,
    region: wire::ContiguousRegion,
    sqid: u16,
    cqid: u16,
    sq_extent: Option<Extent>,
    cq_extent: Option<Extent>,
    pending: Option<Token>,
    fault: Option<Error>,
    last: u64,
}
impl<const N: usize> RegisteredTransfer<N> {
    pub fn new(
        sqid: u16,
        cqid: u16,
        region: wire::ContiguousRegion,
        now: u64,
        polls: u32,
    ) -> Result<Self, Error> {
        if sqid == cqid {
            return Err(Error::QueueId);
        }
        Ok(Self {
            sq: Registration::new(sqid)?,
            cq: Registration::new(cqid)?,
            transfer: Transfer::new(sqid, cqid, now, polls)?,
            region,
            sqid,
            cqid,
            sq_extent: None,
            cq_extent: None,
            pending: None,
            fault: None,
            last: now,
        })
    }
    pub fn states(&self) -> (State, State) {
        (self.cq.state(), self.sq.state())
    }
    pub fn retained(&self) -> usize {
        self.transfer.retained()
    }
    fn healthy(&self) -> Result<(), Error> {
        self.fault.map_or(Ok(()), Err)
    }
    fn fail<T>(&mut self, io: &mut impl Memory, error: Error) -> Result<T, Error> {
        let first = *self.fault.get_or_insert(error);
        self.cq.quarantine(first);
        self.sq.quarantine(first);
        self.transfer.stop(io);
        Err(first)
    }
    fn clock(&mut self, io: &mut impl Memory, now: u64) -> Result<(), Error> {
        self.healthy()?;
        if now < self.last {
            return self.fail(io, Error::ClockBackwards);
        }
        self.last = now;
        Ok(())
    }
    /// Full extent/direction/count/stride checks precede command publication.
    pub fn register(
        &mut self,
        io: &mut impl Transport,
        kind: QueueKind,
        config: wire::RegistrationConfig<'_>,
        now: u64,
        timeout: u64,
        polls: u32,
    ) -> Result<Token, Error> {
        self.clock(io, now)?;
        let (qid, stride, out, other) = match kind {
            QueueKind::Completion => (self.cqid, 24, false, self.sq_extent),
            QueueKind::Submission => {
                if self.cq.state() != State::Registered {
                    return Err(Error::Busy);
                }
                (self.sqid, 32, true, self.cq_extent)
            }
        };
        if config.qid != qid {
            return Err(Error::QueueId);
        }
        if usize::from(config.count) != N
            || config.stride != stride
            || config.out != out
            || (out && config.vector_or_cq != self.cqid)
        {
            return Err(Error::Invalid);
        }
        self.region.register_queue(config)?;
        let extent = Extent::new(config.address, u64::from(config.count) * config.stride)?;
        if other.is_some_and(|r| r.overlaps(extent)) {
            return Err(Error::Invalid);
        }
        let request = match kind {
            QueueKind::Completion => self.cq.register(config, now, timeout, polls)?,
            QueueKind::Submission => self.sq.register(config, now, timeout, polls)?,
        };
        match kind {
            QueueKind::Completion => self.cq_extent = Some(extent),
            QueueKind::Submission => self.sq_extent = Some(extent),
        }
        if let Err(error) = io.publish_command(&request) {
            return self.fail(io, error);
        }
        Ok(request.token)
    }
    pub fn poll_registration(
        &mut self,
        io: &mut impl Memory,
        kind: QueueKind,
        now: u64,
        reply: Option<(Token, u16)>,
    ) -> Result<bool, Error> {
        self.clock(io, now)?;
        let result = match kind {
            QueueKind::Completion => self.cq.poll(now, reply),
            QueueKind::Submission => self.sq.poll(now, reply),
        };
        match result {
            Ok(done) => Ok(done),
            Err(error) => self.fail(io, error),
        }
    }
    pub fn submit(
        &mut self,
        io: &mut impl Memory,
        address: u64,
        length: u64,
        now: u64,
        timeout: u64,
    ) -> Result<(), Error> {
        self.clock(io, now)?;
        // No ownership token or I/O before bounds, ring exclusion and both acks.
        self.region.submission(address, length)?;
        let extent = Extent::new(address, length)?;
        if [self.cq_extent, self.sq_extent]
            .into_iter()
            .flatten()
            .any(|r| r.overlaps(extent))
        {
            return Err(Error::Invalid);
        }
        if self.cq.state() != State::Registered {
            return Err(Error::Busy);
        }
        let token = match self.sq.begin_transfer() {
            Ok(token) => token,
            Err(error) if matches!(self.sq.state(), State::Quarantined(_)) => {
                return self.fail(io, error);
            }
            Err(error) => return Err(error),
        };
        self.pending = Some(token);
        match self
            .transfer
            .submit_in_region(io, &self.region, address, length, now, timeout)
        {
            Ok(()) => Ok(()),
            Err(error) => self.fail(io, error),
        }
    }
    pub fn poll(
        &mut self,
        io: &mut impl Memory,
        now: u64,
        out: &mut [u8],
    ) -> Result<Option<usize>, Error> {
        self.clock(io, now)?;
        let token = self.pending.ok_or(Error::Busy)?;
        match self.transfer.poll(io, now, out) {
            Ok(None) => Ok(None),
            Ok(Some(length)) => {
                if let Err(error) = self.sq.finish_transfer(token) {
                    return self.fail(io, error);
                }
                self.pending = None;
                Ok(Some(length))
            }
            Err(error) => self.fail(io, error),
        }
    }
    /// Terminal model stop, not physical drain. All queues/backing stay retained.
    pub fn stop(&mut self, io: &mut impl Memory) {
        let _: Result<(), Error> = self.fail(io, Error::Stopped);
    }
}
