// Copyright 2026 Hirofumi Iwasaki
// SPDX-License-Identifier: Apache-2.0
use musha_api::{Application, Context, Error, Step};
#[cfg(any(feature = "live-file-probe", feature = "file-app-probe"))]
mod stream_probe;
const VIEWER: bool = (!cfg!(feature = "qemu-debug") && !cfg!(feature = "t2-diagnostics"))
    || cfg!(feature = "file-viewer-probe");
struct Diagnostic {
    viewer: musha_viewer::Viewer,
    #[cfg(any(feature = "live-file-probe", feature = "file-app-probe"))]
    stream: stream_probe::Scan,
    page: usize,
    complete: bool,
    writing: bool,
    exit_requested: bool,
    file_handle: Option<musha_api::FileHandle>,
    file_bytes: u64,
    file_hash: u64,
    file_done: bool,
    file_expected: Option<(u64, u64)>,
    file_checks: u64,
    recheck_after_error: bool,
}
impl Diagnostic {
    fn request_file_recheck(&mut self) {
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
            self.viewer.closed();
        }
    }
    fn init(&mut self, ctx: &mut Context<'_>) -> Result<(), Error> {
        if ctx.version() != musha_api::API_VERSION {
            return Err(Error::Unsupported);
        }
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
        self.file_step(ctx)?;
        if VIEWER && self.viewer.render_step(ctx)? {
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
pub(crate) fn run(arena: &mut [u8], info: &crate::BootInfo) -> Result<(), Error> {
    // Framebuffer, cached file and arena have a single application owner.
    let mut ctx = unsafe { Context::new(arena, info.framebuffer) }?;
    let mut app = Diagnostic {
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
    };
    app.init(&mut ctx)?;
    let mut network = None;
    let mut steps = 0u64;
    let mut max_gap = 0u64;
    let mut last_cycle = None;
    let mut over_budget = 0u64;
    let result = (|| {
        let mut time = crate::acpi::Time::new(info.timer).map_err(|_| Error::Unsupported)?;
        let mut ready = false;
        let mut had_keyboard = false;
        let mut input_error = None;
        let mut last_udp = 0;
        // External slot 0 has priority; slot 1 is reserved for a future T2 adapter.
        let mut inputs = musha_input::sources::Sources::<2>::default();
        let mut external = None;
        let mut cycle = |event: crate::xhci::AppEvent<'_>| -> Result<bool, &'static str> {
            let events = match event {
                crate::xhci::AppEvent::Files(files) => {
                    if let Err(error) = app.files(&mut ctx, files) {
                        input_error = Some(error);
                        return Err("APP INPUT ERROR");
                    }
                    return Ok(app.exit_requested);
                }
                crate::xhci::AppEvent::Retained(_) => return Err("UNHANDLED STORAGE LOOP"),
                crate::xhci::AppEvent::File(bytes) => {
                    ctx.install_boot_file(bytes).map_err(|_| "APP FILE CACHE")?;
                    return Ok(false);
                }
                crate::xhci::AppEvent::FileError(error) => {
                    ctx.record_file_error(error);
                    return Ok(false);
                }
                crate::xhci::AppEvent::Ready(input) => {
                    if VIEWER {
                        crate::diagnostics::hide(true);
                    }
                    ctx.finish_file_discovery(false);
                    if input {
                        external = Some(
                            inputs
                                .connect(0, |k, d| ctx.push_key(k, d))
                                .map_err(|_| "INPUT SOURCE CONNECT")?,
                        );
                    }
                    ctx.set_input_active(inputs.active().is_some());
                    had_keyboard = input;
                    match crate::net::Session::start(info, &mut time) {
                        Ok(session) => network = session,
                        Err(error) => crate::net::failed(error),
                    }
                    ready = true;
                    crate::debug(b"MUSHA: RUNTIME_POLL_READY\n");
                    &[][..]
                }
                crate::xhci::AppEvent::Keys(events) => events,
            };
            let step = (|| {
                let before = time.now().map_err(|_| Error::Io)?;
                if let Some(last) = last_cycle {
                    max_gap = max_gap.max(before - last);
                }
                last_cycle = Some(before);
                ctx.advance(before)?;
                if let Some(session) = network.as_mut() {
                    if let Err(error) = session.poll(before) {
                        crate::net::failed(error);
                        network = None;
                        app.recheck_after_error = true;
                    }
                }
                if let Some(session) = network.as_ref() {
                    let udp = session.udp_count();
                    if udp != last_udp {
                        last_udp = udp;
                        app.request_file_recheck();
                    }
                }
                for &(key, down) in events {
                    let source = external.ok_or(Error::Invalid)?;
                    inputs
                        .key(source, key, down, |k, d| ctx.push_key(k, d))
                        .map_err(|_| Error::Invalid)?;
                }
                let state = app.step(&mut ctx)?;
                steps += 1;
                if time.now().map_err(|_| Error::Io)? - before > 1 {
                    over_budget += 1;
                    if over_budget == 1 {
                        crate::debug(b"MUSHA: APP_STEP_BUDGET_EXCEEDED\n");
                    }
                }
                Ok(app.exit_requested || state == Step::Complete)
            })();
            match step {
                Ok(stop) => Ok(stop),
                Err(error) => {
                    input_error = Some(error);
                    Err("APP INPUT ERROR")
                }
            }
        };
        let usb_result = crate::xhci::diagnose(info, &mut cycle);
        drop(cycle);
        if input_error.is_none() && usb_result != Err("QUIESCE FAILED") {
            app.files_closed(&mut ctx);
        }
        crate::debug(b"MUSHA: FILE_SERVICE_CLOSED\n");
        // Every diagnose return (Esc, deadline, disconnect, hardware/app error)
        // detaches the input source. This changes CPU input state only; xHCI owns
        // DMA quiescence. Do not invoke a failed app again solely for cleanup.
        if let Some(source) = external.take() {
            inputs
                .disconnect(source, |k, d| {
                    ctx.push_key(k, d);
                    crate::debug(b"MUSHA: INPUT_DETACH_KEY_UP=");
                    crate::debug(&crate::cpu::hex(k as u64));
                    crate::debug(b"\n");
                })
                .map_err(|_| Error::Invalid)?;
        }
        crate::debug(b"MUSHA: INPUT_SOURCE_DETACHED\n");
        let input_completed = had_keyboard && usb_result.is_ok();
        ctx.finish_file_discovery(usb_result.is_err());
        ctx.set_input_active(inputs.active().is_some());
        if let Some(error) = input_error {
            return Err(error);
        }
        if !ready {
            if VIEWER {
                crate::diagnostics::hide(true);
            }
            match crate::net::Session::start(info, &mut time) {
                Ok(session) => network = session,
                Err(error) => crate::net::failed(error),
            }
            crate::debug(b"MUSHA: RUNTIME_POLL_READY NO_INPUT\n");
        }
        // With no keyboard, progress continues for a bounded diagnostic window.
        // Esc or completion of the debug keyboard window stops all services.
        let tail_started = time.now().map_err(|_| Error::Io)?;
        for _ in 0..100_000_000 {
            let before = time.now().map_err(|_| Error::Io)?;
            if let Some(last) = last_cycle {
                max_gap = max_gap.max(before - last);
            }
            last_cycle = Some(before);
            ctx.advance(before)?;
            if let Some(session) = network.as_mut() {
                if let Err(error) = session.poll(before) {
                    crate::net::failed(error);
                    network = None;
                    app.recheck_after_error = true;
                }
            }
            if let Some(session) = network.as_ref() {
                let udp = session.udp_count();
                if udp != last_udp {
                    last_udp = udp;
                    app.request_file_recheck();
                }
            }
            let state = app.step(&mut ctx)?;
            steps += 1;
            let expired = network
                .as_ref()
                .is_none_or(|session| before - session.started() >= 10000);
            if state == Step::Complete && (input_completed || app.exit_requested || expired) {
                return Ok(());
            }
            if before - tail_started >= 11000 {
                return Err(Error::Io);
            }
        }
        Err(Error::Io)
    })();
    if result.is_err() {
        app.complete = false;
    }
    app.shutdown(&mut ctx);
    if result.is_ok() {
        if let Some(session) = network.as_mut() {
            session.finish(info.framebuffer);
        }
    }
    drop(network); // Also runs on application/clock errors, before propagating.
    if VIEWER {
        // Final backend status drawing can overlap the UI. Repaint after all
        // device owners have stopped; preserve the unavailable/ended screen.
        app.viewer.closed();
        while !app.viewer.painted() {
            app.viewer.render_step(&mut ctx)?;
        }
    }
    result?;
    crate::debug(b"MUSHA: RUNTIME_COOPERATIVE_OK STEPS=");
    crate::debug(&crate::cpu::hex(steps));
    crate::debug(b" MAX_GAP_MS=");
    crate::debug(&crate::cpu::hex(max_gap));
    crate::debug(b" FILE_RECHECKS=");
    crate::debug(&crate::cpu::hex(app.file_checks.saturating_sub(1)));
    crate::debug(b" OVER_BUDGET=");
    crate::debug(&crate::cpu::hex(over_budget));
    crate::debug(b"\n");
    crate::debug(b"MUSHA: APP_LIFECYCLE_OK STEPS=");
    crate::debug(&crate::cpu::hex(steps));
    crate::debug(b"\n");
    Ok(())
}
