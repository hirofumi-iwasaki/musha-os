// Copyright 2026 Hirofumi Iwasaki
// SPDX-License-Identifier: Apache-2.0
//! Resumable per-file allocation validation after an immutable volume is mounted.
use crate::{Error, read_plan::Plan, u32le};
#[derive(Clone, Copy)]
pub(crate) struct Geometry {
    pub blocks: u64,
    pub end: u64,
    pub fat: u64,
    pub data: u64,
    pub clusters: u32,
    pub spc: u32,
    pub size: usize,
    pub exfat: bool,
}
#[derive(Clone, Copy)]
pub(crate) struct File {
    pub first: u32,
    pub length: u64,
    pub valid: u64,
    pub contiguous: bool,
    pub bitmap: Option<(u32, u64)>,
}
/// Detached immutable file metadata. Contains no backend closure or workspace borrow.
/// The owner must discard it when its mounted medium is replaced or disconnected;
/// route requests through that mount's API generation before creating a planner.
#[derive(Clone, Copy)]
pub struct Source {
    geometry: Geometry,
    file: File,
}
impl Source {
    pub(crate) fn new(geometry: Geometry, file: File) -> Self {
        Self { geometry, file }
    }
    /// No I/O. Every range revalidates allocation using fresh sector requests.
    pub fn begin_plan(self, offset: u64, requested: usize) -> Result<Planner, Error> {
        if self.geometry.exfat && requested > 4096 {
            return Err(Error::Limit);
        }
        Planner::new(self.geometry, self.file, offset, requested)
    }
}
impl Geometry {
    fn cluster(self, n: u32) -> Result<u64, Error> {
        if n < 2 || n as u64 >= self.clusters as u64 + 2 {
            return Err(Error::Corrupt);
        }
        let lba = self.data + (n as u64 - 2) * self.spc as u64;
        if lba + self.spc as u64 > self.end || self.end > self.blocks {
            return Err(Error::Corrupt);
        }
        Ok(lba)
    }
    fn fat_lba(self, n: u32) -> Result<u64, Error> {
        self.cluster(n)?;
        Ok(self.fat + n as u64 * 4 / self.size as u64)
    }
    fn next(self, n: u32, bytes: &[u8]) -> Result<Option<u32>, Error> {
        let raw = u32le(bytes, (n as usize * 4) % self.size);
        let next = if self.exfat { raw } else { raw & 0x0fffffff };
        if if self.exfat {
            next == u32::MAX
        } else {
            next >= 0x0ffffff8
        } {
            return Ok(None);
        }
        self.cluster(next)?;
        Ok(Some(next))
    }
}
#[derive(Clone, Copy)]
struct Chain {
    current: u32,
    anchor: u32,
    power: u64,
    distance: u64,
}
impl Chain {
    fn new(n: u32) -> Self {
        Self {
            current: n,
            anchor: n,
            power: 1,
            distance: 0,
        }
    }
    fn advance(&mut self, n: u32) -> Result<(), Error> {
        self.distance += 1;
        if n == self.anchor {
            return Err(Error::Corrupt);
        }
        self.current = n;
        if self.distance == self.power {
            self.anchor = n;
            self.power *= 2;
            self.distance = 0;
        }
        Ok(())
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Request {
    serial: u64,
    lba: u64,
    bytes: usize,
}
impl Request {
    pub fn lba(self) -> u64 {
        self.lba
    }
    pub fn bytes(self) -> usize {
        self.bytes
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Progress {
    Yield,
    Pending,
    Read(Request),
    Complete,
}
#[derive(Clone, Copy)]
enum Stage {
    Cluster,
    BitmapChain,
    BitmapBit,
    Map,
    FileNext,
    Done,
}
/// One planner per read; tokens belong only to this instance. poll performs one
/// bounded metadata step, never I/O. complete consumes at most one sector.
/// Mount/discovery is still synchronous. Media must remain unchanged throughout.
pub struct Planner {
    geometry: Geometry,
    file: File,
    stage: Stage,
    chain: Chain,
    bitmap: Chain,
    bitmap_left: u64,
    offset: u64,
    index: u64,
    count: u64,
    plan: Option<Plan>,
    pending: Option<Request>,
    serial: u64,
    error: Option<Error>,
    canceled: bool,
}
impl Planner {
    pub(crate) fn new(g: Geometry, f: File, offset: u64, requested: usize) -> Result<Self, Error> {
        if requested > 4096 || f.length > 64 * 1024 * 1024 {
            return Err(Error::TooLarge);
        }
        if f.valid > f.length || (f.length == 0 && f.first != 0) {
            return Err(Error::Corrupt);
        }
        let count = f.length.div_ceil(g.spc as u64 * g.size as u64);
        if count > g.clusters as u64 {
            return Err(Error::Corrupt);
        }
        let bytes = f.length.saturating_sub(offset).min(requested as u64) as usize;
        // exFAT historically validates even EOF reads; preserve that behavior.
        let done = f.length == 0 || (!g.exfat && bytes == 0);
        Ok(Self {
            geometry: g,
            file: f,
            stage: if done { Stage::Done } else { Stage::Cluster },
            chain: Chain::new(f.first),
            bitmap: Chain::new(0),
            bitmap_left: 0,
            offset,
            index: 0,
            count,
            plan: Some(Plan::new(bytes, g.size)),
            pending: None,
            serial: 0,
            error: None,
            canceled: false,
        })
    }
    fn request(&mut self, lba: u64) -> Result<Progress, Error> {
        if self.serial == 1_000_000 {
            return Err(Error::Limit);
        }
        if lba >= self.geometry.blocks {
            return Err(Error::Corrupt);
        }
        self.serial += 1;
        let r = Request {
            serial: self.serial,
            lba,
            bytes: self.geometry.size,
        };
        self.pending = Some(r);
        Ok(Progress::Read(r))
    }
    pub fn poll(&mut self) -> Result<Progress, Error> {
        if let Some(error) = self.error {
            return Err(error);
        }
        if self.canceled {
            return Err(Error::Io);
        }
        if self.pending.is_some() {
            return Ok(Progress::Pending);
        }
        let result = self.step().map(|progress| {
            if matches!(self.stage, Stage::Done) {
                Progress::Complete
            } else {
                progress
            }
        });
        if let Err(error) = result {
            self.error = Some(error);
            self.plan = None;
        }
        result
    }
    fn step(&mut self) -> Result<Progress, Error> {
        let g = self.geometry;
        let cb = g.spc as u64 * g.size as u64;
        match self.stage {
            Stage::Cluster => {
                g.cluster(self.chain.current)?;
                if let Some((first, length)) = self.file.bitmap {
                    let byte = (self.chain.current as u64 - 2) / 8;
                    if byte >= length {
                        return Err(Error::Corrupt);
                    }
                    self.bitmap = Chain::new(first);
                    self.bitmap_left = byte / cb;
                    self.stage = if self.bitmap_left == 0 {
                        Stage::BitmapBit
                    } else {
                        Stage::BitmapChain
                    };
                } else {
                    self.stage = Stage::Map;
                }
                Ok(Progress::Yield)
            }
            Stage::BitmapChain => self.request(g.fat_lba(self.bitmap.current)?),
            Stage::BitmapBit => {
                let byte = (self.chain.current as u64 - 2) / 8;
                self.request(g.cluster(self.bitmap.current)? + (byte % cb) / g.size as u64)
            }
            Stage::Map => {
                let plan = self.plan.as_mut().ok_or(Error::Corrupt)?;
                let start = self.index * cb;
                let end = self
                    .file
                    .valid
                    .min(self.offset.saturating_add(plan.len() as u64))
                    .min(start + cb);
                let mut pos = self.offset.max(start);
                // Across the whole plan, at most nine sector pieces are appended.
                while pos < end {
                    let inside = pos - start;
                    let skip = (inside % g.size as u64) as usize;
                    let n = (end - pos).min((g.size - skip) as u64) as usize;
                    plan.push(
                        g.cluster(self.chain.current)? + inside / g.size as u64,
                        skip,
                        n,
                        (pos - self.offset) as usize,
                    )?;
                    pos += n as u64;
                }
                if self.file.contiguous {
                    self.index += 1;
                    if self.index == self.count {
                        self.stage = Stage::Done;
                    } else {
                        self.chain.current =
                            self.chain.current.checked_add(1).ok_or(Error::Corrupt)?;
                        self.stage = Stage::Cluster;
                    }
                } else {
                    self.stage = Stage::FileNext;
                }
                Ok(Progress::Yield)
            }
            Stage::FileNext => self.request(g.fat_lba(self.chain.current)?),
            Stage::Done => Ok(Progress::Complete),
        }
    }
    /// Only acknowledge transport completion after its DMA is no longer active.
    /// Cancellation keeps the pending token until that acknowledgement; late data
    /// is discarded. False rejects stale/duplicate tokens without changing state.
    pub fn complete(&mut self, r: Request, result: Result<&[u8], Error>) -> bool {
        if self.pending != Some(r) {
            return false;
        }
        self.pending = None;
        if self.canceled {
            return true;
        }
        let result = result.and_then(|b| {
            if b.len() != self.geometry.size {
                return Err(Error::Corrupt);
            }
            match self.stage {
                Stage::BitmapChain => {
                    let next = self
                        .geometry
                        .next(self.bitmap.current, b)?
                        .ok_or(Error::Corrupt)?;
                    self.bitmap.advance(next)?;
                    self.bitmap_left -= 1;
                    if self.bitmap_left == 0 {
                        self.stage = Stage::BitmapBit;
                    }
                }
                Stage::BitmapBit => {
                    let n = self.chain.current - 2;
                    let byte = n as usize / 8;
                    if b[byte % self.geometry.size] & (1 << (n % 8)) == 0 {
                        return Err(Error::Corrupt);
                    }
                    self.stage = Stage::Map;
                }
                Stage::FileNext => {
                    let next = self.geometry.next(self.chain.current, b)?;
                    self.index += 1;
                    match (next, self.index == self.count) {
                        (None, true) => self.stage = Stage::Done,
                        (Some(next), false) => {
                            self.chain.advance(next)?;
                            self.stage = Stage::Cluster;
                        }
                        _ => return Err(Error::Corrupt),
                    }
                }
                _ => return Err(Error::Corrupt),
            }
            Ok(())
        });
        if let Err(error) = result {
            self.error = Some(error);
            self.plan = None;
        }
        true
    }
    pub fn take_plan(&mut self) -> Result<Option<Plan>, Error> {
        if let Some(error) = self.error {
            return Err(error);
        }
        if self.canceled {
            return Err(Error::Io);
        }
        if matches!(self.stage, Stage::Done) {
            Ok(self.plan.take())
        } else {
            Ok(None)
        }
    }
    pub(crate) fn run<F: FnMut(u64, &mut [u8]) -> Result<(), Error>>(
        &mut self,
        read: &mut F,
    ) -> Result<Plan, Error> {
        let mut sector = [0; 4096];
        loop {
            match self.poll()? {
                Progress::Yield => {}
                Progress::Read(r) => {
                    let result = read(r.lba(), &mut sector[..r.bytes()]);
                    self.complete(r, result.map(|()| &sector[..r.bytes()]));
                }
                Progress::Complete => return self.take_plan()?.ok_or(Error::Corrupt),
                Progress::Pending => return Err(Error::Io),
            }
        }
    }
    pub fn cancel(&mut self) {
        self.canceled = true;
        self.plan = None;
    }
    pub fn is_quiescent(&self) -> bool {
        self.pending.is_none()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn setup(exfat: bool, contiguous: bool) -> Planner {
        Planner::new(
            Geometry {
                blocks: 70000,
                end: 70000,
                fat: 1,
                data: 600,
                clusters: 66000,
                spc: 1,
                size: 512,
                exfat,
            },
            File {
                first: 2,
                length: 700,
                valid: 650,
                contiguous,
                bitmap: exfat.then_some((5, 8250)),
            },
            1,
            699,
        )
        .unwrap()
    }
    fn reply(r: Request, exfat: bool) -> [u8; 512] {
        let mut b = [0; 512];
        if r.lba == 1 {
            b[8..12].copy_from_slice(&3u32.to_le_bytes());
            b[12..16]
                .copy_from_slice(&(if exfat { u32::MAX } else { 0x0fffffffu32 }).to_le_bytes());
        } else {
            b.fill(255);
        }
        b
    }
    #[test]
    fn sector_waits_yield_and_complete_plan_only_after_all_checks() {
        for (exfat, contiguous) in [(false, false), (true, false), (true, true)] {
            let mut p = setup(exfat, contiguous);
            let mut previous = None;
            let mut turns = 0;
            loop {
                turns += 1;
                assert!(turns < 30);
                match p.poll().unwrap() {
                    Progress::Yield => assert!(p.take_plan().unwrap().is_none()),
                    Progress::Read(r) => {
                        for _ in 0..10 {
                            assert_eq!(p.poll(), Ok(Progress::Pending));
                            assert!(p.take_plan().unwrap().is_none());
                        }
                        if let Some(old) = previous {
                            assert!(!p.complete(old, Ok(&reply(r, exfat))));
                        }
                        assert!(p.complete(r, Ok(&reply(r, exfat))));
                        assert!(!p.complete(r, Ok(&reply(r, exfat))));
                        previous = Some(r);
                    }
                    Progress::Complete => break,
                    Progress::Pending => panic!(),
                }
            }
            let plan = p.take_plan().unwrap().unwrap();
            assert_eq!(plan.len(), 699);
            let mut out = [0xa5; 699];
            let mut data_requests = 0;
            plan.execute(
                &mut |lba, b| {
                    assert!(lba == 600 || lba == 601);
                    data_requests += 1;
                    b.fill(lba as u8);
                    Ok(())
                },
                &mut out,
            )
            .unwrap();
            assert_eq!(data_requests, 2);
            assert!(out[649..].iter().all(|&b| b == 0));
            assert!(p.take_plan().unwrap().is_none());
        }
    }
    #[test]
    fn cancel_waiting_on_each_metadata_request_discards_late_data() {
        for cancel_at in 0..4 {
            let mut p = setup(true, false);
            let mut requests = 0;
            loop {
                match p.poll().unwrap() {
                    Progress::Read(r) => {
                        if requests == cancel_at {
                            p.cancel();
                            assert!(!p.is_quiescent());
                            assert_eq!(p.poll(), Err(Error::Io));
                            assert!(p.take_plan().is_err());
                            assert!(p.complete(r, Ok(&reply(r, true))));
                            assert!(p.is_quiescent());
                            assert!(p.take_plan().is_err());
                            break;
                        }
                        requests += 1;
                        p.complete(r, Ok(&reply(r, true)));
                    }
                    Progress::Yield => {}
                    _ => panic!("cancel target not reached"),
                }
            }
        }
    }
    #[test]
    fn malformed_metadata_and_io_never_publish_partial_plan() {
        for kind in 0..5 {
            let mut p = setup(true, false);
            let mut n = 0;
            let error = loop {
                match p.poll() {
                    Ok(Progress::Read(r)) => {
                        let mut b = reply(r, true);
                        if kind == 0 {
                            b.fill(0);
                        }
                        if kind == 1 && r.lba == 1 {
                            b[8..12].copy_from_slice(&2u32.to_le_bytes());
                        }
                        if kind == 2 && r.lba == 1 {
                            b[8..12].copy_from_slice(&u32::MAX.to_le_bytes());
                        }
                        p.complete(
                            r,
                            if kind == 3 {
                                Err(Error::Io)
                            } else if kind == 4 {
                                Ok(&b[..511])
                            } else {
                                Ok(&b)
                            },
                        );
                        n += 1;
                        assert!(n < 10);
                    }
                    Ok(Progress::Yield) => {}
                    Err(e) => break e,
                    _ => panic!(),
                }
            };
            assert_eq!(error, if kind == 3 { Error::Io } else { Error::Corrupt });
            assert!(p.take_plan().is_err());
        }
    }
    #[test]
    fn fragmented_bitmap_walk_cycles_and_io_budget() {
        for cycle in [false, true] {
            let mut p = setup(true, true);
            p.file.first = 8194;
            p.chain = Chain::new(8194);
            p.file.length = 512;
            p.file.valid = 512;
            p.count = 1;
            let mut steps = 0;
            loop {
                steps += 1;
                assert!(steps < 20);
                match p.poll() {
                    Ok(Progress::Read(r)) => {
                        let mut b = [255; 512];
                        if r.lba == 1 {
                            b[20..24]
                                .copy_from_slice(&(if cycle { 5u32 } else { 7u32 }).to_le_bytes());
                            b[28..32].copy_from_slice(&9u32.to_le_bytes());
                        } else {
                            assert_eq!(r.lba, 607);
                        }
                        p.complete(r, Ok(&b));
                    }
                    Ok(Progress::Yield) => {}
                    Ok(Progress::Complete) => {
                        assert!(!cycle);
                        assert!(p.take_plan().unwrap().is_some());
                        break;
                    }
                    Err(e) => {
                        assert!(cycle);
                        assert_eq!(e, Error::Corrupt);
                        break;
                    }
                    _ => panic!(),
                }
            }
        }
        let mut p = setup(false, false);
        p.serial = 1_000_000;
        loop {
            match p.poll() {
                Ok(Progress::Yield) => {}
                Err(e) => {
                    assert_eq!(e, Error::Limit);
                    break;
                }
                _ => panic!(),
            }
        }
        assert!(core::mem::size_of::<Planner>() < 2048);
    }
}
