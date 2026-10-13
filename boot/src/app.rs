// Copyright 2026 Hirofumi Iwasaki
// SPDX-License-Identifier: Apache-2.0
use musha_api::{Application, Context, Error, Step};
#[cfg(any(feature = "live-file-probe", feature = "file-app-probe"))]
mod stream_probe;
pub(crate) const VIEWER: bool = (!cfg!(feature = "qemu-debug")
    && !cfg!(feature = "t2-diagnostics"))
    || cfg!(feature = "file-viewer-probe");
pub(crate) struct Diagnostic {
    pub(crate) viewer: musha_viewer::Viewer,
    #[cfg(any(feature = "live-file-probe", feature = "file-app-probe"))]
    pub(crate) stream: stream_probe::Scan,
    pub(crate) page: usize,
    pub(crate) complete: bool,
    pub(crate) writing: bool,
    pub(crate) exit_requested: bool,
    pub(crate) file_handle: Option<musha_api::FileHandle>,
    pub(crate) file_bytes: u64,
    pub(crate) file_hash: u64,
    pub(crate) file_done: bool,
    pub(crate) file_expected: Option<(u64, u64)>,
    pub(crate) file_checks: u64,
    pub(crate) recheck_after_error: bool,
}
impl Diagnostic {
    pub(crate) fn request_file_recheck(&mut self) {
        if self.file_done && self.file_expected.is_some() {
            self.file_done = false;
            self.file_bytes = 0;
            self.file_hash = 0xcbf29ce484222325;
        }
    }
    fn file_step(&mut self, ctx: &mut Context<'_>) -> Result<(), Error> {
        if self.file_done {
            return Ok(());
        }
        if self.file_handle.is_none() {
            match ctx.file_open("/MUSHA.TXT") {
                Ok(handle) => self.file_handle = Some(handle),
                Err(Error::Again) => return Ok(()),
                Err(Error::NotFound | Error::Unsupported | Error::Io) => {
                    crate::debug(b"MUSHA: APP_FILE_UNAVAILABLE\n");
                    self.file_done = true;
                    return Ok(());
                }
                Err(error) => return Err(error),
            }
        }
        let handle = self.file_handle.unwrap();
        let mut bytes = [0u8; 64];
        let count = ctx.file_read(handle, &mut bytes)?;
        if count != 0 {
            for byte in &bytes[..count] {
                self.file_hash = (self.file_hash ^ *byte as u64).wrapping_mul(0x100000001b3);
            }
            self.file_bytes += count as u64;
            return Ok(());
        }
        ctx.file_close(handle)?;
        if ctx.file_read(handle, &mut bytes) != Err(Error::Invalid) {
            return Err(Error::Io);
        }
        self.file_handle = None;
        self.file_done = true;
        if let Some(expected) = self.file_expected {
            if expected != (self.file_bytes, self.file_hash) {
                return Err(Error::Io);
            }
        } else {
            self.file_expected = Some((self.file_bytes, self.file_hash));
        }
        self.file_checks += 1;
        if self.file_checks > 1 {
            let checks = self.file_checks - 1;
            if checks.is_power_of_two() {
                crate::debug(b"MUSHA: APP_FILE_RECHECK_OK COUNT=");
                crate::debug(&crate::cpu::hex(checks));
                crate::debug(b"\n");
            }
            return Ok(());
        }
        crate::debug(b"MUSHA: APP_FILE_OK BYTES=");
        crate::debug(&crate::cpu::hex(self.file_bytes));
        crate::debug(b" HASH=");
        crate::debug(&crate::cpu::hex(self.file_hash));
        crate::debug(b" CLOSED_HANDLE_REJECTED\n");
        if !VIEWER && ctx.screen_size().1 >= 508 {
            ctx.text("APP FILE READ OK", 24, 484, [0, 240, 100]);
        }
        Ok(())
    }
}
impl Application for Diagnostic {
    fn files(
        &mut self,
        ctx: &mut Context<'_>,
        files: &mut musha_api::stream::Files<'_>,
    ) -> Result<(), Error> {
        if VIEWER {
            self.viewer.service(files, ctx.now_ms());
        }
        #[cfg(any(feature = "live-file-probe", feature = "file-app-probe"))]
        if !VIEWER {
            self.stream.step(files)?;
        }
        Ok(())
    }
    fn files_closed(&mut self, _ctx: &mut Context<'_>) {
        if VIEWER {
            self.viewer.closed(false);
        }
    }
    fn init(&mut self, ctx: &mut Context<'_>) -> Result<(), Error> {
        if ctx.version() != musha_api::API_VERSION {
            return Err(Error::Unsupported);
        }
        crate::bootstate::update(|r, t| {
            r.running(t);
        });
        crate::debug(b"MUSHA: APP_RUNNING STATIC_LINKED\n");
        if !VIEWER {
            ctx.text("Hello Musha-OS!", 24, 24, [240, 240, 240]);
        }
        Ok(())
    }
    fn step(&mut self, ctx: &mut Context<'_>) -> Result<Step, Error> {
        #[cfg(feature = "app-step-error")]
        if self.file_checks > 1 {
            return Err(Error::Io);
        }
        if self.recheck_after_error && self.file_done {
            self.recheck_after_error = false;
            self.request_file_recheck();
        }
        if !VIEWER {
            self.file_step(ctx)?;
        } else {
            self.file_done = true;
        }
        if VIEWER && self.viewer.render_step(ctx)? {
            crate::bootstate::frame();
            crate::debug(b"MUSHA: VIEWER_FRAME ");
            crate::debug(match self.viewer.view() {
                musha_viewer::View::List => b"LIST",
                musha_viewer::View::Loading => b"LOADING",
                musha_viewer::View::Page => b"PAGE",
                musha_viewer::View::Error => b"ERROR",
                musha_viewer::View::Catalog => b"CATALOG",
                musha_viewer::View::Unavailable => b"UNAVAILABLE",
            });
            for value in [
                self.viewer.selected() as u64,
                self.viewer.offset(),
                self.viewer.data().len() as u64,
                musha_xhci::storage::hash(self.viewer.data()),
            ] {
                crate::debug(b" ");
                crate::debug(&crate::cpu::hex(value));
            }
            crate::debug(b"\n");
        }
        while let Some(event) = ctx.next_key() {
            crate::debug(if event.pressed {
                b"MUSHA: APP_KEY_DOWN="
            } else {
                b"MUSHA: APP_KEY_UP="
            });
            crate::debug(&crate::cpu::hex(event.usage as u64));
            crate::debug(b"\n");
            #[cfg(feature = "file-app-close")]
            if event.pressed && event.usage == 4 {
                self.stream.request_close();
            }
            if VIEWER && event.pressed {
                self.viewer.key(event.usage, ctx.now_ms());
                self.exit_requested = self.viewer.exit_requested();
            }
            if !VIEWER && event.pressed && event.usage == 0x29 {
                self.exit_requested = true;
            }
            if !VIEWER && event.pressed {
                let (width, height) = ctx.screen_size();
                if height >= 380 && width > 24 {
                    let py = 356 * height.saturating_sub(48).min(1100) / 550;
                    if py + 42 <= height {
                        ctx.rectangle(24, py, width - 24, 42, [12, 20, 32])?;
                    }
                    ctx.text("KEY CODE", 24, 356, [0, 220, 240]);
                    ctx.text(
                        core::str::from_utf8(&crate::cpu::hex(event.usage as u64))
                            .map_err(|_| Error::Invalid)?,
                        240,
                        356,
                        [0, 220, 240],
                    );
                }
            }
        }
        if self.complete {
            return Ok(
                if ctx.input_active() || !self.file_done || (VIEWER && !self.viewer.painted()) {
                    Step::Continue
                } else {
                    Step::Complete
                },
            );
        }
        if VIEWER {
            self.complete = true;
            return Ok(Step::Continue);
        }
        // Bounded work: probe at most 64 pages per callback; the full arena scan
        // is spread across iterations so the runtime can poll its clock/devices.
        let arena = ctx.arena();
        let end = (self.page + 64).min(arena.len().div_ceil(4096));
        while self.page < end {
            let page = self.page;
            let start = page * 4096;
            let last = (start + 4096).min(arena.len()) - 1;
            // SAFETY: this exclusive arena slice owns mapped RAM. Bounds are
            // computed from this chunk; volatile accesses retain the RAM probe.
            unsafe {
                if self.writing {
                    arena
                        .as_mut_ptr()
                        .add(start)
                        .write_volatile((page as u8) ^ 0x5a);
                    arena
                        .as_mut_ptr()
                        .add(last)
                        .write_volatile((page as u8) ^ 0xa5);
                } else if arena.as_ptr().add(start).read_volatile() != ((page as u8) ^ 0x5a)
                    || arena.as_ptr().add(last).read_volatile() != ((page as u8) ^ 0xa5)
                {
                    return Err(Error::Io);
                }
            }
            self.page += 1;
        }
        if self.page * 4096 < arena.len() {
            return Ok(Step::Continue);
        }
        if self.writing {
            self.writing = false;
            self.page = 0;
            return Ok(Step::Continue);
        }
        let bytes = ctx.arena_bytes();
        if !VIEWER {
            ctx.text("ARENA READY", 24, 100, [0, 220, 240]);
            ctx.text("ARENA BYTES", 24, 132, [0, 220, 240]);
            let digits = crate::cpu::hex(bytes as u64);
            ctx.text(
                core::str::from_utf8(&digits).map_err(|_| Error::Invalid)?,
                240,
                132,
                [0, 220, 240],
            );
        }
        let digits = crate::cpu::hex(bytes as u64);
        crate::debug(b"MUSHA: ARENA_BYTES=");
        crate::debug(&digits);
        crate::debug(b"\n");
        self.complete = true;
        Ok(
            if ctx.input_active() || !self.file_done || (VIEWER && !self.viewer.painted()) {
                Step::Continue
            } else {
                Step::Complete
            },
        )
    }
    fn shutdown(&mut self, ctx: &mut Context<'_>) {
        if let Some(handle) = self.file_handle.take() {
            let _ = ctx.file_close(handle);
        }
        if self.complete && !VIEWER {
            ctx.text("APP COMPLETE", 24, 324, [0, 240, 100]);
        }
    }
}

impl Diagnostic {
    pub(crate) fn new() -> Self {
        Self {
            viewer: musha_viewer::Viewer::new(),
            #[cfg(any(feature = "live-file-probe", feature = "file-app-probe"))]
            stream: stream_probe::Scan::new(),
            page: 0,
            complete: false,
            writing: true,
            exit_requested: false,
            file_handle: None,
            file_bytes: 0,
            file_hash: 0xcbf29ce484222325,
            file_done: false,
            file_expected: None,
            file_checks: 0,
            recheck_after_error: false,
        }
    }
}
