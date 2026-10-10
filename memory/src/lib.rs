// Copyright 2026 Hirofumi Iwasaki
// SPDX-License-Identifier: Apache-2.0
#![no_std]
pub const PAGE: usize = 4096;
/// Runtime ownership is distinct from GetMemoryMap protection capabilities.
pub const RUNTIME_ATTRIBUTE: u64 = 1 << 63;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Invalid,
    Overflow,
    Overlap,
    NoMemory,
    Unsupported,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Range {
    pub start: usize,
    pub end: usize,
}
impl Range {
    pub fn new(start: usize, bytes: usize) -> Result<Self, Error> {
        let end = start.checked_add(bytes).ok_or(Error::Overflow)?;
        if bytes == 0 || end > 1usize << 47 {
            return Err(Error::Invalid);
        }
        Ok(Self { start, end })
    }
    pub fn overlaps(self, other: Self) -> bool {
        self.start < other.end && other.start < self.end
    }
    pub fn len(self) -> usize {
        self.end - self.start
    }
    pub fn aligned(self) -> Result<Self, Error> {
        let end = self.end.checked_add(PAGE - 1).ok_or(Error::Overflow)? & !(PAGE - 1);
        Range::new(self.start & !(PAGE - 1), end - (self.start & !(PAGE - 1)))
    }
}
#[derive(Clone, Copy)]
pub struct Region {
    pub kind: u32,
    pub range: Range,
    pub attributes: u64,
}
pub struct MemoryMap<'a> {
    bytes: &'a [u8],
    stride: usize,
}
fn u32at(b: &[u8], offset: usize) -> Result<u32, Error> {
    Ok(u32::from_le_bytes(
        b.get(offset..offset.checked_add(4).ok_or(Error::Overflow)?)
            .ok_or(Error::Invalid)?
            .try_into()
            .map_err(|_| Error::Invalid)?,
    ))
}
fn u64at(b: &[u8], offset: usize) -> Result<u64, Error> {
    Ok(u64::from_le_bytes(
        b.get(offset..offset.checked_add(8).ok_or(Error::Overflow)?)
            .ok_or(Error::Invalid)?
            .try_into()
            .map_err(|_| Error::Invalid)?,
    ))
}
impl<'a> MemoryMap<'a> {
    pub fn new(bytes: &'a [u8], stride: usize, version: u32) -> Result<Self, Error> {
        if stride < 40
            || stride % 8 != 0
            || bytes.is_empty()
            || bytes.len() % stride != 0
            || version != 1
        {
            return Err(Error::Invalid);
        }
        let map = Self { bytes, stride };
        for i in 0..map.count() {
            let region = map.region(i)?;
            for j in 0..i {
                if region.range.overlaps(map.region(j)?.range) {
                    return Err(Error::Overlap);
                }
            }
        }
        Ok(map)
    }
    pub fn count(&self) -> usize {
        self.bytes.len() / self.stride
    }
    pub fn region(&self, index: usize) -> Result<Region, Error> {
        let base = index.checked_mul(self.stride).ok_or(Error::Overflow)?;
        let b = self
            .bytes
            .get(base..base.checked_add(self.stride).ok_or(Error::Overflow)?)
            .ok_or(Error::Invalid)?;
        let start = usize::try_from(u64at(b, 8)?).map_err(|_| Error::Overflow)?;
        let bytes = usize::try_from(u64at(b, 24)?)
            .map_err(|_| Error::Overflow)?
            .checked_mul(PAGE)
            .ok_or(Error::Overflow)?;
        if start % PAGE != 0 {
            return Err(Error::Invalid);
        }
        Ok(Region {
            kind: u32at(b, 0)?,
            range: Range::new(start, bytes)?,
            attributes: u64at(b, 32)?,
        })
    }
    /// Require complete non-runtime LoaderData coverage, independent
    /// of descriptor order and firmware splitting at attribute boundaries.
    pub fn loader_coverage(&self, target: Range) -> Result<(), Error> {
        if target.start == 0
            || target.start % PAGE != 0
            || target.end % PAGE != 0
            || target.end <= target.start
            || target.end > 1usize << 32
        {
            return Err(Error::Invalid);
        }
        let mut cursor = target.start;
        for _ in 0..self.count() {
            let mut covering = None;
            for i in 0..self.count() {
                let region = self.region(i)?;
                if region.range.start <= cursor && cursor < region.range.end {
                    covering = Some(region);
                    break;
                }
            }
            let region = covering.ok_or(Error::Invalid)?;
            if region.kind != 2 || region.attributes & RUNTIME_ATTRIBUTE != 0 {
                return Err(Error::Invalid);
            }
            cursor = region.range.end.min(target.end);
            if cursor == target.end {
                return Ok(());
            }
        }
        Err(Error::Invalid)
    }
    pub fn arena(
        &self,
        reserved: &[Range],
        minimum: usize,
        maximum: usize,
    ) -> Result<Range, Error> {
        if minimum == 0 || minimum % PAGE != 0 || maximum < minimum || maximum % PAGE != 0 {
            return Err(Error::Invalid);
        }
        for r in reserved {
            if r.start % PAGE != 0 || r.end % PAGE != 0 || r.end <= r.start || r.end > 1usize << 47
            {
                return Err(Error::Invalid);
            }
        }
        let mut best = None;
        for i in 0..self.count() {
            let region = self.region(i)?;
            // Conventional + WB-capable, non-runtime. RP/RO/XP are capabilities;
            // the caller installs its own RW/NX mappings after ExitBootServices.
            if region.kind != 7
                || region.attributes & 8 == 0
                || region.attributes & RUNTIME_ATTRIBUTE != 0
            {
                continue;
            }
            let mut cursor = region.range.start.max(PAGE);
            while cursor < region.range.end {
                let next = reserved
                    .iter()
                    .filter(|r| r.end > cursor && r.start < region.range.end)
                    .min_by_key(|r| r.start);
                let gap_end = next.map_or(region.range.end, |r| {
                    r.start.max(cursor).min(region.range.end)
                });
                let bytes = (gap_end - cursor).min(maximum);
                if bytes >= minimum && best.is_none_or(|r: Range| bytes > r.len()) {
                    best = Some(Range::new(cursor, bytes)?);
                }
                if let Some(r) = next {
                    cursor = r.end.max(cursor).min(region.range.end);
                } else {
                    break;
                }
            }
        }
        let result = best.ok_or(Error::NoMemory)?;
        if reserved.iter().any(|r| r.overlaps(result)) {
            return Err(Error::Overlap);
        }
        Ok(result)
    }
}

pub struct Image<'a> {
    bytes: &'a [u8],
    sections: usize,
    count: usize,
}
#[derive(Clone, Copy)]
pub struct Section {
    pub offset: usize,
    pub bytes: usize,
    pub writable: bool,
    pub executable: bool,
}
impl<'a> Image<'a> {
    pub fn new(bytes: &'a [u8]) -> Result<Self, Error> {
        if bytes.get(..2) != Some(b"MZ") {
            return Err(Error::Invalid);
        }
        let pe = u32at(bytes, 60)? as usize;
        if bytes.get(pe..pe.checked_add(4).ok_or(Error::Overflow)?) != Some(b"PE\0\0") {
            return Err(Error::Invalid);
        }
        let coff = pe.checked_add(4).ok_or(Error::Overflow)?;
        let h = bytes
            .get(coff..coff.checked_add(20).ok_or(Error::Overflow)?)
            .ok_or(Error::Invalid)?;
        let count = u16::from_le_bytes([h[2], h[3]]) as usize;
        let optional = coff + 20;
        let optional_bytes = u16::from_le_bytes([h[16], h[17]]) as usize;
        if h[..2] != [0x64, 0x86]
            || optional_bytes < 112
            || bytes.get(optional..optional + 2) != Some(&[0x0b, 0x02])
            || u32at(bytes, optional + 32)? as usize != PAGE
        {
            return Err(Error::Unsupported);
        }
        let sections = optional
            .checked_add(optional_bytes)
            .ok_or(Error::Overflow)?;
        if count == 0
            || count > 96
            || sections.checked_add(count * 40).ok_or(Error::Overflow)? > bytes.len()
        {
            return Err(Error::Invalid);
        }
        let image = Self {
            bytes,
            sections,
            count,
        };
        for i in 0..count {
            let a = image.section(i)?;
            if a.bytes == 0 {
                continue;
            }
            let end = a.offset.checked_add(a.bytes).ok_or(Error::Overflow)?;
            if a.offset % PAGE != 0
                || a.offset < sections + count * 40
                || end > bytes.len()
                || a.writable && a.executable
            {
                return Err(Error::Invalid);
            }
            let end = (end + PAGE - 1) & !(PAGE - 1);
            for j in 0..i {
                let b = image.section(j)?;
                if b.bytes > 0
                    && a.offset < (b.offset + b.bytes + PAGE - 1) & !(PAGE - 1)
                    && b.offset < end
                {
                    return Err(Error::Overlap);
                }
            }
        }
        Ok(image)
    }
    pub fn count(&self) -> usize {
        self.count
    }
    pub fn section(&self, index: usize) -> Result<Section, Error> {
        if index >= self.count {
            return Err(Error::Invalid);
        }
        let base = self.sections + index * 40;
        let flags = u32at(self.bytes, base + 36)?;
        Ok(Section {
            offset: u32at(self.bytes, base + 12)? as usize,
            bytes: u32at(self.bytes, base + 8)? as usize,
            writable: flags & 0x80000000 != 0,
            executable: flags & 0x20000000 != 0,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn descriptor(kind: u32, start: u64, pages: u64, attrs: u64) -> [u8; 48] {
        let mut b = [0; 48];
        b[..4].copy_from_slice(&kind.to_le_bytes());
        b[8..16].copy_from_slice(&start.to_le_bytes());
        b[24..32].copy_from_slice(&pages.to_le_bytes());
        b[32..40].copy_from_slice(&attrs.to_le_bytes());
        b
    }
    #[test]
    fn reserved_holes_and_unsorted_input() {
        let b = descriptor(7, 4096, 32, 8);
        let map = MemoryMap::new(&b, 48, 1).unwrap();
        let reservations = [
            Range::new(9 * PAGE, 4 * PAGE).unwrap(),
            Range::new(2 * PAGE, 2 * PAGE).unwrap(),
        ];
        assert_eq!(
            map.arena(&reservations, 4 * PAGE, 64 * PAGE).unwrap(),
            Range::new(13 * PAGE, 20 * PAGE).unwrap()
        );
    }
    #[test]
    fn rejects_bad_maps() {
        assert!(MemoryMap::new(&[0; 39], 40, 1).is_err());
        assert!(MemoryMap::new(&descriptor(7, 4096, u64::MAX, 8), 48, 1).is_err());
        assert!(MemoryMap::new(&descriptor(7, 1, 2, 8), 48, 1).is_err());
        let b = [descriptor(7, 4096, 4, 8), descriptor(2, 8192, 1, 8)].concat();
        assert!(matches!(MemoryMap::new(&b, 48, 1), Err(Error::Overlap)));
    }
    #[test]
    fn dma_coverage_accepts_split_unsorted_and_partial_descriptors() {
        let b = [
            descriptor(2, 4 * PAGE as u64, 4, 1),
            descriptor(2, PAGE as u64, 3, 8),
        ]
        .concat();
        let map = MemoryMap::new(&b, 48, 1).unwrap();
        assert_eq!(
            map.loader_coverage(Range::new(2 * PAGE, 5 * PAGE).unwrap()),
            Ok(())
        );
        assert_eq!(
            map.loader_coverage(Range::new(PAGE, 7 * PAGE).unwrap()),
            Ok(())
        );
    }
    #[test]
    fn dma_coverage_rejects_gaps_types_and_runtime() {
        let target = Range::new(PAGE, 4 * PAGE).unwrap();
        for (kind, attr, start) in [(2, 8, 4), (7, 8, 3), (0, 8, 3), (2, 1u64 << 63, 3)] {
            let b = [
                descriptor(2, PAGE as u64, 2, 8),
                descriptor(kind, start * PAGE as u64, 2, attr),
            ]
            .concat();
            let map = MemoryMap::new(&b, 48, 1).unwrap();
            assert_eq!(map.loader_coverage(target), Err(Error::Invalid));
        }
        let b = descriptor(2, PAGE as u64, 8, 8);
        let map = MemoryMap::new(&b, 48, 1).unwrap();
        for range in [
            Range {
                start: 0,
                end: PAGE,
            },
            Range {
                start: PAGE + 1,
                end: 2 * PAGE,
            },
            Range {
                start: PAGE,
                end: 2 * PAGE + 1,
            },
            Range {
                start: PAGE,
                end: PAGE,
            },
            Range {
                start: PAGE,
                end: (1usize << 32) + PAGE,
            },
        ] {
            assert_eq!(map.loader_coverage(range), Err(Error::Invalid));
        }
    }
    #[test]
    fn excludes_firmware_runtime_and_non_wb_regions() {
        for (kind, attrs) in [(2, 8), (7, 0), (7, 8 | (1 << 63))] {
            let b = descriptor(kind, 4096, 8, attrs);
            assert_eq!(
                MemoryMap::new(&b, 48, 1)
                    .unwrap()
                    .arena(&[], PAGE, 4 * PAGE),
                Err(Error::NoMemory)
            );
        }
    }
    #[test]
    fn protection_capabilities_do_not_deny_owned_ram() {
        // H6 Mac descriptor: RP/RO/XP plus all four cache capabilities.
        let capabilities = 0x2600f;
        let b = [
            descriptor(2, PAGE as u64, 2, capabilities),
            descriptor(2, 3 * PAGE as u64, 2, 8),
        ]
        .concat();
        let map = MemoryMap::new(&b, 48, 1).unwrap();
        assert_eq!(
            map.loader_coverage(Range::new(PAGE, 4 * PAGE).unwrap()),
            Ok(())
        );
        let b = descriptor(7, PAGE as u64, 12, capabilities);
        let map = MemoryMap::new(&b, 48, 1).unwrap();
        let reserved = Range::new(PAGE, 2 * PAGE).unwrap();
        let arena = map.arena(&[reserved], PAGE, 4 * PAGE).unwrap();
        assert_eq!(arena, Range::new(3 * PAGE, 4 * PAGE).unwrap());
        for kind in [2, 7] {
            let b = descriptor(kind, PAGE as u64, 12, capabilities | RUNTIME_ATTRIBUTE);
            let map = MemoryMap::new(&b, 48, 1).unwrap();
            assert!(
                map.loader_coverage(Range::new(PAGE, PAGE).unwrap())
                    .is_err()
            );
            assert!(map.arena(&[], PAGE, 4 * PAGE).is_err());
        }
    }
    #[test]
    fn zero_page_and_capacity() {
        let b = descriptor(7, 0, 8, 8);
        let m = MemoryMap::new(&b, 48, 1).unwrap();
        assert_eq!(
            m.arena(&[], PAGE, 2 * PAGE).unwrap(),
            Range::new(PAGE, 2 * PAGE).unwrap()
        );
        assert_eq!(m.arena(&[], 8 * PAGE, 8 * PAGE), Err(Error::NoMemory));
        assert!(Range::new(usize::MAX, 2).is_err());
    }
    #[test]
    fn overlapping_reservations_and_alignment() {
        let b = descriptor(7, 4096, 16, 8);
        let m = MemoryMap::new(&b, 48, 1).unwrap();
        let r = [
            Range::new(PAGE, 8 * PAGE).unwrap(),
            Range::new(3 * PAGE, 8 * PAGE).unwrap(),
        ];
        assert_eq!(
            m.arena(&r, PAGE, 16 * PAGE).unwrap(),
            Range::new(11 * PAGE, 6 * PAGE).unwrap()
        );
        assert_eq!(
            m.arena(&[Range::new(PAGE + 1, PAGE).unwrap()], PAGE, 16 * PAGE),
            Err(Error::Invalid)
        );
    }
    fn pe_image(flags: u32) -> [u8; 8192] {
        let mut b = [0; 8192];
        b[..2].copy_from_slice(b"MZ");
        b[60..64].copy_from_slice(&128u32.to_le_bytes());
        b[128..132].copy_from_slice(b"PE\0\0");
        b[132..134].copy_from_slice(&0x8664u16.to_le_bytes());
        b[134..136].copy_from_slice(&1u16.to_le_bytes());
        b[148..150].copy_from_slice(&112u16.to_le_bytes());
        b[152..154].copy_from_slice(&0x20bu16.to_le_bytes());
        b[184..188].copy_from_slice(&4096u32.to_le_bytes());
        b[272..276].copy_from_slice(&16u32.to_le_bytes());
        b[276..280].copy_from_slice(&4096u32.to_le_bytes());
        b[300..304].copy_from_slice(&flags.to_le_bytes());
        b
    }
    #[test]
    fn pe_section_permissions() {
        let b = pe_image(0x60000000);
        let image = Image::new(&b).unwrap();
        let s = image.section(0).unwrap();
        assert!(s.executable);
        assert!(!s.writable);
        assert_eq!(s.offset, 4096);
        let b = pe_image(0xc0000000);
        let s = Image::new(&b).unwrap().section(0).unwrap();
        assert!(s.writable);
        assert!(!s.executable);
    }
    #[test]
    fn rejects_writable_code_and_bad_pe() {
        assert!(Image::new(&pe_image(0xe0000000)).is_err());
        assert!(Image::new(&[0; 8]).is_err());
        let mut b = pe_image(0x60000000);
        b[276..280].copy_from_slice(&8192u32.to_le_bytes());
        assert!(Image::new(&b).is_err());
        let mut b = pe_image(0x60000000);
        b[134..136].copy_from_slice(&2u16.to_le_bytes());
        b[312..316].copy_from_slice(&32u32.to_le_bytes());
        b[316..320].copy_from_slice(&4096u32.to_le_bytes());
        b[340..344].copy_from_slice(&0x40000000u32.to_le_bytes());
        assert!(matches!(Image::new(&b), Err(Error::Overlap)));
    }
    extern crate std;
}
