// Copyright 2026 Hirofumi Iwasaki
// SPDX-License-Identifier: Apache-2.0
//! PCI configuration mechanism 1: segment zero, firmware-configured buses.
use core::arch::asm;
use musha_framebuffer::Framebuffer;

// The boot CPU is the sole caller; interrupts stay disabled. CF8/CFC transactions
// cannot interleave. Only the configuration ADDRESS port is written; device
// registers, BAR sizing, bus numbering and bus mastering are never modified.
pub(crate) unsafe fn read(bus: u8, device: u8, function: u8, register: u8) -> u32 {
    let address = 0x8000_0000u32
        | ((bus as u32) << 16)
        | ((device as u32) << 11)
        | ((function as u32) << 8)
        | (register as u32 & 0xfc);
    let value: u32;
    unsafe {
        asm!("out dx, eax", in("dx") 0xcf8u16, in("eax") address, options(nomem, nostack));
        asm!("in eax, dx", in("dx") 0xcfcu16, out("eax") value, options(nomem, nostack));
    }
    value
}
fn field(label: &[u8], value: u64) {
    super::debug(label);
    super::debug(&super::cpu::hex(value));
}
pub(crate) fn diagnose(fb: Framebuffer) {
    let mut count = 0u64;
    let mut xhci = 0u64;
    let mut intel_lan = 0u64;
    for bus in 0..=255u8 {
        for device in 0..32u8 {
            // PCI functions are present only if function zero exists.
            let id = unsafe { read(bus, device, 0, 0) };
            if id as u16 == 0xffff {
                continue;
            }
            let header = unsafe { read(bus, device, 0, 0x0c) };
            let functions = if header & 0x0080_0000 != 0 { 8 } else { 1 };
            for function in 0..functions {
                let id = unsafe { read(bus, device, function, 0) };
                if id as u16 == 0xffff {
                    continue;
                }
                let class = unsafe { read(bus, device, function, 8) } >> 8;
                count += 1;
                if class == 0x0c0330 {
                    xhci += 1;
                }
                if class >> 8 == 0x0200 && id as u16 == 0x8086 {
                    intel_lan += 1;
                }
                field(
                    b"MUSHA: PCI BDF=",
                    ((bus as u64) << 8) | ((device as u64) << 3) | function as u64,
                );
                field(b" ID=", id as u64);
                field(b" CLASS=", class as u64);
                super::debug(b"\n");
            }
        }
    }
    field(b"MUSHA: PCI_COUNT=", count);
    field(b" XHCI=", xhci);
    field(b" INTEL_LAN=", intel_lan);
    super::debug(b"\nMUSHA: PCI_ENUMERATION_OK\n");
    // Discovery is diagnostic: zero devices in a class is reported, not treated
    // as proof of a working driver or as a reason to stop unrelated hardware.
    unsafe {
        fb.text("PCI DEVICES", 24, 164, fb.color(0, 220, 240));
        let digits = super::cpu::hex(count);
        fb.text(
            core::str::from_utf8_unchecked(&digits),
            240,
            164,
            fb.color(0, 220, 240),
        );
        fb.text("XHCI", 24, 196, fb.color(0, 220, 240));
        let digits = super::cpu::hex(xhci);
        fb.text(
            core::str::from_utf8_unchecked(&digits),
            240,
            196,
            fb.color(0, 220, 240),
        );
        fb.text("INTEL LAN", 24, 228, fb.color(0, 220, 240));
        let digits = super::cpu::hex(intel_lan);
        fb.text(
            core::str::from_utf8_unchecked(&digits),
            240,
            228,
            fb.color(0, 220, 240),
        );
    }
}

#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct Controller {
    pub base: usize,
    pub bytes: usize,
    pub bus: u8,
    pub device: u8,
    pub function: u8,
}
impl Controller {
    pub const NONE: Self = Self {
        base: 0,
        bytes: 0,
        bus: 0,
        device: 0,
        function: 0,
    };
}
// EFI protocol/resource pointers are trusted firmware allocations, used and
// freed while Boot Services is live. Copy only scalar BAR and BDF information.
pub(crate) unsafe fn discover_xhci(bs: *mut r_efi::efi::BootServices) -> Controller {
    use r_efi::{efi, protocols::pci_io};
    let mut guid = pci_io::PROTOCOL_GUID;
    let mut count = 0;
    let mut handles = core::ptr::null_mut();
    unsafe {
        let status = ((*bs).locate_handle_buffer)(
            efi::BY_PROTOCOL,
            &mut guid,
            core::ptr::null_mut(),
            &mut count,
            &mut handles,
        );
        if status.is_error() || handles.is_null() {
            return Controller::NONE;
        }
        let mut found = Controller::NONE;
        if count <= 1024 {
            for handle in core::slice::from_raw_parts(handles, count) {
                let mut protocol = core::ptr::null_mut();
                if ((*bs).handle_protocol)(*handle, &mut guid, &mut protocol).is_error()
                    || protocol.is_null()
                {
                    continue;
                }
                let p = protocol as *mut pci_io::Protocol;
                let mut class = 0u32;
                if ((*p).pci.read)(
                    p,
                    pci_io::WIDTH_UINT32,
                    8,
                    1,
                    (&mut class as *mut u32).cast(),
                )
                .is_error()
                    || class >> 8 != 0x0c0330
                {
                    continue;
                }
                let (mut segment, mut bus, mut device, mut function) = (0, 0, 0, 0);
                if ((*p).get_location)(p, &mut segment, &mut bus, &mut device, &mut function)
                    .is_error()
                    || segment != 0
                    || bus > 255
                    || device > 31
                    || function > 7
                {
                    continue;
                }
                let mut resources = core::ptr::null_mut();
                if ((*p).get_bar_attributes)(p, 0, core::ptr::null_mut(), &mut resources).is_error()
                    || resources.is_null()
                {
                    continue;
                }
                // UEFI specifies one QWORD resource descriptor + end tag.
                let descriptor = core::slice::from_raw_parts(resources as *const u8, 48);
                let range = musha_platform::bar_resource(descriptor);
                ((*bs).free_pool)(resources);
                if let Some((base, bytes)) = range {
                    found = Controller {
                        base,
                        bytes,
                        bus: bus as u8,
                        device: device as u8,
                        function: function as u8,
                    };
                    break;
                }
            }
        }
        ((*bs).free_pool)(handles.cast());
        found
    }
}
/// Disable bus mastering after ownership/halt. A 16-bit write avoids clearing
/// adjacent PCI Status W1C bits. Keep memory decoding for reset diagnostics.
pub(crate) unsafe fn disable_dma(c: Controller) {
    let command = unsafe { read(c.bus, c.device, c.function, 4) } as u16;
    let address = 0x80000004u32
        | ((c.bus as u32) << 16)
        | ((c.device as u32) << 11)
        | ((c.function as u32) << 8);
    unsafe {
        asm!("out dx, eax",in("dx") 0xcf8u16,in("eax") address,options(nomem,nostack));
        asm!("out dx, ax",in("dx") 0xcfcu16,in("ax") command & !4,options(nomem,nostack));
    }
}
