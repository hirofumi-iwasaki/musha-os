// Copyright 2026 Hirofumi Iwasaki
// SPDX-License-Identifier: Apache-2.0
//! A bounded, single-command scan bootstrap; no radio/USB side effects.
use super::{Error, Event, word};
const OPCODES: [u16; 5] = [0x0c03, 0x0c01, 0x2001, 0x200b, 0x200c];
const PARAMETERS: [&[u8]; 5] = [
    &[],
    &[0, 0xe0, 0, 0, 0, 0, 0, 0x20], // Command Complete/Status, Hardware Error, LE Meta.
    &[2, 0, 0, 0, 0, 0, 0, 0],       // LE Advertising Report only.
    &[0, 0x10, 0, 0x10, 0, 0, 0],    // Passive, 10ms interval/window, public own address, all.
    &[1, 1],                         // Enable legacy scan; filter duplicates.
];
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    Initializing,
    Scanning,
    Failed,
    Detached,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Command {
    pub bytes: [u8; 11],
    pub length: usize,
}
pub struct Scan {
    generation: u64,
    step: usize,
    credits: u8,
    pending: Option<u64>,
    state: State,
    last_now: u64,
    startup_deadline: Option<u64>,
}
impl Scan {
    pub fn new(generation: u64) -> Self {
        Self {
            generation,
            step: 0,
            credits: 1,
            pending: None,
            state: State::Initializing,
            last_now: 0,
            startup_deadline: None,
        }
    }
    pub fn state(&self) -> State {
        self.state
    }
    fn check(&mut self, generation: u64, now: u64) -> Result<(), Error> {
        if generation != self.generation || self.state == State::Detached {
            return Err(Error::Stale);
        }
        if self.state == State::Failed {
            return Err(Error::Invalid);
        }
        if now < self.last_now {
            self.state = State::Failed;
            return Err(Error::Invalid);
        }
        self.last_now = now;
        if self.startup_deadline.is_none() {
            self.startup_deadline = Some(now.checked_add(5000).ok_or(Error::Invalid)?);
        }
        if self.state == State::Initializing && self.startup_deadline.is_some_and(|d| now >= d) {
            self.state = State::Failed;
            return Err(Error::Timeout);
        }
        if self.pending.is_some_and(|deadline| now >= deadline) {
            self.state = State::Failed;
            return Err(Error::Timeout);
        }
        Ok(())
    }
    /// Caller must treat submission failure as disconnect/failure; never retry an
    /// ambiguously submitted command. No second command is emitted until Complete.
    pub fn next(&mut self, generation: u64, now: u64) -> Result<Option<Command>, Error> {
        self.check(generation, now)?;
        if self.state == State::Scanning {
            return Ok(None);
        }
        if self.pending.is_some() || self.credits == 0 {
            return Err(Error::Busy);
        }
        let deadline = now.checked_add(1000).ok_or(Error::Invalid)?;
        let mut command = Command {
            bytes: [0; 11],
            length: 3 + PARAMETERS[self.step].len(),
        };
        command.bytes[..2].copy_from_slice(&OPCODES[self.step].to_le_bytes());
        command.bytes[2] = PARAMETERS[self.step].len() as u8;
        command.bytes[3..command.length].copy_from_slice(PARAMETERS[self.step]);
        self.pending = Some(deadline);
        self.credits -= 1;
        Ok(Some(command))
    }
    pub fn event(&mut self, generation: u64, now: u64, bytes: &[u8]) -> Result<(), Error> {
        self.check(generation, now)?;
        let event = Event::parse(bytes)?;
        if event.code == 0x10 {
            if event.parameters.len() != 1 {
                return Err(Error::Length);
            }
            self.state = State::Failed;
            return Err(Error::Controller(event.parameters[0]));
        }
        if event.code == 0x0f {
            if event.parameters.len() != 4 {
                return Err(Error::Length);
            }
            if word(&event.parameters[2..]) == 0 && event.parameters[0] == 0 {
                self.credits = event.parameters[1];
                return Ok(());
            }
            if self.pending.is_none() || word(&event.parameters[2..]) != OPCODES[self.step] {
                return Err(Error::Stale);
            }
            self.credits = event.parameters[1];
            // These synchronous commands require Command Complete, not Status.
            self.state = State::Failed;
            return Err(Error::Controller(event.parameters[0]));
        }
        if event.code != 0x0e {
            return Ok(());
        }
        let (credits, opcode, result) = event.complete()?;
        if opcode == 0 && result.is_empty() {
            self.credits = credits;
            return Ok(());
        }
        if self.pending.is_none() || opcode != OPCODES[self.step] {
            return Err(Error::Stale);
        }
        if result.len() != 1 {
            return Err(Error::Length);
        }
        self.credits = credits;
        self.pending = None;
        if result[0] != 0 {
            self.state = State::Failed;
            return Err(Error::Controller(result[0]));
        }
        self.step += 1;
        if self.step == OPCODES.len() {
            self.state = State::Scanning;
        }
        Ok(())
    }
    /// Invalidates late completions. It does not confirm hardware/DMA shutdown.
    pub fn detach(&mut self) {
        self.pending = None;
        self.state = State::Detached;
    }
}
