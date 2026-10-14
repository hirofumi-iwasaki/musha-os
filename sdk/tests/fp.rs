// SPDX-License-Identifier: Apache-2.0
#![no_std]
#[repr(C)]pub struct Mixed{pub value:f64,pub count:u64}
unsafe extern "sysv64" {fn c_double(a:f64,b:f64)->f64;}
#[unsafe(no_mangle)]pub extern "sysv64" fn rust_double(a:f64,b:f64)->f64{a+b}
#[unsafe(no_mangle)]pub unsafe extern "sysv64" fn rust_to_c(a:f64,b:f64)->f64{unsafe{c_double(a,b)}}
#[unsafe(no_mangle)]pub extern "sysv64" fn rust_mixed(x:Mixed)->Mixed{x}
