// Copyright 2026 Hirofumi Iwasaki
// SPDX-License-Identifier: Apache-2.0
use musha_api::{Application, Context, Error, Step};
struct Diagnostic {
    page: usize,
    complete: bool,
    writing: bool,
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
        Ok(Step::Complete)
    }
    fn shutdown(&mut self, ctx: &mut Context<'_>) {
        if self.complete {
            ctx.text("APP COMPLETE", 24, 324, [0, 240, 100]);
        }
    }
}
pub(crate) fn run(arena: &mut [u8], info: &crate::BootInfo) -> Result<(), Error> {
    let mut time = crate::acpi::Time::new(info.timer).map_err(|_| Error::Unsupported)?;
    // SAFETY: paging validated framebuffer and arena are disjoint, and the
    // runtime suspends its drawing for the app's entire context lifetime.
    let mut ctx = unsafe { Context::new(arena, info.framebuffer) }?;
    let mut app = Diagnostic {
        page: 0,
        complete: false,
        writing: true,
    };
    app.init(&mut ctx)?;
    let mut steps = 0u64;
    // Every recoverable failure after a successful init runs shutdown exactly
    // once; app errors and clock errors share the same cleanup boundary.
    let result = (|| {
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
    app.shutdown(&mut ctx);
    result?;
    crate::debug(b"MUSHA: APP_LIFECYCLE_OK STEPS=");
    crate::debug(&crate::cpu::hex(steps));
    crate::debug(b"\n");
    Ok(())
}
