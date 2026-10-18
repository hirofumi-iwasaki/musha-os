// Copyright 2026 Hirofumi Iwasaki
// SPDX-License-Identifier: Apache-2.0
//! Internal SETTINGS-only write preflight. No public application write API.
//! All discovery/validation completes before any disk mutation is permitted.
use crate::{Error, Reader, Volume, discover_volume, u16le, u32le, visit};
pub const DEFAULT_CAPACITY: usize = 4096;
const MAX_CLUSTERS: usize = 8;
/// Sector reads allowed for one preflight and, separately, one audit.
const READ_BUDGET: usize = 20000;
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
    /// The bounded read budget ran out before the check completed.
    Budget,
}
/// Why a request was refused, for diagnostics only: the step, the specific
/// check (`detail`) with an optional value, and sector reads per step.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Trace {
    pub step: &'static str,
    pub detail: &'static str,
    pub value: u32,
    pub preflight_reads: u32,
    pub audit_reads: u32,
}
impl Trace {
    fn layout(&mut self, detail: &'static str, value: u32) -> Refusal {
        self.detail = detail;
        self.value = value;
        Refusal::Layout
    }
    fn refuse(&mut self, refusal: Refusal, detail: &'static str) -> Refusal {
        self.detail = detail;
        refusal
    }
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
    t: &mut Trace,
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
                            return Err(t.layout("dangling long name before end marker", 0));
                        }
                        let next = (i + 1) * 32;
                        if next < r.size {
                            if b[next] != 0 {
                                return Err(t.layout("data after directory end marker", 0));
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
                                    return Err(t.layout("data after directory end marker", 1));
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
                        return Err(t.layout("duplicate directory entry", 0));
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
    preflight_traced(
        blocks,
        size,
        name,
        capacity,
        protected,
        read,
        &mut Trace::default(),
    )
}
/// `preflight` that records the refusing check and the sectors read.
pub fn preflight_traced<F: FnMut(u64, &mut [u8]) -> Result<(), Error>>(
    blocks: u64,
    size: usize,
    name: &[u8],
    capacity: usize,
    protected: bool,
    read: &mut F,
    t: &mut Trace,
) -> Result<Plan, Refusal> {
    t.step = "preflight";
    let name = short_name(name).map_err(|e| t.refuse(e, "file name"))?;
    if protected {
        return Err(t.refuse(Refusal::Protected, "write protected"));
    }
    if !matches!(size, 512 | 4096) || blocks == 0 {
        return Err(t.layout("sector size", size as u32));
    }
    if capacity == 0 || capacity > DEFAULT_CAPACITY {
        return Err(t.refuse(Refusal::Capacity, "record capacity"));
    }
    let mut r = Reader {
        read,
        blocks,
        size,
        budget: READ_BUDGET,
    };
    let result = scan(&mut r, &name, capacity, t);
    t.preflight_reads = (READ_BUDGET - r.budget) as u32;
    match result {
        Err(Refusal::Filesystem(Error::Limit)) if r.budget == 0 => {
            Err(t.refuse(Refusal::Budget, "preflight read budget"))
        }
        Err(Refusal::Filesystem(e)) => {
            if t.detail.is_empty() {
                t.detail = "filesystem read or structure";
            }
            Err(Refusal::Filesystem(e))
        }
        other => other,
    }
}
fn scan<F: FnMut(u64, &mut [u8]) -> Result<(), Error>>(
    r: &mut Reader<'_, F>,
    name: &[u8; 11],
    capacity: usize,
    t: &mut Trace,
) -> Result<Plan, Refusal> {
    let size = r.size;
    let v = discover_volume(r).inspect_err(|_| t.detail = "volume")?;
    let mut b = [0; 4096];
    r.sector(v.start, &mut b)?;
    // Require mirrored two-FAT layout and valid primary + backup FSInfo.
    if b[16] != 2 {
        return Err(t.layout("FAT count", b[16] as u32));
    }
    if u16le(&b, 40) != 0 {
        return Err(t.layout("FAT ExtFlags (mirroring off)", u16le(&b, 40) as u32));
    }
    let reserved = u16le(&b, 14) as u64;
    let fatsize = u32le(&b, 36) as u64;
    let info = u16le(&b, 48) as u64;
    let backup = u16le(&b, 50) as u64;
    if info == 0 || info >= reserved {
        return Err(t.layout("FSInfo sector", info as u32));
    }
    if backup == 0 || backup + info >= reserved {
        return Err(t.layout("backup boot sector", backup as u32));
    }
    let info = [v.start + info, v.start + backup + info];
    let mut hint = None;
    for (which, lba) in info.into_iter().enumerate() {
        r.sector(lba, &mut b)?;
        if u32le(&b, 0) != 0x41615252
            || u32le(&b, 484) != 0x61417272
            || u32le(&b, 508) != 0xaa550000
        {
            return Err(t.layout("FSInfo signature (0 primary, 1 backup)", which as u32));
        }
        let free = u32le(&b, 488);
        if free != u32::MAX && free > v.clusters {
            return Err(t.layout("FSInfo free count", free));
        }
        let next = u32le(&b, 492);
        if next != u32::MAX && !(2..v.clusters + 2).contains(&next) {
            return Err(t.layout("FSInfo next-free hint", next));
        }
        if which == 0 && next != u32::MAX {
            hint = Some(next);
        }
    }
    let (settings, _) = entry(r, v, v.root, b"SETTINGS   ", t)?;
    let settings = settings.ok_or_else(|| t.refuse(Refusal::MissingDirectory, "SETTINGS"))?;
    if settings.lfn {
        return Err(t.layout("SETTINGS has a long name", 0));
    }
    if settings.bytes[11] & 0x3f != 0x10 {
        return Err(t.layout("SETTINGS attribute", settings.bytes[11] as u32));
    }
    let settings_cluster = first(&settings.bytes);
    let (file, empty) = entry(r, v, settings_cluster, name, t)?;
    let cluster_bytes = size * v.spc as usize;
    let needed = capacity.div_ceil(cluster_bytes);
    if needed > MAX_CLUSTERS {
        return Err(t.refuse(Refusal::Capacity, "clusters for record"));
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
        if file.lfn {
            return Err(t.layout("record has a long name", 0));
        }
        if file.bytes[11] & 0x1f != 0 {
            return Err(t.layout("record attribute", file.bytes[11] as u32));
        }
        if u32le(&file.bytes, 28) as usize > capacity {
            return Err(t.layout("record size", u32le(&file.bytes, 28)));
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
                return Err(t.layout("record cluster chain", c));
            }
            plan.clusters[plan.count] = c;
            plan.count += 1;
            match v.next(r, c)? {
                Some(next) => c = next,
                None => break,
            }
        }
        if plan.count != needed {
            return Err(t.refuse(Refusal::Capacity, "record chain length"));
        }
        // Only the FAT sectors holding the chain are compared between mirrors.
        for i in 0..plan.count {
            mirrored(r, &plan, fat_index(&plan, plan.clusters[i]), t)?;
        }
    } else {
        let (lba, offset) =
            empty.ok_or_else(|| t.refuse(Refusal::FullDirectory, "SETTINGS directory full"))?;
        plan.directory_lba = lba;
        plan.directory_offset = offset;
        // Search for free clusters from the FSInfo hint, wrapping once, reading
        // and mirror-comparing only the FAT sectors visited. The whole FAT is not
        // scanned: on large media it exceeds the read budget (IMG_7741).
        let per = (size / 4) as u64;
        let last = (v.clusters as u64 + 1) / per;
        let start = hint.map_or(0, |h| h as u64 / per).min(last);
        let mut n = start;
        loop {
            let a = mirrored(r, &plan, n, t)?;
            for (i, e) in a[..size].chunks_exact(4).enumerate() {
                let c = (n * per + i as u64) as u32;
                if c >= 2
                    && c < v.clusters + 2
                    && u32le(e, 0) & 0x0fffffff == 0
                    && plan.count < needed
                {
                    plan.clusters[plan.count] = c;
                    plan.count += 1;
                }
            }
            if plan.count == needed {
                break;
            }
            n = if n == last { 0 } else { n + 1 };
            if n == start {
                return Err(t.refuse(Refusal::NoSpace, "no free cluster"));
            }
        }
    }
    Ok(plan)
}
fn fat_index(p: &Plan, cluster: u32) -> u64 {
    cluster as u64 * 4 / p.sector_size as u64
}
/// Read one FAT sector from both mirrors; they must be identical.
fn mirrored<F: FnMut(u64, &mut [u8]) -> Result<(), Error>>(
    r: &mut Reader<'_, F>,
    p: &Plan,
    n: u64,
    t: &mut Trace,
) -> Result<[u8; 4096], Refusal> {
    if n >= p.fatsize {
        return Err(t.layout("FAT index beyond FAT", n as u32));
    }
    let mut a = [0; 4096];
    let mut b = [0; 4096];
    r.sector(p.volume.fat + n, &mut a)?;
    r.sector(p.volume.fat + p.fatsize + n, &mut b)?;
    if a[..p.sector_size] != b[..p.sector_size] {
        return Err(t.layout("FAT mirrors differ at FAT sector", n as u32));
    }
    Ok(a)
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
fn audit<D: Disk>(disk: &mut D, blocks: u64, p: &Plan, t: &mut Trace) -> Result<(), WriteError> {
    t.step = "audit";
    let mut r = Reader {
        read: &mut |l: u64, b: &mut [u8]| disk.read(l, b),
        blocks,
        size: p.sector_size,
        budget: READ_BUDGET,
    };
    let result = audit_inner(&mut r, p, t);
    t.audit_reads = (READ_BUDGET - r.budget) as u32;
    match result {
        Err(WriteError::Io(Error::Limit)) if r.budget == 0 => Err(WriteError::Refused(
            t.refuse(Refusal::Budget, "audit read budget"),
        )),
        other => other,
    }
}
fn audit_inner<F: FnMut(u64, &mut [u8]) -> Result<(), Error>>(
    r: &mut Reader<'_, F>,
    p: &Plan,
    t: &mut Trace,
) -> Result<(), WriteError> {
    // Each selected cluster may have only its predecessor as incoming FAT
    // reference (none for a first/free cluster), which rejects shared chain
    // tails. Only the FAT sectors holding the selected clusters are examined;
    // a whole-FAT scan does not fit the read budget on large media.
    let mut sectors = [u64::MAX; MAX_CLUSTERS];
    for (i, &c) in p.clusters().iter().enumerate() {
        let n = fat_index(p, c);
        if sectors.contains(&n) {
            continue;
        }
        sectors[i] = n;
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
                    return Err(WriteError::Refused(t.layout("shared cluster chain", next)));
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
                        return Err(WriteError::Refused(
                            t.layout("directory entries audited (limit 4096)", entries),
                        ));
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
                        return Err(WriteError::Refused(
                            t.layout("another directory entry uses the record cluster", head),
                        ));
                    }
                    if e[11] & 0x10 != 0 {
                        if used == dirs.len() {
                            return Err(WriteError::Refused(
                                t.layout("directories audited (limit 64)", used as u32),
                            ));
                        }
                        if dirs[..used].contains(&head) {
                            return Err(WriteError::Refused(
                                t.layout("directory referenced twice", head),
                            ));
                        }
                        p.volume.cluster(head).map_err(WriteError::Io)?;
                        dirs[used] = head;
                        used += 1;
                    }
                }
            }
            match p.volume.next(r, c).map_err(WriteError::Io)? {
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
    write_owned_traced(
        disk,
        blocks,
        size,
        name,
        record,
        recognized,
        workspace,
        &mut Trace::default(),
    )
}
/// `write_owned` that records the refusing step/check and sectors read.
#[allow(clippy::too_many_arguments)]
pub fn write_owned_traced(
    disk: &mut impl Disk,
    blocks: u64,
    size: usize,
    name: &[u8],
    record: &[u8],
    recognized: fn(&[u8]) -> bool,
    workspace: &mut Workspace,
    t: &mut Trace,
) -> Result<bool, WriteError> {
    t.step = "request";
    if name != OWNED_NAME {
        t.detail = "file name not owned";
        return Err(WriteError::Unowned);
    }
    if record.len() != DEFAULT_CAPACITY || !recognized(record) {
        t.detail = "record not recognized";
        return Err(WriteError::Unowned);
    }
    let protected = disk.protected();
    let p = preflight_traced(
        blocks,
        size,
        name,
        DEFAULT_CAPACITY,
        protected,
        &mut |l, b| disk.read(l, b),
        t,
    )?;
    audit(disk, blocks, &p, t)?;
    t.step = "verify-record";
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
            t.detail = "existing PAIRING.DAT not an OS record";
            return Err(WriteError::Unowned);
        }
    }
    t.step = "write";
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
            // The free count is no longer verified by a whole-FAT scan: keep it
            // as a decremented hint, or mark it unknown when it cannot cover
            // the allocation. Unknown (0xFFFFFFFF) is valid FAT32.
            let free = u32le(&patch.new, 488);
            if free != u32::MAX {
                let free = free.checked_sub(p.count as u32).unwrap_or(u32::MAX);
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
    // Synthetic transcripts/nonces only; this module is never linked as runtime.
    fn verified_pairing() -> musha_platform::bluetooth::legacy::VerifiedBond {
        use musha_platform::bluetooth::{
            Event,
            legacy::{Action, Association, Legacy, c1},
        };
        let mut s = Legacy::new(
            1,
            0x40,
            0,
            Association::JustWorks,
            [1; 16],
            0,
            [1; 6],
            0,
            [2; 6],
        )
        .unwrap();
        let response = [2, 3, 0, 1, 16, 0, 1];
        let request = s.request();
        s.receive(1, 1, &response).unwrap();
        let confirm = c1(
            &[0; 16], &[2; 16], &request, &response, 0, 0, &[1; 6], &[2; 6],
        );
        let mut p = [3; 17];
        p[1..].copy_from_slice(&confirm);
        s.receive(1, 2, &p).unwrap();
        p[0] = 4;
        p[1..].fill(2);
        assert!(matches!(s.receive(1, 3, &p), Ok(Action::Encrypt(_))));
        s.encryption(1, 4, Event::parse(&[8, 4, 0, 0x40, 0, 1]).unwrap())
            .unwrap();
        s.receive(1, 5, &[6; 17]).unwrap();
        match s.receive(1, 6, &[7; 11]).unwrap() {
            Action::Bond(b) => b,
            _ => panic!(),
        }
    }
    #[test]
    fn verified_smp_bond_connects_to_real_settings_writer_and_readback() {
        let mut d = Memory::new();
        let root = d.sector(1056);
        let hash = musha_platform::config_hash::sha256(b"pair=on\r\n");
        let result = verified_pairing().save(None, Some(hash), |record| {
            d.run(record)
                .map(|_| ())
                .map_err(|_| musha_platform::bluetooth::Error::Invalid)
        });
        assert_eq!(result, Ok(Some(hash)));
        assert_eq!(d.sector(1056), root);
        let p = preflight(66581, 512, OWNED_NAME, 4096, false, &mut |l, b| {
            d.read(l, b)
        })
        .unwrap();
        let mut record = [0; 4096];
        for (i, b) in record.chunks_mut(512).enumerate() {
            d.read(p.data_sector(i).unwrap(), b).unwrap();
        }
        let mut out = [*verified_pairing().bond(); 8];
        let (n, consumed) = musha_platform::pairing::decode_state(&record, &mut out).unwrap();
        assert_eq!(n, 1);
        assert_eq!(consumed, Some(hash));
        assert!(!musha_platform::pairing::requested(
            true,
            Some(hash),
            consumed
        ));
        assert!(musha_platform::pairing::requested(
            true,
            Some(musha_platform::config_hash::sha256(b"pair=on\n#again\n")),
            consumed
        ));
        let before = d.writes;
        d.sync_fail = true;
        assert_eq!(
            verified_pairing().save(Some(&record), Some([9; 32]), |r| d
                .run(r)
                .map(|_| ())
                .map_err(|_| musha_platform::bluetooth::Error::Invalid)),
            Err(musha_platform::bluetooth::bonding::SaveError::PersistenceUnknown)
        );
        assert!(d.writes > before); // RAM not published; disk is explicitly uncertain.
    }
}

#[cfg(test)]
mod large_media_tests {
    //! Geometry of the IMG_7741 medium: 60,123,103 sectors of 512 bytes and
    //! 16 KiB clusters (macOS-formatted). Contents are synthetic.
    extern crate std;
    use super::*;
    use std::collections::BTreeMap;
    const TOTAL: u64 = 60_123_103;
    const SPC: u64 = 32;
    const RESERVED: u64 = 32;
    fn fatsize() -> u64 {
        // Smallest FAT that covers the data clusters of this volume.
        let mut fat = 1;
        loop {
            let clusters = (TOTAL - RESERVED - 2 * fat) / SPC;
            if (clusters + 2) * 4 <= fat * 512 {
                return fat;
            }
            fat += 1;
        }
    }
    struct Large {
        fat: u64,
        sectors: BTreeMap<u64, [u8; 512]>,
        attribute: u8,
        lfn: bool,
        fats: u8,
        backup_signature: bool,
        full_until: u64,
        reads: usize,
    }
    impl Large {
        fn new() -> Self {
            Self {
                fat: fatsize(),
                sectors: BTreeMap::new(),
                attribute: 0x10,
                lfn: false,
                fats: 2,
                backup_signature: true,
                full_until: 0,
                reads: 0,
            }
        }
        fn data(&self) -> u64 {
            RESERVED + 2 * self.fat
        }
        fn clusters(&self) -> u32 {
            ((TOTAL - self.data()) / SPC) as u32
        }
        fn run(&mut self, t: &mut Trace) -> Result<bool, WriteError> {
            let mut record = [0; 4096];
            musha_platform::pairing::encode_state(&[], Some([1; 32]), &mut record).unwrap();
            write_owned_traced(
                self,
                TOTAL,
                512,
                OWNED_NAME,
                &record,
                musha_platform::pairing::recognized,
                &mut Workspace::new(),
                t,
            )
        }
    }
    impl Disk for Large {
        fn read(&mut self, l: u64, b: &mut [u8]) -> Result<(), Error> {
            assert!(l < TOTAL);
            self.reads += 1;
            if let Some(s) = self.sectors.get(&l) {
                b.copy_from_slice(s);
                return Ok(());
            }
            b.fill(0);
            let put16 =
                |b: &mut [u8], i: usize, v: u16| b[i..i + 2].copy_from_slice(&v.to_le_bytes());
            let put32 =
                |b: &mut [u8], i: usize, v: u32| b[i..i + 4].copy_from_slice(&v.to_le_bytes());
            let root = self.data();
            match l {
                0 => {
                    b[0] = 0xeb;
                    put16(b, 11, 512);
                    b[13] = SPC as u8;
                    put16(b, 14, RESERVED as u16);
                    b[16] = self.fats;
                    put32(b, 32, TOTAL as u32);
                    put32(b, 36, self.fat as u32);
                    put32(b, 44, 2);
                    put16(b, 48, 1);
                    put16(b, 50, 6);
                    b[510..512].copy_from_slice(&[0x55, 0xaa]);
                }
                1 | 7 => {
                    if l == 1 || self.backup_signature {
                        put32(b, 0, 0x41615252);
                        put32(b, 484, 0x61417272);
                        put32(b, 508, 0xaa550000);
                    }
                    put32(b, 488, self.clusters() - 2);
                    put32(b, 492, 4);
                }
                _ if l >= RESERVED && l < RESERVED + 2 * self.fat => {
                    let n = (l - RESERVED) % self.fat;
                    if n < self.full_until {
                        for e in b.chunks_exact_mut(4) {
                            e.copy_from_slice(&0x0fffffffu32.to_le_bytes());
                        }
                    }
                    if n == 0 {
                        for c in 0..4 {
                            put32(b, c * 4, 0x0fffffff);
                        }
                    }
                }
                _ if l == root => {
                    let mut at = 0;
                    if self.lfn {
                        b[11] = 0x0f;
                        b[0] = 0x41;
                        at = 32;
                    }
                    b[at..at + 11].copy_from_slice(b"SETTINGS   ");
                    b[at + 11] = self.attribute;
                    put16(b, at + 26, 3);
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
    #[test]
    fn whole_fat_scan_exceeded_the_read_budget_on_this_medium() {
        // About 1.88 million clusters: one FAT is ~14,700 sectors, so the former
        // mirrored whole-FAT preflight needed ~29,400 reads against 20,000.
        let d = Large::new();
        assert!(d.clusters() > 1_800_000);
        assert!(2 * d.fat > READ_BUDGET as u64, "fat {}", d.fat);
    }
    #[test]
    fn create_then_overwrite_read_few_sectors_on_large_media() {
        let mut d = Large::new();
        let mut t = Trace::default();
        assert_eq!(d.run(&mut t), Ok(true), "{t:?}");
        assert!(t.preflight_reads < 32 && t.audit_reads < 32, "{t:?}");
        // FSInfo keeps a decremented hint; both FSInfo sectors match.
        let mut info = [0; 512];
        d.read(1, &mut info).unwrap();
        assert_eq!(u32le(&info, 488), d.clusters() - 3);
        let mut t = Trace::default();
        assert_eq!(d.run(&mut t), Ok(false), "{t:?}");
        assert!(t.preflight_reads < 32 && t.audit_reads < 32, "{t:?}");
        assert!(d.reads < 200, "{}", d.reads);
    }
    #[test]
    fn exhausted_search_is_reported_as_budget_not_layout() {
        let mut d = Large::new();
        d.full_until = 10_500; // first free cluster lies beyond 21,000 reads
        let mut t = Trace::default();
        assert_eq!(d.run(&mut t), Err(WriteError::Refused(Refusal::Budget)));
        assert_eq!((t.step, t.detail), ("preflight", "preflight read budget"));
        assert_eq!(t.preflight_reads as usize, READ_BUDGET);
        assert!(d.sectors.is_empty());
    }
    #[test]
    fn layout_refusals_name_the_failing_check() {
        for (setup, refusal, detail, value) in [
            (
                (|d: &mut Large| d.attribute = 0x30) as fn(&mut Large),
                Refusal::Layout,
                "SETTINGS attribute",
                0x30,
            ),
            (
                |d: &mut Large| d.lfn = true,
                Refusal::Layout,
                "SETTINGS has a long name",
                0,
            ),
            (
                |d: &mut Large| d.backup_signature = false,
                Refusal::Layout,
                "FSInfo signature (0 primary, 1 backup)",
                1,
            ),
            // One FAT changes the data start; the volume parser refuses first.
            (
                |d: &mut Large| d.fats = 1,
                Refusal::Filesystem(Error::Corrupt),
                "volume",
                0,
            ),
        ] {
            let mut d = Large::new();
            setup(&mut d);
            let mut t = Trace::default();
            assert_eq!(d.run(&mut t), Err(WriteError::Refused(refusal)));
            assert_eq!((t.step, t.detail, t.value), ("preflight", detail, value));
            assert!(d.sectors.is_empty());
        }
    }
}
