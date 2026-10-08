// Copyright 2026 Hirofumi Iwasaki
// SPDX-License-Identifier: Apache-2.0
#![no_std]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    Io,
    Unsupported,
    Corrupt,
    NotFound,
    TooLarge,
    Limit,
}
fn u16le(b: &[u8], i: usize) -> u16 {
    u16::from_le_bytes([b[i], b[i + 1]])
}
fn u32le(b: &[u8], i: usize) -> u32 {
    u32::from_le_bytes(b[i..i + 4].try_into().unwrap())
}
struct Reader<'a, F> {
    read: &'a mut F,
    blocks: u64,
    size: usize,
    budget: usize,
}
impl<F: FnMut(u64, &mut [u8]) -> Result<(), Error>> Reader<'_, F> {
    fn sector(&mut self, lba: u64, b: &mut [u8; 4096]) -> Result<(), Error> {
        if lba >= self.blocks {
            return Err(Error::Corrupt);
        }
        if self.budget == 0 {
            return Err(Error::Limit);
        }
        self.budget -= 1;
        (self.read)(lba, &mut b[..self.size])
    }
}
#[derive(Clone, Copy)]
struct Volume {
    start: u64,
    total: u64,
    fat: u64,
    data: u64,
    clusters: u32,
    spc: u32,
    root: u32,
}
impl Volume {
    fn parse(b: &[u8], start: u64, partition: u64, size: usize) -> Result<Self, Error> {
        if b[510..512] != [0x55, 0xaa] || !matches!(b[0], 0xeb | 0xe9) {
            return Err(Error::Unsupported);
        }
        if u16le(b, 11) as usize != size
            || b[13] == 0
            || !b[13].is_power_of_two()
            || b[13] > 128
            || u16le(b, 14) == 0
            || !matches!(b[16], 1 | 2)
        {
            return Err(Error::Corrupt);
        }
        let spc = b[13] as u32;
        let reserved = u16le(b, 14) as u64;
        let total = if u16le(b, 19) != 0 {
            u16le(b, 19) as u64
        } else {
            u32le(b, 32) as u64
        };
        let fatsize = if u16le(b, 22) != 0 {
            u16le(b, 22) as u64
        } else {
            u32le(b, 36) as u64
        };
        let rootsecs = (u16le(b, 17) as u64 * 32).div_ceil(size as u64);
        let overhead = reserved + fatsize * b[16] as u64 + rootsecs;
        if total > partition || total <= overhead || fatsize == 0 {
            return Err(Error::Corrupt);
        }
        let clusters = (total - overhead) / spc as u64;
        if clusters < 65525 {
            return Err(Error::Unsupported);
        }
        if clusters > 0x0fffffee
            || u16le(b, 17) != 0
            || u16le(b, 19) != 0
            || u16le(b, 22) != 0
            || u16le(b, 42) != 0
            || fatsize * size as u64 / 4 < clusters + 2
        {
            return Err(Error::Corrupt);
        }
        let flags = u16le(b, 40);
        if flags & !0x008f != 0 {
            return Err(Error::Corrupt);
        }
        let active = if flags & 128 != 0 {
            (flags & 15) as u64
        } else {
            0
        };
        if active >= b[16] as u64 {
            return Err(Error::Corrupt);
        }
        let root = u32le(b, 44) & 0x0fffffff;
        if root < 2 || root as u64 >= clusters + 2 {
            return Err(Error::Corrupt);
        }
        Ok(Self {
            start,
            total,
            fat: start + reserved + active * fatsize,
            data: start + overhead,
            clusters: clusters as u32,
            spc,
            root,
        })
    }
    fn cluster(&self, n: u32) -> Result<u64, Error> {
        if n < 2 || n >= self.clusters + 2 {
            return Err(Error::Corrupt);
        }
        let lba = self.data + (n as u64 - 2) * self.spc as u64;
        if lba + self.spc as u64 > self.start + self.total {
            return Err(Error::Corrupt);
        }
        Ok(lba)
    }
    fn next<F: FnMut(u64, &mut [u8]) -> Result<(), Error>>(
        &self,
        r: &mut Reader<'_, F>,
        n: u32,
    ) -> Result<Option<u32>, Error> {
        self.cluster(n)?;
        let index = n as u64 * 4;
        let mut sector = [0u8; 4096];
        r.sector(self.fat + index / r.size as u64, &mut sector)?;
        let value = u32le(&sector, index as usize % r.size) & 0x0fffffff;
        if value >= 0x0ffffff8 {
            return Ok(None);
        }
        self.cluster(value)?;
        Ok(Some(value))
    }
}
fn visit(seen: &mut [u32; 128], count: &mut usize, cluster: u32) -> Result<(), Error> {
    if seen[..*count].contains(&cluster) {
        return Err(Error::Corrupt);
    }
    if *count == seen.len() {
        return Err(Error::Limit);
    }
    seen[*count] = cluster;
    *count += 1;
    Ok(())
}
/// Reads a root-directory short-name file from FAT32, never writes to the disk.
/// Supports superfloppy or exactly one primary MBR FAT32 partition.
/// sector_size must match the block device (512 or 4096). At most 1024 reads.
pub fn read_root<F: FnMut(u64, &mut [u8]) -> Result<(), Error>>(
    blocks: u64,
    sector_size: usize,
    name: &[u8; 11],
    out: &mut [u8],
    read: &mut F,
) -> Result<usize, Error> {
    if !matches!(sector_size, 512 | 4096) || blocks == 0 {
        return Err(Error::Unsupported);
    }
    let mut r = Reader {
        read,
        blocks,
        size: sector_size,
        budget: 1024,
    };
    let mut boot = [0u8; 4096];
    r.sector(0, &mut boot)?;
    let direct = if matches!(boot[0], 0xeb | 0xe9) {
        Some(Volume::parse(&boot, 0, blocks, sector_size))
    } else {
        None
    };
    let volume = if let Some(Ok(volume)) = direct {
        volume
    } else {
        if boot[510..512] != [0x55, 0xaa] {
            return Err(Error::Unsupported);
        }
        let mut partition = None;
        for i in 0..4 {
            let off = 446 + i * 16;
            let kind = boot[off + 4];
            if kind == 0xee {
                return Err(Error::Unsupported);
            } // GPT requires a separate validator
            if matches!(kind, 0x0b | 0x0c) {
                if partition.is_some() || !matches!(boot[off], 0 | 128) {
                    return Err(Error::Corrupt);
                }
                let start = u32le(&boot, off + 8) as u64;
                let length = u32le(&boot, off + 12) as u64;
                if start == 0 || length == 0 || start + length > blocks {
                    return Err(Error::Corrupt);
                }
                partition = Some((start, length));
            }
        }
        let (start, length) = partition.ok_or_else(|| {
            direct
                .and_then(|result| result.err())
                .unwrap_or(Error::Unsupported)
        })?;
        r.sector(start, &mut boot)?;
        Volume::parse(&boot, start, length, sector_size)?
    };
    let mut seen = [0u32; 128];
    let mut count = 0;
    let mut cluster = volume.root;
    let (first, length) = 'directory: loop {
        visit(&mut seen, &mut count, cluster)?;
        let start = volume.cluster(cluster)?;
        for index in 0..volume.spc {
            r.sector(start + index as u64, &mut boot)?;
            for e in boot[..sector_size].chunks_exact(32) {
                if e[0] == 0 {
                    return Err(Error::NotFound);
                }
                if e[0] == 0xe5 || e[11] & 0x18 != 0 || e[11] & 0x0f == 0x0f {
                    continue;
                }
                if &e[..11] == name {
                    let first = ((u16le(e, 20) as u32) << 16 | u16le(e, 26) as u32) & 0x0fffffff;
                    break 'directory (first, u32le(e, 28) as usize);
                }
            }
        }
        cluster = volume.next(&mut r, cluster)?.ok_or(Error::NotFound)?;
    };
    if length > out.len() {
        return Err(Error::TooLarge);
    }
    if length == 0 {
        return if first == 0 {
            Ok(0)
        } else {
            Err(Error::Corrupt)
        };
    }
    let mut seen = [0u32; 128];
    let mut count = 0;
    let mut cluster = first;
    let mut offset = 0;
    loop {
        visit(&mut seen, &mut count, cluster)?;
        let start = volume.cluster(cluster)?;
        for index in 0..volume.spc {
            r.sector(start + index as u64, &mut boot)?;
            let bytes = (length - offset).min(sector_size);
            out[offset..offset + bytes].copy_from_slice(&boot[..bytes]);
            offset += bytes;
            if offset == length {
                if volume.next(&mut r, cluster)?.is_some() {
                    return Err(Error::Corrupt);
                }
                return Ok(length);
            }
        }
        cluster = volume.next(&mut r, cluster)?.ok_or(Error::Corrupt)?;
    }
}
#[cfg(test)]
mod tests;
