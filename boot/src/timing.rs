// SPDX-License-Identifier: Apache-2.0
//! Normal-build PM-epoch timings. BSP only; no controller writes.
use core::cell::UnsafeCell;
use musha_platform::diagnostics::Line;
struct Records(UnsafeCell<([Line; 256], usize)>);
unsafe impl Sync for Records {} // BSP, interrupts disabled, no reentrant callbacks.
static RECORDS: Records = Records(UnsafeCell::new(([Line::EMPTY; 256], 0)));
pub fn record(args: core::fmt::Arguments<'_>) {
    {
        use core::fmt::Write;
        let mut line = Line::EMPTY;
        let _ = write!(line, "{}", args);
        crate::debug(b"MUSHA: TIMING ");
        crate::debug(line.as_str().as_bytes());
        crate::debug(b"\n");
        unsafe {
            let (rows, count) = &mut *RECORDS.0.get();
            if *count < rows.len() - 1 {
                rows[*count] = line;
                *count += 1;
            } else {
                let mut overflow = Line::EMPTY;
                let _ = write!(overflow, "TIMING LIMIT / MORE OMITTED");
                rows[255] = overflow;
                *count = 256;
            }
        }
    }
}
pub fn count() -> usize {
    return unsafe { (*RECORDS.0.get()).1 };
}
pub fn row(index: usize) -> musha_platform::diagnostics::Line {
    return unsafe { (*RECORDS.0.get()).0[index] };
}
pub struct Span {
    data: (u16, &'static str, Option<u64>, bool),
}
impl Span {
    pub fn new(bdf: u16, stage: &'static str) -> Self {
        return Self {
            data: (bdf, stage, crate::bootstate::epoch_now(), false),
        };
    }
    pub fn finish(mut self, result: &'static str) {
        self.emit(result);
    }
    fn emit(&mut self, result: &'static str) {
        {
            let (bdf, stage, start, done) = &mut self.data;
            if *done {
                return;
            }
            *done = true;
            let end = crate::bootstate::epoch_now();
            record(format_args!("TIME {:04X} {} START {:?}", bdf, stage, start));
            record(format_args!(
                "END {:?} MS {:?} {}",
                end,
                end.zip(*start).and_then(|(e, s)| e.checked_sub(s)),
                result
            ));
        }
    }
}
impl Drop for Span {
    fn drop(&mut self) {
        self.emit("EARLY RETURN");
    }
}

use core::sync::atomic::{AtomicUsize, Ordering};
static SESSION: AtomicUsize = AtomicUsize::new(0);
static RESCAN: AtomicUsize = AtomicUsize::new(0);
static CYCLE: AtomicUsize = AtomicUsize::new(0);
pub fn begin_session() {
    SESSION.fetch_add(1, Ordering::Relaxed);
    RESCAN.store(0, Ordering::Relaxed);
}
pub fn set_rescan(n: usize) {
    RESCAN.store(n, Ordering::Relaxed);
}
pub fn hold_cycle(n: usize) {
    CYCLE.store(n, Ordering::Relaxed);
}
pub fn display_state() -> (usize, usize, usize) {
    (
        SESSION.load(Ordering::Relaxed),
        RESCAN.load(Ordering::Relaxed),
        CYCLE.load(Ordering::Relaxed),
    )
}

/// Compact devices-level summary, using the same PM epoch as stage spans.
pub fn summary(fb: musha_framebuffer::Framebuffer, y: usize, scale: usize) {
    use core::fmt::Write;
    let r = crate::bootstate::report();
    let mut line = Line::EMPTY;
    let _ = write!(
        line,
        "PM ms core={} ready={} now={} records={}",
        Milliseconds(r.core_ms),
        Milliseconds(r.ready_ms),
        Milliseconds(crate::bootstate::epoch_now()),
        count()
    );
    unsafe {
        fb.text_scaled(line.as_str(), 8, y, fb.color(180, 200, 220), scale);
    }
}

struct Milliseconds(Option<u64>);
impl core::fmt::Display for Milliseconds {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self.0 {
            Some(n) => write!(f, "{}", n),
            None => f.write_str("NA"),
        }
    }
}
