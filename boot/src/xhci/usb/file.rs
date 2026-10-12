// Copyright 2026 Hirofumi Iwasaki
// SPDX-License-Identifier: Apache-2.0
//! Opt-in end-to-end adapter exercise. Enumerate and read an immutable file catalog with bounded requests.
//! Catalog discovery precedes input polling; metadata/data reads below do not.
use super::{Host, storage::Device};
use core::{
    cell::UnsafeCell,
    sync::atomic::{AtomicBool, Ordering},
};
use musha_api::stream::{
    Cursor, Entry, Error as ApiError, Files, Format, Handle, Request as FileRequest,
};
use musha_fs::{
    operation::{Progress, Read, Request},
    planner::Source,
};
pub(super) struct Prepared {
    pub source: Source,
    pub count: usize,
    pub format: Format,
    pub bytes: usize,
    pub hash: u64,
}
struct State {
    files: Files<'static>,
    operation: Option<Read>,
    request: Option<FileRequest>,
    sector: Option<Request>,
    handle: Option<Handle>,
    sector_bytes: [u8; 4096],
    result: [u8; 4096],
    done: bool,
}
struct Shared<T>(UnsafeCell<T>);
// Single boot CPU, interrupts disabled. CLAIMED permits exactly one session and
// is never released. No reference into these CPU buffers is passed to DMA.
unsafe impl<T> Sync for Shared<T> {}
static CLAIMED: AtomicBool = AtomicBool::new(false);
static STATE: Shared<State> = Shared(UnsafeCell::new(State {
    files: Files::new(),
    operation: None,
    request: None,
    sector: None,
    handle: None,
    sector_bytes: [0; 4096],
    result: [0; 4096],
    done: false,
}));
const MAX_FILES: usize = musha_api::stream::MAX_FILES;
static CATALOG: Shared<[Entry; MAX_FILES]> = Shared(UnsafeCell::new(
    [const {
        Entry {
            name: [0; 255],
            name_len: 0,
            bytes: 0,
        }
    }; MAX_FILES],
));
static SOURCES: Shared<[Option<Source>; MAX_FILES]> = Shared(UnsafeCell::new([None; MAX_FILES]));
const _: () = assert!(
    core::mem::size_of::<State>()
        + core::mem::size_of::<[Entry; MAX_FILES]>()
        + core::mem::size_of::<[Option<Source>; MAX_FILES]>()
        < 128 * 1024
);
pub(super) struct Session {
    state: &'static mut State,
    sources: &'static [Option<Source>],
    cursor: Cursor,
    index: usize,
    original_bytes: usize,
    position: usize,
    hash: u64,
    expected: u64,
    bytes: usize,
}
impl Session {
    pub(super) fn new(prepared: Prepared) -> Result<Self, &'static str> {
        if CLAIMED.swap(true, Ordering::AcqRel) {
            return Err("LIVE FILE SESSION LIMIT");
        }
        let state = unsafe { &mut *STATE.0.get() };
        let catalog = unsafe { &mut *CATALOG.0.get() };
        let sources = unsafe { &mut *SOURCES.0.get() };
        if prepared.format == Format::Fat32 {
            for (to, from) in catalog[0].name.iter_mut().zip(b"MUSHA.TXT") {
                *to = *from as u16;
            }
            catalog[0].name_len = 9;
            catalog[0].bytes = prepared.bytes as u64;
            sources[0] = Some(prepared.source);
        }
        state
            .files
            .mount(prepared.format, &catalog[..prepared.count])
            .map_err(|_| "LIVE FILE API MOUNT")?;
        let cursor = state.files.list_begin().map_err(|_| "LIVE FILE LIST")?;
        let expected = prepared.hash;
        let bytes = prepared.bytes;
        crate::debug(b"MUSHA: LIVE_FILE_API_READY\n");
        Ok(Self {
            state,
            sources: &sources[..prepared.count],
            cursor,
            index: 0,
            original_bytes: bytes,
            position: 0,
            hash: 0xcbf29ce484222325,
            expected,
            bytes,
        })
    }
    pub(super) fn step(
        &mut self,
        host: &mut Host<'_>,
        device: &mut Device,
    ) -> Result<(), &'static str> {
        let s = &mut self.state;
        if s.done {
            return Ok(());
        }
        if let Some(sector) = s.sector {
            if device.poll_read(host, &mut s.sector_bytes[..sector.bytes()])? {
                if !s
                    .operation
                    .as_mut()
                    .ok_or("LIVE FILE OPERATION")?
                    .complete(sector, Ok(&s.sector_bytes[..sector.bytes()]))
                {
                    return Err("LIVE FILE TOKEN");
                }
                s.sector = None;
            }
            return Ok(());
        }
        if let Some(operation) = s.operation.as_mut() {
            match operation.poll().map_err(|_| "LIVE FILE ALLOCATION")? {
                Progress::Yield => {}
                Progress::Pending => return Err("LIVE FILE OWNERSHIP"),
                Progress::Read(r) => {
                    device.begin_read(host, r.lba())?;
                    s.sector = Some(r);
                    device.delay_read(host)?;
                }
                Progress::Complete => {
                    let n = operation
                        .finish(&mut s.result)
                        .map_err(|_| "LIVE FILE FINISH")?
                        .ok_or("LIVE FILE FINISH")?;
                    if !s.files.complete(
                        s.request.take().ok_or("LIVE FILE REQUEST")?,
                        Ok(&s.result[..n]),
                    ) {
                        return Err("LIVE FILE API TOKEN");
                    }
                    s.operation = None;
                }
            }
            return Ok(());
        }
        if s.handle.is_none() {
            match s
                .files
                .list_next(&mut self.cursor)
                .map_err(|_| "LIVE FILE LIST")?
            {
                Some((id, entry)) => {
                    self.bytes = usize::try_from(entry.bytes).map_err(|_| "LIVE FILE LENGTH")?;
                    self.position = 0;
                    self.hash = 0xcbf29ce484222325;
                    crate::debug(b"MUSHA: LIVE_FILE_NAME ");
                    for &unit in &entry.name[..entry.name_len as usize] {
                        crate::debug(&crate::cpu::hex(unit as u64));
                    }
                    crate::debug(b"\n");
                    s.handle = Some(s.files.open_id(id).map_err(|_| "LIVE FILE OPEN")?);
                }
                None => {
                    s.done = true;
                    crate::debug(b"MUSHA: LIVE_FILE_API_OK BYTES=");
                    crate::debug(&crate::cpu::hex(self.original_bytes as u64));
                    crate::debug(b" HASH=");
                    crate::debug(&crate::cpu::hex(self.expected));
                    crate::debug(b"\n");
                }
            }
            return Ok(());
        }
        let handle = s.handle.ok_or("LIVE FILE HANDLE")?;
        let chunk = if self.index == 0 { 7 } else { 512 };
        match s.files.read(handle, &mut s.result[..chunk]) {
            Err(ApiError::Again) => {
                let r = s.files.take_request().ok_or("LIVE FILE REQUEST")?;
                if r.index() != self.index
                    || r.offset() != self.position as u64
                    || r.length() != (self.bytes - self.position).min(chunk)
                {
                    return Err("LIVE FILE REQUEST RANGE");
                }
                s.operation = Some(Read::new(
                    self.sources[self.index]
                        .ok_or("LIVE FILE SOURCE")?
                        .begin_plan(r.offset(), r.length())
                        .map_err(|_| "LIVE FILE PLAN")?,
                ));
                s.request = Some(r);
                crate::debug(b"MUSHA: LIVE_FILE_REQUEST\n");
            }
            Ok(n) => {
                if n != 0 {
                    for &b in &s.result[..n] {
                        self.hash = (self.hash ^ b as u64).wrapping_mul(0x100000001b3);
                    }
                    self.position += n;
                    return Ok(());
                }
                if self.position != self.bytes || (self.index == 0 && self.hash != self.expected) {
                    return Err("LIVE FILE CONTENT");
                }
                if s.files.read(handle, &mut s.result[..1]) != Ok(0) {
                    return Err("LIVE FILE EOF");
                }
                s.files.close(handle).map_err(|_| "LIVE FILE CLOSE")?;
                s.handle = None;
                crate::debug(b"MUSHA: LIVE_FILE_ENTRY_OK INDEX=");
                crate::debug(&crate::cpu::hex(self.index as u64));
                crate::debug(b" BYTES=");
                crate::debug(&crate::cpu::hex(self.position as u64));
                crate::debug(b" HASH=");
                crate::debug(&crate::cpu::hex(self.hash));
                crate::debug(b"\n");
                self.index += 1;
            }
            Err(_) => return Err("LIVE FILE READ"),
        }
        Ok(())
    }
}
impl Drop for Session {
    fn drop(&mut self) {
        self.state.files.disconnect();
        if let Some(operation) = self.state.operation.as_mut() {
            operation.cancel();
        }
        // Do not acknowledge a canceled API request here: DMA may still be active.
        // The outer controller/slot cleanup owns quiescence. CLAIMED stays true,
        // so this diagnostic workspace is never reused after termination.
    }
}

#[cfg(feature = "live-exfat-probe")]
const _: () = assert!(core::mem::size_of::<musha_fs::exfat::Workspace>() < 256 * 1024);
#[cfg(feature = "live-exfat-probe")]
static EXFAT_WORK: Shared<musha_fs::exfat::Workspace> =
    Shared(UnsafeCell::new(musha_fs::exfat::Workspace::new()));
#[cfg(feature = "live-exfat-probe")]
static EXFAT_BUSY: AtomicBool = AtomicBool::new(false);
/// Synchronous startup only. The detached source does not borrow this workspace.
#[cfg(feature = "live-exfat-probe")]
pub(super) fn prepare_exfat(
    blocks: u64,
    size: usize,
    read: impl FnMut(u64, &mut [u8]) -> Result<(), musha_fs::Error>,
    snapshot: &mut [u8],
) -> Result<Prepared, &'static str> {
    if CLAIMED.load(Ordering::Acquire) || EXFAT_BUSY.swap(true, Ordering::AcqRel) {
        return Err("EXFAT WORKSPACE BUSY");
    }
    // One diagnostic mount only. Keep this claim, including on failure: published
    // catalog/source memory must not be changed while any Prepared can exist.
    let work = unsafe { &mut *EXFAT_WORK.0.get() };
    let mut fs = musha_fs::exfat::FileSystem::discover(blocks, size, read, work)
        .map_err(|_| "EXFAT MOUNT")?;
    let name = [77, 85, 83, 72, 65, 46, 84, 88, 84];
    let index = fs
        .entries()
        .iter()
        .position(|e| e.name() == name)
        .ok_or("EXFAT FILE MISSING")?;
    let bytes = usize::try_from(fs.entries()[index].len()).map_err(|_| "EXFAT FILE LENGTH")?;
    if bytes > snapshot.len() {
        return Err("EXFAT SNAPSHOT LIMIT");
    }
    if fs
        .read_at(index, 0, &mut snapshot[..bytes])
        .map_err(|_| "EXFAT FILE READ")?
        != bytes
    {
        return Err("EXFAT FILE LENGTH");
    }
    let catalog = unsafe { &mut *CATALOG.0.get() };
    let sources = unsafe { &mut *SOURCES.0.get() };
    // Snapshot oracle first; retain every other root entry with its full name.
    for (to, from) in core::iter::once(index)
        .chain((0..fs.entries().len()).filter(|&i| i != index))
        .enumerate()
    {
        let entry = &fs.entries()[from];
        catalog[to].name[..entry.name().len()].copy_from_slice(entry.name());
        catalog[to].name_len = entry.name().len() as u8;
        catalog[to].bytes = entry.len();
        sources[to] = Some(fs.source(from).map_err(|_| "EXFAT FILE SOURCE")?);
    }
    Ok(Prepared {
        source: fs.source(index).map_err(|_| "EXFAT FILE SOURCE")?,
        count: fs.entries().len(),
        format: Format::Exfat,
        bytes,
        hash: musha_xhci::storage::hash(&snapshot[..bytes]),
    })
}
