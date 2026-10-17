// Copyright 2026 Hirofumi Iwasaki
// SPDX-License-Identifier: Apache-2.0
//! LE L2CAP signaling channel (CID 5), central role. Only Connection Parameter
//! Update is supported; other requests get Command Reject. No dynamic channels.
#![forbid(unsafe_code)]
use super::{Error, word};

pub const CHANNEL: u16 = 5;
const COMMAND_REJECT: u8 = 0x01;
const UPDATE_REQUEST: u8 = 0x12;
const UPDATE_RESPONSE: u8 = 0x13;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Parameters {
    /// Connection interval bounds, units of 1.25 ms.
    pub min: u16,
    pub max: u16,
    /// Peripheral latency in connection events.
    pub latency: u16,
    /// Supervision timeout, units of 10 ms.
    pub timeout: u16,
}
impl Parameters {
    /// Core Vol 3 Part A 4.20 / Vol 6 Part B 4.5.1 ranges, and a supervision
    /// timeout longer than (1 + latency) * max interval * 2.
    pub fn valid(self) -> bool {
        (6..=3200).contains(&self.min)
            && (6..=3200).contains(&self.max)
            && self.min <= self.max
            && self.latency <= 499
            && (10..=3200).contains(&self.timeout)
            && self.timeout as u32 * 4 > (1 + self.latency as u32) * self.max as u32
    }
    /// HCI LE Connection Update (0x2013) parameters; CE lengths are 0.
    pub fn command(self, handle: u16) -> [u8; 14] {
        let mut p = [0; 14];
        for (i, v) in [handle, self.min, self.max, self.latency, self.timeout]
            .into_iter()
            .enumerate()
        {
            p[i * 2..i * 2 + 2].copy_from_slice(&v.to_le_bytes());
        }
        p
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Signal {
    /// Accepted: send `response`, then HCI LE Connection Update.
    Update {
        identifier: u8,
        parameters: Parameters,
        response: [u8; 6],
    },
    /// Out of range: send the rejected `response`; the connection is kept.
    Rejected {
        identifier: u8,
        parameters: Parameters,
        response: [u8; 6],
    },
    /// Unsupported or malformed request: send Command Reject (not understood).
    Reject {
        identifier: u8,
        code: u8,
        response: [u8; 6],
    },
    /// A peer response, Command Reject, indication or repeated identifier:
    /// nothing is sent and no command is issued.
    Ignore {
        identifier: u8,
        code: u8,
        duplicate: bool,
    },
}

/// Codes that are responses or indications; answering them is never allowed.
fn unanswered(code: u8) -> bool {
    matches!(
        code,
        0x01 | 0x03
            | 0x05
            | 0x07
            | 0x09
            | 0x0b
            | 0x0d
            | 0x0f
            | 0x11
            | 0x13
            | 0x15
            | 0x16
            | 0x18
            | 0x1a
    )
}
fn reply(code: u8, identifier: u8, value: u16) -> [u8; 6] {
    let mut r = [code, identifier, 2, 0, 0, 0];
    r[4..].copy_from_slice(&value.to_le_bytes());
    r
}

/// One LE signaling PDU carries exactly one command. A retransmitted request
/// with the last answered identifier is answered only once.
pub struct Signaling {
    answered: Option<u8>,
}
impl Default for Signaling {
    fn default() -> Self {
        Self::new()
    }
}
impl Signaling {
    pub const fn new() -> Self {
        Self { answered: None }
    }
    pub fn receive(&mut self, sdu: &[u8]) -> Result<Signal, Error> {
        if sdu.len() < 4 || sdu.len() != 4 + word(&sdu[2..4]) as usize {
            return Err(Error::Length);
        }
        let (code, identifier) = (sdu[0], sdu[1]);
        if identifier == 0 {
            return Err(Error::Invalid);
        }
        if unanswered(code) {
            return Ok(Signal::Ignore {
                identifier,
                code,
                duplicate: false,
            });
        }
        if self.answered == Some(identifier) {
            return Ok(Signal::Ignore {
                identifier,
                code,
                duplicate: true,
            });
        }
        self.answered = Some(identifier);
        if code != UPDATE_REQUEST || sdu.len() != 12 {
            return Ok(Signal::Reject {
                identifier,
                code,
                response: reply(COMMAND_REJECT, identifier, 0),
            });
        }
        let parameters = Parameters {
            min: word(&sdu[4..]),
            max: word(&sdu[6..]),
            latency: word(&sdu[8..]),
            timeout: word(&sdu[10..]),
        };
        Ok(if parameters.valid() {
            Signal::Update {
                identifier,
                parameters,
                response: reply(UPDATE_RESPONSE, identifier, 0),
            }
        } else {
            Signal::Rejected {
                identifier,
                parameters,
                response: reply(UPDATE_RESPONSE, identifier, 1),
            }
        })
    }
}

/// LE Connection Update Complete (subevent 0x03) metadata.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UpdateComplete {
    pub status: u8,
    pub handle: u16,
    pub interval: u16,
    pub latency: u16,
    pub timeout: u16,
}
pub fn update_complete(p: &[u8]) -> Option<UpdateComplete> {
    if p.len() != 10 || p[0] != 3 {
        return None;
    }
    Some(UpdateComplete {
        status: p[1],
        handle: word(&p[2..]),
        interval: word(&p[4..]),
        latency: word(&p[6..]),
        timeout: word(&p[8..]),
    })
}
