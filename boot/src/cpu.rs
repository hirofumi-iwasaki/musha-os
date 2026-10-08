// Copyright 2026 Hirofumi Iwasaki
// SPDX-License-Identifier: Apache-2.0
//! Fatal-only x86-64 exception setup. No handler returns to interrupted code.
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
#[repr(C, align(16))]
struct Tables {
    gdt: [u64; 5],
    idt: [Gate; 256],
    tss: [u8; 104],
}
static mut TABLES: Tables = Tables {
    gdt: [0; 5],
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
            ],
        );
        let entries = core::ptr::addr_of_mut!((*table).idt).cast::<Gate>();
        for vector in 0..256 {
            let handler = if vector < 32 { STUBS[vector] } else { unknown };
            let ist = if vector == 8 {
                1
            } else if vector == 2 {
                2
            } else {
                0
            };
            entries
                .add(vector)
                .write(Gate::new(handler as *const () as usize, ist));
        }
        let gdtr = Descriptor {
            limit: 39,
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
        asm!("mov {0:x}, cs","str {1:x}",out(reg) cs,out(reg) tr,options(nomem,nostack,preserves_flags));
        if actual_gdt.base != gdtr.base
            || actual_gdt.limit != 39
            || actual_idt.base != idtr.base
            || actual_idt.limit != 4095
            || cs != 8
            || tr != 24
        {
            stop();
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
        "mov ax, 24",
        "ltr ax",
        "ret"
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
        "call {handler}","ud2",handler=sym fatal);
}
pub(crate) fn hex(value: u64) -> [u8; 16] {
    let mut bytes = [b'0'; 16];
    for (i, ch) in bytes.iter_mut().enumerate() {
        *ch = b"0123456789ABCDEF"[((value >> ((15 - i) * 4)) & 15) as usize];
    }
    bytes
}
extern "win64" fn fatal(frame: *const FaultFrame) -> ! {
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
        if frame.vector == 8 || frame.vector == 2 {
            let rsp: usize;
            asm!("mov {}, rsp",out(reg) rsp,options(nomem,nostack,preserves_flags));
            let low = (*info).emergency_base + if frame.vector == 2 { 16384 } else { 0 };
            if rsp < low || rsp >= low + 16384 {
                stop();
            }
        }
        let fb = (*info).framebuffer;
        let mut cr2 = 0u64;
        if frame.vector == 14 {
            asm!("mov {}, cr2",out(reg) cr2,options(nomem,nostack,preserves_flags));
        }
        for y in 100..260 {
            for x in 0..fb.width.min(600) {
                fb.pixel(x, y, fb.color(32, 0, 0));
            }
        }
        fb.text("CPU EXCEPTION", 24, 104, fb.color(255, 80, 80));
        for (i, (label, value)) in [
            ("VECTOR", frame.vector),
            ("ERROR", frame.error),
            ("RIP", frame.rip),
            ("CR2", cr2),
        ]
        .iter()
        .enumerate()
        {
            fb.text(label, 24, 136 + i * 28, fb.color(255, 220, 220));
            let bytes = hex(*value);
            // SAFETY: hex constructs only ASCII characters.
            fb.text(
                core::str::from_utf8_unchecked(&bytes),
                160,
                136 + i * 28,
                fb.color(255, 220, 220),
            );
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
