// SPDX-License-Identifier: Apache-2.0
#![no_std]
#![no_main]
mod common;
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Mixed {
    value: f64,
    count: u64,
}
unsafe extern "sysv64" {
    fn c_double(a: f64, b: f64) -> f64;
    fn c_to_rust(a: f64, b: f64) -> f64;
    fn c_mixed(x: Mixed) -> Mixed;
    fn c_stack(a: u64, b: u64, c: u64, d: u64, e: u64, f: u64, g: u64) -> u64;
    fn c_to_rust_stack(a: u64, b: u64, c: u64, d: u64, e: u64, f: u64, g: u64) -> u64;
    fn c_fp_stack(a: f64, b: f64, c: f64, d: f64, e: f64, f: f64, g: f64, h: f64, i: f64) -> f64;
    fn c_to_rust_fp_stack(
        a: f64,
        b: f64,
        c: f64,
        d: f64,
        e: f64,
        f: f64,
        g: f64,
        h: f64,
        i: f64,
    ) -> f64;
}
#[unsafe(no_mangle)]
pub extern "sysv64" fn rust_double(a: f64, b: f64) -> f64 {
    a + b
}
#[unsafe(no_mangle)]
pub extern "sysv64" fn rust_mixed(x: Mixed) -> Mixed {
    Mixed {
        value: x.value + 0.5,
        count: x.count + 1,
    }
}
#[unsafe(no_mangle)]
pub extern "sysv64" fn rust_stack(a: u64, b: u64, c: u64, d: u64, e: u64, f: u64, g: u64) -> u64 {
    a + b + c + d + e + f + g
}
#[unsafe(no_mangle)]
pub extern "sysv64" fn rust_fp_stack(
    a: f64,
    b: f64,
    c: f64,
    d: f64,
    e: f64,
    f: f64,
    g: f64,
    h: f64,
    i: f64,
) -> f64 {
    a + b + c + d + e + f + g + h + i
}
fn raw_fp() -> bool {
    let sentinel = 0x4056_d000_0000_0000u64;
    let observed: u64;
    let mut status = musha_sys::SYS_TIME;
    let mut value = 0u64;
    let mut x87 = 0f64;
    // One raw boundary, not a language call: SysV itself may clobber XMM15.
    unsafe {
        core::arch::asm!(
        "movq xmm15, {sentinel}","fld1","syscall","movq {observed}, xmm15","fstp qword ptr [{x87}]",
        sentinel=in(reg)sentinel, observed=out(reg)observed, x87=in(reg)&mut x87,
        inout("rax")status, inout("rdx")value,
        in("rdi")0u64,in("rsi")0u64,in("r10")0u64,in("r8")0u64,in("r9")0u64,
        out("rcx")_,out("r11")_,out("xmm15")_);
    }
    status == 0 && observed == sentinel && x87 == 1.0
}
#[unsafe(no_mangle)]
pub unsafe extern "sysv64" fn musha_main(info: *const musha::MushaStartInfoV1) -> i32 {
    if info.is_null() {
        return 1;
    }
    let start = unsafe { info.read() };
    if musha::validate_start(&start).is_err() || start.abi_minor != 2 {
        return 2;
    }
    if musha::guest::query() != Ok(3) {
        return 3;
    }
    unsafe {
        if c_double(1.25, 2.5) != 3.75 || c_to_rust(1.5, 2.75) != 4.25 {
            return 4;
        }
        let mixed = c_mixed(Mixed {
            value: 2.25,
            count: 41,
        });
        if mixed.value != 2.75 || mixed.count != 42 {
            return 5;
        }
        if c_stack(1, 2, 3, 4, 5, 6, 7) != 28 || c_to_rust_stack(1, 2, 3, 4, 5, 6, 7) != 28 {
            return 6;
        }
        if c_fp_stack(1., 2., 3., 4., 5., 6., 7., 8., 9.) != 45.
            || c_to_rust_fp_stack(1., 2., 3., 4., 5., 6., 7., 8., 9.) != 45.
        {
            return 7;
        }
    }
    if !raw_fp() || musha::guest::wait(1, false) != Ok(0) || !raw_fp() {
        return 8;
    }
    if musha::guest::log("R2 MIXED FP OK\n").is_err() {
        return 9;
    }
    0
}
