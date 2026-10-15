// Copyright 2026 Hirofumi Iwasaki
// SPDX-License-Identifier: Apache-2.0
//! Internal SETTINGS-only write preflight. No public application write API.
//! All discovery/validation completes before any disk mutation is permitted.
use crate::{Error, Reader, Volume, discover_volume, u16le, u32le, visit};
pub const DEFAULT_CAPACITY: usize = 4096;
const MAX_CLUSTERS: usize = 8;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refusal {
    Filesystem(Error),
    Name,
    Protected,
    MissingDirectory,
    FullDirectory,
    NoSpace,
    Capacity,
    Layout,
}
impl From<Error> for Refusal {
    fn from(e: Error) -> Self {
        Self::Filesystem(e)
    }
}
/// A single short filename, not a path. Rejects aliases, separators and dot entries.
pub fn short_name(name: &[u8]) -> Result<[u8; 11], Refusal> {
    let mut out = [b' '; 11];
    let mut base = 0;
    let mut extension = 0;
    let mut dot = false;
    for &c in name {
        if c == b'.' && !dot && base != 0 {
            dot = true;
            continue;
        }
        if !(c.is_ascii_uppercase() || c.is_ascii_digit() || b"_-".contains(&c)) {
            return Err(Refusal::Name);
        }
        if dot {
            if extension == 3 {
                return Err(Refusal::Name);
            }
            out[8 + extension] = c;
            extension += 1;
        } else {
            if base == 8 {
                return Err(Refusal::Name);
            }
            out[base] = c;
            base += 1;
        }
    }
    if base == 0 || (dot && extension == 0) {
        return Err(Refusal::Name);
    }
    Ok(out)
}
#[derive(Clone, Copy)]
pub struct Plan {
    volume: Volume,
    pub sector_size: usize,
    pub capacity: usize,
    pub created: bool,
    directory_lba: u64,
    directory_offset: usize,
    clusters: [u32; MAX_CLUSTERS],
    count: usize,
    fatsize: u64,
    info: [u64; 2],
}
impl Plan {
    pub fn clusters(&self) -> &[u32] {
        &self.clusters[..self.count]
    }
    pub fn directory_sector(&self) -> u64 {
        self.directory_lba
    }
    pub fn directory_offset(&self) -> usize {
        self.directory_offset
    }
    pub fn data_sector(&self, index: usize) -> Option<u64> {
        let cluster = index / self.volume.spc as usize;
        if cluster >= self.count {
            return None;
        }
        self.volume
            .cluster(self.clusters[cluster])
            .ok()?
            .checked_add((index % self.volume.spc as usize) as u64)
    }
    pub fn allocated_sectors(&self) -> usize {
        self.count * self.volume.spc as usize
    }
    pub fn fat_sectors(&self, cluster: u32) -> [u64; 2] {
        let relative = cluster as u64 * 4 / self.sector_size as u64;
        [
            self.volume.fat + relative,
            self.volume.fat + self.fatsize + relative,
        ]
    }
    pub fn fsinfo_sectors(&self) -> [u64; 2] {
        self.info
    }
}
struct Entry {
    lba: u64,
    offset: usize,
    bytes: [u8; 32],
    lfn: bool,
}
fn entry<F: FnMut(u64, &mut [u8]) -> Result<(), Error>>(
    r: &mut Reader<'_, F>,
    v: Volume,
    first: u32,
    name: &[u8; 11],
) -> Result<(Option<Entry>, Option<(u64, usize)>), Refusal> {
    let mut seen = [0; 128];
    let mut count = 0;
    let mut c = first;
    let mut found = None;
    let mut empty = None;
    let mut lfn = false;
    loop {
        visit(&mut seen, &mut count, c)?;
        let start = v.cluster(c)?;
        for n in 0..v.spc {
            let mut b = [0; 4096];
            let lba = start + n as u64;
            r.sector(lba, &mut b)?;
            for (i, e) in b[..r.size].chunks_exact(32).enumerate() {
                if e[0] == 0 {
                    if found.is_none() && empty.is_none() {
                        // Inserting here must not revive bytes after the old end
                        // marker. Refuse rather than change an unrelated entry.
                        if lfn {
                            return Err(Refusal::Layout);
                        }
                        let next = (i + 1) * 32;
                        if next < r.size {
                            if b[next] != 0 {
                                return Err(Refusal::Layout);
                            }
                        } else {
                            let following = if n + 1 < v.spc {
                                Some(lba + 1)
                            } else {
                                v.next(r, c)?
                                    .map(|cluster| v.cluster(cluster))
                                    .transpose()?
                            };
                            if let Some(following) = following {
                                let mut sector = [0; 4096];
                                r.sector(following, &mut sector)?;
                                if sector[0] != 0 {
                                    return Err(Refusal::Layout);
                                }
                            }
                        }
                    }
                    return Ok((found, empty.or(Some((lba, i * 32)))));
                }
                if e[0] == 0xe5 {
                    if empty.is_none() && !lfn {
                        empty = Some((lba, i * 32));
                    }
                    lfn = false;
                    continue;
                }
                if e[11] == 0x0f {
                    lfn = true;
                    continue;
                }
                if &e[..11] == name {
                    if found.is_some() {
                        return Err(Refusal::Layout);
                    }
                    found = Some(Entry {
                        lba,
                        offset: i * 32,
                        bytes: e.try_into().unwrap(),
                        lfn,
                    });
                }
                lfn = false;
            }
        }
        match v.next(r, c)? {
            Some(next) => c = next,
            None => return Ok((found, empty)),
        }
    }
}
fn first(e: &[u8]) -> u32 {
    ((u16le(e, 20) as u32) << 16 | u16le(e, 26) as u32) & 0x0fffffff
}
/// Read-only preflight. Fixed-capacity on-disk size policy is deliberately not
/// chosen here; the owner decision must precede the mutation executor.
pub fn preflight<F: FnMut(u64, &mut [u8]) -> Result<(), Error>>(
    blocks: u64,
    size: usize,
    name: &[u8],
    capacity: usize,
    protected: bool,
    read: &mut F,
) -> Result<Plan, Refusal> {
    let name = short_name(name)?;
    if protected {
        return Err(Refusal::Protected);
    }
    if !matches!(size, 512 | 4096) || blocks == 0 {
        return Err(Refusal::Layout);
    }
    if capacity == 0 || capacity > DEFAULT_CAPACITY {
        return Err(Refusal::Capacity);
    }
    let mut r = Reader {
        read,
        blocks,
        size,
        budget: 20000,
    };
    let v = discover_volume(&mut r)?;
    let mut b = [0; 4096];
    r.sector(v.start, &mut b)?;
    // Require mirrored two-FAT layout and valid primary + backup FSInfo.
    if b[16] != 2 || u16le(&b, 40) != 0 {
        return Err(Refusal::Layout);
    }
    let reserved = u16le(&b, 14) as u64;
    let fatsize = u32le(&b, 36) as u64;
    let info = u16le(&b, 48) as u64;
    let backup = u16le(&b, 50) as u64;
    if info == 0 || info >= reserved || backup == 0 || backup + info >= reserved {
        return Err(Refusal::Layout);
    }
    let info = [v.start + info, v.start + backup + info];
    for lba in info {
        r.sector(lba, &mut b)?;
        if u32le(&b, 0) != 0x41615252
            || u32le(&b, 484) != 0x61417272
            || u32le(&b, 508) != 0xaa550000
        {
            return Err(Refusal::Layout);
        }
        let free = u32le(&b, 488);
        if free != u32::MAX && free > v.clusters {
            return Err(Refusal::Layout);
        }
    }
    let (settings, _) = entry(&mut r, v, v.root, b"SETTINGS   ")?;
    let settings = settings.ok_or(Refusal::MissingDirectory)?;
    if settings.lfn || settings.bytes[11] & 0x3f != 0x10 {
        return Err(Refusal::Layout);
    }
    let settings_cluster = first(&settings.bytes);
    let (file, empty) = entry(&mut r, v, settings_cluster, &name)?;
    let cluster_bytes = size * v.spc as usize;
    let needed = capacity.div_ceil(cluster_bytes);
    if needed > MAX_CLUSTERS {
        return Err(Refusal::Capacity);
    }
    let mut plan = Plan {
        volume: v,
        sector_size: size,
        capacity,
        created: file.is_none(),
        directory_lba: 0,
        directory_offset: 0,
        clusters: [0; MAX_CLUSTERS],
        count: 0,
        fatsize,
        info,
    };
    if let Some(file) = file {
        if file.lfn || file.bytes[11] & 0x1f != 0 || u32le(&file.bytes, 28) as usize > capacity {
            return Err(Refusal::Layout);
        }
        plan.directory_lba = file.lba;
        plan.directory_offset = file.offset;
        let mut c = first(&file.bytes);
        loop {
            if plan.count == MAX_CLUSTERS
                || plan.clusters[..plan.count].contains(&c)
                || c == v.root
                || c == settings_cluster
            {
                return Err(Refusal::Layout);
            }
            plan.clusters[plan.count] = c;
            plan.count += 1;
            match v.next(&mut r, c)? {
                Some(next) => c = next,
                None => break,
            }
        }
        if plan.count != needed {
            return Err(Refusal::Capacity);
        }
    } else {
        let (lba, offset) = empty.ok_or(Refusal::FullDirectory)?;
        plan.directory_lba = lba;
        plan.directory_offset = offset;
    }
    // Compare every FAT byte before writes; mirrored corruption never gets repaired
    // implicitly. Select free clusters only after complete discovery succeeds.
    let mut actual_free = 0u32;
    for n in 0..fatsize {
        let mut a = [0; 4096];
        let mut b = [0; 4096];
        r.sector(v.fat + n, &mut a)?;
        r.sector(v.fat + fatsize + n, &mut b)?;
        if a[..size] != b[..size] {
            return Err(Refusal::Layout);
        }
        for (i, e) in a[..size].chunks_exact(4).enumerate() {
            let c = (n * size as u64 / 4 + i as u64) as u32;
            if c >= 2 && c < v.clusters + 2 && u32le(e, 0) & 0x0fffffff == 0 {
                actual_free += 1;
                if plan.created && plan.count < needed {
                    plan.clusters[plan.count] = c;
                    plan.count += 1;
                }
            }
        }
    }
    if plan.count != needed {
        return Err(Refusal::NoSpace);
    }
    for lba in info {
        r.sector(lba, &mut b)?;
        let count = u32le(&b, 488);
        let next = u32le(&b, 492);
        if (count != u32::MAX && count != actual_free)
            || (next != u32::MAX && !(2..v.clusters + 2).contains(&next))
        {
            return Err(Refusal::Layout);
        }
    }
    Ok(plan)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn names_are_files_not_paths_or_aliases() {
        assert_eq!(short_name(b"CONFIG.TXT"), Ok(*b"CONFIG  TXT"));
        for n in [
            b"".as_slice(),
            b"../X",
            b"SETTINGS/X",
            b"X\\Y",
            b"x.txt",
            b".X",
            b"X.",
            b"TOOLONGXX",
            b"X.LONG",
            b"X Y",
        ] {
            assert_eq!(short_name(n), Err(Refusal::Name));
        }
    }
    #[test]
    fn rejected_requests_never_read_or_write() {
        let mut read = |_: u64, _: &mut [u8]| -> Result<(), Error> { panic!("unexpected I/O") };
        assert!(matches!(
            preflight(1, 512, b"X", 4096, true, &mut read),
            Err(Refusal::Protected)
        ));
        assert!(matches!(
            preflight(1, 512, b"../X", 4096, false, &mut read),
            Err(Refusal::Name)
        ));
        assert!(matches!(
            preflight(1, 512, b"X", 4097, false, &mut read),
            Err(Refusal::Capacity)
        ));
    }
}

#[cfg(test)]
mod media_tests {
    use super::*;
    pub(super) struct Media {
        missing: bool,
        full: bool,
        no_space: bool,
        mismatch: bool,
        existing: bool,
        cycle: bool,
        exfat: bool,
    }
    impl Media {
        pub(super) fn new() -> Self {
            Self {
                missing: false,
                full: false,
                no_space: false,
                mismatch: false,
                existing: false,
                cycle: false,
                exfat: false,
            }
        }
        pub(super) fn read(&self, lba: u64, b: &mut [u8]) -> Result<(), Error> {
            assert!(lba < 66581);
            b.fill(0);
            let put16 =
                |b: &mut [u8], i: usize, v: u16| b[i..i + 2].copy_from_slice(&v.to_le_bytes());
            let put32 =
                |b: &mut [u8], i: usize, v: u32| b[i..i + 4].copy_from_slice(&v.to_le_bytes());
            match lba {
                0 => {
                    b[0] = 0xeb;
                    put16(b, 11, 512);
                    b[13] = 1;
                    put16(b, 14, 32);
                    b[16] = 2;
                    put32(b, 32, 66581);
                    put32(b, 36, 512);
                    put32(b, 44, 2);
                    put16(b, 48, 1);
                    put16(b, 50, 6);
                    b[510..512].copy_from_slice(&[0x55, 0xaa]);
                    if self.exfat {
                        b[3..11].copy_from_slice(b"EXFAT   ");
                        b[11..64].fill(0);
                    }
                }
                1 | 7 => {
                    put32(b, 0, 0x41615252);
                    put32(b, 484, 0x61417272);
                    put32(b, 488, if self.existing { 65515 } else { 65523 });
                    put32(b, 492, 4);
                    put32(b, 508, 0xaa550000);
                }
                1056 => {
                    if !self.missing {
                        b[..11].copy_from_slice(b"SETTINGS   ");
                        b[11] = 0x10;
                        put16(b, 26, 3);
                    }
                }
                1057 => {
                    if self.full {
                        for e in b.chunks_exact_mut(32) {
                            e[0] = b'Y';
                            e[11] = 0x20;
                        }
                    } else if self.existing {
                        b[..11].copy_from_slice(b"CONFIG  TXT");
                        b[11] = 0x20;
                        put16(b, 26, 4);
                        put32(b, 28, 4096);
                    }
                }
                32..=1055 => {
                    let n = if lba >= 544 { lba - 544 } else { lba - 32 };
                    if self.no_space {
                        for e in b.chunks_exact_mut(4) {
                            e.copy_from_slice(&0x0fffffffu32.to_le_bytes());
                        }
                    }
                    if n == 0 {
                        for c in [0, 1, 2, 3] {
                            put32(b, c * 4, 0x0fffffff);
                        }
                        if self.existing {
                            for (a, next) in [
                                (4, 6),
                                (6, 8),
                                (8, 10),
                                (10, 12),
                                (12, 14),
                                (14, 16),
                                (16, 18),
                                (18, if self.cycle { 4 } else { 0x0fffffff }),
                            ] {
                                put32(b, a * 4, next);
                            }
                        }
                    }
                    if self.mismatch && lba == 544 {
                        b[100] ^= 1;
                    }
                }
                _ => {}
            }
            Ok(())
        }
        fn plan(&self) -> Result<Plan, Refusal> {
            preflight(66581, 512, b"CONFIG.TXT", 4096, false, &mut |l, b| {
                self.read(l, b)
            })
        }
    }
    #[test]
    fn free_and_existing_fragmented_chain_are_bounded() {
        let mut d = Media::new();
        let p = d.plan().unwrap();
        assert!(p.created);
        assert_eq!(p.clusters(), &[4, 5, 6, 7, 8, 9, 10, 11]);
        assert_eq!(p.allocated_sectors(), 8);
        assert_eq!(p.data_sector(8), None);
        d.existing = true;
        let p = d.plan().unwrap();
        assert!(!p.created);
        assert_eq!(p.clusters(), &[4, 6, 8, 10, 12, 14, 16, 18]);
        assert_eq!(p.fsinfo_sectors(), [1, 7]);
        assert_eq!(p.fat_sectors(4), [32, 544]);
        assert_eq!(p.directory_sector(), 1057);
    }
    #[test]
    fn absence_capacity_corruption_and_exfat_refuse_without_mutation() {
        let mut d = Media::new();
        d.missing = true;
        assert!(matches!(d.plan(), Err(Refusal::MissingDirectory)));
        let mut d = Media::new();
        d.full = true;
        assert!(matches!(d.plan(), Err(Refusal::FullDirectory)));
        let mut d = Media::new();
        d.no_space = true;
        assert!(matches!(d.plan(), Err(Refusal::NoSpace)));
        let mut d = Media::new();
        d.mismatch = true;
        assert!(matches!(d.plan(), Err(Refusal::Layout)));
        let mut d = Media::new();
        d.existing = true;
        d.cycle = true;
        assert!(matches!(d.plan(), Err(Refusal::Layout)));
        let mut d = Media::new();
        d.exfat = true;
        assert!(d.plan().is_err());
    }
}

/// Internal synchronous adapter. Firmware and application pointers never enter it.
/// The caller owns device lifetime and must stop DMA before returning after a
/// transport error. Never retry a timed-out transfer with the same DMA buffers.
pub trait Disk {
    fn read(&mut self, lba: u64, bytes: &mut [u8]) -> Result<(), Error>;
    fn write(&mut self, lba: u64, bytes: &[u8]) -> Result<(), Error>;
    fn sync(&mut self) -> Result<(), Error>;
    fn protected(&self) -> bool;
}
#[derive(Clone, Copy)]
struct Patch {
    lba: u64,
    old: [u8; 4096],
    new: [u8; 4096],
}
const EMPTY_PATCH: Patch = Patch {
    lba: 0,
    old: [0; 4096],
    new: [0; 4096],
};
/// Caller-owned bounded CPU workspace; never DMA storage. Reuse only after return.
pub struct Workspace {
    patches: [Patch; 19],
    count: usize,
}
impl Workspace {
    pub const fn new() -> Self {
        Self {
            patches: [EMPTY_PATCH; 19],
            count: 0,
        }
    }
    fn patch(
        &mut self,
        disk: &mut impl Disk,
        lba: u64,
        size: usize,
    ) -> Result<&mut Patch, WriteError> {
        if let Some(i) = self.patches[..self.count].iter().position(|p| p.lba == lba) {
            return Ok(&mut self.patches[i]);
        }
        if self.count == self.patches.len() {
            return Err(WriteError::Refused(Refusal::Layout));
        }
        let p = &mut self.patches[self.count];
        p.lba = lba;
        disk.read(lba, &mut p.old[..size]).map_err(WriteError::Io)?;
        p.new = p.old;
        self.count += 1;
        Ok(p)
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WriteError {
    Refused(Refusal),
    Unowned,
    Io(Error),
    RecoveryRequired,
}
impl From<Refusal> for WriteError {
    fn from(e: Refusal) -> Self {
        Self::Refused(e)
    }
}
/// The only writable filename currently defined. CONFIG.TXT is never writable.
pub const OWNED_NAME: &[u8] = b"PAIRING.DAT";
fn audit<D: Disk>(disk: &mut D, blocks: u64, p: &Plan) -> Result<(), WriteError> {
    let mut r = Reader {
        read: &mut |l: u64, b: &mut [u8]| disk.read(l, b),
        blocks,
        size: p.sector_size,
        budget: 20000,
    };
    // Every selected cluster must have exactly its predecessor as incoming FAT
    // reference (none for a first/free cluster). This rejects shared chain tails.
    for n in 0..p.fatsize {
        let mut b = [0; 4096];
        r.sector(p.volume.fat + n, &mut b).map_err(WriteError::Io)?;
        for (i, e) in b[..p.sector_size].chunks_exact(4).enumerate() {
            let c = (n * p.sector_size as u64 / 4 + i as u64) as u32;
            if c < 2 || c >= p.volume.clusters + 2 {
                continue;
            }
            let next = u32le(e, 0) & 0x0fffffff;
            if let Some(j) = p.clusters().iter().position(|x| *x == next) {
                if p.created || j == 0 || c != p.clusters[j - 1] {
                    return Err(WriteError::Refused(Refusal::Layout));
                }
            }
        }
    }
    // Directory heads are not FAT incoming links. Audit every directory entry to
    // reject another file/directory referring directly to a selected cluster.
    let mut dirs = [0u32; 64];
    dirs[0] = p.volume.root;
    let mut used = 1;
    let mut index = 0;
    let mut entries = 0;
    while index < used {
        let mut c = dirs[index];
        index += 1;
        let mut seen = [0; 128];
        let mut count = 0;
        'chain: loop {
            visit(&mut seen, &mut count, c).map_err(WriteError::Io)?;
            let start = p.volume.cluster(c).map_err(WriteError::Io)?;
            for sector in 0..p.volume.spc {
                let lba = start + sector as u64;
                let mut b = [0; 4096];
                r.sector(lba, &mut b).map_err(WriteError::Io)?;
                for (i, e) in b[..p.sector_size].chunks_exact(32).enumerate() {
                    entries += 1;
                    if entries > 4096 {
                        return Err(WriteError::Refused(Refusal::Layout));
                    }
                    if e[0] == 0 {
                        break 'chain;
                    }
                    if e[0] == 0xe5 || e[11] & 0x0f == 0x0f || e[11] & 8 != 0 || e[0] == b'.' {
                        continue;
                    }
                    let head = first(e);
                    let target =
                        lba == p.directory_lba && i * 32 == p.directory_offset && !p.created;
                    if p.clusters().contains(&head) && !target {
                        return Err(WriteError::Refused(Refusal::Layout));
                    }
                    if e[11] & 0x10 != 0 {
                        if used == dirs.len() || dirs[..used].contains(&head) {
                            return Err(WriteError::Refused(Refusal::Layout));
                        }
                        p.volume.cluster(head).map_err(WriteError::Io)?;
                        dirs[used] = head;
                        used += 1;
                    }
                }
            }
            match p.volume.next(&mut r, c).map_err(WriteError::Io)? {
                Some(next) => c = next,
                None => break,
            }
        }
    }
    Ok(())
}
/// Fixed 4096-byte physical file, recognized versioned record only. The validator
/// must check magic/version/bounds/checksum, not just filename. No delete/rename/
/// directory creation or allocation on overwrite. Caller serializes media access.
pub fn write_owned(
    disk: &mut impl Disk,
    blocks: u64,
    size: usize,
    name: &[u8],
    record: &[u8],
    recognized: fn(&[u8]) -> bool,
    workspace: &mut Workspace,
) -> Result<bool, WriteError> {
    if name != OWNED_NAME {
        return Err(WriteError::Unowned);
    }
    if record.len() != DEFAULT_CAPACITY || !recognized(record) {
        return Err(WriteError::Unowned);
    }
    let protected = disk.protected();
    let p = preflight(
        blocks,
        size,
        name,
        DEFAULT_CAPACITY,
        protected,
        &mut |l, b| disk.read(l, b),
    )?;
    audit(disk, blocks, &p)?;
    if !p.created {
        let mut old = [0; DEFAULT_CAPACITY];
        for (i, b) in old.chunks_mut(size).enumerate() {
            disk.read(p.data_sector(i).ok_or(WriteError::Unowned)?, b)
                .map_err(WriteError::Io)?;
        }
        // A same-named user file, unknown version or damaged record is never repaired.
        let mut directory = [0; 4096];
        disk.read(p.directory_lba, &mut directory[..size])
            .map_err(WriteError::Io)?;
        if u32le(&directory, p.directory_offset + 28) != DEFAULT_CAPACITY as u32
            || !recognized(&old)
        {
            return Err(WriteError::Unowned);
        }
    }
    workspace.count = 0;
    if p.created {
        for (i, &cluster) in p.clusters().iter().enumerate() {
            let next = p.clusters().get(i + 1).copied().unwrap_or(0x0fffffff);
            for lba in p.fat_sectors(cluster) {
                let patch = workspace.patch(disk, lba, size)?;
                let off = cluster as usize * 4 % size;
                let raw = u32le(&patch.new, off);
                patch.new[off..off + 4].copy_from_slice(&((raw & 0xf0000000) | next).to_le_bytes());
            }
        }
        for lba in p.info {
            let patch = workspace.patch(disk, lba, size)?;
            let free = u32le(&patch.new, 488);
            if free != u32::MAX {
                let free = free
                    .checked_sub(p.count as u32)
                    .ok_or(WriteError::Refused(Refusal::Layout))?;
                patch.new[488..492].copy_from_slice(&free.to_le_bytes());
            }
            // Unknown is valid FAT32 and avoids inventing a free-cluster hint.
            patch.new[492..496].copy_from_slice(&u32::MAX.to_le_bytes());
        }
        let patch = workspace.patch(disk, p.directory_lba, size)?;
        let off = p.directory_offset;
        let e = &mut patch.new[off..off + 32];
        e.fill(0);
        e[..11].copy_from_slice(&short_name(name)?);
        e[11] = 0x20;
        // Deterministic valid FAT date (1980-01-01); no firmware clock calls.
        e[16..18].copy_from_slice(&33u16.to_le_bytes());
        e[24..26].copy_from_slice(&33u16.to_le_bytes());
        e[20..22].copy_from_slice(&((p.clusters[0] >> 16) as u16).to_le_bytes());
        e[26..28].copy_from_slice(&(p.clusters[0] as u16).to_le_bytes());
        e[28..32].copy_from_slice(&(DEFAULT_CAPACITY as u32).to_le_bytes());
    }
    // Initialize every byte of every allocated cluster before it becomes live.
    // Existing records overwrite only the bounded 4096-byte file, never its slack.
    let sectors = if p.created {
        p.allocated_sectors()
    } else {
        DEFAULT_CAPACITY / size
    };
    for i in 0..sectors {
        let mut b = [0; 4096];
        let offset = i * size;
        if offset < record.len() {
            b[..size].copy_from_slice(&record[offset..offset + size]);
        }
        disk.write(p.data_sector(i).ok_or(WriteError::Unowned)?, &b[..size])
            .map_err(WriteError::Io)?;
    }
    disk.sync().map_err(WriteError::Io)?;
    for i in 0..workspace.count {
        let patch = &workspace.patches[i];
        if let Err(error) = disk.write(patch.lba, &patch.new[..size]) {
            // Include the failed sector: the device may have partially written it.
            return rollback(disk, workspace, i + 1, size, error);
        }
    }
    if let Err(error) = disk.sync() {
        return rollback(disk, workspace, workspace.count, size, error);
    }
    Ok(p.created)
}
fn rollback(
    disk: &mut impl Disk,
    w: &Workspace,
    count: usize,
    size: usize,
    error: Error,
) -> Result<bool, WriteError> {
    let mut failed = false;
    for patch in w.patches[..count].iter().rev() {
        if disk.write(patch.lba, &patch.old[..size]).is_err() {
            failed = true;
        }
    }
    if disk.sync().is_err() {
        failed = true;
    }
    if failed {
        Err(WriteError::RecoveryRequired)
    } else {
        Err(WriteError::Io(error))
    }
}

#[cfg(test)]
mod writer_tests {
    extern crate std;
    use super::*;
    use std::collections::BTreeMap;
    struct Memory {
        base: super::media_tests::Media,
        sectors: BTreeMap<u64, [u8; 512]>,
        writes: usize,
        fail: Option<usize>,
        persistent: bool,
        protected: bool,
        sync_fail: bool,
    }
    impl Memory {
        fn new() -> Self {
            Self {
                base: super::media_tests::Media::new(),
                sectors: BTreeMap::new(),
                writes: 0,
                fail: None,
                persistent: false,
                protected: false,
                sync_fail: false,
            }
        }
        fn record(seed: u8) -> [u8; 4096] {
            let mut b = [0; 4096];
            musha_platform::pairing::encode_state(&[], Some([seed; 32]), &mut b).unwrap();
            b
        }
        fn run(&mut self, b: &[u8]) -> Result<bool, WriteError> {
            write_owned(
                self,
                66581,
                512,
                OWNED_NAME,
                b,
                musha_platform::pairing::recognized,
                &mut Workspace::new(),
            )
        }
        fn sector(&mut self, l: u64) -> [u8; 512] {
            let mut b = [0; 512];
            self.read(l, &mut b).unwrap();
            b
        }
    }
    impl Disk for Memory {
        fn read(&mut self, l: u64, b: &mut [u8]) -> Result<(), Error> {
            if let Some(s) = self.sectors.get(&l) {
                b.copy_from_slice(s);
                Ok(())
            } else {
                self.base.read(l, b)
            }
        }
        fn write(&mut self, l: u64, b: &[u8]) -> Result<(), Error> {
            self.writes += 1;
            if self.protected
                || self
                    .fail
                    .is_some_and(|n| self.writes == n || (self.persistent && self.writes >= n))
            {
                return Err(Error::Io);
            }
            self.sectors.insert(l, b.try_into().unwrap());
            Ok(())
        }
        fn sync(&mut self) -> Result<(), Error> {
            if self.sync_fail {
                self.sync_fail = false;
                Err(Error::Io)
            } else {
                Ok(())
            }
        }
        fn protected(&self) -> bool {
            self.protected
        }
    }
    #[test]
    fn create_and_overwrite_keep_fixed_chain_and_other_bytes() {
        let mut d = Memory::new();
        let before = d.sector(1057);
        assert_eq!(d.run(&Memory::record(1)), Ok(true));
        let fat = d.sector(32);
        assert_eq!(fat, d.sector(544));
        let dir = d.sector(1057);
        assert_eq!(&dir[..11], b"PAIRING DAT");
        assert_eq!(u32le(&dir, 28), 4096);
        assert_eq!(&dir[32..], &before[32..]);
        assert_eq!(u32le(&d.sector(1), 488), 65515);
        assert_eq!(d.sector(1), d.sector(7));
        assert_eq!(d.run(&Memory::record(2)), Ok(false));
        assert_eq!(d.sector(32), fat);
        assert_eq!(d.sector(1057), dir);
        let mut bytes = [0; 4096];
        for (i, b) in bytes.chunks_mut(512).enumerate() {
            d.read(1058 + i as u64, b).unwrap();
        }
        assert_eq!(bytes, Memory::record(2));
        assert!(
            d.sectors
                .keys()
                .all(|l| matches!(*l, 1 | 7 | 32 | 544 | 1057..=1065))
        );
    }
    #[test]
    fn config_same_named_user_file_and_bad_records_never_written() {
        let mut d = Memory::new();
        let b = Memory::record(1);
        assert_eq!(
            write_owned(
                &mut d,
                66581,
                512,
                b"CONFIG.TXT",
                &b,
                musha_platform::pairing::recognized,
                &mut Workspace::new()
            ),
            Err(WriteError::Unowned)
        );
        assert_eq!(d.writes, 0);
        assert_eq!(d.run(&b[..4095]), Err(WriteError::Unowned));
        assert_eq!(d.run(&[0; 4097]), Err(WriteError::Unowned));
        assert_eq!(d.run(&[0; 4096]), Err(WriteError::Unowned));
        d.protected = true;
        assert_eq!(d.run(&b), Err(WriteError::Refused(Refusal::Protected)));
        assert_eq!(d.writes, 0);
        d.protected = false;
        d.run(&b).unwrap();
        let mut raw = d.sector(1058);
        raw[0] ^= 1;
        d.sectors.insert(1058, raw);
        let n = d.writes;
        assert_eq!(d.run(&b), Err(WriteError::Unowned));
        assert_eq!(d.writes, n);
    }
    #[test]
    fn recoverable_error_at_every_write_restores_metadata() {
        let mut complete = Memory::new();
        complete.run(&Memory::record(1)).unwrap();
        let total = complete.writes;
        for fail in 1..=total {
            let mut d = Memory::new();
            let before = [
                d.sector(1),
                d.sector(7),
                d.sector(32),
                d.sector(544),
                d.sector(1057),
            ];
            d.fail = Some(fail);
            assert_eq!(
                d.run(&Memory::record(1)),
                Err(WriteError::Io(Error::Io)),
                "write {fail}"
            );
            assert_eq!(
                [
                    d.sector(1),
                    d.sector(7),
                    d.sector(32),
                    d.sector(544),
                    d.sector(1057)
                ],
                before,
                "write {fail}"
            );
        }
    }
    #[test]
    fn persistent_failure_is_explicitly_not_structural_success() {
        let mut d = Memory::new();
        d.fail = Some(10);
        d.persistent = true;
        assert_eq!(d.run(&Memory::record(1)), Err(WriteError::RecoveryRequired));
        let mut d = Memory::new();
        d.sync_fail = true;
        assert_eq!(d.run(&Memory::record(1)), Err(WriteError::Io(Error::Io)));
        assert_eq!(d.sector(32), d.sector(544));
    }
    #[test]
    fn end_marker_garbage_is_never_revived() {
        let b = Memory::record(1);
        let mut d = Memory::new();
        let mut dir = d.sector(1057);
        dir[32] = b'G';
        d.sectors.insert(1057, dir);
        assert_eq!(d.run(&b), Err(WriteError::Refused(Refusal::Layout)));
        assert_eq!(d.writes, 0);
        // Deleted entries can be reused without moving the existing end marker.
        let mut d = Memory::new();
        let mut dir = d.sector(1057);
        dir[0] = 0xe5;
        dir[32] = b'X';
        dir[43] = 32;
        dir[64] = 0;
        d.sectors.insert(1057, dir);
        assert_eq!(d.run(&b), Ok(true));
        assert_eq!(d.sector(1057)[32], b'X');
        assert_eq!(d.sector(1057)[64], 0);
        // End marker at a cluster boundary: next cluster starts with old garbage.
        let mut d = Memory::new();
        let mut dir = d.sector(1057);
        for e in dir.chunks_exact_mut(32) {
            e[0] = b'X';
            e[11] = 32;
        }
        dir[480] = 0;
        d.sectors.insert(1057, dir);
        for l in [32, 544] {
            let mut fat = d.sector(l);
            fat[12..16].copy_from_slice(&12u32.to_le_bytes());
            fat[48..52].copy_from_slice(&0x0fffffffu32.to_le_bytes());
            d.sectors.insert(l, fat);
        }
        let mut next = [0; 512];
        next[0] = b'G';
        d.sectors.insert(1066, next);
        assert_eq!(d.run(&b), Err(WriteError::Refused(Refusal::Layout)));
        assert_eq!(d.writes, 0);
    }
    #[test]
    fn large_cluster_is_completely_zero_initialized() {
        struct Large {
            inner: Memory,
        }
        impl Disk for Large {
            fn read(&mut self, l: u64, b: &mut [u8]) -> Result<(), Error> {
                if let Some(s) = self.inner.sectors.get(&l) {
                    b.copy_from_slice(s);
                    return Ok(());
                }
                match l {
                    0 => {
                        self.inner.base.read(0, b)?;
                        b[13] = 16;
                        b[32..36].copy_from_slice(&1049456u32.to_le_bytes());
                    }
                    1072 => self.inner.base.read(1057, b)?,
                    1057 => b.fill(0),
                    1088..=1103 => b.fill(0xa5), // Deliberately nonzero free cluster.
                    _ => self.inner.base.read(l, b)?,
                }
                Ok(())
            }
            fn write(&mut self, l: u64, b: &[u8]) -> Result<(), Error> {
                self.inner.write(l, b)
            }
            fn sync(&mut self) -> Result<(), Error> {
                Ok(())
            }
            fn protected(&self) -> bool {
                false
            }
        }
        let mut d = Large {
            inner: Memory::new(),
        };
        let record = Memory::record(1);
        assert_eq!(
            write_owned(
                &mut d,
                1049456,
                512,
                OWNED_NAME,
                &record,
                musha_platform::pairing::recognized,
                &mut Workspace::new()
            ),
            Ok(true)
        );
        for l in 1096..=1103 {
            assert_eq!(d.inner.sector(l), [0; 512]);
        }
        assert_eq!(u32le(&d.inner.sector(32), 16) & 0xfffffff, 0xfffffff);
    }
    #[test]
    fn sector_4096_create_and_overwrite() {
        struct SectorDisk {
            sectors: BTreeMap<u64, [u8; 4096]>,
        }
        impl Disk for SectorDisk {
            fn read(&mut self, l: u64, b: &mut [u8]) -> Result<(), Error> {
                assert_eq!(b.len(), 4096);
                assert!(l < 65685);
                b.fill(0);
                if let Some(s) = self.sectors.get(&l) {
                    b.copy_from_slice(s);
                    return Ok(());
                }
                let put16 =
                    |b: &mut [u8], i: usize, v: u16| b[i..i + 2].copy_from_slice(&v.to_le_bytes());
                let put32 =
                    |b: &mut [u8], i: usize, v: u32| b[i..i + 4].copy_from_slice(&v.to_le_bytes());
                match l {
                    0 => {
                        b[0] = 0xeb;
                        put16(b, 11, 4096);
                        b[13] = 1;
                        put16(b, 14, 32);
                        b[16] = 2;
                        put32(b, 32, 65685);
                        put32(b, 36, 64);
                        put32(b, 44, 2);
                        put16(b, 48, 1);
                        put16(b, 50, 6);
                        b[510..512].copy_from_slice(&[0x55, 0xaa]);
                    }
                    1 | 7 => {
                        put32(b, 0, 0x41615252);
                        put32(b, 484, 0x61417272);
                        put32(b, 488, 65523);
                        put32(b, 492, 4);
                        put32(b, 508, 0xaa550000);
                    }
                    32 | 96 => {
                        for c in 0..4 {
                            put32(b, c * 4, 0xfffffff);
                        }
                    }
                    160 => {
                        b[..11].copy_from_slice(b"SETTINGS   ");
                        b[11] = 0x10;
                        put16(b, 26, 3);
                    }
                    _ => {}
                }
                Ok(())
            }
            fn write(&mut self, l: u64, b: &[u8]) -> Result<(), Error> {
                self.sectors.insert(l, b.try_into().unwrap());
                Ok(())
            }
            fn sync(&mut self) -> Result<(), Error> {
                Ok(())
            }
            fn protected(&self) -> bool {
                false
            }
        }
        let mut d = SectorDisk {
            sectors: BTreeMap::new(),
        };
        let record = Memory::record(1);
        assert_eq!(
            write_owned(
                &mut d,
                65685,
                4096,
                OWNED_NAME,
                &record,
                musha_platform::pairing::recognized,
                &mut Workspace::new()
            ),
            Ok(true)
        );
        assert_eq!(d.sectors.get(&162), Some(&record));
        let fat = *d.sectors.get(&32).unwrap();
        assert_eq!(
            write_owned(
                &mut d,
                65685,
                4096,
                OWNED_NAME,
                &Memory::record(2),
                musha_platform::pairing::recognized,
                &mut Workspace::new()
            ),
            Ok(false)
        );
        assert_eq!(d.sectors.get(&32), Some(&fat));
    }
    #[test]
    fn crosslinked_alias_is_refused_before_write() {
        let mut d = Memory::new();
        d.run(&Memory::record(1)).unwrap();
        let mut root = d.sector(1056);
        let e = &mut root[32..64];
        e[..11].copy_from_slice(b"ALIAS   DAT");
        e[11] = 0x20;
        e[26..28].copy_from_slice(&4u16.to_le_bytes());
        e[28..32].copy_from_slice(&4096u32.to_le_bytes());
        d.sectors.insert(1056, root);
        let n = d.writes;
        assert_eq!(
            d.run(&Memory::record(2)),
            Err(WriteError::Refused(Refusal::Layout))
        );
        assert_eq!(d.writes, n);
    }
}
