// SPDX-License-Identifier: Apache-2.0
// Compilation fixture only; custom target + rebuilt core must precede ELF linking.
#![no_std]
use musha_sys::{MushaStartInfoV1,raw,SYS_QUERY};
#[unsafe(no_mangle)]pub unsafe extern "sysv64" fn musha_main(info:*const MushaStartInfoV1)->i32{
 if info.is_null(){return 1;}let (status,_)=unsafe{raw(SYS_QUERY,[0;6])};status as i32
}
