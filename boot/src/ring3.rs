// Copyright 2026 Hirofumi Iwasaki
// SPDX-License-Identifier: Apache-2.0
//! External ELF execution under a saved continuation, plus C1/C2/C3 probes.
//! No scheduler, IRQ return, general user copies, or public application entry.
use core::arch::{asm, naked_asm, x86_64::__cpuid};
use musha_memory::PAGE;
#[allow(dead_code)]
#[path = "../../sdk/rust/musha-sys/src/generated.rs"]
mod abi;
#[cfg(elf_app)]
mod elf;
#[cfg(elf_app)]
pub(crate) mod io;
#[cfg(any(feature = "ring3-lifecycle-probe", external_app))]
pub mod lifecycle;
#[cfg(c1_probe)]
const CODE: usize = abi::IMAGE_BASE as usize;
const TOP: usize = abi::STACK_TOP as usize;
#[cfg(c1_probe)]
const STACK: usize = TOP - PAGE;
const USER_CS: u64 = 0x33;
const USER_SS: u64 = 0x2b;
static mut KERNEL_TOP: usize = 0;
static mut USER_SP: usize = 0;
#[cfg(c1_probe)]
static mut BACKING: usize = 0;
#[cfg(c1_probe)]
static mut CODE_BYTES: usize = 0;
static mut SEEN_QUERY: bool = false;
#[cfg(c1_probe)]
core::arch::global_asm!(include_str!("ring3/probe.S"), query = const abi::SYS_QUERY,
    log = const abi::SYS_LOG, exit = const abi::SYS_EXIT,
    code = const CODE, stack = const STACK);
#[cfg(c1_probe)]
unsafe extern "C" {
    static musha_probe_start: u8;
    static musha_probe_end: u8;
}
unsafe fn rdmsr(n: u32) -> u64 {
    let (lo, hi): (u32, u32);
    unsafe {
        asm!("rdmsr",in("ecx") n,out("eax") lo,out("edx") hi,options(nomem,nostack));
    }
    lo as u64 | (hi as u64) << 32
}
unsafe fn wrmsr(n: u32, v: u64) {
    unsafe {
        asm!("wrmsr",in("ecx") n,in("eax") v as u32,in("edx") (v>>32) as u32,options(nomem,nostack));
    }
}
fn failed(reason: &[u8]) -> ! {
    crate::debug(b"MUSHA: RING3_REJECT ");
    crate::debug(reason);
    crate::debug(b"\n");
    #[cfg(external_app)]
    crate::appconsole::rejected(reason);
    #[cfg(any(feature = "ring3-lifecycle-probe", external_app))]
    lifecycle::reject(reason);
    #[cfg(not(any(feature = "ring3-lifecycle-probe", external_app)))]
    crate::stop()
}
pub fn run(arena: &mut [u8], _info: &crate::BootInfo) -> ! {
    // Only lifecycle recovery may restore mappings and authorize arena reuse.
    let pa = arena.as_mut_ptr() as usize;
    if arena.len() < 20 * PAGE || pa % PAGE != 0 {
        failed(b"ARENA");
    }
    #[cfg(c1_probe)]
    let case = match option_env!("MUSHA_RING3_CASE").unwrap_or("normal") {
        "normal" => 0,
        "ro" => 1,
        "nx" => 2,
        "guard" => 3,
        "supervisor" => 4,
        "rsp" => 5,
        "privileged" => 6,
        "nmi-transition" => 7,
        _ => failed(b"CASE"),
    };
    unsafe {
        if __cpuid(0x80000001).edx & (1 << 11) == 0
            || __cpuid(1).edx & ((1 << 24) | (1 << 25) | (1 << 26))
                != ((1 << 24) | (1 << 25) | (1 << 26))
        {
            failed(b"CPU");
        }
        let mut cr0: usize;
        let mut cr4: usize;
        asm!("mov {}, cr0",out(reg) cr0,options(nomem,nostack));
        asm!("mov {}, cr4",out(reg) cr4,options(nomem,nostack));
        // Only x87/SSE2 is saved. No AVX/XSAVE/FSGSBASE/CET contract in C1.
        if cr4 & (1 << 23) != 0 {
            failed(b"CET");
        }
        cr0 = (cr0 | (1 << 1) | (1 << 5)) & !((1 << 2) | (1 << 3));
        cr4 = (cr4 | (1 << 9) | (1 << 10)) & !((1 << 18) | (1 << 16));
        asm!("mov cr0, {}",in(reg) cr0,options(nostack));
        asm!("mov cr4, {}",in(reg) cr4,options(nostack));
        core::ptr::write_bytes(pa as *mut u8, 0, 20 * PAGE);
        KERNEL_TOP = pa + 19 * PAGE;
        #[cfg(elf_app)]
        let (argument, second, entry) = (abi::START_BASE as usize, 0, elf::load(arena));
        #[cfg(c1_probe)]
        let (argument, second, entry) = {
            let src = core::ptr::addr_of!(musha_probe_start);
            let size = (core::ptr::addr_of!(musha_probe_end) as usize)
                .checked_sub(src as usize)
                .filter(|n| *n > 0 && *n <= PAGE)
                .unwrap_or_else(|| failed(b"PAYLOAD"));
            core::ptr::copy_nonoverlapping(src, pa as *mut u8, size);
            BACKING = pa;
            CODE_BYTES = size;
            if crate::memory::probe_user_page(CODE, pa, false, true).is_err()
                || crate::memory::probe_user_page(STACK, pa + PAGE, true, false).is_err()
            {
                failed(b"MAPPINGS");
            }
            (case, pa, CODE)
        };
        if crate::memory::probe_guard(pa + 2 * PAGE).is_err() {
            failed(b"GUARD");
        }
        crate::cpu::user_kernel_stack(KERNEL_TOP);
        wrmsr(0xc0000081, 8u64 << 32); // STAR: kernel CS=8, SS=16; no SYSRET.
        wrmsr(0xc0000082, syscall_entry as *const () as u64);
        wrmsr(0xc0000084, 0xffff_ffff); // SFMASK: no user flags in the kernel.
        wrmsr(0xc0000080, rdmsr(0xc0000080) | 1); // EFER.SCE, retain NXE.
        wrmsr(0xc0000100, 0);
        wrmsr(0xc0000101, 0);
        wrmsr(0xc0000102, 0);
        #[cfg(c1_probe)]
        crate::debug(b"MUSHA: RING3_ENTERING C1_PROBE\n");
        // C1 uses test controls; C2 passes the read-only SDK start record.
        if cfg!(elf_app) {
            crate::debug(b"MUSHA: RING3_ELF_ENTERING\n");
        }
        #[cfg(external_app)]
        crate::external::entering();
        enter(argument, second, entry);
    }
}
#[unsafe(naked)]
unsafe extern "win64" fn enter(_argument: usize, _second: usize, _entry: usize) -> ! {
    naked_asm!(
        "cli", "cld", "mov rdi, rcx", "mov rsi, rdx",
        "mov ax, {ss}", "mov ds, ax", "mov es, ax",
        "xor eax, eax", "mov fs, ax", "mov gs, ax",
        "fninit", "fldz", "fldz", "fldz", "fldz", "fldz", "fldz", "fldz", "fldz", "fninit", "sub rsp, 16", "mov dword ptr [rsp], 0x1f80", "ldmxcsr [rsp]", "add rsp, 16",
        "pxor xmm0,xmm0", "pxor xmm1,xmm1", "pxor xmm2,xmm2", "pxor xmm3,xmm3",
        "pxor xmm4,xmm4", "pxor xmm5,xmm5", "pxor xmm6,xmm6", "pxor xmm7,xmm7",
        "pxor xmm8,xmm8", "pxor xmm9,xmm9", "pxor xmm10,xmm10", "pxor xmm11,xmm11",
        "pxor xmm12,xmm12", "pxor xmm13,xmm13", "pxor xmm14,xmm14", "pxor xmm15,xmm15",
        "push {ss}", "push {top}", "push 2", "push {cs}", "push r8",
        "xor eax,eax", "xor ebx,ebx", "xor ecx,ecx", "xor edx,edx", "xor ebp,ebp",
        "xor r8d,r8d", "xor r9d,r9d", "xor r10d,r10d", "xor r11d,r11d",
        "xor r12d,r12d", "xor r13d,r13d", "xor r14d,r14d", "xor r15d,r15d", "iretq",
        ss=const USER_SS, cs=const USER_CS, top=const TOP);
}
#[repr(C)]
struct Frame {
    rax: u64,
    rbx: u64,
    rdx: u64,
    rsi: u64,
    rdi: u64,
    rbp: u64,
    r8: u64,
    r9: u64,
    r10: u64,
    r12: u64,
    r13: u64,
    r14: u64,
    r15: u64,
    rip: u64,
    cs: u64,
    flags: u64,
    rsp: u64,
    ss: u64,
}
const _: () = {
    assert!(core::mem::size_of::<Frame>() == 144);
    assert!(core::mem::offset_of!(Frame, rip) == 104);
    assert!(core::mem::offset_of!(Frame, rsp) == 128);
};
#[unsafe(naked)]
unsafe extern "win64" fn syscall_entry() -> ! {
    naked_asm!(
        // No push, call, or compiler prologue may precede the RSP switch.
        // Permanent supervisor RIP-relative globals; BSP, no returning IRQ/NMI.
        "mov [rip + {user_sp}], rsp",
        // Deterministic NMI injection while CS=0 and RSP is still user-owned.
        // This case never resumes; its debug output may clobber RAX/RDX.
        ".if {nmi}", "mov dx, 0xe9", "mov al, 64", "out dx, al", "out dx, al",
        "7:", "pause", "jmp 7b", ".endif",
        "mov rsp, [rip + {kernel_top}]",
        "push {ss}", "push qword ptr [rip + {user_sp}]", "push r11", "push {cs}", "push rcx",
        "push r15", "push r14", "push r13", "push r12", "push r10", "push r9", "push r8",
        "push rbp", "push rdi", "push rsi", "push rdx", "push rbx", "push rax", "cld",
        // 144-byte frame keeps 16-byte alignment; 32-byte Win64 home area.
        "sub rsp, 544", "fxsave64 [rsp + 32]", "fninit",
        "mov dword ptr [rsp], 0x1f80", "ldmxcsr [rsp]", "lea rcx, [rsp + 544]",
        "call {dispatch}", "fxrstor64 [rsp + 32]", "add rsp, 544",
        "pop rax", "pop rbx", "pop rdx", "pop rsi", "pop rdi", "pop rbp",
        "pop r8", "pop r9", "pop r10", "pop r12", "pop r13", "pop r14", "pop r15",
        "xor ecx, ecx", "xor r11d, r11d", "iretq", user_sp=sym USER_SP, kernel_top=sym KERNEL_TOP,
        dispatch=sym dispatch, ss=const USER_SS, cs=const USER_CS,
        nmi=const cfg!(ring3_transition_nmi) as usize);
}
extern "win64" fn dispatch(frame: &mut Frame) {
    unsafe {
        let p = frame as *mut Frame as usize;
        if p < KERNEL_TOP - 16 * PAGE || p + 144 != KERNEL_TOP {
            failed(b"KERNEL_STACK");
        }
        #[cfg(elf_app)]
        let valid = elf::valid_return(frame.rip, frame.rsp);
        #[cfg(c1_probe)]
        let valid = musha_memory::user::valid_return(
            frame.rip,
            frame.rsp,
            CODE as u64,
            CODE_BYTES as u64,
            STACK as u64,
            PAGE as u64,
        );
        if !valid || frame.cs != USER_CS || frame.ss != USER_SS {
            failed(b"RETURN");
        }
        frame.flags = (frame.flags & 0x8d5) | 2; // arithmetic only; DF/IF/IOPL/TF/NT/AC=0.
        let (status, value) = match frame.rax {
            abi::SYS_QUERY => {
                if !SEEN_QUERY {
                    crate::debug(b"MUSHA: RING3_QUERY KERNEL_STACK_OK\n");
                    SEEN_QUERY = true;
                    #[cfg(external_app)]
                    crate::external::running();
                    #[cfg(elf_app)]
                    crate::debug(b"MUSHA: RING3_ELF_RUNNING\n");
                }
                #[cfg(elf_app)]
                {
                    (0, elf::features())
                }
                #[cfg(c1_probe)]
                {
                    (0, abi::FEATURE_BASE)
                }
            }
            #[cfg(elf_app)]
            abi::SYS_LOG => elf::log(frame.rdi, frame.rsi),
            #[cfg(c1_probe)]
            abi::SYS_LOG => {
                let length = frame.rsi;
                if length > abi::MAX_LOG_BYTES {
                    (abi::ERR_LIMIT, 0)
                } else if length == 0 {
                    (0, 0)
                } else {
                    // Translate validated fixture pages to supervisor aliases.
                    // Never dereference a caller-provided virtual pointer in Rust.
                    let address = musha_memory::user::offset(
                        frame.rdi,
                        length,
                        CODE as u64,
                        CODE_BYTES as u64,
                    )
                    .map(|n| BACKING + n)
                    .or_else(|| {
                        musha_memory::user::offset(frame.rdi, length, STACK as u64, PAGE as u64)
                            .map(|n| BACKING + PAGE + n)
                    });
                    if let Some(address) = address {
                        crate::debug(core::slice::from_raw_parts(
                            address as *const u8,
                            length as usize,
                        ));
                        (0, length)
                    } else {
                        (abi::ERR_INVALID, 0)
                    }
                }
            }
            #[cfg(elf_app)]
            abi::SYS_TIME | abi::SYS_WAIT | abi::SYS_INPUT => {
                if frame.rdx != 0 || frame.r10 != 0 || frame.r8 != 0 || frame.r9 != 0 {
                    (abi::ERR_INVALID, 0)
                } else if frame.rax == abi::SYS_TIME {
                    if !elf::permitted(abi::PERMISSION_TIME) {
                        (abi::ERR_DENIED, 0)
                    } else if frame.rdi != 0 || frame.rsi != 0 {
                        (abi::ERR_INVALID, 0)
                    } else {
                        io::time()
                    }
                } else if frame.rax == abi::SYS_WAIT {
                    io::wait(frame.rdi, frame.rsi)
                } else {
                    elf::input(frame.rdi, frame.rsi)
                }
            }
            abi::SYS_EXIT => {
                #[cfg(elf_app)]
                {
                    crate::debug(b"MUSHA: RING3_ELF_EXIT_STATUS=");
                    crate::debug(&crate::cpu::hex(frame.rdi));
                    crate::debug(b"\n");
                }
                #[cfg(external_app)]
                {
                    crate::appconsole::message(format_args!(
                        "\nAUTORUN.ELF exited: status {}\n",
                        frame.rdi as i32
                    ));
                    crate::bootstate::update(|r, _| r.finish(true));
                    crate::debug(b"MUSHA: EXTERNAL_APP_EXIT\n");
                    lifecycle::exit(frame.rdi);
                }
                #[cfg(any(feature = "ring3-lifecycle-probe", external_app))]
                lifecycle::exit(frame.rdi);
                #[cfg(not(any(feature = "ring3-lifecycle-probe", external_app)))]
                {
                    if frame.rdi != 0 || !SEEN_QUERY {
                        failed(b"USER_ASSERT");
                    }
                    crate::debug(b"MUSHA: RING3_EXIT_OK\n");
                    crate::stop();
                }
            }
            _ => (abi::ERR_UNKNOWN_CALL, 0),
        };
        frame.rax = status as i64 as u64;
        frame.rdx = value;
    }
}

/// CPU faults originating at CPL3 must land on the dedicated TSS.RSP0 stack.
pub fn verify_fault_stack(frame: usize, cs: u64, vector: u64) {
    if cs & 3 == 3 && !matches!(vector, 1 | 2 | 8 | 18) {
        unsafe {
            if frame < KERNEL_TOP - 16 * PAGE
                || frame.checked_add(56).is_none_or(|end| end > KERNEL_TOP)
            {
                failed(b"FAULT_STACK");
            }
        }
        crate::debug(b"MUSHA: RING3_FAULT_KERNEL_STACK_OK\n");
    }
}
