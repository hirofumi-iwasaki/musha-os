// SPDX-License-Identifier: Apache-2.0
#![no_std]
mod generated;
pub use generated::*;
/// Unsafe: invoke only on a kernel implementing candidate r1 with valid buffers.
/// Never call on the development host or current Musha EFI.
#[cfg(target_arch="x86_64")]
pub unsafe fn raw(nr:u64,args:[u64;6])->(i64,u64){
 let mut status=nr;let mut value=args[2];
 unsafe {core::arch::asm!("syscall",inlateout("rax")status,inlateout("rdx")value,in("rdi")args[0],in("rsi")args[1],in("r10")args[3],in("r8")args[4],in("r9")args[5],lateout("rcx")_,lateout("r11")_);}
 (status as i64,value)
}
