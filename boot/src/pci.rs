// Copyright 2026 Hirofumi Iwasaki
// SPDX-License-Identifier: Apache-2.0
//! PCI configuration mechanism 1: segment zero, firmware-configured buses.
use core::arch::asm;
use musha_framebuffer::Framebuffer;

// The boot CPU is the sole caller; interrupts stay disabled. CF8/CFC transactions
// cannot interleave. Only the configuration ADDRESS port is written; device
// registers, BAR sizing, bus numbering and bus mastering are never modified.
unsafe fn read(bus: u8, device: u8, function: u8, register: u8) -> u32 {
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
