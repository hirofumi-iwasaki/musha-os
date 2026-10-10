// Copyright (c) 2026 Hirofumi Iwasaki
// SPDX-License-Identifier: BSD-2-Clause
//! Bounded host ownership model for one SQ. Completions may arrive out of order;
//! reclamation stays in submission order. A hardware adapter must copy/ack CQ
//! entries exactly once and retain DMA allocations for any quarantined queue.
use crate::{
    Error,
    wire::{Completion, PENDING},
};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Ticket {
    pub index: u16,
    pub sequence: u64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Status {
    Free,
    InFlight,
    Complete,
}
#[derive(Clone, Copy)]
struct Slot {
    status: Status,
    sequence: u64,
    length: u64,
    deadline: u64,
    polls: u32,
}
const EMPTY: Slot = Slot {
    status: Status::Free,
    sequence: 0,
    length: 0,
    deadline: 0,
    polls: 0,
};
pub struct Queue<const N: usize> {
    qid: u16,
    slots: [Slot; N],
    head: usize,
    tail: usize,
    count: usize,
    sequence: u64,
    last: u64,
    max_polls: u32,
    fault: Option<Error>,
}
impl<const N: usize> Queue<N> {
    pub fn new(qid: u16, now: u64, max_polls: u32) -> Result<Self, Error> {
        if !(2..=256).contains(&N) || !(1..256).contains(&qid) || max_polls == 0 {
            return Err(Error::Invalid);
        }
        Ok(Self {
            qid,
            slots: [EMPTY; N],
            head: 0,
            tail: 0,
            count: 0,
            sequence: 0,
            last: now,
            max_polls,
            fault: None,
        })
    }
    pub fn fault(&self) -> Option<Error> {
        self.fault
    }
    /// Includes completed but unretired entries. On a fault none may be reused.
    pub fn retained(&self) -> usize {
        self.count
    }
    fn fail<T>(&mut self, e: Error) -> Result<T, Error> {
        self.fault = Some(e);
        Err(e)
    }
    pub fn poll(&mut self, now: u64) -> Result<(), Error> {
        if let Some(e) = self.fault {
            return Err(e);
        }
        if now < self.last {
            return self.fail(Error::ClockBackwards);
        }
        self.last = now;
        for i in 0..N {
            let s = &mut self.slots[i];
            if s.status != Status::InFlight {
                continue;
            }
            if now >= s.deadline {
                return self.fail(Error::Timeout);
            }
            if s.polls == 0 {
                return self.fail(Error::PollLimit);
            }
            s.polls -= 1;
        }
        Ok(())
    }
    /// Reserves ownership before a future adapter publishes the descriptor.
    /// Publication failure must stop/quarantine, never silently roll back.
    pub fn submit(&mut self, length: u64, now: u64, timeout: u64) -> Result<Ticket, Error> {
        self.poll(now)?;
        if length == 0 || timeout == 0 {
            return Err(Error::Invalid);
        }
        let deadline = now.checked_add(timeout).ok_or(Error::Invalid)?;
        if self.count == N - 1 {
            return Err(Error::QueueFull);
        }
        let Some(next) = self.sequence.checked_add(1) else {
            return self.fail(Error::SequenceOverflow);
        };
        if self.slots[self.tail].status != Status::Free {
            return self.fail(Error::Slot);
        }
        let t = Ticket {
            index: self.tail as u16,
            sequence: next,
        };
        self.sequence = next;
        self.slots[self.tail] = Slot {
            status: Status::InFlight,
            sequence: next,
            length,
            deadline,
            polls: self.max_polls,
        };
        self.tail = (self.tail + 1) % N;
        self.count += 1;
        Ok(t)
    }
    /// Returns the *local* ticket; BCE completion entries contain no generation.
    /// An old replay after index reuse is indistinguishable on the wire.
    pub fn complete(&mut self, c: Completion, now: u64) -> Result<Option<Ticket>, Error> {
        self.poll(now)?;
        if c.flags & PENDING == 0 {
            if c.flags != 0 {
                return self.fail(Error::Flags);
            }
            return Ok(None); // not published; all other fields may be stale
        }
        if c.flags != PENDING {
            return self.fail(Error::Flags);
        }
        if c.qid != self.qid {
            return self.fail(Error::QueueId);
        }
        if c.index as usize >= N {
            return self.fail(Error::Slot);
        }
        let s = &self.slots[c.index as usize];
        if s.status != Status::InFlight {
            return self.fail(Error::Duplicate);
        }
        if c.status != 0 {
            return self.fail(Error::Status);
        }
        if c.data_size > s.length {
            return self.fail(Error::Length);
        }
        let ticket = Ticket {
            index: c.index,
            sequence: s.sequence,
        };
        self.slots[c.index as usize].status = Status::Complete;
        Ok(Some(ticket))
    }
    /// CPU copy/processing must precede retire. Only successful completion grants
    /// a reusable slot; stopping and timeout never grant buffer reclamation.
    pub fn retire(&mut self, t: Ticket) -> Result<(), Error> {
        if let Some(e) = self.fault {
            return Err(e);
        }
        if t.index as usize != self.head
            || self.count == 0
            || self.slots[self.head].sequence != t.sequence
        {
            return Err(Error::Ticket);
        }
        if self.slots[self.head].status != Status::Complete {
            return Err(Error::NotComplete);
        }
        self.slots[self.head] = EMPTY;
        self.head = (self.head + 1) % N;
        self.count -= 1;
        Ok(())
    }
    pub fn stop(&mut self) {
        if self.fault.is_none() {
            self.fault = Some(Error::Stopped);
        }
    }
}

/// CQ acknowledgement plan. The adapter clears `slot`, synchronizes that write,
/// and rings the consumer doorbell with `next` before calling `acknowledged`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Ack {
    pub slot: u16,
    pub next: u16,
}
/// One-entry-at-a-time CQ cursor. It owns metadata only, never mapped DMA memory.
pub struct CompletionCursor<const N: usize> {
    index: usize,
    pending: Option<Ack>,
    fault: Option<Error>,
}
impl<const N: usize> CompletionCursor<N> {
    pub fn new() -> Result<Self, Error> {
        if !(2..=256).contains(&N) {
            return Err(Error::Invalid);
        }
        Ok(Self {
            index: 0,
            pending: None,
            fault: None,
        })
    }
    pub fn index(&self) -> u16 {
        self.index as u16
    }
    fn fail<T>(&mut self, error: Error) -> Result<T, Error> {
        self.fault = Some(error);
        Err(error)
    }
    /// Supply a coherent, CPU-owned copy of the current CQ entry only.
    pub fn offer(&mut self, index: u16, bytes: &[u8]) -> Result<Option<(Completion, Ack)>, Error> {
        if let Some(error) = self.fault {
            return Err(error);
        }
        if self.pending.is_some() {
            return Err(Error::Busy);
        }
        if index as usize != self.index {
            return self.fail(Error::Slot);
        }
        let completion = match Completion::decode(bytes) {
            Ok(c) => c,
            Err(e) => return self.fail(e),
        };
        if completion.flags == 0 {
            return Ok(None);
        }
        if completion.flags != PENDING {
            return self.fail(Error::Flags);
        }
        let ack = Ack {
            slot: index,
            next: ((self.index + 1) % N) as u16,
        };
        self.pending = Some(ack);
        Ok(Some((completion, ack)))
    }
    /// Metadata advance after hardware acknowledgement succeeds. If ack or SQ
    /// validation fails, stop both models and retain all associated allocations.
    pub fn acknowledged(&mut self, ack: Ack) -> Result<(), Error> {
        if let Some(error) = self.fault {
            return Err(error);
        }
        if self.pending != Some(ack) {
            return self.fail(Error::Ticket);
        }
        self.index = ack.next as usize;
        self.pending = None;
        Ok(())
    }
    pub fn stop(&mut self) {
        if self.fault.is_none() {
            self.fault = Some(Error::Stopped);
        }
    }
}
