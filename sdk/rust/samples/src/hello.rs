// SPDX-License-Identifier: Apache-2.0
#![no_std]
#![no_main]
mod common;
#[unsafe(no_mangle)]
pub unsafe extern "sysv64" fn musha_main(info: *const musha::MushaStartInfoV1) -> i32 {
    // SAFETY: crt0 receives the kernel-owned read-only, aligned start mapping;
    // copy immediately, never expose an unconstrained reference or retain it.
    if info.is_null() || (info as usize) % 8 != 0 {
        return 1;
    }
    let start = unsafe { info.read() };
    match run(start) {
        Ok(()) => 0,
        Err(status) => status,
    }
}
fn run(start: musha::MushaStartInfoV1) -> Result<(), i32> {
    musha::validate_start(&start).map_err(|_| 2)?;
    if musha::guest::query().map_err(|_| 3)? & 1 == 0 {
        return Err(3);
    }
    musha::guest::log("Hello Musha-OS from Rust!\n").map_err(|_| 4)?;
    Ok(())
}
