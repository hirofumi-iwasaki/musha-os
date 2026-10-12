// Copyright 2026 Hirofumi Iwasaki
// SPDX-License-Identifier: Apache-2.0
//! Root short-name catalog and bounded reads from a validated FAT32 region.
use crate::{Error, Reader, Volume, u16le, u32le, visit};
pub const MAX_FILES: usize = 128;
pub const MAX_FILE_BYTES: u64 = 64 * 1024 * 1024;
#[derive(Clone, Copy)]
pub struct Entry {
    name: [u8; 11],
    first: u32,
    bytes: u64,
}
impl Entry {
    /// Raw padded on-disk 8.3 name. OEM bytes must be escaped by the UI.
    pub fn name(&self) -> &[u8; 11] {
        &self.name
    }
    pub fn len(&self) -> u64 {
        self.bytes
    }
    pub fn is_empty(&self) -> bool {
        self.bytes == 0
    }
}
pub struct FileSystem<F> {
    read: F,
    blocks: u64,
    size: usize,
    volume: Volume,
    entries: [Entry; MAX_FILES],
    count: usize,
}
impl<F: FnMut(u64, &mut [u8]) -> Result<(), Error>> FileSystem<F> {
    /// The caller supplies a partition region, or the entire superfloppy.
    /// Backend contents must remain immutable until this mount is discarded.
    pub fn mount(
        blocks: u64,
        size: usize,
        start: u64,
        length: u64,
        mut read: F,
    ) -> Result<Self, Error> {
        if !matches!(size, 512 | 4096) {
            return Err(Error::Unsupported);
        }
        if length == 0 || start.checked_add(length).is_none_or(|end| end > blocks) {
            return Err(Error::Corrupt);
        }
        let mut r = Reader {
            read: &mut read,
            blocks,
            size,
            budget: 1_000_000,
        };
        let mut sector = [0; 4096];
        r.sector(start, &mut sector)?;
        let volume = Volume::parse(&sector, start, length, size)?;
        let mut entries = [Entry {
            name: [0; 11],
            first: 0,
            bytes: 0,
        }; MAX_FILES];
        let mut count = 0;
        let mut cluster = volume.root;
        let mut seen = [0; 128];
        let mut visited = 0;
        let mut scanned = 0;
        let mut ended = false;
        loop {
            visit(&mut seen, &mut visited, cluster)?;
            let lba = volume.cluster(cluster)?;
            for i in 0..volume.spc {
                scanned += size;
                if scanned > 1024 * 1024 {
                    return Err(Error::Limit);
                }
                if ended {
                    continue;
                }
                r.sector(lba + i as u64, &mut sector)?;
                for raw in sector[..size].chunks_exact(32) {
                    if raw[0] == 0 {
                        ended = true;
                        break;
                    }
                    if raw[0] == 0xe5 || raw[11] & 0x18 != 0 || raw[11] & 0x0f == 0x0f {
                        continue;
                    }
                    if count == MAX_FILES {
                        return Err(Error::Limit);
                    }
                    let mut name: [u8; 11] = raw[..11].try_into().unwrap();
                    if name[0] == 5 {
                        name[0] = 0xe5;
                    }
                    if entries[..count]
                        .iter()
                        .any(|e| e.name.eq_ignore_ascii_case(&name))
                    {
                        return Err(Error::Corrupt);
                    }
                    let first = (u16le(raw, 20) as u32) << 16 | u16le(raw, 26) as u32;
                    if first & 0xf0000000 != 0 {
                        return Err(Error::Corrupt);
                    }
                    let bytes = u32le(raw, 28) as u64;
                    if bytes == 0 {
                        if first != 0 {
                            return Err(Error::Corrupt);
                        }
                    } else {
                        volume.cluster(first)?;
                    }
                    entries[count] = Entry { name, first, bytes };
                    count += 1;
                }
            }
            match volume.next(&mut r, cluster)? {
                Some(next) => cluster = next,
                None => break,
            }
        }
        Ok(Self {
            read,
            blocks,
            size,
            volume,
            entries,
            count,
        })
    }
    pub fn entries(&self) -> &[Entry] {
        &self.entries[..self.count]
    }
    /// At most 4096 bytes, atomic caller-buffer update. Each call validates the
    /// full declared chain, including its exact end; cyclic/long/short chains fail.
    pub fn read_at(&mut self, index: usize, offset: u64, out: &mut [u8]) -> Result<usize, Error> {
        let plan = self.plan_read(index, offset, out.len())?;
        plan.execute(&mut self.read, out)
    }
    /// Validates metadata and maps a range, without reading file data sectors.
    /// Metadata I/O is still synchronous; do not call from an application step.
    pub fn plan_read(
        &mut self,
        index: usize,
        offset: u64,
        requested: usize,
    ) -> Result<crate::read_plan::Plan, Error> {
        let e = *self.entries().get(index).ok_or(Error::NotFound)?;
        if e.bytes > MAX_FILE_BYTES {
            return Err(Error::TooLarge);
        }
        if requested > 4096 {
            return Err(Error::TooLarge);
        }
        if offset >= e.bytes || requested == 0 {
            return Ok(crate::read_plan::Plan::new(0, self.size));
        }
        let mut r = Reader {
            read: &mut self.read,
            blocks: self.blocks,
            size: self.size,
            budget: 1_000_000,
        };
        let cluster_bytes = self.volume.spc as u64 * self.size as u64;
        let clusters = e.bytes.div_ceil(cluster_bytes);
        let mut cluster = e.first;
        for i in 0..clusters {
            match (self.volume.next(&mut r, cluster)?, i + 1 == clusters) {
                (None, true) => {}
                (Some(next), false) => cluster = next,
                _ => return Err(Error::Corrupt),
            }
        }
        cluster = e.first;
        for _ in 0..offset / cluster_bytes {
            cluster = self.volume.next(&mut r, cluster)?.ok_or(Error::Corrupt)?;
        }
        let count = (e.bytes - offset).min(requested as u64) as usize;
        let mut plan = crate::read_plan::Plan::new(count, self.size);
        let mut copied = 0;
        let mut inside = offset % cluster_bytes;
        while copied < count {
            let lba = self.volume.cluster(cluster)? + inside / self.size as u64;
            let skip = (inside % self.size as u64) as usize;
            let n = (count - copied).min(self.size - skip);
            plan.push(lba, skip, n, copied)?;
            copied += n;
            inside += n as u64;
            if inside == cluster_bytes && copied < count {
                cluster = self.volume.next(&mut r, cluster)?.ok_or(Error::Corrupt)?;
                inside = 0;
            }
        }
        Ok(plan)
    }
}
