// SPDX-License-Identifier: Apache-2.0
#![no_std]
pub use musha_sys::MushaStartInfoV1;
#[derive(Debug, PartialEq, Eq)]
pub struct Error(pub i32);
/// Validate the two-register result before exposing any value.
pub fn decode(status: i64, value: u64) -> Result<u64, Error> {
    if status > 0 || status < i32::MIN as i64 || (status < 0 && value != 0) {
        return Err(Error(musha_sys::ERR_INVALID));
    }
    if status == 0 {
        Ok(value)
    } else {
        Err(Error(status as i32))
    }
}
/// Validate a copied record. Does not prove mapping, ownership or lifetime.
pub fn validate_start(s: &MushaStartInfoV1) -> Result<(), Error> {
    let range = |base: u64, len: u64| {
        base >= 4096
            && base % 4096 == 0
            && len > 0
            && len % 4096 == 0
            && base.checked_add(len).is_some_and(|end| end <= 0x8000_0000)
    };
    if s.struct_size != 64
        || s.abi_major != 0
        || !matches!(s.abi_minor, 1 | 2)
        || s.flags != 0
        || s.reserved0 != 0
        || s.reserved1 != 0
        || !range(s.arena_base, s.arena_bytes)
        || !range(s.stack_base, s.stack_bytes)
        || s.arena_bytes > musha_sys::ARENA_LIMIT
        || s.stack_bytes > musha_sys::STACK_LIMIT
        || s.arena_base < s.stack_base + s.stack_bytes
            && s.stack_base < s.arena_base + s.arena_bytes
    {
        return Err(Error(musha_sys::ERR_INVALID));
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn raw_results() {
        assert_eq!(decode(0, 91), Ok(91));
        assert_eq!(decode(-7, 0), Err(Error(-7)));
        for (s, v) in [(1, 0), (-1, 42), (0xffff_ffff, 0), (-2147483649, 0)] {
            assert_eq!(decode(s, v), Err(Error(-3)));
        }
    }
    #[test]
    fn records() {
        let mut s = MushaStartInfoV1 {
            struct_size: 64,
            abi_minor: 1,
            arena_base: 0x10000000,
            arena_bytes: 4096,
            stack_base: 0x70000000 - 4096,
            stack_bytes: 4096,
            ..Default::default()
        };
        assert!(validate_start(&s).is_ok());
        s.reserved1 = 1;
        assert!(validate_start(&s).is_err());
        s.reserved1 = 0;
        s.stack_base = s.arena_base;
        assert!(validate_start(&s).is_err());
        s.arena_base = u64::MAX;
        assert!(validate_start(&s).is_err());
    }
}

/// Guest-only wrappers. No syscall instruction is available through this safe API
/// on development hosts. Borrows remain within the synchronous syscall boundary.
#[cfg(all(target_arch = "x86_64", target_os = "none"))]
pub mod guest {
    use super::{Error, decode};
    pub fn query() -> Result<u64, Error> {
        let (status, value) = unsafe { musha_sys::raw(musha_sys::SYS_QUERY, [0; 6]) };
        decode(status, value)
    }
    pub fn log(text: &str) -> Result<usize, Error> {
        if text.len() > musha_sys::MAX_LOG_BYTES as usize {
            return Err(Error(musha_sys::ERR_INVALID));
        }
        let (status, value) = unsafe {
            musha_sys::raw(
                musha_sys::SYS_LOG,
                [text.as_ptr() as u64, text.len() as u64, 0, 0, 0, 0],
            )
        };
        let written = decode(status, value)?;
        if written > text.len() as u64 {
            return Err(Error(musha_sys::ERR_INVALID));
        }
        Ok(written as usize)
    }
    pub fn time() -> Result<u64, Error> {
        let (status, value) = unsafe { musha_sys::raw(musha_sys::SYS_TIME, [0; 6]) };
        decode(status, value)
    }
    pub fn wait(timeout_ms: u64, input: bool) -> Result<u64, Error> {
        let (status, value) = unsafe { musha_sys::raw(musha_sys::SYS_WAIT, [timeout_ms, input as u64, 0, 0, 0, 0]) };
        let reason = decode(status, value)?;
        if reason > 1 { return Err(Error(musha_sys::ERR_INVALID)); }
        Ok(reason)
    }
    pub fn input() -> Result<musha_sys::MushaInputEvent, Error> {
        let mut event = musha_sys::MushaInputEvent::default();
        let (status, value) = unsafe { musha_sys::raw(musha_sys::SYS_INPUT,
            [&mut event as *mut _ as u64, musha_sys::INPUT_SIZE, 0, 0, 0, 0]) };
        let size = decode(status, value)?;
        if size != musha_sys::INPUT_SIZE || event.struct_size != 48 || event.reserved != 0 {
            return Err(Error(musha_sys::ERR_INVALID));
        }
        Ok(event)
    }
    pub fn exit(status: i32) -> ! {
        unsafe {
            musha_sys::raw(musha_sys::SYS_EXIT, [status as i64 as u64, 0, 0, 0, 0, 0]);
        }
        // The contract forbids return; do not continue with a broken kernel.
        unsafe {
            core::arch::asm!("ud2", options(noreturn));
        }
    }
}
