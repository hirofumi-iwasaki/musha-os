// Copyright 2026 Hirofumi Iwasaki
// SPDX-License-Identifier: Apache-2.0
//! x86-64 exception setup. C3 may terminate selected user faults to a saved
//! kernel continuation; no handler returns to the interrupted instruction.
use crate::{BootInfo, stop};
use core::{
    arch::{asm, naked_asm},
    sync::atomic::{AtomicBool, AtomicPtr, Ordering},
};

#[derive(Clone, Copy)]
#[repr(C, packed)]
struct Gate {
    low: u16,
    selector: u16,
    ist: u8,
    attributes: u8,
    middle: u16,
    high: u32,
    reserved: u32,
}
impl Gate {
    const EMPTY: Self = Self {
        low: 0,
        selector: 0,
        ist: 0,
        attributes: 0,
        middle: 0,
        high: 0,
        reserved: 0,
    };
    fn new(address: usize, ist: u8) -> Self {
        Self {
            low: address as u16,
            selector: 8,
            ist,
            attributes: 0x8e,
            middle: (address >> 16) as u16,
            high: (address >> 32) as u32,
            reserved: 0,
        }
    }
}
const GDT_COUNT: usize = 7;
// First selector beyond the GDT; 0x28 became valid with the Ring 3 data entry.
#[cfg(any(feature = "fault-gp", feature = "fault-df"))]
pub(crate) const FAULT_INVALID_SELECTOR: u16 = (GDT_COUNT * 8) as u16;
#[repr(C, align(16))]
struct Tables {
    gdt: [u64; GDT_COUNT],
    idt: [Gate; 256],
    tss: [u8; 104],
}
static mut TABLES: Tables = Tables {
    gdt: [0; GDT_COUNT],
    idt: [Gate::EMPTY; 256],
    tss: [0; 104],
};
static INFO: AtomicPtr<BootInfo> = AtomicPtr::new(core::ptr::null_mut());
static FAULTING: AtomicBool = AtomicBool::new(false);
#[repr(C, packed)]
struct Descriptor {
    limit: u16,
    base: u64,
}
#[repr(C)]
struct FaultFrame {
    vector: u64,
    error: u64,
    rip: u64,
    cs: u64,
    rflags: u64,
    rsp: u64,
    ss: u64,
}

// Stubs push one synthetic zero only for vectors without hardware error codes.
// Keep the C frame offsets aligned with common's naked assembly argument.
const _: () = {
    assert!(core::mem::offset_of!(FaultFrame, vector) == 0);
    assert!(core::mem::offset_of!(FaultFrame, error) == 8);
    assert!(core::mem::offset_of!(FaultFrame, rip) == 16);
    assert!(core::mem::offset_of!(FaultFrame, cs) == 24);
};

// SAFETY: called once on BSP with interrupts disabled and permanent LoaderData
// mappings. TABLES is never moved/reclaimed. No references to static mut escape.
pub unsafe fn initialize(info: *const BootInfo) {
    unsafe {
        INFO.store(info.cast_mut(), Ordering::Release);
        let table = core::ptr::addr_of_mut!(TABLES);
        let tss = core::ptr::addr_of_mut!((*table).tss).cast::<u8>();
        let boot = &*info;
        for (offset, value) in [
            (4, boot.stack_base + boot.stack_bytes),
            (36, boot.emergency_base + 16384),
            (44, boot.emergency_base + 32768),
            // IST3: #PF, so a runtime stack overflow is reported as #PF with
            // CR2 instead of escalating to #DF while delivering on that stack.
            (52, boot.emergency_base + 49152),
        ] {
            core::ptr::copy_nonoverlapping(
                (value as u64).to_le_bytes().as_ptr(),
                tss.add(offset),
                8,
            );
        }
        core::ptr::copy_nonoverlapping(104u16.to_le_bytes().as_ptr(), tss.add(102), 2);
        let base = tss as u64;
        let descriptor =
            103u64 | ((base & 0xffffff) << 16) | (0x89u64 << 40) | (((base >> 24) & 0xff) << 56);
        core::ptr::write(
            core::ptr::addr_of_mut!((*table).gdt),
            [
                0,
                0x00af9a000000ffff,
                0x00cf92000000ffff,
                descriptor,
                base >> 32,
                0x00cff2000000ffff, // ring 3 data, selector 0x2b
                0x00affa000000ffff, // ring 3 long code, selector 0x33
            ],
        );
        let entries = core::ptr::addr_of_mut!((*table).idt).cast::<Gate>();
        for vector in 0..256 {
            let handler = if vector < 32 { STUBS[vector] } else { unknown };
            let ist = if vector == 8 {
                1
            } else if vector == 2 || matches!(vector, 1 | 18) {
                2
            } else if vector == 14 {
                3
            } else {
                0
            };
            entries
                .add(vector)
                .write(Gate::new(handler as *const () as usize, ist));
        }
        let gdtr = Descriptor {
            limit: (GDT_COUNT * 8 - 1) as u16,
            base: core::ptr::addr_of!((*table).gdt) as u64,
        };
        load_gdt(&gdtr);
        let idtr = Descriptor {
            limit: 4095,
            base: entries as u64,
        };
        asm!("lidt [{}]",in(reg) &idtr,options(readonly,nostack,preserves_flags));
        // Read back hardware registers rather than treating construction as proof.
        let mut actual_gdt = Descriptor { limit: 0, base: 0 };
        let mut actual_idt = Descriptor { limit: 0, base: 0 };
        asm!("sgdt [{}]","sidt [{}]",in(reg) &mut actual_gdt,in(reg) &mut actual_idt,options(nostack,preserves_flags));
        let cs: u16;
        let tr: u16;
        let fs: u16;
        let gs: u16;
        asm!("mov {0:x}, fs", "mov {1:x}, gs", out(reg) fs, out(reg) gs, options(nomem,nostack,preserves_flags));
        asm!("mov {0:x}, cs","str {1:x}",out(reg) cs,out(reg) tr,options(nomem,nostack,preserves_flags));
        if actual_gdt.base != gdtr.base
            || actual_gdt.limit != gdtr.limit
            || actual_idt.base != idtr.base
            || actual_idt.limit != 4095
            || cs != 8
            || tr != 24
            || fs != 16
            || gs != 16
        {
            stop();
        }
        #[cfg(feature = "segment-takeover-probe")]
        crate::debug(b"MUSHA: SEGMENT_TAKEOVER_PROBE OLD_FS_GS=18 NEW=10 OK\n");
        #[cfg(all(feature = "segment-takeover-probe", feature = "i218-phy-probe"))]
        {
            let mut time = crate::acpi::Time::new(boot.timer).unwrap();
            assert!(time.delay_us(0).is_err());
            let before = time.now().unwrap();
            time.delay_us(50_000).unwrap();
            assert!(time.now().unwrap().saturating_sub(before) >= 50);
            crate::debug(b"MUSHA: I218_LONG_DELAY_50000US_OK ZERO_REJECTED\n");
        }
    }
}
#[unsafe(naked)]
unsafe extern "win64" fn load_gdt(_gdtr: *const Descriptor) {
    naked_asm!(
        "lgdt [rcx]",
        "push 8",
        "lea rax, [rip + 2f]",
        "push rax",
        "retfq",
        "2:",
        "mov ax, 16",
        "mov ds, ax",
        "mov es, ax",
        "mov ss, ax",
        // Model firmware's cached data selector0x18 before this slot becomes TSS.
        // Restore the descriptor before LTR; this fixture never enables DMA.
        ".if {inject}",
        "mov r10, [rcx+2]", "mov r11, [r10+24]",
        "mov rax, 0x00cf92000000ffff", "mov [r10+24], rax",
        "mov ax, 24", "mov fs, ax", "mov gs, ax",
        "mov [r10+24], r11",
        "mov ax, fs", "cmp ax, 24", "jne 3f",
        "mov ax, gs", "cmp ax, 24", "je 4f",
        "3:", "ud2", "4:",
        ".endif",
        // Own every data selector when replacing the firmware GDT. Cached
        // firmware FS/GS selectors may now name a TSS or lie outside our table.
        "mov ax, 16", "mov fs, ax", "mov gs, ax",
        "mov ax, 24",
        "ltr ax",
        "ret",
        inject = const cfg!(feature = "segment-takeover-probe") as u8
    );
}
macro_rules! stub {
    ($name:ident,$vector:literal,no_error) => {
        #[unsafe(naked)] extern "win64" fn $name() -> ! {
            naked_asm!("push 0","push {vector}","jmp {common}",vector=const $vector,common=sym common);
        }
    };
    ($name:ident,$vector:literal,error) => {
        #[unsafe(naked)] extern "win64" fn $name() -> ! {
            naked_asm!("push {vector}","jmp {common}",vector=const $vector,common=sym common);
        }
    };
}
stub!(vector_0, 0, no_error);
stub!(vector_1, 1, no_error);
stub!(vector_2, 2, no_error);
stub!(vector_3, 3, no_error);
stub!(vector_4, 4, no_error);
stub!(vector_5, 5, no_error);
stub!(vector_6, 6, no_error);
stub!(vector_7, 7, no_error);
stub!(vector_8, 8, error);
stub!(vector_9, 9, no_error);
stub!(vector_10, 10, error);
stub!(vector_11, 11, error);
stub!(vector_12, 12, error);
stub!(vector_13, 13, error);
stub!(vector_14, 14, error);
stub!(vector_15, 15, no_error);
stub!(vector_16, 16, no_error);
stub!(vector_17, 17, error);
stub!(vector_18, 18, no_error);
stub!(vector_19, 19, no_error);
stub!(vector_20, 20, no_error);
stub!(vector_21, 21, error);
stub!(vector_22, 22, no_error);
stub!(vector_23, 23, no_error);
stub!(vector_24, 24, no_error);
stub!(vector_25, 25, no_error);
stub!(vector_26, 26, no_error);
stub!(vector_27, 27, no_error);
stub!(vector_28, 28, no_error);
stub!(vector_29, 29, error);
stub!(vector_30, 30, error);
stub!(vector_31, 31, no_error);
stub!(unknown, 255, no_error);
const STUBS: [extern "win64" fn() -> !; 32] = [
    vector_0, vector_1, vector_2, vector_3, vector_4, vector_5, vector_6, vector_7, vector_8,
    vector_9, vector_10, vector_11, vector_12, vector_13, vector_14, vector_15, vector_16,
    vector_17, vector_18, vector_19, vector_20, vector_21, vector_22, vector_23, vector_24,
    vector_25, vector_26, vector_27, vector_28, vector_29, vector_30, vector_31,
];
#[unsafe(naked)]
extern "win64" fn common() -> ! {
    naked_asm!("cli","cld","mov rcx, rsp","and rsp, -16","sub rsp, 32",
        ".if {recover}","fninit","mov dword ptr [rsp],0x1f80","ldmxcsr [rsp]",".endif",
        "call {handler}","ud2",handler=sym fatal,recover=const cfg!(any(feature="ring3-lifecycle-probe", external_app)) as usize);
}
/// Bytes at the bottom of the runtime stack never written since entry (the
/// low-water mark). Reads only mapped stack pages.
fn untouched(info: &BootInfo) -> usize {
    let base = info.stack_base + 4096;
    let top = info.stack_base + info.stack_bytes;
    let mut n = 0usize;
    while base + n + 8 <= top
        && unsafe { ((base + n) as *const u64).read_volatile() } == crate::STACK_FILL
    {
        n += 8;
    }
    n
}
/// True when a ring-3 #PF frame lies on the dedicated #PF IST stack.
pub(crate) fn on_page_fault_stack(frame: usize) -> bool {
    let info = INFO.load(Ordering::Acquire);
    if info.is_null() {
        return false;
    }
    let low = unsafe { (*info).emergency_base } + 32768;
    frame >= low && frame.checked_add(56).is_some_and(|end| end <= low + 16384)
}
/// Kernel-mode fatal context: interrupted RSP, runtime stack use and low-water
/// mark, image-relative RIP, and up to eight image addresses found on the
/// stack (return-address candidates). Only mapped stack pages are read; values
/// outside the image are never printed, so no data or key material appears.
unsafe fn stack_report(info: &BootInfo, rip: u64, rsp: u64) {
    let base = info.stack_base + 4096; // first page is the unmapped guard
    let top = info.stack_base + info.stack_bytes;
    let image = info.image_base as u64..(info.image_base + info.image_bytes) as u64;
    let rsp = rsp as usize;
    let untouched = untouched(info);
    let offset = |v: u64| v.wrapping_sub(info.image_base as u64);
    if (info.stack_base..top).contains(&rsp) {
        crate::bootlog::event(
            musha_platform::boot_log::Level::Error,
            "ERROR",
            format_args!(
                "cpu: rip image+0x{:x} rsp 0x{:x} stack used {} of {} bytes, never touched {}",
                offset(rip),
                rsp,
                top - rsp,
                top - base,
                untouched
            ),
        );
    } else {
        crate::bootlog::event(
            musha_platform::boot_log::Level::Error,
            "ERROR",
            format_args!(
                "cpu: rip image+0x{:x} rsp 0x{:x} outside runtime stack; never touched {}",
                offset(rip),
                rsp,
                untouched
            ),
        );
        return;
    }
    let mut found = [0u64; 8];
    let mut count = 0;
    let mut at = rsp.max(base) & !7;
    while at + 8 <= top && count < found.len() && at < rsp.max(base) + 16384 {
        let value = unsafe { (at as *const u64).read_volatile() };
        if image.contains(&value) {
            found[count] = offset(value);
            count += 1;
        }
        at += 8;
    }
    let mut text = [0u8; 8 * 25];
    let mut used = 0;
    for v in &found[..count] {
        for b in b" image+0x" {
            text[used] = *b;
            used += 1;
        }
        let digits = hex(*v);
        let first = digits.iter().position(|d| *d != b'0').unwrap_or(15);
        for d in &digits[first..] {
            text[used] = d.to_ascii_lowercase();
            used += 1;
        }
    }
    crate::bootlog::event(
        musha_platform::boot_log::Level::Error,
        "ERROR",
        format_args!(
            "cpu: stack image addresses{}",
            core::str::from_utf8(&text[..used]).unwrap_or("")
        ),
    );
}
pub(crate) fn hex(value: u64) -> [u8; 16] {
    let mut bytes = [b'0'; 16];
    for (i, ch) in bytes.iter_mut().enumerate() {
        *ch = b"0123456789ABCDEF"[((value >> ((15 - i) * 4)) & 15) as usize];
    }
    bytes
}
extern "win64" fn fatal(frame: *const FaultFrame) -> ! {
    #[cfg(any(feature = "ring3-lifecycle-probe", external_app))]
    unsafe {
        crate::ring3::lifecycle::fault(
            frame as usize,
            (*frame).cs,
            (*frame).vector,
            (*frame).error,
            (*frame).rip,
            (*frame).rsp,
        );
    }
    let _visible = musha_framebuffer::OutputGuard::allow();
    if FAULTING.swap(true, Ordering::AcqRel) {
        stop();
    }
    let info = INFO.load(Ordering::Acquire);
    if info.is_null() {
        stop();
    }
    // SAFETY: the naked stub passes the CPU frame with normalized vector/error;
    // permanent BootInfo was published before installing this IDT. Fatal paths
    // never return, so caller registers need not be restored.
    unsafe {
        let frame = &*frame;
        let ist2 = frame.vector == 2 || matches!(frame.vector, 1 | 18);
        if frame.vector == 8 || ist2 || frame.vector == 14 {
            let rsp: usize;
            asm!("mov {}, rsp",out(reg) rsp,options(nomem,nostack,preserves_flags));
            let low = (*info).emergency_base
                + match frame.vector {
                    8 => 0,
                    14 => 32768,
                    _ => 16384,
                };
            if rsp < low || rsp >= low + 16384 {
                stop();
            }
            crate::debug(b"MUSHA: RING3_FATAL_IST_OK\n");
        }
        {
            crate::ring3::verify_fault_stack(
                frame as *const FaultFrame as usize,
                frame.cs,
                frame.vector,
            );
            crate::debug(b"MUSHA: RING3_FAULT CS=");
            crate::debug(&hex(frame.cs));
            crate::debug(b"\nMUSHA: RING3_FAULT_RSP=");
            crate::debug(&hex(frame.rsp));
            crate::debug(b"\n");
        }
        // #DF commonly follows a #PF whose delivery failed; CR2 then still
        // holds that first fault address, so report it for both vectors.
        let mut cr2 = 0u64;
        if matches!(frame.vector, 8 | 14) {
            asm!("mov {}, cr2",out(reg) cr2,options(nomem,nostack,preserves_flags));
        }
        let name = match frame.vector {
            0 => "#DE",
            1 => "#DB",
            2 => "NMI",
            3 => "#BP",
            4 => "#OF",
            5 => "#BR",
            6 => "#UD",
            7 => "#NM",
            8 => "#DF",
            10 => "#TS",
            11 => "#NP",
            12 => "#SS",
            13 => "#GP",
            14 => "#PF",
            16 => "#MF",
            17 => "#AC",
            18 => "#MC",
            19 => "#XM",
            20 => "#VE",
            21 => "#CP",
            29 => "#VC",
            30 => "#SX",
            _ => "unknown",
        };
        if musha_platform::exceptions::hardware_error(frame.vector) {
            crate::bootlog::event(
                musha_platform::boot_log::Level::Error,
                "ERROR",
                format_args!(
                    "cpu: exception {} (vector {}) error 0x{:x} rip 0x{:x} cr2 0x{:x}",
                    name, frame.vector, frame.error, frame.rip, cr2
                ),
            );
        } else {
            crate::bootlog::event(
                musha_platform::boot_log::Level::Error,
                "ERROR",
                format_args!(
                    "cpu: exception {} (vector {}) error n/a rip 0x{:x} cr2 0x{:x}",
                    name, frame.vector, frame.rip, cr2
                ),
            );
        }
        if frame.cs & 3 == 0 {
            stack_report(&*info, frame.rip, frame.rsp);
        }
        #[cfg(external_app)]
        if frame.cs & 3 == 3 {
            crate::appconsole::fault(frame.vector, frame.error, frame.rip, frame.rsp);
        }
        crate::debug(b"MUSHA: EXCEPTION VECTOR=");
        crate::debug(&hex(frame.vector));
        crate::debug(b" ERROR=");
        crate::debug(&hex(frame.error));
        crate::debug(b" RIP=");
        crate::debug(&hex(frame.rip));
        crate::debug(b" CR2=");
        crate::debug(&hex(cr2));
        crate::debug(b"\n");
    }
    stop()
}

// Deliberately remove the #GP gate, then the caller triggers #GP. A fault while
// delivering this contributory exception must enter #DF on its dedicated IST.
#[cfg(feature = "fault-df")]
pub unsafe fn inject_double_fault() {
    unsafe {
        core::ptr::addr_of_mut!(TABLES.idt[13].attributes).write(0);
    }
}

const _: () = assert!(core::mem::size_of::<Gate>() == 16);
const _: () = assert!(core::mem::size_of::<Descriptor>() == 10);
const _: () = assert!(core::mem::size_of::<FaultFrame>() == 56);

// BSP-only probe: both hardware privilege-change exceptions and explicit
// SYSCALL entry use this supervisor stack. NMI/#DB/#MC use fatal-only IST2.
pub unsafe fn user_kernel_stack(top: usize) {
    unsafe {
        core::ptr::addr_of_mut!(TABLES.tss)
            .cast::<u8>()
            .add(4)
            .cast::<u64>()
            .write_unaligned(top as u64);
    }
}
