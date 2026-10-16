// SPDX-License-Identifier: Apache-2.0
//! BSP only, interrupts disabled. One PM epoch; EFI-entry time is unavailable.
use core::cell::UnsafeCell;
use musha_platform::{Clock, Timer, readiness::Report};
struct State {
    report: Report,
    clock: Option<Clock>,
    timer: Option<Timer>,
    now: Option<u64>,
}
struct Shared(UnsafeCell<State>);
// SAFETY: only the boot CPU accesses this with interrupts disabled; no callbacks.
unsafe impl Sync for Shared {}
static STATE: Shared = Shared(UnsafeCell::new(State {
    report: Report::NEW,
    clock: None,
    timer: None,
    now: None,
}));
pub fn sample(timer: Timer, raw: u32) {
    unsafe {
        let s = &mut *STATE.0.get();
        if s.clock.is_none() && timer.port != 0 {
            s.clock = Clock::new(timer, raw);
            s.timer = Some(timer);
        }
        s.now = s.clock.as_mut().and_then(|c| c.sample(raw));
    }
}
pub fn update(f: impl FnOnce(&mut Report, Option<u64>)) {
    unsafe {
        let s = &mut *STATE.0.get();
        f(&mut s.report, s.now);
    }
}
pub fn report() -> Report {
    unsafe { (*STATE.0.get()).report }
}
pub fn dump() {
    let r = report();
    crate::debug(b"MUSHA: BOOT_TIMES CORE READY ENTRY RUNNING FRAME MS UNKNOWN=FFFFFFFFFFFFFFFF ");
    for t in [
        r.core_ms,
        r.ready_ms,
        r.entry_ms,
        r.running_ms,
        r.first_frame_ms,
    ] {
        crate::debug(&crate::cpu::hex(t.unwrap_or(u64::MAX)));
        crate::debug(b" ");
    }
    crate::debug(b"\n");
}

pub fn frame() {
    musha_framebuffer::flush_writes();
    let timer = unsafe { (*STATE.0.get()).timer };
    if let Some(timer) = timer {
        let _ = crate::acpi::Time::new(timer).and_then(|mut t| t.now());
    }
    update(|r, t| r.frame(t));
}

/// Refresh and read the shared PM epoch; never substitute a local-clock epoch.
pub fn epoch_now() -> Option<u64> {
    let timer = unsafe { (*STATE.0.get()).timer }?;
    crate::acpi::Time::new(timer)
        .and_then(|mut t| t.now())
        .ok()?;
    unsafe { (*STATE.0.get()).now }
}
