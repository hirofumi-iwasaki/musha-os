// Copyright 2026 Hirofumi Iwasaki
// SPDX-License-Identifier: Apache-2.0
use musha_api::{Application, Context, Error, Step};
struct Diagnostic {
    page: usize,
    complete: bool,
    writing: bool,
    exit_requested: bool,
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
            return Ok(if ctx.input_active() {
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
        Ok(if ctx.input_active() {
            Step::Continue
        } else {
            Step::Complete
        })
    }
    fn shutdown(&mut self, ctx: &mut Context<'_>) {
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
    };
    app.init(&mut ctx)?;
    let mut steps = 0u64;
    // Every recoverable failure after a successful init runs shutdown exactly
    // once; app errors and clock errors share the same cleanup boundary.
    let result = (|| {
        ctx.set_input_active(true);
        let mut input_error = None;
        crate::xhci::diagnose(info, &mut |events| {
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
