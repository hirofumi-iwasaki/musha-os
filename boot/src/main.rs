// Copyright 2026 Hirofumi Iwasaki
// SPDX-License-Identifier: Apache-2.0
#![no_std]
#![no_main]

mod cpu;

#[cfg(any(
    all(feature = "fault-ud", feature = "fault-gp"),
    all(feature = "fault-ud", feature = "fault-df"),
    all(feature = "fault-gp", feature = "fault-df")
))]
compile_error!("Select only one injected fault");

use core::{
    arch::{asm, naked_asm},
    panic::PanicInfo,
};
use musha_framebuffer::Framebuffer;
use r_efi::{efi, protocols::graphics_output as gop};

const STACK_PAGES: usize = 16;
const MAP_PAGES: usize = 32;
#[repr(C)]
struct BootInfo {
    magic: u64,
    version: u32,
    size: u32,
    framebuffer: Framebuffer,
    map_base: usize,
    map_size: usize,
    descriptor_size: usize,
    descriptor_version: u32,
    stack_base: usize,
    stack_bytes: usize,
    emergency_base: usize,
}
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    stop()
}
fn stop() -> ! {
    loop {
        // SAFETY: runs only on the boot CPU at firmware/kernel privilege.
        unsafe {
            asm!("cli", "hlt", options(nomem, nostack));
        }
    }
}

// Explicit Win64 ABI matches the UEFI target. RCX = info, RDX = stack top.
// No Rust prologue may execute after replacing RSP. Allocate shadow space and
// maintain 16-byte call alignment. runtime never returns to the old stack.
#[unsafe(naked)]
unsafe extern "win64" fn enter_runtime(_info: *const BootInfo, _stack_top: usize) -> ! {
    naked_asm!("cli", "cld", "mov rsp, rdx", "and rsp, -16", "sub rsp, 32",
        "call {entry}", "ud2", entry = sym runtime);
}
extern "win64" fn runtime(info: *const BootInfo) -> ! {
    // SAFETY: handoff is in dedicated LOADER_DATA pages, not the abandoned
    // firmware stack. Its framebuffer mapping remains firmware identity-mapped.
    let info = unsafe { &*info };
    if info.magic != 0x4d55534841424f4f || info.version != 1 {
        stop();
    }
    let rsp: usize;
    unsafe {
        asm!("mov {}, rsp", out(reg) rsp, options(nomem, nostack, preserves_flags));
    }
    if rsp < info.stack_base || rsp >= info.stack_base + info.stack_bytes {
        stop();
    }
    unsafe {
        cpu::initialize(info);
    }
    unsafe {
        for y in 0..100 {
            for x in 0..info.framebuffer.width.min(600) {
                info.framebuffer
                    .pixel(x, y, info.framebuffer.color(12, 20, 32));
            }
        }
        info.framebuffer.text(
            "Hello Musha-OS!",
            24,
            24,
            info.framebuffer.color(240, 240, 240),
        );
        info.framebuffer
            .text("RUNTIME READY", 24, 64, info.framebuffer.color(0, 240, 100));
    }
    debug(b"MUSHA: EXIT_BOOT_SERVICES_OK STACK_OK GOP_OK CPU_TABLES_OK\n");
    #[cfg(feature = "fault-ud")]
    unsafe {
        asm!("ud2", options(noreturn));
    }
    #[cfg(feature = "fault-df")]
    unsafe {
        cpu::inject_double_fault();
    }
    #[cfg(any(feature = "fault-gp", feature = "fault-df"))]
    unsafe {
        asm!("mov ax, 40","mov ds, ax",out("ax") _,options(nostack));
    }

    #[cfg(not(feature = "fault-ud"))]
    stop()
}

fn debug(bytes: &[u8]) {
    #[cfg(feature = "qemu-debug")]
    for byte in bytes {
        // SAFETY: only enabled for QEMU tests; no firmware services involved.
        unsafe {
            asm!("out dx, al",in("dx") 0xe9u16,in("al") *byte,options(nomem,nostack));
        }
    }
    #[cfg(not(feature = "qemu-debug"))]
    let _ = bytes;
}

#[unsafe(no_mangle)]
pub extern "efiapi" fn efi_main(
    image: efi::Handle,
    system_table: *mut efi::SystemTable,
) -> efi::Status {
    if system_table.is_null() {
        return efi::Status::INVALID_PARAMETER;
    }
    // SAFETY: firmware supplies valid protocol tables during Boot Services.
    // Each retrieved pointer and framebuffer layout is checked before use.
    unsafe {
        let bs = (*system_table).boot_services;
        if bs.is_null() {
            return efi::Status::UNSUPPORTED;
        }
        let mut protocol = core::ptr::null_mut();
        let mut guid = gop::PROTOCOL_GUID;
        let status = ((*bs).locate_protocol)(&mut guid, core::ptr::null_mut(), &mut protocol);
        if status.is_error() {
            return status;
        }
        if protocol.is_null() {
            return efi::Status::UNSUPPORTED;
        }
        let mode = (*(protocol as *const gop::Protocol)).mode;
        if mode.is_null()
            || (*mode).info.is_null()
            || (*mode).size_of_info < core::mem::size_of::<gop::ModeInformation>()
        {
            return efi::Status::UNSUPPORTED;
        }
        let m = &*(*mode).info;
        let fb = Framebuffer {
            base: (*mode).frame_buffer_base as usize,
            bytes: (*mode).frame_buffer_size,
            width: m.horizontal_resolution as usize,
            height: m.vertical_resolution as usize,
            stride: m.pixels_per_scan_line as usize,
            format: m.pixel_format,
        };
        // Bitmask and BLT-only modes are explicitly unsupported in this milestone.
        if !fb.valid() || fb.width < 320 || fb.height < 260 {
            return efi::Status::UNSUPPORTED;
        }
        fb.text("Hello Musha-OS!", 24, 24, fb.color(240, 240, 240));
        let mut bases = [0u64; 4];
        let page_counts = [STACK_PAGES, MAP_PAGES, 1, 8];
        for i in 0..4 {
            let status = ((*bs).allocate_pages)(
                efi::ALLOCATE_ANY_PAGES,
                efi::LOADER_DATA,
                page_counts[i],
                &mut bases[i],
            );
            if status.is_error() {
                for j in 0..i {
                    ((*bs).free_pages)(bases[j], page_counts[j]);
                }
                return status;
            }
        }
        let [stack, map, handoff, emergency] = bases;
        let info = handoff as *mut BootInfo;
        core::ptr::write(
            info,
            BootInfo {
                magic: 0x4d55534841424f4f,
                version: 1,
                size: core::mem::size_of::<BootInfo>() as u32,
                framebuffer: fb,
                map_base: map as usize,
                map_size: 0,
                descriptor_size: 0,
                descriptor_version: 0,
                stack_base: stack as usize,
                stack_bytes: STACK_PAGES * 4096,
                emergency_base: emergency as usize,
            },
        );
        // Final map and ExitBootServices are adjacent: no allocation, logging or
        // protocol calls in between. Retry only INVALID_PARAMETER (stale key).
        for _ in 0..3 {
            let mut size = MAP_PAGES * 4096;
            let mut key = 0;
            let mut stride = 0;
            let mut version = 0;
            let status = ((*bs).get_memory_map)(
                &mut size,
                map as *mut efi::MemoryDescriptor,
                &mut key,
                &mut stride,
                &mut version,
            );
            if status.is_error()
                || stride < core::mem::size_of::<efi::MemoryDescriptor>()
                || size % stride != 0
            {
                // No returning to firmware after a failed ExitBootServices attempt.
                fb.text("BOOT FAILED", 24, 64, fb.color(255, 0, 0));
                stop();
            }
            (*info).map_size = size;
            (*info).descriptor_size = stride;
            (*info).descriptor_version = version;
            let status = ((*bs).exit_boot_services)(image, key);
            if status == efi::Status::SUCCESS {
                enter_runtime(info, stack as usize + STACK_PAGES * 4096);
            }
            if status != efi::Status::INVALID_PARAMETER {
                break;
            }
        }
        fb.text("BOOT FAILED", 24, 64, fb.color(255, 0, 0));
        stop();
    }
}
