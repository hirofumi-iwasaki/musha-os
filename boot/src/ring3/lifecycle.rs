// SPDX-License-Identifier: Apache-2.0
//! C3: BSP/IF=0 only. Unwind-free escape to a saved Win64 call frame.
//! No Drop-bearing resources may be acquired between invoke and finish.
use super::{asm, naked_asm, rdmsr, wrmsr};
use musha_memory::PAGE;
static mut ACTIVE: bool = false;
static mut RETURN_SP: usize = 0;
static mut SNAPSHOT: crate::memory::ProbeSnapshot = crate::memory::ProbeSnapshot::new();
static mut DETAIL: u64 = 0;
const MSRS: [u32; 7] = [
    0xc0000080, 0xc0000081, 0xc0000082, 0xc0000084, 0xc0000100, 0xc0000101, 0xc0000102,
];

#[derive(Clone, Copy)]
pub struct Outcome {
    pub kind: u64,
    pub detail: u64,
}
#[cfg(feature = "ring3-lifecycle-probe")]
pub fn run(arena: &mut [u8], info: &crate::BootInfo) {
    let _ = attempts(arena, info, 3, true);
    crate::debug(b"MUSHA: C3_OS_RESUMED\n");
}
/// Only the guest frames are abandoned. The enclosing USB callback returns
/// normally, so session cleanup/Drop can run before the caller scrubs the arena.
#[cfg(external_app)]
pub fn once(arena: &mut [u8], info: &crate::BootInfo) -> Outcome {
    attempts(arena, info, 1, false)
}
fn attempts(arena: &mut [u8], info: &crate::BootInfo, count: u64, scrub_each: bool) -> Outcome {
    unsafe {
        let flags: u64;
        let cr0: u64;
        let cr4: u64;
        asm!("pushfq", "pop {}",out(reg) flags,options(preserves_flags));
        asm!("mov {}, cr0",out(reg) cr0,options(nomem,nostack));
        asm!("mov {}, cr4",out(reg) cr4,options(nomem,nostack));
        // The continuation only preserves x87/SSE state; no lazy FPU or IRQs.
        if flags & (1 << 9) != 0 || cr0 & ((1 << 2) | (1 << 3)) != 0 || cr4 & (1 << 9) == 0 {
            crate::stop();
        }
        let mut msrs = [0u64; 7];
        for (i, n) in MSRS.iter().enumerate() {
            msrs[i] = rdmsr(*n);
        }
        let (ds, es, fs, gs): (u16, u16, u16, u16);
        asm!("mov {0:x}, ds", "mov {1:x}, es", "mov {2:x}, fs", "mov {3:x}, gs",out(reg) ds,out(reg) es,out(reg) fs,out(reg) gs,options(nomem,nostack));
        let snapshot = &mut *core::ptr::addr_of_mut!(SNAPSHOT);
        if snapshot.save().is_err() {
            crate::stop();
        }
        let mut last = Outcome { kind: 2, detail: 0 };
        for attempt in 0..count {
            super::SEEN_QUERY = false;
            DETAIL = 0;
            ACTIVE = true;
            // The assembly wrapper owns its saved stack and FPU state outside
            // the application arena. finish never returns to the abandoned Rust frames.
            let outcome = invoke(arena.as_mut_ptr(), arena.len(), info);
            ACTIVE = false;
            // The loan points into the enclosing live USB callback. Confirm DMA
            // stop before restoring mappings, and never reuse on failure.
            let stopped = super::io::detach();
            #[cfg(feature = "app-recovery-stop-failure")]
            let stopped = {
                let _ = stopped;
                false
            };
            if !stopped {
                crate::debug(b"MUSHA: APP_RECOVERY_STOP_UNCONFIRMED QUARANTINED\n");
                crate::stop();
            }
            last = Outcome {
                kind: outcome,
                detail: DETAIL,
            };
            if snapshot.restore().is_err() {
                crate::stop();
            }
            crate::cpu::user_kernel_stack(info.stack_base + info.stack_bytes);
            asm!("mov ds, {0:x}","mov es, {1:x}","mov fs, {2:x}","mov gs, {3:x}",in(reg) ds,in(reg) es,in(reg) fs,in(reg) gs,options(nostack));
            for (i, n) in MSRS.iter().enumerate() {
                wrmsr(*n, msrs[i]);
            }
            asm!("mov cr4, {}",in(reg) cr4,options(nostack));
            asm!("mov cr0, {}",in(reg) cr0,options(nostack));
            super::elf::clear();
            super::KERNEL_TOP = 0;
            super::USER_SP = 0;
            // Restore the kernel guard identity mapping before touching arena.
            if scrub_each {
                arena.fill(0);
            }
            if (scrub_each && arena.iter().any(|v| *v != 0))
                || crate::memory::probe_vacant(super::abi::IMAGE_BASE as usize, PAGE).is_err()
                || crate::memory::probe_vacant(super::abi::ARENA_BASE as usize, PAGE).is_err()
                || crate::memory::probe_vacant(super::abi::START_BASE as usize, PAGE).is_err()
                || crate::memory::probe_vacant(super::TOP - PAGE, PAGE).is_err()
            {
                crate::stop();
            }
            crate::debug(if scrub_each {
                b"MUSHA: C3_RECLAIMED ATTEMPT="
            } else {
                b"MUSHA: APP_CPU_RESTORED ATTEMPT="
            });
            crate::debug(&crate::cpu::hex(attempt));
            crate::debug(b" OUTCOME=");
            crate::debug(&crate::cpu::hex(outcome));
            crate::debug(b" DETAIL=");
            crate::debug(&crate::cpu::hex(DETAIL));
            crate::debug(b"\n");
        }
        last
    }
}
#[unsafe(naked)]
unsafe extern "win64" fn invoke(
    _arena: *mut u8,
    _len: usize,
    _info: *const crate::BootInfo,
) -> u64 {
    naked_asm!(
        "push rbx","push rbp","push rdi","push rsi","push r12","push r13","push r14","push r15",
        // Entry RSP=8 mod16; eight pushes and 552 bytes produce alignment 0.
        "sub rsp,552","fxsave64 [rsp+32]","mov [rip+{saved}],rsp",
        "call {launch}","ud2",saved=sym RETURN_SP,launch=sym launch);
}
extern "win64" fn launch(arena: *mut u8, len: usize, info: *const crate::BootInfo) -> ! {
    #[cfg(feature = "ring3-lifecycle-kernel-fault")]
    unsafe {
        asm!("ud2", options(nostack));
    }
    unsafe { super::run(core::slice::from_raw_parts_mut(arena, len), &*info) }
}
#[unsafe(naked)]
unsafe extern "win64" fn finish(_outcome: u64) -> ! {
    naked_asm!("cli","cld","mov rax,rcx","mov rsp,[rip+{saved}]",
        "fxrstor64 [rsp+32]","add rsp,552",
        "pop r15","pop r14","pop r13","pop r12","pop rsi","pop rdi","pop rbp","pop rbx","ret",saved=sym RETURN_SP);
}
pub fn exit(status: u64) -> ! {
    unsafe {
        if ACTIVE {
            DETAIL = status;
            finish(0);
        }
    }
    crate::stop()
}
pub fn reject(reason: &[u8]) -> ! {
    // Do not turn kernel stack/invariant failures into application failures.
    let recoverable = matches!(
        reason,
        b"ELF_FORMAT"
            | b"ELF_SIZE"
            | b"ELF_MEMORY"
            | b"ELF_COLLISION"
            | b"ELF_COPY"
            | b"ELF_MAP"
            | b"INJECTED_MAP_FAILURE"
            | b"RETURN"
            | b"CPU"
            | b"CET"
            | b"ARENA"
            | b"GUARD"
    );
    unsafe {
        if ACTIVE && recoverable {
            finish(2);
        }
    }
    crate::stop()
}
/// Only synchronous, app-origin faults terminate an app. NMI/#MC/#DF and every
/// CPL0 fault retain the fatal handler. No exception is resumed in user space.
pub fn fault(frame: usize, cs: u64, vector: u64, error: u64) {
    unsafe {
        if ACTIVE && cs & 3 == 3 && matches!(vector, 0 | 6 | 13 | 14) {
            super::verify_fault_stack(frame, cs, vector);
            DETAIL = vector;
            crate::debug(b"MUSHA: C3_APP_FAULT VECTOR=");
            crate::debug(&crate::cpu::hex(vector));
            crate::debug(b" ERROR=");
            crate::debug(&crate::cpu::hex(error));
            crate::debug(b"\n");
            finish(1);
        }
    }
}
