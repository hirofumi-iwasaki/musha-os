// SPDX-License-Identifier: Apache-2.0
//! Allocation-free, whole-record eviction log. The storage survives firmware exit.
pub const CAPACITY: usize = 64 * 1024;
const HEADER: usize = 12;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Level {
    Error = 0,
    Ready = 1,
    Phases = 2,
    Devices = 3,
    Application = 4,
    Trace = 5,
}
pub struct Ring {
    bytes: [u8; CAPACITY],
    head: usize,
    used: usize,
    lost: u64,
}
impl Default for Ring {
    fn default() -> Self {
        Self::new()
    }
}
impl Ring {
    pub const fn new() -> Self {
        Self {
            bytes: [0; CAPACITY],
            head: 0,
            used: 0,
            lost: 0,
        }
    }
    pub fn lost(&self) -> u64 {
        self.lost
    }
    pub fn used(&self) -> usize {
        self.used
    }
    fn byte(&self, offset: usize) -> u8 {
        self.bytes[(self.head + offset) % CAPACITY]
    }
    fn length(&self, offset: usize) -> usize {
        u16::from_le_bytes([self.byte(offset), self.byte(offset + 1)]) as usize
    }
    /// An oversize record is rejected explicitly; existing records remain intact.
    pub fn push(&mut self, level: Level, milliseconds: u64, early: bool, text: &[u8]) -> bool {
        let Some(total) = text
            .len()
            .checked_add(HEADER)
            .filter(|n| *n <= u16::MAX as usize && *n <= CAPACITY)
        else {
            self.lost = self.lost.saturating_add(1);
            return false;
        };
        while CAPACITY - self.used < total {
            let old = self.length(0);
            self.head = (self.head + old) % CAPACITY;
            self.used -= old;
            self.lost = self.lost.saturating_add(1);
        }
        let mut header = [0; HEADER];
        header[..2].copy_from_slice(&(total as u16).to_le_bytes());
        header[2] = level as u8;
        header[3] = u8::from(early);
        header[4..].copy_from_slice(&milliseconds.to_le_bytes());
        for byte in header.iter().chain(text) {
            self.bytes[(self.head + self.used) % CAPACITY] = *byte;
            self.used += 1;
        }
        true
    }
    pub fn records(&self) -> Records<'_> {
        Records { ring: self, at: 0 }
    }
}
pub struct Record<'a> {
    ring: &'a Ring,
    at: usize,
    length: usize,
}
impl Record<'_> {
    pub fn level(&self) -> Level {
        match self.ring.byte(self.at + 2) {
            0 => Level::Error,
            1 => Level::Ready,
            2 => Level::Phases,
            3 => Level::Devices,
            5 => Level::Trace,
            _ => Level::Application,
        }
    }
    pub fn early(&self) -> bool {
        self.ring.byte(self.at + 3) != 0
    }
    pub fn milliseconds(&self) -> u64 {
        let mut bytes = [0; 8];
        for (i, b) in bytes.iter_mut().enumerate() {
            *b = self.ring.byte(self.at + 4 + i);
        }
        u64::from_le_bytes(bytes)
    }
    pub fn bytes(&self) -> impl Iterator<Item = u8> + '_ {
        (HEADER..self.length).map(|i| self.ring.byte(self.at + i))
    }
}
pub struct Records<'a> {
    ring: &'a Ring,
    at: usize,
}
impl<'a> Iterator for Records<'a> {
    type Item = Record<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.at == self.ring.used {
            return None;
        }
        let length = self.ring.length(self.at);
        let record = Record {
            ring: self.ring,
            at: self.at,
            length,
        };
        self.at += length;
        Some(record)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn retains_metadata_and_bytes() {
        let mut ring = Ring::new();
        assert!(ring.push(Level::Devices, 123, true, b"settings: defaults"));
        let r = ring.records().next().unwrap();
        assert_eq!(r.level(), Level::Devices);
        assert_eq!(r.milliseconds(), 123);
        assert!(r.early());
        assert!(r.bytes().eq(b"settings: defaults".iter().copied()));
    }
    #[test]
    fn evicts_whole_records_across_wrap() {
        let mut ring = Ring::new();
        for n in 0..10_000 {
            assert!(ring.push(Level::Phases, n, false, b"event"));
        }
        assert!(ring.lost() > 0);
        let mut previous = None;
        for r in ring.records() {
            if let Some(p) = previous {
                assert_eq!(r.milliseconds(), p + 1);
            }
            previous = Some(r.milliseconds());
            assert!(r.bytes().eq(b"event".iter().copied()));
        }
        assert_eq!(previous, Some(9999));
        assert!(ring.used() <= CAPACITY);
    }
    #[test]
    fn oversize_does_not_destroy_old_record() {
        let mut ring = Ring::new();
        ring.push(Level::Ready, 0, true, b"early");
        assert!(!ring.push(Level::Devices, 0, false, &[0; CAPACITY]));
        assert_eq!(ring.records().count(), 1);
        assert_eq!(ring.lost(), 1);
    }
}
