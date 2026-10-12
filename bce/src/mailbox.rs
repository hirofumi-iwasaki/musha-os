// Copyright (c) 2026 Hirofumi Iwasaki
// SPDX-License-Identifier: BSD-2-Clause
//! One-shot protocol handshake. Failure is terminal: no same-channel retry.
use crate::{
    Error,
    wire::{Message, PROTOCOL_VERSION, SET_PROTOCOL},
};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    Idle,
    Waiting,
    Complete,
    Quarantined(Error),
}
pub struct Handshake {
    state: State,
    last: u64,
    deadline: u64,
    polls: u32,
}
impl Default for Handshake {
    fn default() -> Self {
        Self {
            state: State::Idle,
            last: 0,
            deadline: 0,
            polls: 0,
        }
    }
}
impl Handshake {
    pub fn state(&self) -> State {
        self.state
    }
    pub fn start(&mut self, now: u64, timeout: u64, max_polls: u32) -> Result<Message, Error> {
        if self.state != State::Idle {
            return Err(Error::Busy);
        }
        if timeout == 0 || max_polls == 0 {
            return Err(Error::Invalid);
        }
        let deadline = now.checked_add(timeout).ok_or(Error::Invalid)?;
        self.last = now;
        self.deadline = deadline;
        self.polls = max_polls;
        self.state = State::Waiting;
        Message::new(SET_PROTOCOL, PROTOCOL_VERSION)
    }
    fn fail<T>(&mut self, error: Error) -> Result<T, Error> {
        self.state = State::Quarantined(error);
        Err(error)
    }
    /// One observation, never a wait. Reply must be the sole owned mailbox reply.
    pub fn poll(&mut self, now: u64, reply: Option<Message>) -> Result<bool, Error> {
        if let State::Quarantined(error) = self.state {
            return Err(error);
        }
        if self.state != State::Waiting {
            return self.fail(Error::UnexpectedReply);
        }
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
        if let Some(reply) = reply {
            if reply.kind() != SET_PROTOCOL || reply.value() != PROTOCOL_VERSION {
                return self.fail(Error::UnexpectedReply);
            }
            self.state = State::Complete;
            return Ok(true);
        }
        Ok(false)
    }
    pub fn stop(&mut self) {
        if !matches!(self.state, State::Quarantined(_)) {
            self.state = State::Quarantined(Error::Stopped);
        }
    }
}
