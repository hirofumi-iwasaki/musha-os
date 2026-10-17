// Copyright 2026 Hirofumi Iwasaki
// SPDX-License-Identifier: Apache-2.0
#![no_std]
#![no_main]

#[cfg(feature = "display-grid-probe")]
fn grid_hold(stage: &str) {
    if option_env!("MUSHA_GRID_HOLD") == Some(stage) {
        loop {
            core::hint::spin_loop();
        }
    }
}

mod acpi;
mod app;
#[cfg(external_app)]
mod appconsole;
mod bluetooth;
mod bootlog;
mod bootstate;
mod cpu;
mod diagnostics;
mod entropy;
mod settings;
mod marker {
    include!(concat!(env!("OUT_DIR"), "/marker.rs"));
}
#[cfg(external_app)]
mod external;
mod graphics;
mod memory;
mod net;
mod pci;
mod ring3;
mod runtime;
#[cfg(feature = "t2-diagnostics")]
mod t2;
mod timing;
mod xhci;

const _: () = assert!(
    (cfg!(feature = "fault-ud") as usize)
        + (cfg!(feature = "fault-gp") as usize)
        + (cfg!(feature = "fault-df") as usize)
        + (cfg!(feature = "fault-pf") as usize)
        + (cfg!(feature = "fault-ro") as usize)
        + (cfg!(feature = "fault-nx") as usize)
        + (cfg!(feature = "fault-guard") as usize)
        + (cfg!(feature = "xhci-timeout") as usize)
        + (cfg!(feature = "xhci-command-timeout") as usize)
        + (cfg!(feature = "usb-descriptor-timeout") as usize)
        + (cfg!(feature = "storage-timeout") as usize)
        <= 1,
    "Select only one injected fault"
);

use core::{
    arch::{asm, naked_asm},
    panic::PanicInfo,
};
use musha_framebuffer::Framebuffer;
use r_efi::{
    efi,
    protocols::{graphics_output as gop, loaded_image},
};

// Diagnostic formatting and GPT file reads share this guarded runtime stack.
const STACK_PAGES: usize = 32;
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
    image_base: usize,
    image_bytes: usize,
    table_base: usize,
    table_bytes: usize,
    timer: musha_platform::Timer,
    xhci: pci::Controller,
    xhcis: pci::Controllers,
    pci_early: [pci::Snapshot; pci::MAX_CONTROLLERS],
    pci_pre_exit: [pci::Snapshot; pci::MAX_CONTROLLERS],
    pci_post_exit: [pci::Snapshot; pci::MAX_CONTROLLERS],
    boot_path: pci::BootPath,
    boot_owner: u16,
    pci_resource_safe: [pci::ResourceEvidence; pci::MAX_CONTROLLERS],
    nic: pci::Controller,
    nic_flash: pci::Controller,
    realtek_diag: pci::realtek::Snapshot,
    net_dma_base: usize,
    net_dma_bytes: usize,
    dma_base: usize,
    dma_bytes: usize,
}
const _: () = assert!(core::mem::size_of::<BootInfo>() <= 4096);
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    bootlog::failure("kernel", "panic");
    stop()
}
fn stop() -> ! {
    bootlog::halted();
    bootlog::dump();
    musha_framebuffer::flush_writes();
    #[cfg(elf_app)]
    ring3::io::shutdown();
    loop {
        // SAFETY: runs only on the boot CPU at firmware/kernel privilege.
        unsafe {
            asm!("cli", "hlt", options(nomem, nostack));
        }
    }
}

#[cfg(feature = "diagnostics-detail")]
fn diagnostic_capture(_info: &BootInfo) -> ! {
    diagnostics::snapshot();
    stop()
}

// Explicit Win64 ABI matches the UEFI target. RCX = info, RDX = stack top.
// No Rust prologue may execute after replacing RSP. Allocate shadow space and
// maintain 16-byte call alignment. runtime never returns to the old stack.
#[unsafe(naked)]
unsafe extern "win64" fn enter_runtime(_info: *const BootInfo, _stack_top: usize) -> ! {
    naked_asm!("cli", "cld", "mov rsp, rdx", "and rsp, -16", "sub rsp, 32",
        "call {entry}", "ud2", entry = sym runtime);
}
#[cfg_attr(feature = "ring3-probe", allow(unreachable_code))]
extern "win64" fn runtime(info: *const BootInfo) -> ! {
    // SAFETY: handoff is in dedicated LOADER_DATA pages, not the abandoned
    // firmware stack. Its framebuffer mapping remains firmware identity-mapped.
    let info = unsafe { &*info };
    if info.magic != 0x4d55534841424f4f || info.version != 1 {
        stop();
    }
    bootlog::firmware_exited();
    bootlog::detail("core", format_args!("ExitBootServices complete"));
    bootlog::detail("core", format_args!("paging and arena initialization"));
    diagnostics::start(info.framebuffer);
    #[cfg(external_app)]
    appconsole::prepare(info.framebuffer);
    diagnostics::set(
        40,
        format_args!(
            "GOP WIDTH/HEIGHT/STRIDE {}/{}/{}",
            info.framebuffer.width, info.framebuffer.height, info.framebuffer.stride
        ),
    );
    diagnostics::observation(format_args!(
        "GOP WIDTH/HEIGHT/STRIDE {}/{}/{}",
        info.framebuffer.width, info.framebuffer.height, info.framebuffer.stride
    ));
    #[cfg(feature = "t2-diagnostics")]
    t2::report();
    pci::report_snapshots(
        &info.xhcis,
        &[
            ("EARLY", &info.pci_early),
            ("PRE EBS", &info.pci_pre_exit),
            ("POST EBS", &info.pci_post_exit),
        ],
    );
    pci::report_boot_path(&info.boot_path);
    diagnostics::observation(format_args!(
        "BOOT XHCI BDF {:04X} FFFF UNKNOWN FFFE MULTIPLE",
        info.boot_owner
    ));
    diagnostics::set(1, format_args!("STATE CPU TABLES"));
    // Stop the supported NIC before constructing the arena or new page tables.
    if info.nic.base != 0 {
        unsafe {
            pci::disable_dma(info.nic);
        }
        if unsafe { pci::read(info.nic.bus, info.nic.device, info.nic.function, 4) } & 4 != 0 {
            stop();
        }
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
    let _ = acpi::Time::new(info.timer).and_then(|mut t| t.now());
    diagnostics::set(1, format_args!("STATE PAGING / ARENA"));
    let arena = match unsafe { memory::initialize(info) } {
        Ok(arena) => arena,
        Err(error) => {
            let (label, message): (&str, &[u8]) = match error {
                musha_memory::Error::Invalid => ("MEMORY INVALID", b"MUSHA: MEMORY_INVALID\n"),
                musha_memory::Error::Overflow => ("MEMORY OVERFLOW", b"MUSHA: MEMORY_OVERFLOW\n"),
                musha_memory::Error::Overlap => ("MEMORY OVERLAP", b"MUSHA: MEMORY_OVERLAP\n"),
                musha_memory::Error::NoMemory => ("MEMORY LOW", b"MUSHA: MEMORY_LOW\n"),
                musha_memory::Error::Unsupported => {
                    ("MEMORY UNSUPPORTED", b"MUSHA: MEMORY_UNSUPPORTED\n")
                }
            };
            diagnostics::set(1, format_args!("STATE {}", label));
            diagnostics::set(17, format_args!("MEMORY RESULT {:?}", error));
            diagnostics::failure("CORE MEMORY", label);
            diagnostics::snapshot();
            debug(message);

            stop();
        }
    };
    bootlog::phase("core", "runtime ready");
    bootlog::device(
        "memory",
        format_args!(
            "image=0x{:x}+{} arena=0x{:x}+{}",
            info.image_base,
            info.image_bytes,
            arena.as_ptr() as usize,
            arena.len()
        ),
    );
    let _ = acpi::Time::new(info.timer).and_then(|mut t| t.now());
    bootstate::update(|r, t| {
        r.core(t);
    });
    debug(b"MUSHA: CORE_READY\n");
    diagnostics::flush();
    diagnostics::set(1, format_args!("STATE PCI / APPLICATION"));
    if !acpi::diagnose(info.timer, info.framebuffer) {
        diagnostics::failure("REQUIRED CLOCK", "UNAVAILABLE OR STALLED");
        diagnostics::snapshot();
        debug(b"MUSHA: BOOT_FAILED REQUIRED_CLOCK\n");
        bootstate::update(|r, _| r.phase = musha_platform::readiness::Phase::BootFailed);
        stop();
    }
    bootlog::phase("timer", "PM clock available");
    #[cfg(feature = "startup-failure-probe")]
    {
        bootlog::failure("clock", "forced test failure");
        stop();
    }
    #[cfg(feature = "ring3-lifecycle-probe")]
    ring3::lifecycle::run(arena, info);
    #[cfg(all(
        feature = "ring3-probe",
        not(any(feature = "ring3-lifecycle-probe", feature = "external-elf-probe"))
    ))]
    ring3::run(arena, info);
    pci::diagnose(info.framebuffer, info.xhci);
    pci::realtek::display(info.realtek_diag);
    #[cfg(feature = "display-grid-probe")]
    {
        diagnostics::flush();
        grid_hold("diagnostics");
    }
    let arena_base = arena.as_mut_ptr() as usize;
    // Diagnostic app owns the sole mutable arena borrow and exercises every page.
    if let Err(error) = runtime::run(arena, info) {
        #[cfg(external_app)]
        if external::claimed() {
            // Never enter/reuse the arena after an unconfirmed device shutdown.
            bootlog::failure("usb", "device session did not complete; reuse forbidden");
            appconsole::rejected(b"Device session did not complete");
            debug(b"MUSHA: EXTERNAL_ELF_REJECT Device session did not complete\n");
            #[cfg(feature = "diagnostics-detail")]
            diagnostic_capture(info);
            #[cfg(not(feature = "diagnostics-detail"))]
            stop();
        }
        if error == musha_api::Error::NotFound {
            diagnostics::failure("storage", "no readable boot media found");
        } else {
            diagnostics::failure("app", "execution failed");
        }
        diagnostics::set(1, format_args!("STATE APP FAILED"));
        diagnostics::snapshot();
        debug(b"MUSHA: APP_FAILED\n");
        #[cfg(feature = "diagnostics-detail")]
        diagnostic_capture(info);

        #[cfg(not(feature = "diagnostics-detail"))]
        stop();
    }
    #[cfg(all(external_app, feature = "diagnostics-detail"))]
    if !external::claimed() {
        diagnostics::set(
            1,
            format_args!("diagnostic session complete / no external app"),
        );
        diagnostics::snapshot();
        debug(b"MUSHA: DIAGNOSTIC_CAPTURE_STOP\n");
        diagnostic_capture(info);
    }
    #[cfg(external_app)]
    external::finish(arena, info);
    #[cfg(feature = "fault-pf")]
    unsafe {
        asm!("xor rax, rax","mov byte ptr [rax], 1",out("rax") _,options(nostack));
    }
    #[cfg(feature = "fault-guard")]
    unsafe {
        asm!("mov byte ptr [{address}], 1",address=in(reg) info.stack_base,options(nostack));
    }
    #[cfg(feature = "fault-ro")]
    unsafe {
        asm!("mov byte ptr [{address}], 1",address=in(reg) runtime as *const () as usize,options(nostack));
    }
    #[cfg(feature = "fault-nx")]
    unsafe {
        asm!("call rax",in("rax") arena_base,clobber_abi("win64"));
    }
    #[cfg(not(feature = "fault-nx"))]
    let _ = arena_base;
    diagnostics::set(1, format_args!("STATE SESSION COMPLETE"));
    diagnostics::snapshot();
    debug(b"MUSHA: EXIT_BOOT_SERVICES_OK STACK_OK GOP_OK CPU_TABLES_OK PAGING_OK ARENA_OK\n");
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
        asm!("mov ax, {selector}", "mov ds, ax", selector = const cpu::FAULT_INVALID_SELECTOR, out("ax") _, options(nostack));
    }

    diagnostics::flush();
    #[cfg(feature = "diagnostics-detail")]
    diagnostic_capture(info);
    #[cfg(all(not(feature = "fault-ud"), not(feature = "diagnostics-detail")))]
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
        let mut image_protocol = core::ptr::null_mut();
        let mut image_guid = loaded_image::PROTOCOL_GUID;
        let status = ((*bs).handle_protocol)(image, &mut image_guid, &mut image_protocol);
        if status.is_error() {
            return status;
        }
        if image_protocol.is_null() {
            return efi::Status::UNSUPPORTED;
        }
        let loaded = &*(image_protocol as *const loaded_image::Protocol);
        let settings = settings::read(bs, system_table, loaded.device_handle);
        bootlog::configure(settings.config);
        entropy::initialize(bs);
        let mut protocol = core::ptr::null_mut();
        let mut guid = gop::PROTOCOL_GUID;
        let status = ((*bs).locate_protocol)(&mut guid, core::ptr::null_mut(), &mut protocol);
        if status.is_error() {
            return status;
        }
        if protocol.is_null() {
            return efi::Status::UNSUPPORTED;
        }
        let fallback =
            graphics::select(protocol as *mut gop::Protocol, bs, settings.config.display);
        // SetMode may replace Mode.Info and the framebuffer; capture only afterward.
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
        if !fb.valid() || fb.width < 320 || fb.height < 356 {
            return efi::Status::UNSUPPORTED;
        }
        #[cfg(feature = "startup-config-probe")]
        {
            debug(b"MUSHA: CONFIG FRAMEBUFFER CAPTURED\n");
            ((*bs).stall)(1_000_000);
        }
        bootlog::initialize(fb, settings, fallback);
        entropy::publish();
        #[cfg(feature = "display-grid-probe")]
        grid_hold("starting");

        let image_base = loaded.image_base as usize;
        let image_bytes = loaded.image_size as usize;
        // CPU-owned zero-fill staging counts toward LoadedImage.ImageSize.
        let image_limit = 32 * 1024 * 1024;
        if image_base == 0 || image_bytes == 0 || image_bytes > image_limit {
            return efi::Status::UNSUPPORTED;
        }
        #[cfg(feature = "t2-diagnostics")]
        t2::discover();
        let xhcis = pci::discover_all(bs, false);
        let pci_early = pci::snapshots(&xhcis);
        let boot_path = pci::boot_path(bs, loaded.device_handle);
        let boot_owner = pci::boot_owner(bs, &xhcis, &boot_path);
        let pci_resource_safe = pci::resource_safety(bs, &xhcis);
        let xhci = xhcis.entries[0];
        let nic = pci::discover(bs, true);
        let realtek_diag = pci::realtek::discover(bs);
        #[cfg(feature = "i218-network")]
        let nic_flash = pci::nic_flash(bs, nic);
        #[cfg(not(feature = "i218-network"))]
        let nic_flash = pci::Controller::NONE;
        let mut bases = [0u64; 7];
        bases[5] = 0xffff_ffff;
        bases[6] = 0xffff_ffff;
        let page_counts = [
            STACK_PAGES,
            MAP_PAGES,
            1,
            8,
            256,
            256 * xhcis.count.max(1),
            16,
        ];
        for i in 0..7 {
            let status = ((*bs).allocate_pages)(
                if i >= 5 {
                    efi::ALLOCATE_MAX_ADDRESS
                } else {
                    efi::ALLOCATE_ANY_PAGES
                },
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
        let [stack, map, handoff, emergency, tables, dma, net_dma] = bases;
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
                image_base,
                image_bytes,
                table_base: tables as usize,
                table_bytes: 256 * 4096,
                timer: musha_platform::Timer { port: 0, bits: 24 },
                xhci,
                xhcis,
                pci_early,
                pci_pre_exit: [pci::Snapshot::EMPTY; pci::MAX_CONTROLLERS],
                pci_post_exit: [pci::Snapshot::EMPTY; pci::MAX_CONTROLLERS],
                boot_path,
                boot_owner,
                pci_resource_safe,
                nic,
                nic_flash,
                realtek_diag,
                net_dma_base: net_dma as usize,
                net_dma_bytes: 16 * 4096,
                dma_base: dma as usize,
                dma_bytes: page_counts[5] * 4096,
            },
        );
        // ACPI is validated while firmware mappings still exist. Copy only the
        // fixed timer contract; no original table pointer enters the runtime.
        let mut initial_size = MAP_PAGES * 4096;
        let mut initial_key = 0;
        let mut initial_stride = 0;
        let mut initial_version = 0;
        let initial_status = ((*bs).get_memory_map)(
            &mut initial_size,
            map as *mut efi::MemoryDescriptor,
            &mut initial_key,
            &mut initial_stride,
            &mut initial_version,
        );
        if !initial_status.is_error() {
            let raw = core::slice::from_raw_parts(map as *const u8, initial_size);
            if let Ok(snapshot) = musha_memory::MemoryMap::new(raw, initial_stride, initial_version)
            {
                #[cfg(feature = "t2-diagnostics")]
                t2::model(system_table, &snapshot);
                if let Some(timer) = acpi::discover(system_table, &snapshot) {
                    (*info).timer = timer;
                }
            }
        }
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
                bootlog::failure("uefi", "memory map unavailable");
                stop();
            }
            (*info).map_size = size;
            (*info).descriptor_size = stride;
            (*info).descriptor_version = version;
            (*info).pci_pre_exit = pci::snapshots(&(*info).xhcis);
            #[cfg(feature = "t2-diagnostics")]
            t2::capture(1);
            let status = ((*bs).exit_boot_services)(image, key);
            if status == efi::Status::SUCCESS {
                (*info).pci_post_exit = pci::snapshots(&(*info).xhcis);
                #[cfg(feature = "t2-diagnostics")]
                t2::capture(2);
                enter_runtime(info, stack as usize + STACK_PAGES * 4096);
            }
            if status != efi::Status::INVALID_PARAMETER {
                break;
            }
        }
        bootlog::failure("uefi", "ExitBootServices failed");
        stop();
    }
}
