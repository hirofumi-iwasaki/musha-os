// Copyright 2026 Hirofumi Iwasaki
// SPDX-License-Identifier: Apache-2.0
//! New-pairing attempts against a controller that may still hold a bond this
//! host does not have (IMG_7743/7744). Such a controller reconnects as soon as
//! it powers on and ignores a Pairing Request; pairing mode entered after that
//! connection has no effect. Policy: give up on the silent connection after
//! 5 s (or at once on a Security Request), disconnect, defer that address for
//! 10 s so the user can enter pairing mode, then try again, at most 3 times.
#![forbid(unsafe_code)]

pub const REPLY_MS: u64 = 5000;
pub const DEFER_MS: u64 = 10000;
pub const MAX_ATTEMPTS: u8 = 3;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Decision {
    /// Keep waiting for the Pairing Response.
    Wait,
    /// Show the guidance and disconnect this connection, then rescan.
    Disconnect,
    /// Attempts used up: keep waiting until the SMP timeout stops Bluetooth.
    Exhausted,
}

pub struct Retry {
    attempts: u8,
    deferred: Option<([u8; 6], u64)>,
}
impl Default for Retry {
    fn default() -> Self {
        Self::new()
    }
}
impl Retry {
    pub const fn new() -> Self {
        Self {
            attempts: 0,
            deferred: None,
        }
    }
    pub fn attempts(&self) -> u8 {
        self.attempts
    }
    /// While awaiting the Pairing Response: `waited` since the Pairing Request,
    /// `security_request` when the peer sent SMP Security Request (0x0b), the
    /// sign of a peer that believes it is already bonded.
    pub fn decide(&self, waited: u64, security_request: bool) -> Decision {
        if waited < REPLY_MS && !security_request {
            Decision::Wait
        } else if self.attempts < MAX_ATTEMPTS {
            Decision::Disconnect
        } else {
            Decision::Exhausted
        }
    }
    /// Record a disconnect issued for `peer`; it is deferred for 10 s.
    pub fn disconnected(&mut self, peer: [u8; 6], now: u64) {
        self.attempts = self.attempts.saturating_add(1);
        self.deferred = Some((peer, now.saturating_add(DEFER_MS)));
    }
    /// Whether an advertiser may be chosen now. Other addresses always may.
    pub fn admits(&mut self, address: &[u8; 6], now: u64) -> bool {
        match self.deferred {
            Some((peer, until)) if &peer == address && now < until => false,
            Some((_, until)) if now >= until => {
                self.deferred = None;
                true
            }
            _ => true,
        }
    }
}
