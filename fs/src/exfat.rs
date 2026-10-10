// Copyright 2026 Hirofumi Iwasaki
// SPDX-License-Identifier: Apache-2.0
//! Bounded read-only exFAT parser for an explicitly selected volume region.
//! The backend must remain immutable for the entire mounted session. This is a
//! synchronous parser, not the application's future asynchronous device API.
use super::{Error, Reader, u16le, u32le};

pub const MAX_FILES: usize = 128;
pub const MAX_FILE_BYTES: u64 = 64 * 1024 * 1024;
const ROOT_LIMIT: u64 = 1024 * 1024;
const IO_LIMIT: usize = 1_000_000;
fn u64le(b: &[u8], i: usize) -> u64 {
    u64::from_le_bytes(b[i..i + 8].try_into().unwrap())
}
fn sum32(sum: u32, b: u8) -> u32 {
    sum.rotate_right(1).wrapping_add(b as u32)
}
fn sum16(sum: u16, b: u8) -> u16 {
    sum.rotate_right(1).wrapping_add(b as u16)
}

#[derive(Clone, Copy)]
pub struct Entry {
    name: [u16; 255],
    name_len: u8,
    length: u64,
    valid: u64,
    first: u32,
    contiguous: bool,
}
impl Entry {
    const EMPTY: Self = Self {
        name: [0; 255],
        name_len: 0,
        length: 0,
        valid: 0,
        first: 0,
        contiguous: false,
    };
    pub fn name(&self) -> &[u16] {
        &self.name[..self.name_len as usize]
    }
    pub fn len(&self) -> u64 {
        self.length
    }
    pub fn is_empty(&self) -> bool {
        self.length == 0
    }
}
/// Allocate outside a small runtime stack. Initialization is separate from mount.
pub struct Workspace {
    upper: [u16; 65536],
    entries: [Entry; MAX_FILES],
}
impl Workspace {
    pub const fn new() -> Self {
        Self {
            upper: [0; 65536],
            entries: [Entry::EMPTY; MAX_FILES],
        }
    }
}
impl Default for Workspace {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Copy)]
struct Volume {
    start: u64,
    total: u64,
    fat: u64,
    heap: u64,
    count: u32,
    spc: u32,
    root: u32,
}
impl Volume {
    fn parse(b: &[u8], start: u64, length: u64, size: usize) -> Result<Self, Error> {
        if &b[3..11] != b"EXFAT   " {
            return Err(Error::Unsupported);
        }
        if b[..3] != [0xeb, 0x76, 0x90]
            || b[11..64].iter().any(|&x| x != 0)
            || b[510..512] != [0x55, 0xaa]
        {
            return Err(Error::Corrupt);
        }
        if u16le(b, 104) != 0x100 || b[110] != 1 {
            return Err(Error::Unsupported);
        }
        if !matches!(b[108], 9 | 12) || 1usize << b[108] != size {
            return Err(Error::Unsupported);
        }
        if b[109] > 25 - b[108] || u16le(b, 106) & 7 != 0 || !(b[112] <= 100 || b[112] == 255) {
            return Err(Error::Corrupt);
        }
        let total = u64le(b, 72);
        let fat = u32le(b, 80) as u64;
        let fatlen = u32le(b, 84) as u64;
        let heap = u32le(b, 88) as u64;
        let count = u32le(b, 92);
        let root = u32le(b, 96);
        let spc = 1u32 << b[109];
        if u64le(b, 64) != start
            || total < 1024 * 1024 / size as u64
            || total > length
            || fat < 24
            || fatlen * (size as u64) < (count as u64 + 2) * 4
            || heap < fat + fatlen
            || count == 0
            || count > 0xfffffff5
            || heap + count as u64 * spc as u64 > total
            || root < 2
            || root as u64 >= count as u64 + 2
        {
            return Err(Error::Corrupt);
        }
        Ok(Self {
            start,
            total,
            fat,
            heap,
            count,
            spc,
            root,
        })
    }
    fn cluster(&self, n: u32) -> Result<u64, Error> {
        if n < 2 || n as u64 >= self.count as u64 + 2 {
            return Err(Error::Corrupt);
        }
        let rel = self.heap + (n as u64 - 2) * self.spc as u64;
        if rel + self.spc as u64 > self.total {
            return Err(Error::Corrupt);
        }
        Ok(self.start + rel)
    }
    fn next<F: FnMut(u64, &mut [u8]) -> Result<(), Error>>(
        &self,
        r: &mut Reader<'_, F>,
        n: u32,
    ) -> Result<Option<u32>, Error> {
        self.cluster(n)?;
        let off = n as u64 * 4;
        let mut b = [0; 4096];
        r.sector(self.start + self.fat + off / r.size as u64, &mut b)?;
        let next = u32le(&b, (off % r.size as u64) as usize);
        if next == 0xffffffff {
            return Ok(None);
        }
        self.cluster(next)?;
        Ok(Some(next))
    }
}
// Brent cycle detection, constant space; the independent byte/IO budgets also
// bound malformed chains. Reserved/bad FAT values are rejected by cluster().
struct Chain {
    current: u32,
    anchor: u32,
    power: u64,
    distance: u64,
}
impl Chain {
    fn new(first: u32) -> Self {
        Self {
            current: first,
            anchor: first,
            power: 1,
            distance: 0,
        }
    }
    fn advance<F: FnMut(u64, &mut [u8]) -> Result<(), Error>>(
        &mut self,
        v: Volume,
        r: &mut Reader<'_, F>,
    ) -> Result<bool, Error> {
        let Some(next) = v.next(r, self.current)? else {
            return Ok(false);
        };
        self.distance += 1;
        if next == self.anchor {
            return Err(Error::Corrupt);
        }
        self.current = next;
        if self.distance == self.power {
            self.anchor = next;
            self.power *= 2;
            self.distance = 0;
        }
        Ok(true)
    }
}
struct Stream {
    chain: Chain,
    offset: u64,
    sector: u32,
    loaded: bool,
    buffer: [u8; 4096],
}
impl Stream {
    fn new(first: u32) -> Self {
        Self {
            chain: Chain::new(first),
            offset: 0,
            sector: 0,
            loaded: false,
            buffer: [0; 4096],
        }
    }
    fn byte<F: FnMut(u64, &mut [u8]) -> Result<(), Error>>(
        &mut self,
        v: Volume,
        r: &mut Reader<'_, F>,
    ) -> Result<Option<u8>, Error> {
        let index = (self.offset % r.size as u64) as usize;
        if !self.loaded {
            if self.sector == v.spc {
                if !self.chain.advance(v, r)? {
                    return Ok(None);
                }
                self.sector = 0;
            }
            r.sector(
                v.cluster(self.chain.current)? + self.sector as u64,
                &mut self.buffer,
            )?;
            self.loaded = true;
        }
        let byte = self.buffer[index];
        self.offset += 1;
        if index + 1 == r.size {
            self.loaded = false;
            self.sector += 1;
        }
        Ok(Some(byte))
    }
    fn entry<F: FnMut(u64, &mut [u8]) -> Result<(), Error>>(
        &mut self,
        v: Volume,
        r: &mut Reader<'_, F>,
    ) -> Result<Option<[u8; 32]>, Error> {
        if self.offset >= ROOT_LIMIT {
            return Err(Error::Limit);
        }
        let mut b = [0; 32];
        let Some(first) = self.byte(v, r)? else {
            return Ok(None);
        };
        b[0] = first;
        for x in &mut b[1..] {
            *x = self.byte(v, r)?.ok_or(Error::Corrupt)?;
        }
        Ok(Some(b))
    }
}
#[derive(Clone, Copy)]
struct Allocation {
    first: u32,
    length: u64,
}

/// Owns the read callback and borrows all long-lived metadata. No heap or writes.
pub struct FileSystem<'a, F> {
    read: F,
    blocks: u64,
    size: usize,
    volume: Volume,
    bitmap: Allocation,
    work: &'a mut Workspace,
    count: usize,
}
impl<'a, F: FnMut(u64, &mut [u8]) -> Result<(), Error>> FileSystem<'a, F> {
    /// Select one exFAT volume from superfloppy, primary MBR, or GPT Basic Data.
    /// Multiple exFAT volumes are ambiguous and return Unsupported. No fallback
    /// to another volume is attempted after a selected exFAT fails validation.
    pub fn discover(
        blocks: u64,
        size: usize,
        mut read: F,
        work: &'a mut Workspace,
    ) -> Result<Self, Error> {
        let (start, length) = select_volume(blocks, size, &mut read)?;
        Self::mount(blocks, size, start, length, read, work)
    }
    /// `start/length` are sector units, supplied by a trusted partition selector.
    /// On error the workspace is unspecified; no mounted object is returned.
    pub fn mount(
        blocks: u64,
        size: usize,
        start: u64,
        length: u64,
        mut read: F,
        work: &'a mut Workspace,
    ) -> Result<Self, Error> {
        if !matches!(size, 512 | 4096) {
            return Err(Error::Unsupported);
        }
        if length < 24 || start.checked_add(length).is_none_or(|end| end > blocks) {
            return Err(Error::Corrupt);
        }
        let mut r = Reader {
            read: &mut read,
            blocks,
            size,
            budget: IO_LIMIT,
        };
        let mut b = [0; 4096];
        r.sector(start, &mut b)?;
        let v = Volume::parse(&b, start, length, size)?;
        let mut checksum = 0;
        for sector in 0..11 {
            if sector != 0 {
                r.sector(start + sector, &mut b)?;
            }
            if (1..=8).contains(&sector) && u32le(&b, size - 4) != 0xaa550000 {
                return Err(Error::Corrupt);
            }
            for (i, &byte) in b[..size].iter().enumerate() {
                if sector != 0 || !matches!(i, 106 | 107 | 112) {
                    checksum = sum32(checksum, byte);
                }
            }
        }
        r.sector(start + 11, &mut b)?;
        if b[..size].chunks_exact(4).any(|x| u32le(x, 0) != checksum) {
            return Err(Error::Corrupt);
        }
        r.sector(start + v.fat, &mut b)?;
        if u32le(&b, 0) != 0xfffffff8 || u32le(&b, 4) != 0xffffffff {
            return Err(Error::Corrupt);
        }
        let mut dir = Stream::new(v.root);
        let mut bitmap = None;
        let mut upper = None;
        while let Some(e) = dir.entry(v, &mut r)? {
            if e[0] == 0 {
                break;
            }
            match e[0] {
                0x81 => {
                    if bitmap.is_some() || e[1] & 1 != 0 {
                        return Err(Error::Corrupt);
                    }
                    bitmap = Some(Allocation {
                        first: u32le(&e, 20),
                        length: u64le(&e, 24),
                    });
                }
                0x82 => {
                    if upper.is_some() {
                        return Err(Error::Corrupt);
                    }
                    upper = Some((
                        Allocation {
                            first: u32le(&e, 20),
                            length: u64le(&e, 24),
                        },
                        u32le(&e, 4),
                    ));
                }
                0x85 => {
                    if e[1] < 2 {
                        return Err(Error::Corrupt);
                    }
                    for _ in 0..e[1] {
                        let s = dir.entry(v, &mut r)?.ok_or(Error::Corrupt)?;
                        if s[0] & 0xc0 != 0xc0 {
                            return Err(Error::Corrupt);
                        }
                    }
                }
                0x83 => {}
                0xc0 | 0xc1 => return Err(Error::Corrupt),
                t if t & 0x80 == 0 || t & 0x20 != 0 => {}
                _ => return Err(Error::Unsupported),
            }
        }
        let bitmap = bitmap.ok_or(Error::Corrupt)?;
        if bitmap.length != (v.count as u64).div_ceil(8) {
            return Err(Error::Corrupt);
        }
        let (up, expected) = upper.ok_or(Error::Corrupt)?;
        if up.length == 0 || up.length > 131072 || up.length % 2 != 0 {
            return Err(Error::Corrupt);
        }
        validate_chain(v, &mut r, bitmap.first, bitmap.length, false, None)?;
        validate_chain(v, &mut r, up.first, up.length, false, Some(bitmap))?;
        // Validate the full root chain even if an end marker appears early.
        validate_root(v, &mut r, bitmap)?;
        let mut source = Stream::new(up.first);
        let mut checksum = 0;
        let mut index = 0usize;
        let mut skip = false;
        for _ in 0..up.length / 2 {
            let lo = source.byte(v, &mut r)?.ok_or(Error::Corrupt)?;
            let hi = source.byte(v, &mut r)?.ok_or(Error::Corrupt)?;
            checksum = sum32(sum32(checksum, lo), hi);
            let word = u16::from_le_bytes([lo, hi]);
            if skip {
                if word == 0 || index + word as usize > 65536 {
                    return Err(Error::Corrupt);
                }
                for _ in 0..word {
                    work.upper[index] = index as u16;
                    index += 1;
                }
                skip = false;
            } else if word == 0xffff && index < 65535 {
                skip = true;
            } else {
                if index >= 65536 {
                    return Err(Error::Corrupt);
                }
                work.upper[index] = word;
                index += 1;
            }
        }
        if skip || index != 65536 || checksum != expected {
            return Err(Error::Corrupt);
        }
        for i in 0..128 {
            if work.upper[i]
                != if (97..=122).contains(&i) {
                    (i - 32) as u16
                } else {
                    i as u16
                }
            {
                return Err(Error::Corrupt);
            }
        }
        // The bitmap must describe its own allocated storage, too.
        validate_chain(v, &mut r, bitmap.first, bitmap.length, false, Some(bitmap))?;
        let count = parse_directory(v, &mut r, work)?;
        Ok(Self {
            read,
            blocks,
            size,
            volume: v,
            bitmap,
            work,
            count,
        })
    }
    pub fn entries(&self) -> &[Entry] {
        &self.work.entries[..self.count]
    }
    /// At most 4096 bytes. Output remains unchanged on failure. Index belongs to
    /// this mount only. Each call revalidates allocation; backend must be stable.
    pub fn read_at(&mut self, index: usize, offset: u64, out: &mut [u8]) -> Result<usize, Error> {
        let e = *self.entries().get(index).ok_or(Error::NotFound)?;
        if e.length > MAX_FILE_BYTES {
            return Err(Error::TooLarge);
        }
        if out.len() > 4096 {
            return Err(Error::Limit);
        }
        let mut r = Reader {
            read: &mut self.read,
            blocks: self.blocks,
            size: self.size,
            budget: IO_LIMIT,
        };
        validate_chain(
            self.volume,
            &mut r,
            e.first,
            e.length,
            e.contiguous,
            Some(self.bitmap),
        )?;
        let n = e.length.saturating_sub(offset).min(out.len() as u64) as usize;
        let mut result = [0; 4096];
        let valid = e.valid.saturating_sub(offset).min(n as u64) as usize;
        let cb = self.size as u64 * self.volume.spc as u64;
        let mut chain = Chain::new(e.first);
        if valid > 0 {
            let jump = offset / cb;
            if e.contiguous {
                chain.current = (e.first as u64 + jump) as u32;
            } else {
                for _ in 0..jump {
                    if !chain.advance(self.volume, &mut r)? {
                        return Err(Error::Corrupt);
                    }
                }
            }
            let mut pos = offset % cb;
            let mut done = 0;
            let mut sector = [0; 4096];
            while done < valid {
                if pos == cb {
                    if e.contiguous {
                        chain.current += 1;
                    } else if !chain.advance(self.volume, &mut r)? {
                        return Err(Error::Corrupt);
                    }
                    pos = 0;
                }
                r.sector(
                    self.volume.cluster(chain.current)? + pos / self.size as u64,
                    &mut sector,
                )?;
                let within = (pos % self.size as u64) as usize;
                let take = (self.size - within).min(valid - done);
                result[done..done + take].copy_from_slice(&sector[within..within + take]);
                done += take;
                pos += take as u64;
            }
        }
        out[..n].copy_from_slice(&result[..n]);
        Ok(n)
    }
}
fn allocated<F: FnMut(u64, &mut [u8]) -> Result<(), Error>>(
    v: Volume,
    r: &mut Reader<'_, F>,
    bitmap: Allocation,
    n: u32,
) -> Result<(), Error> {
    v.cluster(n)?;
    let byte = (n as u64 - 2) / 8;
    if byte >= bitmap.length {
        return Err(Error::Corrupt);
    }
    let cb = r.size as u64 * v.spc as u64;
    let mut chain = Chain::new(bitmap.first);
    for _ in 0..byte / cb {
        if !chain.advance(v, r)? {
            return Err(Error::Corrupt);
        }
    }
    let mut b = [0; 4096];
    r.sector(
        v.cluster(chain.current)? + (byte % cb) / r.size as u64,
        &mut b,
    )?;
    if b[(byte % r.size as u64) as usize] & (1 << ((n - 2) % 8)) == 0 {
        return Err(Error::Corrupt);
    }
    Ok(())
}
fn validate_chain<F: FnMut(u64, &mut [u8]) -> Result<(), Error>>(
    v: Volume,
    r: &mut Reader<'_, F>,
    first: u32,
    length: u64,
    contiguous: bool,
    bitmap: Option<Allocation>,
) -> Result<(), Error> {
    if length == 0 {
        return if first == 0 {
            Ok(())
        } else {
            Err(Error::Corrupt)
        };
    }
    let count = length.div_ceil(r.size as u64 * v.spc as u64);
    if count > v.count as u64 {
        return Err(Error::Corrupt);
    }
    let mut chain = Chain::new(first);
    for i in 0..count {
        v.cluster(chain.current)?;
        if let Some(b) = bitmap {
            allocated(v, r, b, chain.current)?;
        }
        if contiguous {
            if i + 1 < count {
                chain.current = chain.current.checked_add(1).ok_or(Error::Corrupt)?;
            }
        } else if chain.advance(v, r)? != (i + 1 < count) {
            return Err(Error::Corrupt);
        }
    }
    Ok(())
}
fn validate_root<F: FnMut(u64, &mut [u8]) -> Result<(), Error>>(
    v: Volume,
    r: &mut Reader<'_, F>,
    bitmap: Allocation,
) -> Result<(), Error> {
    let mut chain = Chain::new(v.root);
    let max = ROOT_LIMIT.div_ceil(r.size as u64 * v.spc as u64);
    for _ in 0..max {
        allocated(v, r, bitmap, chain.current)?;
        if !chain.advance(v, r)? {
            return Ok(());
        }
    }
    Err(Error::Limit)
}
fn parse_directory<F: FnMut(u64, &mut [u8]) -> Result<(), Error>>(
    v: Volume,
    r: &mut Reader<'_, F>,
    w: &mut Workspace,
) -> Result<usize, Error> {
    let mut dir = Stream::new(v.root);
    let mut count = 0;
    while let Some(primary) = dir.entry(v, r)? {
        if primary[0] == 0 {
            break;
        }
        if primary[0] != 0x85 {
            continue;
        }
        if primary[1] < 2 {
            return Err(Error::Corrupt);
        }
        let expected = u16le(&primary, 2);
        let mut sum = 0;
        for (i, &b) in primary.iter().enumerate() {
            if !matches!(i, 2 | 3) {
                sum = sum16(sum, b);
            }
        }
        let mut entry = Entry::EMPTY;
        let mut namehash = 0;
        let mut required = 0;
        let mut units = 0;
        for i in 0..primary[1] as usize {
            let s = dir.entry(v, r)?.ok_or(Error::Corrupt)?;
            for &b in &s {
                sum = sum16(sum, b);
            }
            if i == 0 {
                if s[0] != 0xc0 || s[1] & !3 != 0 || s[1] & 1 == 0 || s[3] == 0 {
                    return Err(Error::Corrupt);
                }
                entry.name_len = s[3];
                entry.length = u64le(&s, 24);
                entry.valid = u64le(&s, 8);
                entry.first = u32le(&s, 20);
                entry.contiguous = s[1] & 2 != 0;
                namehash = u16le(&s, 4);
                required = (s[3] as usize).div_ceil(15);
                if entry.valid > entry.length || primary[1] as usize <= required {
                    return Err(Error::Corrupt);
                }
                if entry.length == 0 {
                    if entry.first != 0 || entry.contiguous {
                        return Err(Error::Corrupt);
                    }
                } else {
                    v.cluster(entry.first)?;
                }
            } else if i <= required {
                if s[0] != 0xc1 || s[1] & 3 != 0 {
                    return Err(Error::Corrupt);
                }
                for j in 0..15 {
                    let ch = u16le(&s, 2 + j * 2);
                    if units < entry.name_len as usize {
                        entry.name[units] = ch;
                        units += 1;
                    }
                }
            } else if s[0] & 0xe0 != 0xe0 {
                return Err(Error::Unsupported);
            }
        }
        if sum != expected || units != entry.name_len as usize {
            return Err(Error::Corrupt);
        }
        let mut hash = 0;
        for &ch in entry.name() {
            if ch < 32 || matches!(ch, 34 | 42 | 47 | 58 | 60 | 62 | 63 | 92 | 124) {
                return Err(Error::Corrupt);
            }
            for byte in w.upper[ch as usize].to_le_bytes() {
                hash = sum16(hash, byte);
            }
        }
        if entry.name() == [46]
            || entry.name() == [46, 46]
            || core::char::decode_utf16(entry.name().iter().copied()).any(|c| c.is_err())
            || hash != namehash
        {
            return Err(Error::Corrupt);
        }
        if u16le(&primary, 4) & 16 != 0 {
            // Subdirectories are not exposed by this API.
            if entry.valid != entry.length {
                return Err(Error::Corrupt);
            }
            continue;
        }
        for old in &w.entries[..count] {
            if old.name_len == entry.name_len
                && old
                    .name()
                    .iter()
                    .zip(entry.name())
                    .all(|(&a, &b)| w.upper[a as usize] == w.upper[b as usize])
            {
                return Err(Error::Corrupt);
            }
        }
        if count == MAX_FILES {
            return Err(Error::Limit);
        }
        w.entries[count] = entry;
        count += 1;
    }
    Ok(count)
}

const BASIC_DATA: [u8; 16] = [
    0xa2, 0xa0, 0xd0, 0xeb, 0xe5, 0xb9, 0x33, 0x44, 0x87, 0xc0, 0x68, 0xb6, 0xb7, 0x26, 0x99, 0xc7,
];
fn select_volume<F: FnMut(u64, &mut [u8]) -> Result<(), Error>>(
    blocks: u64,
    size: usize,
    read: &mut F,
) -> Result<(u64, u64), Error> {
    if !matches!(size, 512 | 4096) {
        return Err(Error::Unsupported);
    }
    if blocks == 0 {
        return Err(Error::Corrupt);
    }
    let mut r = Reader {
        read,
        blocks,
        size,
        budget: IO_LIMIT,
    };
    let mut boot = [0; 4096];
    r.sector(0, &mut boot)?;
    if &boot[3..11] == b"EXFAT   " {
        return Ok((0, blocks));
    }
    if boot[510..512] != [0x55, 0xaa] {
        return Err(Error::Unsupported);
    }
    let mut candidates = [(0, 0); 128];
    let count = if (0..4).any(|i| boot[446 + i * 16 + 4] == 0xee) {
        super::gpt::partitions(&mut r, &boot, &BASIC_DATA, &mut candidates)?
    } else {
        let mut used = [(0u64, 0u64); 4];
        let mut used_count = 0;
        let mut selected = 0;
        for e in boot[446..510].chunks_exact(16) {
            if e == [0; 16] {
                continue;
            }
            let first = u32le(e, 8) as u64;
            let length = u32le(e, 12) as u64;
            if !matches!(e[0], 0 | 128)
                || e[4] == 0
                || first == 0
                || length == 0
                || first + length > blocks
                || used[..used_count]
                    .iter()
                    .any(|&(a, b)| first < a + b && a < first + length)
            {
                return Err(Error::Corrupt);
            }
            used[used_count] = (first, length);
            used_count += 1;
            if matches!(e[4], 0x05 | 0x0f | 0x85) {
                return Err(Error::Unsupported);
            }
            if e[4] == 0x07 {
                candidates[selected] = (first, length);
                selected += 1;
            }
        }
        selected
    };
    let mut selected = None;
    for &(start, length) in &candidates[..count] {
        r.sector(start, &mut boot)?;
        if &boot[3..11] == b"EXFAT   " {
            if selected.is_some() {
                return Err(Error::Unsupported);
            }
            selected = Some((start, length));
        }
    }
    selected.ok_or(Error::Unsupported)
}
