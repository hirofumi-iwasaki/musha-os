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
    let pointer = words[0] as u64 | ((words[1] as u64) << 32);
    (words[3] >> 10) & 63 == 33
        && words[2] >> 24 == 1
        && pointer == expected as u64
        && expected % 16 == 0
        && words[3] >> 16 == 0
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
