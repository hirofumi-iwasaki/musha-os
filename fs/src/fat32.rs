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
    /// Synchronous startup discovery: superfloppy, primary MBR FAT32, or GPT ESP.
    /// Shares read_root's partition validation; never call from a live app step.
    pub fn discover(blocks: u64, size: usize, mut read: F) -> Result<Self, Error> {
        if !matches!(size, 512 | 4096) || blocks == 0 {
            return Err(Error::Unsupported);
        }
        let volume = crate::discover_volume(&mut Reader {
            read: &mut read,
            blocks,
            size,
            budget: 1024,
        })?;
        Self::mount(blocks, size, volume.start, volume.total, read)
    }
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
                        let stem = name[..8].iter().take_while(|&&c| c != b' ').copied();
                        let ext = name[8..].iter().take_while(|&&c| c != b' ').copied();
                        return Err(crate::duplicate_name(
                            stem.chain((name[8] != b' ').then_some(b'.'))
                                .chain(ext)
                                .map(u16::from),
                        ));
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
    /// Builds a resumable metadata validator without performing block I/O.
    pub fn begin_plan(
        &self,
        index: usize,
        offset: u64,
        requested: usize,
    ) -> Result<crate::planner::Planner, Error> {
        self.source(index)?.begin_plan(offset, requested)
    }
    /// Detach validated file metadata from the synchronous startup reader.
    /// The returned source remains valid only for this immutable mounted medium.
    pub fn source(&self, index: usize) -> Result<crate::planner::Source, Error> {
        let e = *self.entries().get(index).ok_or(Error::NotFound)?;
        Ok(crate::planner::Source::new(
            crate::planner::Geometry {
                blocks: self.blocks,
                end: self.volume.start + self.volume.total,
                fat: self.volume.fat,
                data: self.volume.data,
                clusters: self.volume.clusters,
                spc: self.volume.spc,
                size: self.size,
                exfat: false,
            },
            crate::planner::File {
                first: e.first,
                length: e.bytes,
                valid: e.bytes,
                contiguous: false,
                bitmap: None,
            },
        ))
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
        let mut planner = self.begin_plan(index, offset, requested)?;
        planner.run(&mut self.read)
    }
}
