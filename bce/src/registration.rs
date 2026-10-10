// Copyright (c) 2026 Abdelkader Boudih <freebsd@seuros.com>
// Copyright (c) 2026 Hirofumi Iwasaki (Rust adaptation and lifecycle model)
// SPDX-License-Identifier: BSD-2-Clause
//! One user queue on an already established command transport. No device I/O.
//! Local tokens correlate validated command completions, not firmware fields.
//! Neither flush nor unregister proves DMA quiescence or authorizes reclamation.
use crate::{Error, wire};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Operation {
    Register,
    Flush,
    Unregister,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    New,
    Waiting(Operation),
    Registered,
    Flushed,
    Unregistered,
    Quarantined(Error),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Token(u16, u64);
pub struct Request {
    pub token: Token,
    pub bytes: [u8; 64],
}
pub struct Registration {
    qid: u16,
    state: State,
    sequence: u64,
    pending: Option<Token>,
    transfer: Option<Token>,
    last: u64,
    deadline: u64,
    polls: u32,
}
impl Registration {
    pub fn new(qid: u16) -> Result<Self, Error> {
        if !(2..256).contains(&qid) {
            return Err(Error::QueueId);
        }
        Ok(Self {
            qid,
            state: State::New,
            sequence: 0,
            pending: None,
            transfer: None,
            last: 0,
            deadline: 0,
            polls: 0,
        })
    }
    pub fn state(&self) -> State {
        self.state
    }
    fn healthy(&self) -> Result<(), Error> {
        if let State::Quarantined(e) = self.state {
            Err(e)
        } else {
            Ok(())
        }
    }
    fn fail<T>(&mut self, e: Error) -> Result<T, Error> {
        if !matches!(self.state, State::Quarantined(_)) {
            self.state = State::Quarantined(e);
        }
        Err(match self.state {
            State::Quarantined(e) => e,
            _ => unreachable!(),
        })
    }
    fn token(&mut self) -> Result<Token, Error> {
        self.sequence = match self.sequence.checked_add(1) {
            Some(n) => n,
            None => return self.fail(Error::SequenceOverflow),
        };
        Ok(Token(self.qid, self.sequence))
    }
    fn start(
        &mut self,
        op: Operation,
        bytes: [u8; 64],
        now: u64,
        timeout: u64,
        polls: u32,
    ) -> Result<Request, Error> {
        if now < self.last {
            return self.fail(Error::ClockBackwards);
        }
        if timeout == 0 || polls == 0 {
            return Err(Error::Invalid);
        }
        let deadline = now.checked_add(timeout).ok_or(Error::Invalid)?;
        let token = self.token()?;
        self.last = now;
        self.deadline = deadline;
        self.polls = polls;
        self.pending = Some(token);
        self.state = State::Waiting(op);
        Ok(Request { token, bytes })
    }
    /// Caller must own this QID and prevalidate its memory/CQ dependency.
    /// The returned command must be published once; publication failure is terminal.
    pub fn register(
        &mut self,
        config: wire::RegistrationConfig<'_>,
        now: u64,
        timeout: u64,
        polls: u32,
    ) -> Result<Request, Error> {
        self.healthy()?;
        if self.state != State::New {
            return Err(Error::Busy);
        }
        if config.qid != self.qid {
            return Err(Error::QueueId);
        }
        let bytes = wire::register_queue(config)?;
        self.start(Operation::Register, bytes, now, timeout, polls)
    }
    /// Conservative subset: no flush/cancel of an outstanding host transfer.
    pub fn command(
        &mut self,
        op: Operation,
        now: u64,
        timeout: u64,
        polls: u32,
    ) -> Result<Request, Error> {
        self.healthy()?;
        if self.transfer.is_some() {
            return Err(Error::Busy);
        }
        let valid = match op {
            Operation::Flush => matches!(self.state, State::Registered | State::Flushed),
            Operation::Unregister => self.state == State::Flushed,
            Operation::Register => false,
        };
        if !valid {
            return Err(Error::Busy);
        }
        let bytes = wire::simple_queue_command(self.qid, op == Operation::Flush)?;
        self.start(op, bytes, now, timeout, polls)
    }
    /// Reply token comes from the command transport's pending-ticket mapping,
    /// after CQ validation/ack. Firmware does not echo this local token.
    pub fn poll(&mut self, now: u64, reply: Option<(Token, u16)>) -> Result<bool, Error> {
        self.healthy()?;
        let State::Waiting(op) = self.state else {
            return self.fail(Error::UnexpectedReply);
        };
        if now < self.last {
            return self.fail(Error::ClockBackwards);
        }
        self.last = now;
        if now >= self.deadline {
            return self.fail(Error::Timeout);
        }
        if self.polls == 0 {
            return self.fail(Error::PollLimit);
        }
        self.polls -= 1;
        let Some((token, status)) = reply else {
            return Ok(false);
        };
        if self.pending != Some(token) {
            return self.fail(Error::Ticket);
        }
        if status != 0 {
            return self.fail(Error::Status);
        }
        self.pending = None;
        self.state = match op {
            Operation::Register => State::Registered,
            Operation::Flush => State::Flushed,
            Operation::Unregister => State::Unregistered,
        };
        Ok(true)
    }
    /// Gate a single transfer before handing work to an existing Transfer adapter.
    /// Integration must use this gate; this module does not change Transfer::new.
    pub fn begin_transfer(&mut self) -> Result<Token, Error> {
        self.healthy()?;
        if !matches!(self.state, State::Registered | State::Flushed) || self.transfer.is_some() {
            return Err(Error::Busy);
        }
        let token = self.token()?;
        self.transfer = Some(token);
        self.state = State::Registered;
        Ok(token)
    }
    /// Only after successful transfer completion and CQ acknowledgement.
    pub fn finish_transfer(&mut self, token: Token) -> Result<(), Error> {
        self.healthy()?;
        if self.transfer != Some(token) {
            return self.fail(Error::Ticket);
        }
        self.transfer = None;
        Ok(())
    }
    /// Partial command publication, transport/transfer failure, or shutdown.
    /// Sticky quarantine: no retry, buffer release, QID reuse or reset permission.
    pub fn quarantine(&mut self, error: Error) {
        let _: Result<(), Error> = self.fail(error);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn sequence_exhaustion_never_reuses_a_token() {
        let mut r = Registration::new(2).unwrap();
        r.state = State::Registered;
        r.sequence = u64::MAX;
        assert_eq!(r.begin_transfer(), Err(Error::SequenceOverflow));
        assert_eq!(r.state(), State::Quarantined(Error::SequenceOverflow));
    }
}
