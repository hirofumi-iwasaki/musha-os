// SPDX-License-Identifier: Apache-2.0
//! Local external-ELF integration probe. Staging is CPU-owned, never a DMA target.
use core::cell::UnsafeCell;
use musha_api::bootfile::{MAX_BYTES, Reader, Rejection, State};
const _: () = assert!(MAX_BYTES as u64 == musha_memory::elf::abi::IMAGE_LIMIT);
struct Shared(UnsafeCell<(Reader, [u8; MAX_BYTES])>);
// BSP only, interrupts disabled, non-reentrant callbacks. No reference escapes
// while staging is mutable; image() is called only after runtime has returned.
unsafe impl Sync for Shared {}
static STAGING: Shared = Shared(UnsafeCell::new((Reader::new(), [0; MAX_BYTES])));
pub fn claimed() -> bool {
    unsafe { (&*STAGING.0.get()).0.claimed() }
}
pub fn waiting() -> bool {
    let state = unsafe { (&*STAGING.0.get()).0.state() };
    state == State::Reading
        || (state == State::Searching
            && crate::bootstate::report().files == musha_platform::readiness::Service::Live)
}
pub fn step(files: &mut musha_api::stream::Files<'_>, now: u64) -> State {
    let (reader, bytes) = unsafe { &mut *STAGING.0.get() };
    let before = reader.state();
    let state = reader.step(files, bytes, now);
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
    if !ok || reader.state() == State::Reading {
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
    unsafe {
        let fb = info.framebuffer;
        for y in 0..100.min(fb.height) {
            for x in 0..fb.width {
                fb.pixel(x, y, fb.color(12, 20, 32));
            }
        }
        fb.text_scaled("MUSHA.ELF rejected", 24, 20, fb.color(255, 80, 80), 2);
        fb.text_scaled(reason, 24, 52, fb.color(255, 255, 255), 2);
    }
    crate::stop()
}
pub fn finish(arena: &mut [u8], info: &crate::BootInfo) {
    let state = unsafe { (&*STAGING.0.get()).0.state() };
    match state {
        State::Ready(_) => {
            if musha_memory::elf::Plan::parse(image()).is_err() {
                reject(info, "Invalid ELF or ABI NOTE");
            }
            crate::debug(b"MUSHA: EXTERNAL_ELF_DEVICES_STOPPED\n");
            crate::ring3::run(arena, info);
        }
        State::Rejected(reason) => reject(
            info,
            match reason {
                Rejection::Size => "File size outside 64 bytes to 16 MiB",
                Rejection::Duplicate => "Ambiguous MUSHA.ELF name",
                Rejection::Timeout => "File read deadline exceeded",
                Rejection::Stream(_) => "File read failed or medium removed",
                Rejection::Interrupted => "Device session did not complete",
            },
        ),
        _ => (),
    }
}
