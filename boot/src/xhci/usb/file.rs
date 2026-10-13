// Copyright 2026 Hirofumi Iwasaki
// SPDX-License-Identifier: Apache-2.0
//! Runtime file service. CPU buffers never become DMA buffers.
use super::{Host, storage::Device};
use core::{
    cell::UnsafeCell,
    sync::atomic::{AtomicBool, Ordering},
};
use musha_api::stream::{Entry, Files, Format, Request as ApiRequest};
use musha_fs::{
    operation::{Progress, Read, Request},
    planner::Source,
};
const MAX: usize = musha_api::stream::MAX_FILES;
struct Shared<T>(UnsafeCell<T>);
// Single boot CPU; mount reservation and session claim exclude aliasing. No
// published catalog is reused during this boot, including after a failed session.
unsafe impl<T> Sync for Shared<T> {}
static RESERVED: AtomicBool = AtomicBool::new(false);
static CLAIMED: AtomicBool = AtomicBool::new(false);
static CATALOG: Shared<[Entry; MAX]> = Shared(UnsafeCell::new(
    [const {
        Entry {
            name: [0; 255],
            name_len: 0,
            bytes: 0,
        }
    }; MAX],
));
static SOURCES: Shared<[Option<Source>; MAX]> = Shared(UnsafeCell::new([None; MAX]));
static WORK: Shared<musha_fs::exfat::Workspace> =
    Shared(UnsafeCell::new(musha_fs::exfat::Workspace::new()));
pub(super) struct Prepared {
    format: Format,
    count: usize,
}
impl Prepared {
    pub(super) fn is_exfat(&self) -> bool {
        self.format == Format::Exfat
    }
}
struct State {
    files: Files<'static>,
    operation: Option<Read>,
    request: Option<ApiRequest>,
    sector: Option<Request>,
    sector_bytes: [u8; 4096],
    result: [u8; 4096],
}
static STATE: Shared<State> = Shared(UnsafeCell::new(State {
    files: Files::new(),
    operation: None,
    request: None,
    sector: None,
    sector_bytes: [0; 4096],
    result: [0; 4096],
}));
const _: () = assert!(
    core::mem::size_of::<State>()
        + core::mem::size_of::<[Entry; MAX]>()
        + core::mem::size_of::<[Option<Source>; MAX]>()
        + core::mem::size_of::<musha_fs::exfat::Workspace>()
        < 384 * 1024
);
struct Reservation(bool);
impl Drop for Reservation {
    fn drop(&mut self) {
        if !self.0 {
            RESERVED.store(false, Ordering::Release);
        }
    }
}
/// Inspect a controller before choosing the live storage host. Never publishes
/// catalog references; the workspace reservation is released before returning.
pub(super) fn supported(
    blocks: u64,
    size: usize,
    mut read: impl FnMut(u64, &mut [u8]) -> Result<(), musha_fs::Error>,
) -> Result<bool, musha_fs::Error> {
    if RESERVED.swap(true, Ordering::AcqRel) {
        return Ok(false);
    }
    let _reservation = Reservation(false);
    let work = unsafe { &mut *WORK.0.get() };
    let result = match musha_fs::exfat::FileSystem::discover(blocks, size, &mut read, work) {
        Ok(_) => Ok(true),
        Err(musha_fs::Error::Unsupported) => {
            match musha_fs::fat32::FileSystem::discover(blocks, size, &mut read) {
                Ok(_) => Ok(true),
                Err(musha_fs::Error::Unsupported) => Ok(false),
                Err(e) => Err(e),
            }
        }
        Err(e) => Err(e),
    };
    // Preserve normal medium selection: only the subsequently selected medium
    // may publish the external-app rejection; discovery itself has no UI effect.
    #[cfg(external_app)]
    if let Err(musha_fs::Error::DuplicateName(name)) = &result {
        if crate::external::boot_name(name) {
            return Ok(true);
        }
    }
    result
}
/// Select the first successfully mounted medium for this boot. Discovery scans
/// do not call this function. No data-sector snapshot is needed for the catalog.
pub(super) fn prepare(
    blocks: u64,
    size: usize,
    mut read: impl FnMut(u64, &mut [u8]) -> Result<(), musha_fs::Error>,
    snapshot: &mut [u8],
) -> Result<Option<(Prepared, Option<usize>)>, musha_fs::Error> {
    if RESERVED.swap(true, Ordering::AcqRel) {
        return Ok(None);
    }
    let mut reservation = Reservation(false);
    let catalog = unsafe { &mut *CATALOG.0.get() };
    let sources = unsafe { &mut *SOURCES.0.get() };
    let work = unsafe { &mut *WORK.0.get() };
    let selected = musha_fs::exfat::FileSystem::discover(blocks, size, &mut read, work);
    let (format, count, bytes) = match selected {
        Ok(mut fs) => {
            let first = fs
                .entries()
                .iter()
                .position(|e| e.name() == [77, 85, 83, 72, 65, 46, 84, 88, 84]);
            let mut bytes = None;
            if let Some(i) = first {
                if !crate::app::VIEWER && fs.entries()[i].len() <= snapshot.len() as u64 {
                    bytes = Some(fs.read_at(i, 0, snapshot)?);
                }
            }
            for (to, from) in first
                .into_iter()
                .chain((0..fs.entries().len()).filter(|i| Some(*i) != first))
                .enumerate()
            {
                let e = &fs.entries()[from];
                catalog[to].name[..e.name().len()].copy_from_slice(e.name());
                catalog[to].name_len = e.name().len() as u8;
                catalog[to].bytes = e.len();
                sources[to] = Some(fs.source(from)?);
            }
            (Format::Exfat, fs.entries().len(), bytes)
        }
        Err(musha_fs::Error::Unsupported) => {
            let mut fs = musha_fs::fat32::FileSystem::discover(blocks, size, &mut read)?;
            let first = fs.entries().iter().position(|e| e.name() == b"MUSHA   TXT");
            let mut bytes = None;
            if let Some(i) = first {
                if !crate::app::VIEWER && fs.entries()[i].len() <= snapshot.len() as u64 {
                    bytes = Some(fs.read_at(i, 0, snapshot)?);
                }
            }
            for (to, from) in first
                .into_iter()
                .chain((0..fs.entries().len()).filter(|i| Some(*i) != first))
                .enumerate()
            {
                let e = &fs.entries()[from];
                let raw = e.name();
                let mut n = 0;
                for &b in raw[..8].iter().take(
                    raw[..8]
                        .iter()
                        .rposition(|&b| b != b' ')
                        .map_or(0, |i| i + 1),
                ) {
                    catalog[to].name[n] = b as u16;
                    n += 1;
                }
                let ext = raw[8..]
                    .iter()
                    .rposition(|&b| b != b' ')
                    .map_or(0, |i| i + 1);
                if ext > 0 {
                    catalog[to].name[n] = b'.' as u16;
                    n += 1;
                    for &b in &raw[8..8 + ext] {
                        catalog[to].name[n] = b as u16;
                        n += 1;
                    }
                }
                catalog[to].name_len = n as u8;
                catalog[to].bytes = e.len();
                sources[to] = Some(fs.source(from)?);
            }
            (Format::Fat32, fs.entries().len(), bytes)
        }
        Err(e) => return Err(e),
    };
    reservation.0 = true;
    crate::debug(if format == Format::Exfat {
        b"MUSHA: STREAM_MOUNT_EXFAT\n"
    } else {
        b"MUSHA: STREAM_MOUNT_FAT32\n"
    });
    Ok(Some((Prepared { format, count }, bytes)))
}
pub(super) struct Session {
    state: &'static mut State,
    sources: &'static [Option<Source>],
}
impl Session {
    pub(super) fn new(p: Prepared) -> Result<Self, &'static str> {
        if CLAIMED.swap(true, Ordering::AcqRel) {
            return Err("FILE SESSION LIMIT");
        }
        let state = unsafe { &mut *STATE.0.get() };
        let catalog = unsafe { &*CATALOG.0.get() };
        let sources = unsafe { &*SOURCES.0.get() };
        state
            .files
            .mount(p.format, &catalog[..p.count])
            .map_err(|_| "FILE MOUNT")?;
        // Readiness requires the actual mounted service, before application entry.
        state.files.format().map_err(|_| "FILE FORMAT")?;
        state.files.list_begin().map_err(|_| "FILE LIST")?;
        crate::bootstate::update(|r, _| r.files = musha_platform::readiness::Service::Live);
        crate::debug(b"MUSHA: LIVE_FILE_API_READY\n");
        Ok(Self {
            state,
            sources: &sources[..p.count],
        })
    }
    pub(super) fn step(
        &mut self,
        host: &mut Host<'_>,
        device: &mut Device,
        tick: &mut super::super::Tick<'_>,
    ) -> Result<bool, &'static str> {
        device.check_connected(host)?;
        let s = &mut self.state;
        if tick(super::super::AppEvent::Files(&mut s.files))? {
            return Ok(true);
        }
        if s.request.is_some_and(|r| s.files.request_canceled(r)) {
            if let Some(op) = s.operation.as_mut() {
                op.cancel();
            }
        }
        if let Some(r) = s.sector {
            if device.poll_read(host, &mut s.sector_bytes[..r.bytes()])? {
                if !s
                    .operation
                    .as_mut()
                    .ok_or("FILE OPERATION")?
                    .complete(r, Ok(&s.sector_bytes[..r.bytes()]))
                {
                    return Err("FILE TOKEN");
                }
                s.sector = None;
            }
        } else if let Some(op) = s.operation.as_mut() {
            match op.poll() {
                Ok(Progress::Yield) => {}
                Ok(Progress::Pending) => return Err("FILE OWNERSHIP"),
                Ok(Progress::Read(r)) => {
                    device.begin_read(host, r.lba())?;
                    s.sector = Some(r);
                    #[cfg(feature = "live-file-probe")]
                    device.delay_read(host)?;
                }
                Ok(Progress::Complete) => {
                    let n = op
                        .finish(&mut s.result)
                        .map_err(|_| "FILE FINISH")?
                        .ok_or("FILE FINISH")?;
                    if !s
                        .files
                        .complete(s.request.take().ok_or("FILE REQUEST")?, Ok(&s.result[..n]))
                    {
                        return Err("FILE API TOKEN");
                    }
                    s.operation = None;
                }
                Err(e) => {
                    s.files
                        .complete(s.request.take().ok_or("FILE REQUEST")?, Err(api_error(e)));
                    s.operation = None;
                }
            }
        } else if let Some(r) = s.files.take_request() {
            let plan = self
                .sources
                .get(r.index())
                .and_then(|s| *s)
                .ok_or("FILE INDEX")?
                .begin_plan(r.offset(), r.length());
            match plan {
                Ok(plan) => {
                    s.operation = Some(Read::new(plan));
                    s.request = Some(r);
                    crate::debug(b"MUSHA: LIVE_FILE_REQUEST\n");
                }
                Err(e) => {
                    s.files.complete(r, Err(api_error(e)));
                }
            }
        }
        Ok(false)
    }
}
impl Drop for Session {
    fn drop(&mut self) {
        // Observe cancellation itself, not the host-dependent number of sectors
        // reached before a QMP key event arrives.
        #[cfg(feature = "live-file-probe")]
        if self.state.request.is_some() && self.state.sector.is_some() {
            crate::debug(b"MUSHA: LIVE_FILE_CANCEL_PENDING_SECTOR\n");
        }
        self.state.files.disconnect();
        crate::bootstate::update(|r, _| r.files = musha_platform::readiness::Service::Stopped);
        if let Some(o) = self.state.operation.as_mut() {
            o.cancel();
        }
    }
}

fn api_error(e: musha_fs::Error) -> musha_api::stream::Error {
    use musha_api::stream::Error as A;
    match e {
        musha_fs::Error::Io => A::Io,
        musha_fs::Error::Unsupported => A::Unsupported,
        musha_fs::Error::Corrupt | musha_fs::Error::DuplicateName(_) => A::Corrupt,
        musha_fs::Error::NotFound => A::Invalid,
        musha_fs::Error::TooLarge => A::TooLarge,
        musha_fs::Error::Limit => A::Limit,
    }
}
