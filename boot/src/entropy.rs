// Copyright 2026 Hirofumi Iwasaki
// SPDX-License-Identifier: Apache-2.0
//! Firmware RNG is called only here, well before the final memory map/EBS pair.
//! Only CPU-owned random bytes survive; no firmware pointer or algorithm handle.
use core::cell::UnsafeCell;
use musha_platform::bluetooth::{
    Error,
    entropy::{Pool, Random},
};
use r_efi::{efi, protocols::rng};
struct Shared(UnsafeCell<Pool<4096>>);
unsafe impl Sync for Shared {} // BSP only; no nested borrow crosses a callback.
static POOL: Shared = Shared(UnsafeCell::new(Pool::new()));
pub unsafe fn initialize(bs: *mut efi::BootServices) {
    let mut ptr = core::ptr::null_mut();
    let mut guid = rng::PROTOCOL_GUID;
    if unsafe { ((*bs).locate_protocol)(&mut guid, core::ptr::null_mut(), &mut ptr) }.is_error()
        || ptr.is_null()
    {
        crate::debug(b"MUSHA: BT_ENTROPY UNAVAILABLE\n");
        return;
    }
    let protocol = ptr as *mut rng::Protocol;
    // Use an explicit SP800-90 algorithm, not a platform-dependent default/raw
    // provider. Failure has no fixed/clock/RDRAND fallback.
    for mut algorithm in [
        rng::ALGORITHM_SP800_90_HASH_256_GUID,
        rng::ALGORITHM_SP800_90_HMAC_256_GUID,
        rng::ALGORITHM_SP800_90_CTR_256_GUID,
    ] {
        let result = unsafe {
            (&mut *POOL.0.get()).initialize(|out| {
                let status =
                    ((*protocol).get_rng)(protocol, &mut algorithm, out.len(), out.as_mut_ptr());
                if status.is_error() {
                    Err(Error::Entropy)
                } else {
                    Ok(())
                }
            })
        };
        if result.is_ok() {
            crate::debug(b"MUSHA: BT_ENTROPY READY\n");
            return;
        }
    }
    crate::debug(b"MUSHA: BT_ENTROPY UNAVAILABLE\n");
}
pub fn fill(out: &mut [u8]) -> Result<(), Error> {
    unsafe { (&mut *POOL.0.get()).fill(out) }
}
pub fn available() -> bool {
    unsafe { (&*POOL.0.get()).remaining() >= 64 }
}
pub struct BootRandom;
impl Random for BootRandom {
    fn fill(&mut self, out: &mut [u8]) -> Result<(), Error> {
        fill(out)
    }
}
