// SPDX-License-Identifier: Apache-2.0
//! Normal-build PM-epoch timings. BSP only; no controller writes.
pub fn record(args: core::fmt::Arguments<'_>) {
    crate::bootlog::detail("time", args);
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
            record(format_args!(
                "bdf=0x{:04x} stage={} start-ms={}",
                bdf,
                stage,
                start.unwrap_or(0)
            ));
            record(format_args!(
                "end-ms={} elapsed-ms={} {}",
                end.unwrap_or(0),
                end.zip(*start)
                    .and_then(|(e, s)| e.checked_sub(s))
                    .unwrap_or(0),
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

pub fn begin_session() {}
pub fn set_rescan(_n: usize) {}
