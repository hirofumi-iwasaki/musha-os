// Copyright 2026 Hirofumi Iwasaki
// SPDX-License-Identifier: Apache-2.0
//! Stored SC key reuse. Codec CRC recognizes structure, not hostile-media
//! authenticity. No IRK/RPA resolution and no automatic downgrade or re-pair.
use super::{Error, Event, legacy::VerifiedBond};
use crate::pairing::{self, Bond, MAX_BONDS};
use zeroize::Zeroize;
const EMPTY: Bond = Bond {
    address_type: 0,
    address: [0; 6],
    key_size: 0,
    flags: 0,
    ediv: 0,
    random: [0; 8],
    ltk: [0; 16],
};
pub struct Store {
    bonds: [Bond; MAX_BONDS],
    count: usize,
}
impl Store {
    pub const fn new() -> Self {
        Self {
            bonds: [EMPTY; MAX_BONDS],
            count: 0,
        }
    }
    pub fn clear(&mut self) {
        for b in &mut self.bonds {
            b.ltk.zeroize();
            *b = EMPTY;
        }
        self.count = 0;
    }
    pub fn load(&mut self, record: &[u8]) -> Result<Option<[u8; 32]>, Error> {
        self.clear();
        let result = pairing::decode_state(record, &mut self.bonds);
        match result {
            Ok((count, hash)) => {
                self.count = count;
                Ok(hash)
            }
            Err(_) => {
                self.clear();
                Err(Error::Invalid)
            }
        }
    }
    fn acceptable(b: &Bond) -> bool {
        b.flags&2!=0 && b.key_size==16 && b.ediv==0 && b.random==[0;8] &&
        // A saved random address must be static; no identity/IRK format guess.
        (b.address_type==0 || b.address_type==1 && b.address[5]&0xc0==0xc0)
    }
    pub fn any(&self) -> bool {
        self.bonds[..self.count].iter().any(Self::acceptable)
    }
    pub fn matches(&self, kind: u8, address: &[u8; 6]) -> bool {
        self.bonds[..self.count]
            .iter()
            .any(|b| Self::acceptable(b) && b.address_type == kind && &b.address == address)
    }
    /// Reconnect can use directed advertising with empty AD data. Only the
    /// stored public/static identity and connectable event type authorize it.
    /// Encryption and ATT binding must still validate before delivering input.
    pub fn connectable(&self, event_type: u8, kind: u8, address: &[u8; 6]) -> bool {
        matches!(event_type, 0 | 1) && self.matches(kind, address)
    }
    pub fn begin(
        &self,
        generation: u64,
        handle: u16,
        now: u64,
        kind: u8,
        address: [u8; 6],
    ) -> Result<Pending, Error> {
        if handle > 0xeff {
            return Err(Error::Invalid);
        }
        let bond = self.bonds[..self.count]
            .iter()
            .find(|b| Self::acceptable(b) && b.address_type == kind && b.address == address)
            .ok_or(Error::Unsupported)?;
        Ok(Pending {
            bond: *bond,
            generation,
            handle,
            deadline: now.checked_add(10000).ok_or(Error::Invalid)?,
            last: now,
            finished: false,
            issued: false,
        })
    }
}
impl Drop for Store {
    fn drop(&mut self) {
        self.clear();
    }
}
/// A transient controller command key. Not Clone/Debug; caller must erase its
/// command CPU copy and stop DMA before clearing an ambiguous transfer buffer.
pub struct CommandKey([u8; 16]);
impl CommandKey {
    pub fn use_with<T>(&self, f: impl FnOnce(&[u8; 16]) -> T) -> T {
        f(&self.0)
    }
}
impl Drop for CommandKey {
    fn drop(&mut self) {
        self.0.zeroize();
    }
}
pub struct Pending {
    bond: Bond,
    generation: u64,
    handle: u16,
    deadline: u64,
    last: u64,
    finished: bool,
    issued: bool,
}
impl Drop for Pending {
    fn drop(&mut self) {
        self.bond.ltk.zeroize();
    }
}
impl Pending {
    pub fn check(&mut self, generation: u64, now: u64) -> Result<(), Error> {
        if generation != self.generation {
            return Err(Error::Stale);
        }
        if self.finished {
            return Err(Error::Invalid);
        }
        if now < self.last || now >= self.deadline {
            self.cancel();
            return Err(Error::Timeout);
        }
        self.last = now;
        Ok(())
    }
    pub fn command_key(&mut self, generation: u64, now: u64) -> Result<CommandKey, Error> {
        self.check(generation, now)?;
        if self.issued {
            return Err(Error::Invalid);
        }
        self.issued = true;
        Ok(CommandKey(self.bond.ltk))
    }
    pub fn cancel(&mut self) {
        self.finished = true;
        self.bond.ltk.zeroize();
    }
    pub fn encryption(
        &mut self,
        generation: u64,
        now: u64,
        event: Event<'_>,
    ) -> Result<VerifiedBond, Error> {
        self.check(generation, now)?;
        let p = event.parameters;
        if !self.issued
            || event.code != 8
            || p.len() != 4
            || p[0] != 0
            || super::word(&p[1..3]) != self.handle
            || p[3] != 1
        {
            self.cancel();
            return Err(Error::Invalid);
        }
        let token = VerifiedBond {
            bond: self.bond,
            generation: self.generation,
            handle: self.handle,
        };
        self.cancel();
        Ok(token)
    }
}
