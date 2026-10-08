// Copyright 2026 Hirofumi Iwasaki
// SPDX-License-Identifier: Apache-2.0
//! Own/reset, build permanent DMA rings, probe commands, then quiesce.
use crate::{BootInfo, acpi, pci};
mod usb;
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
    fn address(&self, offset: usize, value: u64, ac64: bool) -> Result<(), &'static str> {
        if offset % 8 != 0 || offset.checked_add(8).is_none_or(|end| end > self.bytes) {
            return Err("MMIO RANGE");
        }
        if ac64 {
            // Native aligned Qword MMIO on x86-64, as required for AC64 xHC.
            unsafe {
                ((self.base + offset) as *mut u64).write_volatile(value);
            }
            Ok(())
        } else {
            if value >> 32 != 0 {
                return Err("DMA ADDRESS");
            }
            self.write(offset, value as u32)
        }
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
pub(crate) enum AppEvent<'a> {
    Ready(bool),
    Keys(&'a [(u8, bool)]),
    File(&'a [u8]),
    FileError(musha_api::Error),
}
pub(crate) fn diagnose(
    info: &BootInfo,
    tick: &mut dyn FnMut(AppEvent<'_>) -> Result<bool, &'static str>,
) -> Result<(), &'static str> {
    let result = reset(info).and_then(|()| command_probe(info, tick));
    let (label, color) = match result {
        Ok(()) => ("USB ENUMERATED", info.framebuffer.color(0, 240, 100)),
        Err(error) => {
            crate::debug(b"MUSHA: XHCI_FAILED ");
            crate::debug(error.as_bytes());
            crate::debug(b"\n");
            ("XHCI FAILED", info.framebuffer.color(255, 180, 0))
        }
    };
    clear_line(info.framebuffer, 292);
    unsafe {
        info.framebuffer.text(label, 24, 292, color);
    }
    result
}

const TRBS: usize = 256;
const COMMANDS: usize = 600;
// SAFETY for DMA helpers: addresses come exclusively from the dedicated UC pool.
// No Rust reference/slice to device-writable memory exists. Controller owns ring
// entries once cycle is published; CPU reuses command entries only on completion.
unsafe fn publish(address: usize, words: [u32; 4]) {
    let p = address as *mut u32;
    unsafe {
        for (i, word) in words[..3].iter().enumerate() {
            p.add(i).write_volatile(*word);
        }
        core::sync::atomic::compiler_fence(core::sync::atomic::Ordering::Release);
        p.add(3).write_volatile(words[3]);
        core::arch::asm!("mfence", options(nostack));
    }
}
unsafe fn dma_u64(address: usize, value: u64) {
    unsafe {
        (address as *mut u64).write_volatile(value);
    }
}
fn event(
    regs: &Registers,
    clock: &mut acpi::Time,
    base: usize,
    cursor: &mut musha_xhci::Cursor,
    runtime: usize,
    ac64: bool,
    expected: usize,
    ports: u32,
    expected_slot: u8,
    transfer: u8,
    timeout_ms: u64,
) -> Result<[u32; 4], &'static str> {
    let limit = if cfg!(feature = "xhci-command-timeout")
        || (transfer != 0 && cfg!(feature = "usb-descriptor-timeout"))
    {
        20
    } else {
        timeout_ms
    };
    let deadline = clock.now()?.checked_add(limit).ok_or("CLOCK OVERFLOW")?;
    let polls = if transfer > 1 { 500_000_000 } else { 5_000_000 };
    for _ in 0..polls {
        if regs.read((regs.read(0)? & 255) as usize + 4)? & ((1 << 2) | (1 << 12)) != 0 {
            return Err("HOST ERROR");
        }
        let address = base + cursor.index * 16;
        let p = address as *const u32;
        let control = unsafe { p.add(3).read_volatile() };
        if control & 1 == cursor.cycle {
            // The device publishes the cycle flag after its payload. LFENCE and
            // the compiler barrier keep payload reads after this observation.
            unsafe {
                core::arch::asm!("lfence", options(nostack));
            }
            core::sync::atomic::compiler_fence(core::sync::atomic::Ordering::Acquire);
            let words = [
                unsafe { p.read_volatile() },
                unsafe { p.add(1).read_volatile() },
                unsafe { p.add(2).read_volatile() },
                control,
            ];
            let kind = (control >> 10) & 63;
            let done = match kind {
                33 => {
                    let slot = if expected_slot == 255 {
                        None
                    } else {
                        Some(expected_slot)
                    };
                    if transfer != 0 || !musha_xhci::command_completion(words, expected, slot, 8) {
                        return Err("BAD COMPLETION");
                    }
                    true
                }
                32 => {
                    if transfer == 0
                        || !musha_xhci::endpoint_completion(
                            words,
                            expected,
                            expected_slot,
                            transfer,
                        )
                    {
                        return Err("BAD TRANSFER");
                    }
                    true
                }
                34 => {
                    let port = words[0] >> 24;
                    if port == 0 || port > ports || words[2] >> 24 != 1 {
                        return Err("PORT EVENT");
                    }
                    false
                }
                _ => return Err("EVENT TYPE"),
            };
            cursor.advance();
            regs.address(runtime + 0x38, (base + cursor.index * 16) as u64 | 8, ac64)?; // ERDP + EHB W1C
            regs.write(runtime + 0x20, (regs.read(runtime + 0x20)? & !3) | 1)?; // IP ack, IE stays off
            if done {
                return Ok(words);
            }
        }
        if timeout_ms == 0 {
            return Err("EVENT PENDING");
        }
        if clock.now()? >= deadline {
            return Err(if transfer != 0 {
                "TRANSFER TIMEOUT"
            } else {
                "COMMAND TIMEOUT"
            });
        }
        core::hint::spin_loop();
    }
    Err("CLOCK STALLED")
}
fn command_probe(
    info: &BootInfo,
    tick: &mut dyn FnMut(AppEvent<'_>) -> Result<bool, &'static str>,
) -> Result<(), &'static str> {
    use musha_xhci::{Cursor, Pool};
    let regs = Registers {
        base: info.xhci.base,
        bytes: info.xhci.bytes,
    };
    let op = (regs.read(0)? & 255) as usize;
    let ac64 = regs.read(0x10)? & 1 != 0;
    let runtime = (regs.read(0x18)? & !31) as usize;
    let doorbell = (regs.read(0x14)? & !3) as usize;
    if runtime < 0x20 || doorbell < 0x20 {
        return Err("REGISTER OFFSET");
    }
    regs.read(runtime + 0x3c)?;
    regs.read(doorbell)?;
    let mut clock = acpi::Time::new(info.timer)?;
    let scratch = musha_xhci::scratchpads(regs.read(8)?);
    if scratch > 128 {
        return Err("SCRATCHPAD LIMIT");
    }
    let mut pool = Pool::new(info.dma_base, info.dma_bytes).ok_or("DMA POOL")?;
    // Reset/halt and PCI BME=0 were verified before any allocation/initialization.
    unsafe {
        core::ptr::write_bytes(info.dma_base as *mut u8, 0, info.dma_bytes);
    }
    let dcbaa = pool.allocate(2048, 64).ok_or("DMA FULL")?;
    let command = pool.allocate(4096, 4096).ok_or("DMA FULL")?;
    let events = pool.allocate(4096, 4096).ok_or("DMA FULL")?;
    let erst = pool.allocate(64, 64).ok_or("DMA FULL")?;
    if scratch != 0 {
        let array = pool.allocate(scratch * 8, 64).ok_or("DMA FULL")?;
        for i in 0..scratch {
            let page = pool.allocate(4096, 4096).ok_or("DMA FULL")?;
            unsafe {
                dma_u64(array + i * 8, page as u64);
            }
        }
        unsafe {
            dma_u64(dcbaa, array as u64);
        }
    }
    unsafe {
        dma_u64(erst, events as u64);
        ((erst + 8) as *mut u32).write_volatile(TRBS as u32);
        publish(
            command + (TRBS - 1) * 16,
            [command as u32, 0, 0, (6 << 10) | 2 | 1],
        );
        core::arch::asm!("mfence", options(nostack));
    }
    let ports = regs.read(4)? >> 24;
    // From enabling BME onwards all exits share cleanup. DMA pages remain
    // reserved for the entire boot, even if halt/cleanup or a command fails.
    let result = (|| {
        unsafe {
            pci::enable_dma(info.xhci);
        }
        if unsafe { pci::read(info.xhci.bus, info.xhci.device, info.xhci.function, 4) } & 4 == 0 {
            return Err("DMA ENABLE");
        }
        regs.address(op + 0x30, dcbaa as u64, ac64)?;
        regs.address(op + 0x18, command as u64 | 1, ac64)?;
        regs.write(
            op + 0x38,
            (regs.read(op + 0x38)? & !255) | (regs.read(4)? & 255).min(8),
        )?;
        regs.write(runtime + 0x20, (regs.read(runtime + 0x20)? & !3) | 1)?;
        regs.write(runtime + 0x24, 0)?;
        regs.write(runtime + 0x28, (regs.read(runtime + 0x28)? & !0xffff) | 1)?;
        regs.address(runtime + 0x38, events as u64, ac64)?;
        regs.address(runtime + 0x30, erst as u64, ac64)?;
        regs.write(op, (regs.read(op)? & !0xc) | 1)?;
        wait(&regs, &mut clock, op + 4, 1, 0, 100)?;
        let mut producer = Cursor::new(TRBS - 1).ok_or("COMMAND RING")?;
        let mut consumer = Cursor::new(TRBS).ok_or("EVENT RING")?;
        for _ in 0..COMMANDS {
            let address = command + producer.index * 16;
            if producer.index == TRBS - 2 {
                unsafe {
                    publish(
                        command + (TRBS - 1) * 16,
                        [command as u32, 0, 0, (6 << 10) | 2 | producer.cycle],
                    );
                }
            }
            unsafe {
                publish(address, [0, 0, 0, (23 << 10) | producer.cycle]);
            }
            #[cfg(not(feature = "xhci-command-timeout"))]
            regs.write(doorbell, 0)?;
            event(
                &regs,
                &mut clock,
                events,
                &mut consumer,
                runtime,
                ac64,
                address,
                ports,
                0,
                0,
                1000,
            )?;
            producer.advance();
        }
        crate::debug(b"MUSHA: XHCI_NOOP_OK COUNT=");
        crate::debug(&crate::cpu::hex(COMMANDS as u64));
        crate::debug(b" COMMAND_WRAP_OK EVENT_WRAP_OK\n");
        let mut host = Host {
            regs: &regs,
            clock: &mut clock,
            command,
            events,
            producer: &mut producer,
            consumer: &mut consumer,
            runtime,
            doorbell,
            ac64,
            ports,
            framebuffer: info.framebuffer,
        };
        usb::enumerate(&mut host, &mut pool, dcbaa, tick)?;
        Ok(())
    })();
    let halted = (|| {
        let command = regs.read(op)?;
        regs.write(op, command & !0xd)?;
        wait(&regs, &mut clock, op + 4, 1, 1, 100)
    })();
    unsafe {
        pci::disable_dma(info.xhci);
    }
    let disabled =
        unsafe { pci::read(info.xhci.bus, info.xhci.device, info.xhci.function, 4) } & 4 == 0;
    if halted.is_err() || !disabled {
        return Err("QUIESCE FAILED");
    }
    crate::debug(b"MUSHA: XHCI_QUIESCED DMA_DISABLED\n");
    result
}

struct Host<'a> {
    regs: &'a Registers,
    clock: &'a mut acpi::Time,
    command: usize,
    events: usize,
    producer: &'a mut musha_xhci::Cursor,
    consumer: &'a mut musha_xhci::Cursor,
    runtime: usize,
    doorbell: usize,
    ac64: bool,
    ports: u32,
    framebuffer: musha_framebuffer::Framebuffer,
}
impl Host<'_> {
    fn command(
        &mut self,
        parameter: usize,
        control: u32,
        slot: u8,
    ) -> Result<[u32; 4], &'static str> {
        let address = self.command + self.producer.index * 16;
        if self.producer.index == TRBS - 2 {
            unsafe {
                publish(
                    self.command + (TRBS - 1) * 16,
                    [
                        self.command as u32,
                        0,
                        0,
                        (6 << 10) | 2 | self.producer.cycle,
                    ],
                );
            }
        }
        unsafe {
            publish(
                address,
                [parameter as u32, 0, 0, control | self.producer.cycle],
            );
        }
        self.regs.write(self.doorbell, 0)?;
        let result = event(
            self.regs,
            self.clock,
            self.events,
            self.consumer,
            self.runtime,
            self.ac64,
            address,
            self.ports,
            slot,
            0,
            1000,
        )?;
        self.producer.advance();
        Ok(result)
    }
    fn delay(&mut self, ms: u64) -> Result<(), &'static str> {
        let end = self.clock.now()?.checked_add(ms).ok_or("CLOCK OVERFLOW")?;
        for _ in 0..5_000_000 {
            if self.clock.now()? >= end {
                return Ok(());
            }
            core::hint::spin_loop();
        }
        Err("CLOCK STALLED")
    }
}

fn clear_line(fb: musha_framebuffer::Framebuffer, y: usize) {
    for row in y..(y + 24).min(fb.height) {
        for x in 24..fb.width {
            unsafe {
                fb.pixel(x, row, fb.color(12, 20, 32));
            }
        }
    }
}
