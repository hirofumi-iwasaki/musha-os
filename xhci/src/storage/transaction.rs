// Copyright 2026 Hirofumi Iwasaki
// SPDX-License-Identifier: Apache-2.0
//! One read-only BOT command, advanced without waiting or accessing DMA.
use super::{Command, cbw, csw};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Command,
    Data,
    Status,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Invalid,
    Timeout,
    Clock,
    Stalled,
    Transfer,
    Status,
    Canceled,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Progress {
    Submit(Phase, usize),
    Pending,
    Complete(bool),
}
#[derive(Clone, Copy)]
enum State {
    Submit(Phase),
    Waiting(Phase),
    Complete(bool),
    Failed(Error),
}
/// Failed transactions never permit another submit. Their transport buffers must
/// remain reserved until the controller has quiesced. A new command needs a new tag.
pub struct Transaction {
    state: State,
    tag: u32,
    bytes: usize,
    command: [u8; 31],
    deadline: u64,
    last: u64,
    polls: u32,
}
impl Transaction {
    pub fn new(tag: u32, command: Command, now: u64, timeout_ms: u64) -> Result<Self, Error> {
        let (command, bytes) = cbw(tag, command).ok_or(Error::Invalid)?;
        if timeout_ms == 0 {
            return Err(Error::Invalid);
        }
        let deadline = now.checked_add(timeout_ms).ok_or(Error::Clock)?;
        Ok(Self {
            state: State::Submit(Phase::Command),
            tag,
            bytes,
            command,
            deadline,
            last: now,
            polls: 0,
        })
    }
    pub fn command(&self) -> &[u8; 31] {
        &self.command
    }
    fn fail(&mut self, error: Error) -> Error {
        self.state = State::Failed(error);
        error
    }
    /// One observation, no loop. The timeout covers the whole BOT transaction.
    pub fn poll(&mut self, now: u64) -> Result<Progress, Error> {
        match self.state {
            State::Failed(e) => return Err(e),
            State::Complete(ok) => return Ok(Progress::Complete(ok)),
            _ => {}
        }
        if now < self.last {
            return Err(self.fail(Error::Clock));
        }
        self.last = now;
        if now >= self.deadline {
            return Err(self.fail(Error::Timeout));
        }
        self.polls += 1;
        if self.polls > 1_000_000 {
            return Err(self.fail(Error::Stalled));
        }
        match self.state {
            State::Submit(phase) => {
                self.state = State::Waiting(phase);
                Ok(Progress::Submit(
                    phase,
                    match phase {
                        Phase::Command => 31,
                        Phase::Data => self.bytes,
                        Phase::Status => 13,
                    },
                ))
            }
            _ => Ok(Progress::Pending),
        }
    }
    /// Called only for a matched successful transport event, with exact length.
    /// `status` is empty except for the 13-byte CSW. No phase is silently skipped.
    pub fn complete(&mut self, phase: Phase, bytes: usize, status: &[u8]) -> Result<(), Error> {
        if let State::Failed(error) = self.state {
            return Err(error);
        }
        let State::Waiting(expected) = self.state else {
            return Err(self.fail(Error::Transfer));
        };
        let length = match phase {
            Phase::Command => 31,
            Phase::Data => self.bytes,
            Phase::Status => 13,
        };
        if expected != phase || bytes != length || (phase != Phase::Status && !status.is_empty()) {
            return Err(self.fail(Error::Transfer));
        }
        self.state = match phase {
            Phase::Command => State::Submit(if self.bytes == 0 {
                Phase::Status
            } else {
                Phase::Data
            }),
            Phase::Data => State::Submit(Phase::Status),
            Phase::Status => {
                let Some(ok) = csw(status, self.tag) else {
                    return Err(self.fail(Error::Status));
                };
                State::Complete(ok)
            }
        };
        Ok(())
    }
    pub fn cancel(&mut self) {
        self.fail(Error::Canceled);
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn status(tag: u32, failed: bool) -> [u8; 13] {
        let mut b = [0; 13];
        b[..4].copy_from_slice(&0x53425355u32.to_le_bytes());
        b[4..8].copy_from_slice(&tag.to_le_bytes());
        b[12] = failed as u8;
        b
    }
    #[test]
    fn commands_advance_once_and_wait_without_resubmission() {
        for command in [
            Command::Ready,
            Command::Sense,
            Command::Capacity,
            Command::Read {
                lba: 1,
                sector_bytes: 512,
            },
            Command::Read {
                lba: 1,
                sector_bytes: 4096,
            },
        ] {
            for failed in [false, true] {
                let mut t = Transaction::new(7, command, 10, 2000).unwrap();
                let n = cbw(7, command).unwrap().1;
                let phases = if n == 0 {
                    &[(Phase::Command, 31), (Phase::Status, 13)][..]
                } else {
                    &[(Phase::Command, 31), (Phase::Data, n), (Phase::Status, 13)][..]
                };
                for &(phase, length) in phases {
                    assert_eq!(t.poll(10), Ok(Progress::Submit(phase, length)));
                    for _ in 0..100 {
                        assert_eq!(t.poll(10), Ok(Progress::Pending));
                    }
                    let reply = status(7, failed);
                    t.complete(
                        phase,
                        length,
                        if phase == Phase::Status { &reply } else { &[] },
                    )
                    .unwrap();
                }
                assert_eq!(t.poll(20), Ok(Progress::Complete(!failed)));
                assert_eq!(t.poll(u64::MAX), Ok(Progress::Complete(!failed)));
            }
        }
    }
    #[test]
    fn bad_order_lengths_and_csw_are_terminal() {
        for kind in 0..5 {
            let mut t = Transaction::new(1, Command::Ready, 0, 20).unwrap();
            t.poll(0).unwrap();
            if kind == 0 {
                assert_eq!(t.complete(Phase::Data, 0, &[]), Err(Error::Transfer));
            } else if kind == 1 {
                assert_eq!(t.complete(Phase::Command, 30, &[]), Err(Error::Transfer));
            } else {
                t.complete(Phase::Command, 31, &[]).unwrap();
                t.poll(0).unwrap();
                let mut b = status(1, false);
                match kind {
                    2 => b[4] = 2,
                    3 => b[8] = 1,
                    _ => b[12] = 2,
                }
                assert_eq!(t.complete(Phase::Status, 13, &b), Err(Error::Status));
            }
            assert!(t.poll(0).is_err());
            assert!(t.complete(Phase::Command, 31, &[]).is_err());
        }
    }
    #[test]
    fn deadline_clock_stall_cancel_and_overflow() {
        assert!(Transaction::new(0, Command::Ready, 0, 1).is_err());
        assert!(Transaction::new(1, Command::Ready, u64::MAX, 1).is_err());
        for phase_count in 0..3 {
            let mut t = Transaction::new(1, Command::Capacity, 10, 20).unwrap();
            for &(phase, bytes) in
                &[(Phase::Command, 31), (Phase::Data, 8), (Phase::Status, 13)][..phase_count]
            {
                t.poll(10).unwrap();
                t.complete(phase, bytes, &[]).unwrap();
            }
            assert_eq!(t.poll(30), Err(Error::Timeout));
        }
        let mut t = Transaction::new(1, Command::Ready, 10, 20).unwrap();
        assert_eq!(t.poll(9), Err(Error::Clock));
        let mut t = Transaction::new(1, Command::Ready, 0, 20).unwrap();
        t.polls = 1_000_000;
        assert_eq!(t.poll(0), Err(Error::Stalled));
        let mut t = Transaction::new(1, Command::Ready, 0, 20).unwrap();
        t.poll(0).unwrap();
        t.cancel();
        assert_eq!(t.complete(Phase::Command, 31, &[]), Err(Error::Canceled));
        assert_eq!(t.poll(0), Err(Error::Canceled));
    }
}
