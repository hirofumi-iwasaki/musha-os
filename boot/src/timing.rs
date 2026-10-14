// SPDX-License-Identifier: Apache-2.0
//! Diagnostic-only PM-epoch timings. BSP only; no controller writes.
#[cfg(feature = "diagnostics-detail")]
use core::cell::UnsafeCell;
#[cfg(feature = "diagnostics-detail")]
use musha_platform::diagnostics::Line;
#[cfg(feature = "diagnostics-detail")]
struct Records(UnsafeCell<([Line; 256], usize)>);
#[cfg(feature = "diagnostics-detail")]
unsafe impl Sync for Records {} // BSP, interrupts disabled, no reentrant callbacks.
#[cfg(feature = "diagnostics-detail")]
static RECORDS: Records = Records(UnsafeCell::new(([Line::EMPTY; 256], 0)));
pub fn record(args: core::fmt::Arguments<'_>) {
    #[cfg(feature = "diagnostics-detail")]
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
    #[cfg(not(feature = "diagnostics-detail"))]
    let _ = args;
}
pub fn count() -> usize {
    #[cfg(feature = "diagnostics-detail")]
    return unsafe { (*RECORDS.0.get()).1 };
    #[cfg(not(feature = "diagnostics-detail"))]
    0
}
pub fn row(index: usize) -> musha_platform::diagnostics::Line {
    #[cfg(feature = "diagnostics-detail")]
    return unsafe { (*RECORDS.0.get()).0[index] };
    #[cfg(not(feature = "diagnostics-detail"))]
    {
        let _ = index;
        musha_platform::diagnostics::Line::EMPTY
    }
}
pub struct Span {
    #[cfg(feature = "diagnostics-detail")]
    data: (u16, &'static str, Option<u64>, bool),
}
impl Span {
    pub fn new(bdf: u16, stage: &'static str) -> Self {
        #[cfg(feature = "diagnostics-detail")]
        return Self {
            data: (bdf, stage, crate::bootstate::epoch_now(), false),
        };
        #[cfg(not(feature = "diagnostics-detail"))]
        {
            let _ = (bdf, stage);
            Self {}
        }
    }
    pub fn finish(mut self, result: &'static str) {
        self.emit(result);
    }
    fn emit(&mut self, result: &'static str) {
        #[cfg(feature = "diagnostics-detail")]
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
        #[cfg(not(feature = "diagnostics-detail"))]
        let _ = result;
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
