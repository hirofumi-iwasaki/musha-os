// SPDX-License-Identifier: Apache-2.0
//! Bounded experimental-r1 ELF loader plan. No allocation, mapping or file casts.
use crate::{Error, PAGE};
#[allow(dead_code)]
#[path = "../../sdk/rust/musha-sys/src/generated.rs"]
pub mod abi;
#[derive(Clone, Copy, Debug, Default)]
pub struct Segment {
    pub address: usize,
    pub offset: usize,
    pub file_bytes: usize,
    pub memory_bytes: usize,
    pub flags: u32,
}
impl Segment {
    pub fn page_base(self) -> usize {
        self.address & !(PAGE - 1)
    }
    pub fn page_bytes(self) -> usize {
        (self.address + self.memory_bytes + PAGE - 1) / PAGE * PAGE - self.page_base()
    }
}
#[derive(Debug)]
pub struct Plan {
    pub entry: usize,
    pub segments: [Segment; 32],
    pub count: usize,
    pub arena_bytes: usize,
    pub stack_bytes: usize,
    pub permissions: u64,
}
fn part(b: &[u8], off: usize, len: usize) -> Result<&[u8], Error> {
    b.get(off..off.checked_add(len).ok_or(Error::Overflow)?)
        .ok_or(Error::Invalid)
}
fn word<const N: usize>(b: &[u8], off: usize) -> Result<[u8; N], Error> {
    part(b, off, N)?.try_into().map_err(|_| Error::Invalid)
}
fn u16at(b: &[u8], off: usize) -> Result<u16, Error> {
    Ok(u16::from_le_bytes(word(b, off)?))
}
fn u32at(b: &[u8], off: usize) -> Result<u32, Error> {
    Ok(u32::from_le_bytes(word(b, off)?))
}
fn u64at(b: &[u8], off: usize) -> Result<u64, Error> {
    Ok(u64::from_le_bytes(word(b, off)?))
}
fn size(b: &[u8], off: usize) -> Result<usize, Error> {
    usize::try_from(u64at(b, off)?).map_err(|_| Error::Overflow)
}
impl Plan {
    pub fn parse(b: &[u8]) -> Result<Self, Error> {
        if b.len() > abi::IMAGE_LIMIT as usize
            || part(b, 0, 7)? != b"\x7fELF\x02\x01\x01"
            || u16at(b, 16)? != 2
            || u16at(b, 18)? != 62
            || u32at(b, 20)? != 1
            || u32at(b, 48)? != 0
            || u16at(b, 52)? != 64
            || u16at(b, 54)? != 56
        {
            return Err(Error::Invalid);
        }
        let ph = size(b, 32)?;
        let num = u16at(b, 56)? as usize;
        if ph < 64 || num == 0 || num > 32 {
            return Err(Error::Invalid);
        }
        part(b, ph, num * 56)?;
        let mut plan = Self {
            entry: size(b, 24)?,
            segments: [Segment::default(); 32],
            count: 0,
            arena_bytes: 0,
            stack_bytes: 0,
            permissions: 0,
        };
        let mut notes = 0;
        for i in 0..num {
            let h = part(b, ph + i * 56, 56)?;
            let kind = u32at(h, 0)?;
            let flags = u32at(h, 4)?;
            let off = size(h, 8)?;
            let fs = size(h, 32)?;
            let data = part(b, off, fs)?;
            match kind {
                0 => (),
                0x6474e551 if flags == 6 => (),
                1 => {
                    let va = size(h, 16)?;
                    let ms = size(h, 40)?;
                    let align = size(h, 48)?;
                    let end = va.checked_add(ms).ok_or(Error::Overflow)?;
                    if fs > ms
                        || !matches!(flags, 4 | 5 | 6)
                        || align < PAGE
                        || !align.is_power_of_two()
                        || off % align != va % align
                        || va < abi::IMAGE_BASE as usize
                        || end > (abi::IMAGE_BASE + abi::IMAGE_LIMIT) as usize
                    {
                        return Err(Error::Invalid);
                    }
                    if ms == 0 {
                        continue;
                    }
                    let s = Segment {
                        address: va,
                        offset: off,
                        file_bytes: fs,
                        memory_bytes: ms,
                        flags,
                    };
                    for old in &plan.segments[..plan.count] {
                        if s.page_base() < old.page_base() + old.page_bytes()
                            && old.page_base() < s.page_base() + s.page_bytes()
                        {
                            return Err(Error::Overlap);
                        }
                    }
                    plan.segments[plan.count] = s;
                    plan.count += 1;
                }
                4 => {
                    // Candidate requires one exact 84-byte NOTE (no hidden extra notes).
                    if notes != 0
                        || data.len() != 84
                        || off % 4 != 0
                        || u32at(data, 0)? != 6
                        || u32at(data, 4)? != 64
                        || u32at(data, 8)? as u64 != abi::NOTE_TYPE
                        || part(data, 12, 8)? != b"MUSHA\0\0\0"
                    {
                        return Err(Error::Invalid);
                    }
                    let d = part(data, 20, 64)?;
                    if u32at(d, 0)? as u64 != abi::NOTE_SIZE
                        || u32at(d, 4)? as u64 != abi::CANDIDATE_REVISION
                        || u16at(d, 8)? as u64 != abi::ABI_MAJOR
                        || u16at(d, 10)? as u64 != abi::ABI_MINOR
                        || u32at(d, 12)? as u64 != abi::CPU_PROFILE
                        || u64at(d, 16)? != abi::FEATURE_BASE
                        || u64at(d, 24)? & !abi::PERMISSION_LOG != 0
                        || u64at(d, 48)? != 0
                        || u64at(d, 56)? != 0
                    {
                        return Err(Error::Unsupported);
                    }
                    plan.arena_bytes = size(d, 32)?;
                    plan.stack_bytes = size(d, 40)?;
                    plan.permissions = u64at(d, 24)?;
                    if plan.arena_bytes < PAGE
                        || plan.arena_bytes > abi::ARENA_LIMIT as usize
                        || plan.arena_bytes % PAGE != 0
                        || plan.stack_bytes < PAGE
                        || plan.stack_bytes > abi::STACK_LIMIT as usize
                        || plan.stack_bytes % PAGE != 0
                    {
                        return Err(Error::Invalid);
                    }
                    notes += 1;
                }
                _ => return Err(Error::Unsupported),
            }
        }
        if notes != 1
            || !plan.segments[..plan.count].iter().any(|s| {
                s.flags == 5 && plan.entry >= s.address && plan.entry < s.address + s.file_bytes
            })
        {
            return Err(Error::Invalid);
        }
        Ok(plan)
    }
    pub fn backing_bytes(&self) -> Result<usize, Error> {
        self.segments[..self.count]
            .iter()
            .try_fold(self.arena_bytes + self.stack_bytes + PAGE, |n, s| {
                n.checked_add(s.page_bytes()).ok_or(Error::Overflow)
            })
    }
    /// Zero padding and BSS, then copy only file-backed bytes into owned backing.
    pub fn load_segment(&self, index: usize, file: &[u8], dest: &mut [u8]) -> Result<(), Error> {
        let s = *self
            .segments
            .get(index)
            .filter(|_| index < self.count)
            .ok_or(Error::Invalid)?;
        if dest.len() != s.page_bytes() {
            return Err(Error::Invalid);
        }
        let src = part(file, s.offset, s.file_bytes)?;
        dest.fill(0);
        let offset = s.address - s.page_base();
        dest[offset..offset + s.file_bytes].copy_from_slice(src);
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    extern crate std;
    use std::vec;
    fn fixture() -> std::vec::Vec<u8> {
        let mut b = vec![0u8; 0x2054];
        b[..7].copy_from_slice(b"\x7fELF\x02\x01\x01");
        for (o, v) in [(16, 2u16), (18, 62), (52, 64), (54, 56), (56, 2)] {
            b[o..o + 2].copy_from_slice(&v.to_le_bytes());
        }
        put(&mut b, 20, 1, 4);
        put(&mut b, 24, abi::IMAGE_BASE, 8);
        put(&mut b, 32, 64, 8);
        for (o, v, n) in [
            (64, 1, 4),
            (68, 5, 4),
            (72, 0x1000, 8),
            (80, abi::IMAGE_BASE, 8),
            (96, 1, 8),
            (104, 4096, 8),
            (112, 4096, 8),
            (120, 4, 4),
            (128, 0x2000, 8),
            (152, 84, 8),
        ] {
            put(&mut b, o, v, n);
        }
        b[0x1000] = 0xc3;
        for (o, v, n) in [
            (0, 6, 4),
            (4, 64, 4),
            (8, abi::NOTE_TYPE, 4),
            (20, 64, 4),
            (24, 1, 4),
            (30, 1, 2),
            (32, 1, 4),
            (36, 1, 8),
            (44, 1, 8),
            (52, 4096, 8),
            (60, 4096, 8),
        ] {
            put(&mut b, 0x2000 + o, v, n);
        }
        b[0x200c..0x2012].copy_from_slice(b"MUSHA\0");
        b
    }
    fn put(b: &mut [u8], o: usize, v: u64, n: usize) {
        b[o..o + n].copy_from_slice(&v.to_le_bytes()[..n]);
    }
    #[test]
    fn loads_and_zeroes_bss() {
        let b = fixture();
        let p = Plan::parse(&b).unwrap();
        let mut dst = [0xa5; 4096];
        p.load_segment(0, &b, &mut dst).unwrap();
        assert_eq!(dst[0], 0xc3);
        assert!(dst[1..].iter().all(|v| *v == 0));
        assert_eq!(p.backing_bytes(), Ok(16384));
        assert!(p.load_segment(1, &b, &mut dst).is_err());
        assert!(p.load_segment(0, &b, &mut dst[..100]).is_err());
    }
    #[test]
    fn malformed_headers_notes_and_limits() {
        let original = fixture();
        for (o, v, n) in [
            (16, 3, 2),
            (18, 183, 2),
            (24, abi::IMAGE_BASE + 1, 8),
            (32, u64::MAX, 8),
            (56, 33, 2),
            (64, 2, 4),
            (68, 7, 4),
            (72, u64::MAX, 8),
            (80, u64::MAX, 8),
            (96, 4097, 8),
            (112, 3, 8),
            (120, 0, 4),
            (152, 85, 8),
            (0x200c, 0, 1),
            (0x2012, 1, 1),
            (0x2018, 2, 4),
            (0x201e, 2, 2),
            (0x2020, 2, 4),
            (0x2024, 2, 8),
            (0x202c, 2, 8),
            (0x2034, 0, 8),
            (0x203c, 1, 8),
            (0x2044, 1, 8),
            (0x204c, 1, 8),
        ] {
            let mut b = original.clone();
            put(&mut b, o, v, n);
            assert!(Plan::parse(&b).is_err(), "offset {o:x}");
        }
        for n in 0..original.len() {
            assert!(Plan::parse(&original[..n]).is_err());
        }
    }
    #[test]
    fn overlap_and_duplicate_note() {
        for duplicate in [true, false] {
            let mut b = fixture();
            put(&mut b, 56, 3, 2);
            let src = if duplicate { 120 } else { 64 };
            let h = b[src..src + 56].to_vec();
            b[176..232].copy_from_slice(&h);
            assert!(Plan::parse(&b).is_err());
        }
    }
}
