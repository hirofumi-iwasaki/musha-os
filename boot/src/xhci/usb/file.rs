// Copyright 2026 Hirofumi Iwasaki
// SPDX-License-Identifier: Apache-2.0
//! Opt-in end-to-end adapter exercise. One selected FAT32 file, repeated bounded requests.
//! Catalog discovery precedes input polling; metadata/data reads below do not.
use super::{Host, storage::Device};
use core::{
    cell::UnsafeCell,
    sync::atomic::{AtomicBool, Ordering},
};
use musha_api::stream::{Entry, Error as ApiError, Files, Format, Handle, Request as FileRequest};
use musha_fs::{
    operation::{Progress, Read, Request},
    planner::Source,
};
pub(super) struct Prepared {
    pub source: Source,
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
static CATALOG: Shared<[Entry; 1]> = Shared(UnsafeCell::new([Entry {
    name: [0; 255],
    name_len: 9,
    bytes: 0,
}]));
const _: () = assert!(core::mem::size_of::<State>() + core::mem::size_of::<[Entry; 1]>() < 32768);
pub(super) struct Session {
    state: &'static mut State,
    source: Source,
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
        for (to, from) in catalog[0].name.iter_mut().zip(b"MUSHA.TXT") {
            *to = *from as u16;
        }
        catalog[0].bytes = prepared.bytes as u64;
        state
            .files
            .mount(Format::Fat32, catalog)
            .map_err(|_| "LIVE FILE API MOUNT")?;
        let id = state
            .files
            .list_next(&mut state.files.list_begin().map_err(|_| "LIVE FILE LIST")?)
            .map_err(|_| "LIVE FILE LIST")?
            .ok_or("LIVE FILE LIST")?
            .0;
        state.handle = Some(state.files.open_id(id).map_err(|_| "LIVE FILE OPEN")?);
        let expected = prepared.hash;
        let bytes = prepared.bytes;
        crate::debug(b"MUSHA: LIVE_FILE_API_READY\n");
        Ok(Self {
            state,
            source: prepared.source,
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
        let handle = s.handle.ok_or("LIVE FILE HANDLE")?;
        match s.files.read(handle, &mut s.result[..7]) {
            Err(ApiError::Again) => {
                let r = s.files.take_request().ok_or("LIVE FILE REQUEST")?;
                if r.index() != 0
                    || r.offset() != self.position as u64
                    || r.length() != (self.bytes - self.position).min(7)
                {
                    return Err("LIVE FILE REQUEST RANGE");
                }
                s.operation = Some(Read::new(
                    self.source
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
                if self.position != self.bytes || self.hash != self.expected {
                    return Err("LIVE FILE CONTENT");
                }
                if s.files.read(handle, &mut s.result[..1]) != Ok(0) {
                    return Err("LIVE FILE EOF");
                }
                s.files.close(handle).map_err(|_| "LIVE FILE CLOSE")?;
                s.done = true;
                crate::debug(b"MUSHA: LIVE_FILE_API_OK BYTES=");
                crate::debug(&crate::cpu::hex(self.position as u64));
                crate::debug(b" HASH=");
                crate::debug(&crate::cpu::hex(self.expected));
                crate::debug(b"\n");
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
