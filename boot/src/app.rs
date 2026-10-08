// Copyright 2026 Hirofumi Iwasaki
// SPDX-License-Identifier: Apache-2.0
use musha_api::{Application, Context, Error, Step};
struct Diagnostic {
    page: usize,
    complete: bool,
    writing: bool,
    exit_requested: bool,
    file_handle: Option<musha_api::FileHandle>,
    file_bytes: u64,
    file_hash: u64,
    file_done: bool,
}
impl Diagnostic {
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
        crate::debug(b"MUSHA: APP_FILE_OK BYTES=");
        crate::debug(&crate::cpu::hex(self.file_bytes));
        crate::debug(b" HASH=");
        crate::debug(&crate::cpu::hex(self.file_hash));
        crate::debug(b" CLOSED_HANDLE_REJECTED\n");
        if ctx.screen_size().1 >= 508 {
            ctx.text("APP FILE READ OK", 24, 484, [0, 240, 100]);
        }
        Ok(())
    }
}
impl Application for Diagnostic {
    fn init(&mut self, ctx: &mut Context<'_>) -> Result<(), Error> {
        if ctx.version() != musha_api::API_VERSION {
            return Err(Error::Unsupported);
        }
        ctx.text("Hello Musha-OS!", 24, 24, [240, 240, 240]);
        Ok(())
    }
    fn step(&mut self, ctx: &mut Context<'_>) -> Result<Step, Error> {
        self.file_step(ctx)?;
        while let Some(event) = ctx.next_key() {
            crate::debug(if event.pressed {
                b"MUSHA: APP_KEY_DOWN="
            } else {
                b"MUSHA: APP_KEY_UP="
            });
            crate::debug(&crate::cpu::hex(event.usage as u64));
            crate::debug(b"\n");
            if event.pressed && event.usage == 0x29 {
                self.exit_requested = true;
            }
            if event.pressed {
                let (width, height) = ctx.screen_size();
                if height >= 380 && width > 24 {
                    ctx.rectangle(24, 356, width - 24, 24, [12, 20, 32])?;
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
            return Ok(if ctx.input_active() || !self.file_done {
                Step::Continue
            } else {
                Step::Complete
            });
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
        ctx.text("ARENA READY", 24, 100, [0, 220, 240]);
        ctx.text("ARENA BYTES", 24, 132, [0, 220, 240]);
        let digits = crate::cpu::hex(bytes as u64);
        ctx.text(
            core::str::from_utf8(&digits).map_err(|_| Error::Invalid)?,
            240,
            132,
            [0, 220, 240],
        );
        crate::debug(b"MUSHA: ARENA_BYTES=");
        crate::debug(&digits);
        crate::debug(b"\n");
        self.complete = true;
        Ok(if ctx.input_active() || !self.file_done {
            Step::Continue
        } else {
            Step::Complete
        })
    }
    fn shutdown(&mut self, ctx: &mut Context<'_>) {
        if let Some(handle) = self.file_handle.take() {
            let _ = ctx.file_close(handle);
        }
        if self.complete {
            ctx.text("APP COMPLETE", 24, 324, [0, 240, 100]);
        }
    }
}
pub(crate) fn run(arena: &mut [u8], info: &crate::BootInfo) -> Result<(), Error> {
    let mut time = None;
    // SAFETY: paging validated framebuffer and arena are disjoint, and the
    // runtime draws only between app callbacks, never concurrently with them.
    let mut ctx = unsafe { Context::new(arena, info.framebuffer) }?;
    let mut app = Diagnostic {
        page: 0,
        complete: false,
        writing: true,
        exit_requested: false,
        file_handle: None,
        file_bytes: 0,
        file_hash: 0xcbf29ce484222325,
        file_done: false,
    };
    app.init(&mut ctx)?;
    let mut steps = 0u64;
    // Every recoverable failure after a successful init runs shutdown exactly
    // once; app errors and clock errors share the same cleanup boundary.
    let result = (|| {
        ctx.set_input_active(true);
        let mut input_error = None;
        let usb_result = crate::xhci::diagnose(info, &mut |event| {
            let events = match event {
                crate::xhci::AppEvent::Keys(events) => events,
                crate::xhci::AppEvent::File(bytes) => {
                    ctx.install_boot_file(bytes).map_err(|_| "APP FILE CACHE")?;
                    return Ok(false);
                }
                crate::xhci::AppEvent::FileError(error) => {
                    ctx.record_file_error(error);
                    return Ok(false);
                }
            };
            let result = (|| {
                if time.is_none() {
                    time =
                        Some(crate::acpi::Time::new(info.timer).map_err(|_| Error::Unsupported)?);
                }
                let timer = time.as_mut().unwrap();
                let before = timer.now().map_err(|_| Error::Io)?;
                ctx.advance(before)?;
                for &(key, down) in events {
                    ctx.push_key(key, down);
                }
                let step = app.step(&mut ctx)?;
                if timer.now().map_err(|_| Error::Io)? - before > 1 {
                    crate::debug(b"MUSHA: APP_STEP_BUDGET_EXCEEDED\n");
                }
                steps += 1;
                Ok(app.exit_requested || step == Step::Complete)
            })();
            match result {
                Ok(stop) => Ok(stop),
                Err(error) => {
                    input_error = Some(error);
                    Err("APP INPUT ERROR")
                }
            }
        });
        ctx.finish_file_discovery(usb_result.is_err());
        ctx.set_input_active(false);
        if let Some(error) = input_error {
            return Err(error);
        }
        let mut time = match time {
            Some(timer) => timer,
            None => crate::acpi::Time::new(info.timer).map_err(|_| Error::Unsupported)?,
        };
        loop {
            let before = time.now().map_err(|_| Error::Io)?;
            ctx.advance(before)?;
            let result = app.step(&mut ctx)?;
            let after = time.now().map_err(|_| Error::Io)?;
            if after - before > 1 {
                crate::debug(b"MUSHA: APP_STEP_BUDGET_EXCEEDED\n");
            }
            steps += 1;
            if result == Step::Complete {
                return Ok(());
            }
        }
    })();
    if result.is_err() {
        app.complete = false;
    }
    app.shutdown(&mut ctx);
    result?;
    crate::debug(b"MUSHA: APP_LIFECYCLE_OK STEPS=");
    crate::debug(&crate::cpu::hex(steps));
    crate::debug(b"\n");
    Ok(())
}
