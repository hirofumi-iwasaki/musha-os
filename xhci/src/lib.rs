// Copyright 2026 Hirofumi Iwasaki
// SPDX-License-Identifier: Apache-2.0
#![no_std]
/// Address-only bump allocator. Hardware access is outside this safe layer.
pub struct Pool {
    next: usize,
    end: usize,
}
impl Pool {
    pub fn new(base: usize, bytes: usize) -> Option<Self> {
        let end = base.checked_add(bytes)?;
        if base == 0 || base % 4096 != 0 || bytes == 0 || bytes % 4096 != 0 || end > 1usize << 32 {
            return None;
        }
        Some(Self { next: base, end })
    }
    pub fn allocate(&mut self, bytes: usize, align: usize) -> Option<usize> {
        if bytes == 0 || !align.is_power_of_two() {
            return None;
        }
        let start = self.next.checked_add(align - 1)? & !(align - 1);
        let end = start.checked_add(bytes)?;
        if end > self.end {
            return None;
        }
        self.next = end;
        Some(start)
    }
}
/// Cursor for one segment. A command ring reserves its last TRB for Link;
/// an event ring uses all TRBs. Advancement occurs only after completion/consume.
pub struct Cursor {
    pub index: usize,
    pub cycle: u32,
    length: usize,
}
impl Cursor {
    pub fn new(length: usize) -> Option<Self> {
        if !(16..=4096).contains(&length) {
            return None;
        }
        Some(Self {
            index: 0,
            cycle: 1,
            length,
        })
    }
    pub fn advance(&mut self) {
        self.index += 1;
        if self.index == self.length {
            self.index = 0;
            self.cycle ^= 1;
        }
    }
}
pub fn scratchpads(params: u32) -> usize {
    (((params >> 21) & 31) << 5 | ((params >> 27) & 31)) as usize
}
/// Require success, the precise submitted pointer, and no unexpected slot/VF.
pub fn completion(words: [u32; 4], expected: usize) -> bool {
    command_completion(words, expected, Some(0), 8)
}
pub fn command_completion(words: [u32; 4], expected: usize, slot: Option<u8>, maximum: u8) -> bool {
    let pointer = words[0] as u64 | ((words[1] as u64) << 32);
    let returned = (words[3] >> 24) as u8;
    (words[3] >> 10) & 63 == 33
        && words[2] >> 24 == 1
        && pointer == expected as u64
        && expected % 16 == 0
        && words[3] & 0xff0000 == 0
        && match slot {
            Some(value) => returned == value,
            None => returned != 0 && returned <= maximum,
        }
}
pub fn transfer_completion(words: [u32; 4], expected: usize, slot: u8) -> bool {
    endpoint_completion(words, expected, slot, 1)
}
pub fn endpoint_completion(words: [u32; 4], expected: usize, slot: u8, endpoint: u8) -> bool {
    let pointer = words[0] as u64 | ((words[1] as u64) << 32);
    endpoint != 0
        && endpoint <= 31
        && (words[3] >> 10) & 63 == 32
        && words[2] >> 24 == 1
        && pointer == expected as u64
        && expected % 16 == 0
        && words[2] & 0xffffff == 0
        && (words[3] >> 24) as u8 == slot
        && (words[3] >> 16) & 31 == endpoint as u32
        && words[3] & (4 | 0xe00000) == 0
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pool_bounds_alignment_and_failure_atomicity() {
        let mut p = Pool::new(0xffffc000, 16384).unwrap();
        assert_eq!(p.allocate(64, 64), Some(0xffffc000));
        assert_eq!(p.allocate(4096, 4096), Some(0xffffd000));
        assert_eq!(p.allocate(usize::MAX, 64), None);
        assert_eq!(p.allocate(8192, 4096), Some(0xffffe000));
        assert_eq!(p.allocate(1, 1), None);
        assert!(Pool::new(0xfffff000, 8192).is_none());
    }
    #[test]
    fn rings_wrap_repeatedly() {
        let mut command = Cursor::new(255).unwrap();
        let mut event = Cursor::new(256).unwrap();
        for n in 0..1024 {
            assert_eq!(command.index, n % 255);
            assert_eq!(command.cycle, 1 ^ ((n / 255) & 1) as u32);
            assert_eq!(event.index, n % 256);
            assert_eq!(event.cycle, 1 ^ ((n / 256) & 1) as u32);
            command.advance();
            event.advance();
        }
    }
    #[test]
    fn rejects_bad_completions() {
        let mut event = [0x12345000, 0, 1 << 24, 33 << 10 | 1];
        assert!(completion(event, 0x12345000));
        assert!(!completion(event, 0x12345010));
        event[2] = 5 << 24;
        assert!(!completion(event, 0x12345000));
        event[2] = 1 << 24;
        event[3] = 34 << 10 | 1;
        assert!(!completion(event, 0x12345000));
    }
    #[test]
    fn scratchpad_high_low_fields() {
        assert_eq!(scratchpads((3 << 21) | (5 << 27)), 101);
        assert_eq!(scratchpads(1 << 26), 0);
    }
}

/// PORTSC control bits that can be preserved without replaying W1C changes or
/// writing PED (whose write-one action disables a port).
pub fn port_control(status: u32) -> u32 {
    status & 0x0e00_c3e0
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Speed {
    Low,
    Full,
    High,
    Super,
}
impl Speed {
    pub fn packet(self) -> u16 {
        match self {
            Self::Low | Self::Full => 8,
            Self::High => 64,
            Self::Super => 512,
        }
    }
    pub fn from_rate(major: u8, rate: u64) -> Option<Self> {
        match (major, rate) {
            (2, 1_500_000) => Some(Self::Low),
            (2, 12_000_000) => Some(Self::Full),
            (2, 480_000_000) => Some(Self::High),
            (3, rate) if rate >= 5_000_000_000 => Some(Self::Super),
            _ => None,
        }
    }
    pub fn descriptor_packet(self, value: u8) -> Option<u16> {
        match self {
            Self::Low if value == 8 => Some(8),
            Self::Full if matches!(value, 8 | 16 | 32 | 64) => Some(value as u16),
            Self::High if value == 64 => Some(64),
            Self::Super if value == 9 => Some(512),
            _ => None,
        }
    }
}
/// Input Slot + EP0 context words; offsets are applied using the xHC CSZ stride.
pub fn initial_context(
    port: u8,
    speed_id: u8,
    packet: u16,
    ring: usize,
) -> Option<([u32; 4], [u32; 5])> {
    if port == 0
        || !(1..=15).contains(&speed_id)
        || !matches!(packet, 8 | 16 | 32 | 64 | 512)
        || ring == 0
        || ring % 64 != 0
        || ring >= 1usize << 32
    {
        return None;
    }
    Some((
        [
            (1 << 27) | ((speed_id as u32) << 20),
            (port as u32) << 16,
            0,
            0,
        ],
        [
            0,
            (3 << 1) | (4 << 3) | ((packet as u32) << 16),
            ring as u32 | 1,
            0,
            8,
        ],
    ))
}
pub fn descriptor(b: &[u8], speed: Speed) -> Option<(u16, u16)> {
    if b.len() != 18
        || b[0] != 18
        || b[1] != 1
        || speed.descriptor_packet(b[7]).is_none()
        || b[17] == 0
    {
        return None;
    }
    Some((
        u16::from_le_bytes([b[8], b[9]]),
        u16::from_le_bytes([b[10], b[11]]),
    ))
}
#[cfg(test)]
mod usb_tests {
    use super::*;
    #[test]
    fn port_writes_do_not_disable_or_ack_accidentally() {
        let original = 0xffffffff;
        let safe = port_control(original);
        assert_eq!(
            safe & ((1 << 1) | (1 << 4) | (1 << 16) | 0xfe0000 | (1 << 31)),
            0
        );
        assert_eq!(safe & (1 << 9), 1 << 9);
    }
    #[test]
    fn packet_rules_and_context_bounds() {
        assert_eq!(Speed::Super.descriptor_packet(9), Some(512));
        assert_eq!(Speed::Super.descriptor_packet(64), None);
        assert_eq!(Speed::Low.descriptor_packet(64), None);
        assert_eq!(Speed::Full.descriptor_packet(64), Some(64));
        assert!(initial_context(0, 1, 8, 4096).is_none());
        assert!(initial_context(1, 16, 8, 4096).is_none());
        let (slot, ep) = initial_context(3, 4, 512, 0x12340000).unwrap();
        assert_eq!(slot[1], 3 << 16);
        assert_eq!(slot[0], (1 << 27) | (4 << 20));
        assert_eq!(ep[2], 0x12340001);
    }
    #[test]
    fn rejects_truncated_and_bad_descriptors() {
        let mut d = [0u8; 18];
        d[0] = 18;
        d[1] = 1;
        d[7] = 64;
        d[8] = 0x34;
        d[9] = 0x12;
        d[17] = 1;
        assert_eq!(descriptor(&d, Speed::High), Some((0x1234, 0)));
        assert_eq!(descriptor(&d[..8], Speed::High), None);
        d[1] = 2;
        assert_eq!(descriptor(&d, Speed::High), None);
    }
}

#[cfg(test)]
mod event_tests {
    use super::*;
    #[test]
    fn slot_and_transfer_event_validation() {
        let mut words = [0x1000, 0, 1 << 24, (33 << 10) | (1 << 24) | 1];
        assert!(command_completion(words, 0x1000, None, 8));
        assert!(!command_completion(words, 0x1000, Some(2), 8));
        words[3] = (33 << 10) | (9 << 24) | 1;
        assert!(!command_completion(words, 0x1000, None, 8));
        words[3] = (32 << 10) | (1 << 24) | (1 << 16) | 1;
        assert!(transfer_completion(words, 0x1000, 1));
        words[2] |= 1;
        assert!(!transfer_completion(words, 0x1000, 1));
        words[2] = 1 << 24;
        words[3] |= 4;
        assert!(!transfer_completion(words, 0x1000, 1));
    }
}

pub mod keyboard;
pub mod storage;

pub mod hub;

pub mod control;
