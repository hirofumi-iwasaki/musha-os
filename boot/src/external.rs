// SPDX-License-Identifier: Apache-2.0
//! Startup/selected external ELF lifecycle. Staging is CPU-owned, never DMA.
use core::cell::UnsafeCell;
use musha_api::bootfile::{MAX_BYTES, Reader, Rejection, State};
const _: () = assert!(MAX_BYTES as u64 == musha_memory::elf::abi::IMAGE_LIMIT);
struct Shared(UnsafeCell<(Reader, [u8; MAX_BYTES])>);
// BSP only, interrupts disabled, non-reentrant callbacks. No reference escapes
// while staging is mutable; image() is called only after runtime has returned.
unsafe impl Sync for Shared {}
#[derive(Clone, Copy)]
pub struct ApplicationName {
    bytes: [u8; 255],
    len: usize,
}
impl ApplicationName {
    pub fn as_str(&self) -> &str {
        core::str::from_utf8(&self.bytes[..self.len]).unwrap_or("application")
    }
}
static mut APP_NAME: ApplicationName = {
    let mut bytes = [0; 255];
    let name = b"AUTORUN.ELF";
    let mut i = 0;
    while i < name.len() {
        bytes[i] = name[i];
        i += 1;
    }
    ApplicationName {
        bytes,
        len: name.len(),
    }
};
pub fn application_name() -> ApplicationName {
    unsafe { APP_NAME }
}
pub fn exited(status: i32) {
    let name = application_name();
    crate::appconsole::message(format_args!(
        "app: {} exited, status {}",
        name.as_str(),
        status
    ));
    #[cfg(feature = "app-log-screen-probe")]
    {
        crate::debug(b"MUSHA: APP_LOG_SCREEN_HOLD\n");
        crate::bootlog::dump();
        loop {
            core::hint::spin_loop();
        }
    }
}
static mut READ_STARTED: u64 = 0;
static mut RESUMED: bool = false;
static mut LAST_OUTCOME: (u64, u64) = (2, 0);
pub fn outcome() -> (u64, u64) {
    unsafe { LAST_OUTCOME }
}
pub fn resumed() -> bool {
    unsafe { RESUMED }
}
static mut READ_TIMING: Option<crate::timing::Span> = None;
static STAGING: Shared = Shared(UnsafeCell::new((Reader::new(), [0; MAX_BYTES])));
/// Linker-owned zero-fill staging must be wholly inside the reserved loaded image.
pub fn staging_range() -> Result<musha_memory::Range, musha_memory::Error> {
    let start = unsafe { core::ptr::addr_of!((*STAGING.0.get()).1) as usize };
    musha_memory::Range::new(start, MAX_BYTES)
}
pub fn claimed() -> bool {
    unsafe { (&*STAGING.0.get()).0.claimed() }
}
pub fn waiting() -> bool {
    let state = unsafe { (&*STAGING.0.get()).0.state() };
    state == State::Reading
        || (state == State::Searching
            && crate::bootstate::report().files == musha_platform::readiness::Service::Live)
}
pub fn step(
    files: &mut musha_api::stream::Files<'_>,
    now: u64,
    _fb: musha_framebuffer::Framebuffer,
) -> State {
    let (reader, bytes) = unsafe { &mut *STAGING.0.get() };
    let before = reader.state();
    if before == State::Searching {
        unsafe {
            READ_STARTED = now;
            if (&*core::ptr::addr_of!(READ_TIMING)).is_none() {
                READ_TIMING = Some(crate::timing::Span::new(0xffff, "ELF READ"));
            }
        }
    }
    let state = reader.step(files, bytes, now);
    if before == State::Searching && reader.claimed() {
        crate::bootlog::detail("app", format_args!("loading AUTORUN.ELF"));
        crate::bootstate::update(|r, t| {
            r.ready(true, t);
        });
        crate::debug(b"MUSHA: OS_READY EXTERNAL_APP\n");
    }
    if before != state && matches!(state, State::Ready(_) | State::Rejected(_)) {
        crate::debug(b"MUSHA: EXTERNAL_ELF_ELAPSED_MS=");
        crate::debug(&crate::cpu::hex(
            now.saturating_sub(unsafe { READ_STARTED }),
        ));
        crate::debug(b"\n");
    }
    if before != state && matches!(state, State::Ready(_) | State::Rejected(_) | State::Absent) {
        unsafe {
            if let Some(span) = core::ptr::addr_of_mut!(READ_TIMING).replace(None) {
                span.finish(match state {
                    State::Ready(_) => "READY",
                    State::Absent => "ABSENT",
                    _ => "REJECTED",
                });
            }
        }
    }
    if before != state {
        crate::debug(match state {
            State::Absent => b"MUSHA: EXTERNAL_ELF_ABSENT VIEWER\n",
            State::Reading => b"MUSHA: EXTERNAL_ELF_READING\n",
            State::Ready(_) => b"MUSHA: EXTERNAL_ELF_READ_COMPLETE\n",
            State::Rejected(_) => b"MUSHA: EXTERNAL_ELF_READ_REJECTED\n",
            _ => b"",
        });
    }
    state
}
pub fn select(
    files: &mut musha_api::stream::Files<'_>,
    id: musha_api::stream::Id,
    now: u64,
) -> bool {
    let mut selected = None;
    if let Ok(mut cursor) = files.list_begin() {
        while let Ok(Some((candidate, entry))) = files.list_next(&mut cursor) {
            if candidate == id {
                let mut name = ApplicationName {
                    bytes: [0; 255],
                    len: entry.name_len as usize,
                };
                for (to, from) in name.bytes.iter_mut().zip(&entry.name[..name.len]) {
                    *to = if (32..=126).contains(from) {
                        *from as u8
                    } else {
                        b'.'
                    };
                }
                selected = Some(name);
                break;
            }
        }
    }
    let reader = unsafe { &mut (*STAGING.0.get()).0 };
    match reader.select(files, id, now) {
        Err(Rejection::Stream(musha_api::stream::Error::Again)) => false,
        Err(error) => {
            reader.reject(error);
            true
        }
        Ok(()) => {
            if let Some(name) = selected {
                unsafe {
                    APP_NAME = name;
                }
            }
            crate::bootstate::update(|r, t| {
                r.prepare_launch(t);
            });
            crate::debug(b"MUSHA: VIEWER_ELF_SELECTED SAME_VALIDATION\n");
            true
        }
    }
}
pub fn session_ended(ok: bool) {
    let reader = unsafe { &mut (*STAGING.0.get()).0 };
    if reader.state() == State::Reading || (!ok && matches!(reader.state(), State::Ready(_))) {
        reader.interrupted();
    }
}
pub fn image() -> &'static [u8] {
    let (reader, bytes) = unsafe { &*STAGING.0.get() };
    match reader.state() {
        State::Ready(n) => &bytes[..n],
        _ => crate::stop(),
    }
}
/// Run one guest inside an ordinary USB callback. Returning through that callback
/// is essential: it drops file sessions before any shared workspace/arena reuse.
fn execute(arena: &mut [u8], info: &crate::BootInfo) -> crate::ring3::lifecycle::Outcome {
    let plan = musha_memory::elf::Plan::parse(image()).ok();
    if plan.as_ref().is_some_and(|p| {
        p.revision == 2 && p.permissions & musha_memory::elf::abi::PERMISSION_INPUT != 0
    }) {
        let mut outcome = None;
        let mut bridge = |event: crate::xhci::AppEvent<'_>| {
            if let crate::xhci::AppEvent::InputPoll(poll) = event {
                let mut input = || {
                    let mut collect = |event: crate::xhci::AppEvent<'_>| {
                        match event {
                            crate::xhci::AppEvent::Keys(keys) => crate::ring3::io::keys(keys),
                            crate::xhci::AppEvent::Pointer(values, buttons) => {
                                crate::ring3::io::pointer(values, buttons)
                            }
                            _ => (),
                        }
                        Ok(false)
                    };
                    poll(&mut collect).and_then(|done| {
                        if done {
                            Err("INPUT SESSION ENDED")
                        } else {
                            Ok(())
                        }
                    })
                };
                // The saved continuation is inside this callback, never outside
                // the session's Drop-bearing scopes. detach runs before return.
                unsafe {
                    crate::ring3::io::attach(&mut input, info);
                }
                crate::debug(b"MUSHA: APP_INPUT_LIVE\n");
                outcome = Some(crate::ring3::lifecycle::once(arena, info));
                return Ok(true);
            }
            Ok(false)
        };
        let result = crate::xhci::run_session(info, &mut bridge);
        drop(bridge);
        if matches!(result, Err("QUIESCE FAILED" | "BT LEASE QUARANTINED")) {
            crate::stop();
        }
        crate::bluetooth::confirm_restart(true);
        // All session callbacks/references have returned and DMA is stopped.
        unsafe {
            crate::xhci::release_files_stopped();
        }
        if let Some(outcome) = outcome {
            return outcome;
        }
        crate::debug(b"MUSHA: APP_INPUT_UNAVAILABLE\n");
    }
    crate::ring3::lifecycle::once(arena, info)
}
/// Initial AUTORUN still wins once. Subsequent mounts enter the viewer, never
/// automatically run it again. Selected IDs belong to each fresh file session.
pub fn finish(arena: &mut [u8], info: &crate::BootInfo) {
    loop {
        let bt_restart = crate::bluetooth::take_restart();
        let had_claim = claimed();
        if !had_claim && !bt_restart {
            break;
        }
        // runtime already returned its Context and confirmed device shutdown.
        unsafe {
            crate::xhci::release_files_stopped();
        }
        let state = unsafe { (&*STAGING.0.get()).0.state() };
        if matches!(state, State::Ready(_)) {
            if unsafe { RESUMED } {
                crate::bootlog::application_screen();
            }
            crate::bootlog::viewer(false);
            let name = application_name();
            crate::appconsole::message(format_args!("app: starting {}", name.as_str()));
            crate::debug(b"MUSHA: EXTERNAL_ELF_DEVICES_STOPPED\n");
            let outcome = execute(arena, info);
            crate::bootlog::ready(false);
            unsafe {
                LAST_OUTCOME = (outcome.kind, outcome.detail);
            }
            crate::debug(b"MUSHA: APP_RECOVERED KIND=");
            crate::debug(&crate::cpu::hex(outcome.kind));
            crate::debug(b" DETAIL=");
            crate::debug(&crate::cpu::hex(outcome.detail));
            crate::debug(b"\n");
        } else if let State::Rejected(reason) = state {
            unsafe {
                LAST_OUTCOME = (2, 0);
            }
            crate::debug(b"MUSHA: EXTERNAL_ELF_READ_REJECTED VIEWER\n");
            let _ = reason;
        }
        // A Ring3 disconnect request may have arrived during execute. Its loan
        // has now returned normally; consume it as part of this existing restart.
        let _ = crate::bluetooth::take_restart();
        // No Context, file handle, input loan or active DMA can reference arena.
        arena.fill(0);
        if arena.iter().any(|&v| v != 0) {
            crate::stop();
        }
        unsafe {
            let (reader, bytes) = &mut *STAGING.0.get();
            if had_claim {
                RESUMED = true;
            }
            *reader = Reader::viewer();
            bytes.fill(0);
        }
        crate::bootstate::update(|r, _| r.restart());
        crate::debug(b"MUSHA: APP_ARENA_SCRUBBED VIEWER_RESTART\n");
        if crate::runtime::run(arena, info).is_err() {
            // A failed restart cannot authorize another arena/DMA reuse.
            crate::debug(b"MUSHA: VIEWER_RESTART_FAILED QUARANTINED\n");
            crate::stop();
        }
    }
}

pub fn entering() {
    crate::bootstate::update(|r, t| {
        r.entering(t);
    });
    crate::debug(b"MUSHA: APP_ENTERING EXTERNAL_APP\n");
}
pub fn running() {
    crate::bootstate::update(|r, t| {
        r.running(t);
    });
    crate::debug(b"MUSHA: APP_RUNNING EXTERNAL_APP\n");
}

pub fn boot_name(name: &[u16; 12]) -> bool {
    name[..musha_api::bootfile::NAME.len()]
        .iter()
        .zip(musha_api::bootfile::NAME)
        .all(|(&a, &b)| a <= 127 && (a as u8).to_ascii_uppercase() == b)
        && name[musha_api::bootfile::NAME.len()..]
            .iter()
            .all(|&c| c == 0)
}
pub fn duplicate_catalog(name: &[u16; 12], _fb: musha_framebuffer::Framebuffer) {
    if boot_name(name) && unsafe { (&mut *STAGING.0.get()).0.duplicate() } {
        // A readable catalog with ambiguous AUTORUN is an app rejection, not
        // an absent volume. Never publish its files or execute either entry.
        crate::bootlog::retain_media_observation();
        crate::bootlog::detail("app", format_args!("loading AUTORUN.ELF"));
        crate::bootstate::update(|r, t| {
            r.ready(true, t);
        });
        crate::debug(b"MUSHA: OS_READY EXTERNAL_APP\n");
    }
}
