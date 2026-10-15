// Copyright 2026 Hirofumi Iwasaki
// SPDX-License-Identifier: Apache-2.0
//! Diagnostic-only resource capture. Never probe, enable decode, or write MMIO.
use super::Controller;
use core::sync::atomic::{AtomicBool, Ordering};
static MAPPED: AtomicBool = AtomicBool::new(false);
static READ: AtomicBool = AtomicBool::new(false);
#[derive(Clone, Copy)]
pub(crate) struct Snapshot {
    pub controller: Controller,
    pub id: u32,
    pub revision: u8,
    pub subsystem: u32,
    pub command: u16,
    pub raw: [u32; 2],
    pub base: [u64; 2],
    pub bytes: [u64; 2],
}
impl Snapshot {
    pub const EMPTY: Self = Self {
        controller: Controller::NONE,
        id: 0,
        revision: 0,
        subsystem: 0,
        command: 0,
        raw: [0; 2],
        base: [0; 2],
        bytes: [0; 2],
    };
}
pub(crate) fn mapped() {
    MAPPED.store(true, Ordering::Relaxed);
}
pub(crate) unsafe fn discover(bs: *mut r_efi::efi::BootServices) -> Snapshot {
    if !cfg!(feature = "t2-diagnostics") {
        return Snapshot::EMPTY;
    }
    use r_efi::{efi, protocols::pci_io};
    let mut guid = pci_io::PROTOCOL_GUID;
    let (mut count, mut handles) = (0, core::ptr::null_mut());
    unsafe {
        if ((*bs).locate_handle_buffer)(
            efi::BY_PROTOCOL,
            &mut guid,
            core::ptr::null_mut(),
            &mut count,
            &mut handles,
        )
        .is_error()
            || handles.is_null()
        {
            return Snapshot::EMPTY;
        }
        let mut found = Snapshot::EMPTY;
        if count <= 1024 {
            for handle in core::slice::from_raw_parts(handles, count) {
                let mut p = core::ptr::null_mut();
                if ((*bs).handle_protocol)(*handle, &mut guid, &mut p).is_error() || p.is_null() {
                    continue;
                }
                let p = p as *mut pci_io::Protocol;
                let mut header = [0u32; 16];
                if ((*p).pci.read)(p, pci_io::WIDTH_UINT32, 0, 16, header.as_mut_ptr().cast())
                    .is_error()
                    || header[0] != 0x816810ec
                    || header[2] >> 8 != 0x020000
                {
                    continue;
                }
                let (mut seg, mut bus, mut dev, mut fun) = (0, 0, 0, 0);
                if ((*p).get_location)(p, &mut seg, &mut bus, &mut dev, &mut fun).is_error()
                    || seg != 0
                    || bus > 255
                    || dev > 31
                    || fun > 7
                {
                    continue;
                }
                found.id = header[0];
                found.revision = header[2] as u8;
                found.command = header[1] as u16;
                found.subsystem = header[11];
                found.controller = Controller {
                    base: 0,
                    bytes: 0,
                    bus: bus as u8,
                    device: dev as u8,
                    function: fun as u8,
                };
                for (i, bar) in [0usize, 2].into_iter().enumerate() {
                    let raw = header[4 + bar];
                    found.raw[i] = raw;
                    found.base[i] = if raw & 1 != 0 {
                        u64::from(raw & !3)
                    } else {
                        u64::from(raw & !15)
                            | if raw & 6 == 4 {
                                u64::from(header[5 + bar]) << 32
                            } else {
                                0
                            }
                    };
                    let mut resource = core::ptr::null_mut();
                    if ((*p).get_bar_attributes)(p, bar as u8, core::ptr::null_mut(), &mut resource)
                        .is_error()
                        || resource.is_null()
                    {
                        continue;
                    }
                    let descriptor = core::slice::from_raw_parts(resource.cast::<u8>(), 48);
                    let bound = musha_platform::bar_resource(descriptor);
                    ((*bs).free_pool)(resource);
                    if let Some((base, bytes)) = bound {
                        if base as u64 == found.base[i] && raw & 1 == 0 && matches!(raw & 6, 0 | 4)
                        {
                            found.bytes[i] = bytes as u64;
                            // Prefer BAR2; firmware-sized page-aligned resource only.
                            if found.command & 2 != 0 {
                                found.controller.base = base;
                                found.controller.bytes = bytes;
                            }
                        }
                    }
                }
                break;
            }
        }
        ((*bs).free_pool)(handles.cast());
        found
    }
}
pub(crate) fn display(s: Snapshot) {
    if !cfg!(feature = "t2-diagnostics") || s.id == 0 {
        return;
    }
    crate::diagnostics::observation(format_args!(
        "RTL BDF {:04X} 10EC:8168 REV {:02X}",
        s.controller.bdf(),
        s.revision
    ));
    crate::diagnostics::observation(format_args!(
        "RTL SUB {:04X}:{:04X} CMD {:04X}",
        s.subsystem as u16,
        s.subsystem >> 16,
        s.command
    ));
    for (i, bar) in [0, 2].into_iter().enumerate() {
        let kind = if s.raw[i] & 1 != 0 {
            "IO"
        } else if s.raw[i] & 6 == 4 {
            "MEM64"
        } else {
            "MEM32"
        };
        crate::diagnostics::observation(format_args!(
            "RTL BAR{} {} BASE {:X}",
            bar, kind, s.base[i]
        ));
        crate::diagnostics::observation(format_args!(
            "RTL BAR{} LEN {:X} RAW {:08X}",
            bar, s.bytes[i], s.raw[i]
        ));
    }
    let stable = unsafe {
        super::read(
            s.controller.bus,
            s.controller.device,
            s.controller.function,
            0,
        ) == s.id
            && super::read(
                s.controller.bus,
                s.controller.device,
                s.controller.function,
                8,
            ) as u8
                == s.revision
            && [0u8, 2].into_iter().enumerate().all(|(i, bar)| {
                let raw = super::read(
                    s.controller.bus,
                    s.controller.device,
                    s.controller.function,
                    0x10 + bar * 4,
                );
                let high = if raw & 1 == 0 && raw & 6 == 4 {
                    super::read(
                        s.controller.bus,
                        s.controller.device,
                        s.controller.function,
                        0x14 + bar * 4,
                    ) as u64
                } else {
                    0
                };
                let base = if raw & 1 != 0 {
                    (raw & !3) as u64
                } else {
                    (raw & !15) as u64 | high << 32
                };
                raw == s.raw[i] && base == s.base[i]
            })
    };
    let command = unsafe {
        super::read(
            s.controller.bus,
            s.controller.device,
            s.controller.function,
            4,
        )
    };
    let address = musha_net::realtek::tx_config_address(
        s.id,
        command,
        MAPPED.load(Ordering::Relaxed),
        s.controller.base,
        s.controller.bytes,
    );
    if !stable || address.is_none() {
        crate::diagnostics::observation(format_args!("RTL TXCONFIG SKIP RESOURCE/UC/DECODE"));
        return;
    }
    if READ.swap(true, Ordering::Relaxed) {
        return;
    }
    // Sole volatile read; memory::initialize validated and mapped this exact
    // firmware range UC, excluding RAM/reserved/other MMIO. Never enables BME.
    let value = unsafe { (address.unwrap() as *const u32).read_volatile() };
    crate::diagnostics::observation(format_args!("RTL TXCONFIG40 {:08X} READONLY ONCE", value));
    crate::debug(b"MUSHA: RTL_TXCONFIG40=");
    crate::debug(&crate::cpu::hex(value as u64));
    crate::debug(b" READONLY\n");
}
