// Copyright 2026 Hirofumi Iwasaki
// SPDX-License-Identifier: Apache-2.0
//! First driver milestone: own, halt and reset; never start a DMA schedule.
use crate::{BootInfo, acpi, pci};
struct Registers {
    base: usize,
    bytes: usize,
}
impl Registers {
    fn read(&self, offset: usize) -> Result<u32, &'static str> {
        if offset % 4 != 0 || offset.checked_add(4).is_none_or(|end| end > self.bytes) {
            return Err("MMIO RANGE");
        }
        // SAFETY: initialize mapped the checked BAR UC/RW/NX; BSP is the sole
        // accessor, and every access is aligned and within the BAR allocation.
        Ok(unsafe { ((self.base + offset) as *const u32).read_volatile() })
    }
    fn write(&self, offset: usize, value: u32) -> Result<(), &'static str> {
        self.read(offset)?;
        unsafe {
            ((self.base + offset) as *mut u32).write_volatile(value);
        }
        Ok(())
    }
}
fn wait(
    regs: &Registers,
    clock: &mut acpi::Time,
    offset: usize,
    mask: u32,
    value: u32,
    limit: u64,
) -> Result<(), &'static str> {
    let deadline = clock.now()?.checked_add(limit).ok_or("CLOCK OVERFLOW")?;
    for _ in 0..5_000_000 {
        if regs.read(offset)? & mask == value {
            return Ok(());
        }
        if clock.now()? >= deadline {
            return Err("TIMEOUT");
        }
        core::hint::spin_loop();
    }
    Err("CLOCK STALLED")
}
fn reset(info: &BootInfo) -> Result<(), &'static str> {
    let c = info.xhci;
    if c.bytes == 0 {
        return Err("NOT FOUND");
    }
    let command = unsafe { pci::read(c.bus, c.device, c.function, 4) };
    if command & 2 == 0 {
        return Err("MEMORY DISABLED");
    }
    let regs = Registers {
        base: c.base,
        bytes: c.bytes,
    };
    let cap = regs.read(0)?;
    let op = (cap & 255) as usize;
    let version = cap >> 16;
    if op < 0x20 || op % 4 != 0 || !(0x100..=0x120).contains(&version) {
        return Err("CAPABILITY");
    }
    let params = regs.read(4)?;
    let ports = (params >> 24) as usize;
    if ports == 0
        || params & 255 == 0
        || op
            .checked_add(0x400 + ports * 16)
            .is_none_or(|end| end > regs.bytes)
    {
        return Err("PORT RANGE");
    }
    let mut clock = acpi::Time::new(info.timer)?;
    // Prove that the clock advances before any ownership or controller writes.
    let started = clock.now()?;
    for iteration in 0..1_000_000 {
        if clock.now()? > started {
            break;
        }
        if iteration == 999_999 {
            return Err("CLOCK STALLED");
        }
    }
    let mut next = ((regs.read(0x10)? >> 16) as usize) * 4;
    let mut traversed = 0;
    while next != 0 {
        traversed += 1;
        if traversed > 256 || next < 0x20 {
            return Err("EXT CAPABILITY");
        }
        let header = regs.read(next)?;
        if header & 255 == 1 {
            regs.read(next + 4)?;
            // Independent byte write to the OS semaphore preserves BIOS's byte.
            unsafe {
                let semaphore = (regs.base + next + 3) as *mut u8;
                semaphore.write_volatile(semaphore.read_volatile() | 1);
            }
            wait(&regs, &mut clock, next, 1 << 16, 0, 1000)?;
            if regs.read(next)? & (1 << 24) == 0 {
                return Err("OWNERSHIP");
            }
            let control = regs.read(next + 4)?;
            // Disable defined SMI enables, preserve reserved bits, and acknowledge
            // only the defined ownership/config/BAR W1C status flags.
            regs.write(next + 4, (control & !0xe011 & !0xe0000000) | 0xe0000000)?;
        }
        let delta = ((header >> 8) & 255) as usize * 4;
        if delta == 0 {
            break;
        }
        next = next.checked_add(delta).ok_or("EXT CAPABILITY")?;
    }
    wait(&regs, &mut clock, op + 4, 1 << 11, 0, 1000)?;
    let command = regs.read(op)?;
    regs.write(op, command & !0xd)?; // R/S, INTE, HSEE off
    wait(&regs, &mut clock, op + 4, 1, 1, 100)?;
    unsafe {
        pci::disable_dma(c);
    }
    if unsafe { pci::read(c.bus, c.device, c.function, 4) } & 4 != 0 {
        return Err("DMA DISABLE");
    }
    #[cfg(feature = "xhci-timeout")]
    wait(&regs, &mut clock, op + 4, 1, 0, 20)?; // held halted: deadline must expire
    regs.write(op, regs.read(op)? | 2)?; // preserve reserved command bits
    wait(&regs, &mut clock, op, 2, 0, 1000)?;
    wait(&regs, &mut clock, op + 4, 1 << 11, 0, 1000)?;
    if regs.read(op + 4)? & 1 == 0 || regs.read(op + 8)? & 1 == 0 {
        return Err("RESET STATE");
    }
    crate::debug(b"MUSHA: XHCI_RESET_OK PORTS=");
    crate::debug(&crate::cpu::hex(ports as u64));
    crate::debug(b" DMA_DISABLED\n");
    Ok(())
}
pub(crate) fn diagnose(info: &BootInfo) {
    let (label, color) = match reset(info) {
        Ok(()) => ("XHCI RESET OK", info.framebuffer.color(0, 240, 100)),
        Err(error) => {
            crate::debug(b"MUSHA: XHCI_FAILED ");
            crate::debug(error.as_bytes());
            crate::debug(b"\n");
            ("XHCI FAILED", info.framebuffer.color(255, 180, 0))
        }
    };
    unsafe {
        info.framebuffer.text(label, 24, 292, color);
    }
}
