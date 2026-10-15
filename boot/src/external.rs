// SPDX-License-Identifier: Apache-2.0
//! One-shot external-ELF boot path. Staging is CPU-owned, never a DMA target.
use core::cell::UnsafeCell;
use musha_api::bootfile::{MAX_BYTES, Reader, Rejection, State};
const _: () = assert!(MAX_BYTES as u64 == musha_memory::elf::abi::IMAGE_LIMIT);
struct Shared(UnsafeCell<(Reader, [u8; MAX_BYTES])>);
// BSP only, interrupts disabled, non-reentrant callbacks. No reference escapes
// while staging is mutable; image() is called only after runtime has returned.
unsafe impl Sync for Shared {}
static mut READ_STARTED: u64 = 0;
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
    fb: musha_framebuffer::Framebuffer,
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
        if crate::bootlog::devices() {
            crate::appconsole::start(fb, b"Loading AUTORUN.ELF ...\n");
        }
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
fn reject(info: &crate::BootInfo, reason: &str) -> ! {
    crate::debug(b"MUSHA: EXTERNAL_ELF_REJECT ");
    crate::debug(reason.as_bytes());
    crate::debug(b"\n");
    let _ = info;
    crate::appconsole::rejected(reason.as_bytes());
    crate::stop()
}
pub fn finish(arena: &mut [u8], info: &crate::BootInfo) {
    let state = unsafe { (&*STAGING.0.get()).0.state() };
    match state {
        State::Ready(_) => {
            if musha_memory::elf::Plan::parse(image()).is_err() {
                reject(info, "Invalid ELF or ABI NOTE");
            }
            crate::bootlog::ready(true);
            crate::appconsole::start(info.framebuffer, b"AUTORUN.ELF  (Ring 3)\n");
            crate::debug(b"MUSHA: EXTERNAL_ELF_DEVICES_STOPPED\n");
            crate::ring3::run(arena, info);
        }
        State::Rejected(reason) => reject(
            info,
            match reason {
                Rejection::Size => "File size outside 64 bytes to 16 MiB",
                Rejection::Duplicate => "Ambiguous AUTORUN.ELF name",
                Rejection::Timeout => "File read deadline exceeded",
                Rejection::Stream(_) => "File read failed or medium removed",
                Rejection::Interrupted => "Device session did not complete",
            },
        ),
        _ => (),
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
pub fn duplicate_catalog(name: &[u16; 12], fb: musha_framebuffer::Framebuffer) {
    if boot_name(name) {
        unsafe {
            (&mut *STAGING.0.get()).0.duplicate();
        }
        if crate::bootlog::devices() {
            crate::appconsole::start(fb, b"Loading AUTORUN.ELF ...\n");
        }
        crate::bootstate::update(|r, t| {
            r.ready(true, t);
        });
        crate::debug(b"MUSHA: OS_READY EXTERNAL_APP\n");
    }
}
