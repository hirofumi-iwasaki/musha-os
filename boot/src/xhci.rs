// Copyright 2026 Hirofumi Iwasaki
// SPDX-License-Identifier: Apache-2.0
//! Own/reset, build permanent DMA rings, probe commands, then quiesce.
use crate::{BootInfo, acpi, pci};
use musha_framebuffer::metrics as grid;
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
// BSP-only rescan deadline, measured on the shared PM epoch. Cleanup clears
// it before halt so the existing DMA-stop deadline is never shortened.
static RESCAN_DEADLINE: core::sync::atomic::AtomicU64 = core::sync::atomic::AtomicU64::new(0);
fn rescan_budget(clock: &mut acpi::Time) -> Result<(), &'static str> {
    let end = RESCAN_DEADLINE.load(core::sync::atomic::Ordering::Relaxed);
    if end != 0 {
        clock.now()?;
        if crate::bootstate::epoch_now().ok_or("NO TIMER")? >= end {
            return Err("RESCAN DEADLINE");
        }
    }
    Ok(())
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
        rescan_budget(clock)?;
        if regs.read(offset)? & mask == value {
            return Ok(());
        }
        if clock.now()? >= deadline {
            crate::timing::record(format_args!(
                "WAIT TIMEOUT OFF {:X} LIMIT {}",
                offset, limit
            ));
            crate::timing::record(format_args!(
                "MASK {:X} WANT {:X} GOT {:X}",
                mask,
                value,
                regs.read(offset)?
            ));
            return Err("TIMEOUT");
        }
        core::hint::spin_loop();
    }
    Err("CLOCK STALLED")
}
fn reset(info: &BootInfo, c: pci::Controller) -> Result<(bool, u64), &'static str> {
    crate::diagnostics::usb_stage("CAPABILITY");
    pci::controller_evidence(c);
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
    #[cfg(feature = "diagnostics-detail")]
    {
        for port in 1..=ports {
            let v = regs.read(op + 0x400 + (port - 1) * 16)?;
            crate::diagnostics::observation(format_args!("PRE OWN P{} PORTSC {:08X}", port, v));
            crate::diagnostics::observation(format_args!(
                "CCS {} PED {} PP {} SPEED {} PLS {}",
                v & 1,
                (v >> 1) & 1,
                (v >> 9) & 1,
                (v >> 10) & 15,
                (v >> 5) & 15
            ));
        }
        if unsafe { pci::read(c.bus, c.device, c.function, 0) } as u16 == 0x8086 {
            for o in musha_xhci::intel_ports::OFFSETS {
                crate::diagnostics::observation(format_args!(
                    "INTEL PRE OWN {:02X} {:08X}",
                    o,
                    unsafe { pci::read(c.bus, c.device, c.function, o) }
                ));
            }
        }
    }
    crate::diagnostics::usb_stage("OWNERSHIP");
    let ownership_time = crate::timing::Span::new(c.bdf(), "OWNERSHIP");
    let mut next = ((regs.read(0x10)? >> 16) as usize) * 4;
    #[cfg(feature = "diagnostics-detail")]
    let mut legacy_seen = false;
    let mut traversed = 0;
    while next != 0 {
        traversed += 1;
        if traversed > 256 || next < 0x20 {
            return Err("EXT CAPABILITY");
        }
        let header = regs.read(next)?;
        if header & 255 == 1 {
            #[cfg(feature = "diagnostics-detail")]
            {
                legacy_seen = true;
            }
            regs.read(next + 4)?;
            #[cfg(feature = "diagnostics-detail")]
            let ownership_start = clock.now()?;
            #[cfg(feature = "diagnostics-detail")]
            crate::diagnostics::observation(format_args!(
                "USBLEGSUP BEFORE {:08X} BIOS {} OS {}",
                header,
                (header >> 16) & 1,
                (header >> 24) & 1
            ));
            // Independent byte write to the OS semaphore preserves BIOS's byte.
            unsafe {
                let semaphore = (regs.base + next + 3) as *mut u8;
                semaphore.write_volatile(semaphore.read_volatile() | 1);
            }
            let ownership_result = wait(&regs, &mut clock, next, 1 << 16, 0, 1000);
            #[cfg(feature = "diagnostics-detail")]
            {
                if let Ok(v) = regs.read(next) {
                    crate::diagnostics::observation(format_args!(
                        "USBLEGSUP AFTER {:08X} BIOS {} OS {}",
                        v,
                        (v >> 16) & 1,
                        (v >> 24) & 1
                    ));
                }
                match clock.now() {
                    Ok(t) => crate::diagnostics::observation(format_args!(
                        "OWNERSHIP ELAPSED {} MS",
                        t.saturating_sub(ownership_start)
                    )),
                    Err(e) => crate::diagnostics::observation(format_args!(
                        "OWNERSHIP TIME UNKNOWN {}",
                        e
                    )),
                }
                crate::diagnostics::observation(format_args!(
                    "OWNERSHIP RESULT {}",
                    ownership_result.err().unwrap_or("OK")
                ));
            }
            ownership_result?;
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
    #[cfg(feature = "diagnostics-detail")]
    if !legacy_seen {
        crate::diagnostics::observation(format_args!("USBLEGSUP ABSENT / NO HANDOFF"));
    }
    ownership_time.finish("OK / ABSENT IF NO LEGACY CAP");
    // After firmware ownership handoff, before halt/reset/ring setup.
    let intel = unsafe { pci::read(c.bus, c.device, c.function, 0) } as u16 == 0x8086;
    if intel {
        for port in 1..=ports {
            crate::diagnostics::observation(format_args!(
                "ROUTE BEFORE P{} PORTSC {:08X}",
                port,
                regs.read(op + 0x400 + (port - 1) * 16)?
            ));
        }
    }
    let routed = pci::route_intel_ports(c);
    let route_epoch = crate::bootstate::epoch_now().unwrap_or(0);
    #[cfg(feature = "diagnostics-detail")]
    if intel {
        for o in musha_xhci::intel_ports::OFFSETS {
            crate::diagnostics::observation(format_args!(
                "INTEL POST ROUTE {:02X} {:08X}",
                o,
                unsafe { pci::read(c.bus, c.device, c.function, o) }
            ));
        }
    }
    if intel {
        for port in 1..=ports {
            crate::diagnostics::observation(format_args!(
                "ROUTE AFTER P{} PORTSC {:08X}",
                port,
                regs.read(op + 0x400 + (port - 1) * 16)?
            ));
        }
    }
    let halt_time = crate::timing::Span::new(c.bdf(), "INITIAL HALT");
    crate::diagnostics::usb_stage("HALT");
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
    halt_time.finish("OK");
    let reset_time = crate::timing::Span::new(c.bdf(), "RESET");
    crate::diagnostics::usb_stage("RESET");
    regs.write(op, regs.read(op)? | 2)?; // preserve reserved command bits
    wait(&regs, &mut clock, op, 2, 0, 1000)?;
    wait(&regs, &mut clock, op + 4, 1 << 11, 0, 1000)?;
    if regs.read(op + 4)? & 1 == 0 || regs.read(op + 8)? & 1 == 0 {
        return Err("RESET STATE");
    }
    reset_time.finish("OK");
    crate::debug(b"MUSHA: XHCI_RESET_OK PORTS=");
    crate::debug(&crate::cpu::hex(ports as u64));
    crate::debug(b" DMA_DISABLED\n");
    Ok((routed, route_epoch))
}
pub(crate) type Tick<'a> = dyn for<'e> FnMut(AppEvent<'e>) -> Result<bool, &'static str> + 'a;
pub(crate) enum AppEvent<'a> {
    Ready(bool),
    Pointer([i32; 3], u32),
    InputPoll(&'a mut dyn FnMut(&mut Tick<'_>) -> Result<bool, &'static str>),
    Files(&'a mut musha_api::stream::Files<'static>),
    Retained(&'a mut dyn FnMut(&mut Tick<'_>) -> Result<bool, &'static str>),
    Keys(&'a [(u8, bool)]),
    File(&'a [u8]),
    FileError(musha_api::Error),
}
#[derive(Clone, Copy, Default)]
struct Probe {
    attempted: bool,
    keyboard: bool,
    pointer: bool,
    storage: usize,
    readable_storage: usize,
}
use musha_platform::controllers::DMA_SLICE;
fn probe(
    info: &BootInfo,
    c: pci::Controller,
    index: usize,
    discovery: bool,
    tick: &mut dyn FnMut(AppEvent<'_>) -> Result<bool, &'static str>,
) -> Result<Probe, &'static str> {
    crate::diagnostics::begin_controller(c);
    let total_time =
        crate::timing::Span::new(c.bdf(), if discovery { "DISCOVERY" } else { "LIVE" });
    let result = pci::recover(info, c, index)
        .and_then(|()| reset(info, c))
        .and_then(|(routed, route_epoch)| {
            crate::diagnostics::usb_stage("RINGS / COMMANDS");
            command_probe(info, c, index, discovery, routed, route_epoch, tick)
        });
    if let Err(error) = result {
        crate::diagnostics::usb_failure(error);
        crate::debug(b"MUSHA: XHCI_FAILED ");
        crate::debug(error.as_bytes());
        crate::debug(b"\n");
    }
    unsafe {
        pci::disable_dma(c);
    }
    if unsafe { pci::read(c.bus, c.device, c.function, 4) } & 4 != 0 {
        return Err("QUIESCE FAILED");
    }
    total_time.finish(result.as_ref().err().copied().unwrap_or("OK"));
    crate::diagnostics::controller_result(
        c,
        result.as_ref().ok().map(|p| (p.keyboard, p.storage)),
        result.err(),
        !discovery && result.as_ref().is_ok_and(|p| p.keyboard),
    );
    result
}
// Keep the storage Host and its exclusive DMA slice alive on the outer frame.
// A nested input Host has a disjoint controller/DMA slice. Every keyboard cycle
// advances the outer file transport once; errors unwind and quiesce both Hosts.
fn live_probe(
    info: &BootInfo,
    index: usize,
    keyboard: Option<usize>,
    tick: &mut Tick<'_>,
) -> Result<Probe, &'static str> {
    let mut bridge = |event: AppEvent<'_>| {
        if let AppEvent::Retained(poll) = event {
            if let Some(k) = keyboard.filter(|&k| k != index) {
                crate::debug(b"MUSHA: CROSS_CONTROLLER_LIVE\n");
                let mut input = |event: AppEvent<'_>| {
                    if matches!(event, AppEvent::Keys(_)) && poll(tick)? {
                        return Ok(true);
                    }
                    if matches!(event, AppEvent::Retained(_)) {
                        return Err("NESTED STORAGE LOOP");
                    }
                    tick(event)
                };
                let saved = crate::diagnostics::suspend_controller();
                let result = probe(info, info.xhcis.entries[k], k, false, &mut input);
                crate::diagnostics::resume_controller(saved);
                result?;
            } else {
                tick(AppEvent::Ready(false))?;
                let mut clock = acpi::Time::new(info.timer)?;
                let end = if cfg!(feature = "qemu-debug") {
                    clock.now()?.checked_add(10000).ok_or("CLOCK OVERFLOW")?
                } else {
                    u64::MAX
                };
                while clock.now()? < end {
                    if poll(tick)? || tick(AppEvent::Keys(&[]))? {
                        break;
                    }
                }
            }
            Ok(true)
        } else {
            tick(event)
        }
    };
    probe(info, info.xhcis.entries[index], index, false, &mut bridge)
}
pub(crate) fn run_session(
    info: &BootInfo,
    tick: &mut dyn FnMut(AppEvent<'_>) -> Result<bool, &'static str>,
) -> Result<(), &'static str> {
    crate::timing::begin_session();
    let controllers = &info.xhcis.entries[..info.xhcis.count];
    crate::diagnostics::controller_limit(info.xhcis.truncated);
    let mut found_device = false;
    let result = (|| {
        if controllers.is_empty() {
            return Err("NOT FOUND");
        }
        let mut keyboard = None;
        let mut pointer = None;
        let mut storage = None;
        let mut readable_storage = None;
        let mut success = false;
        let mut attempted = false;
        let mut safe_empty = [false; 16];
        let mut last_error = "NO CONTROLLER";
        let owner = controllers.iter().position(|c| c.bdf() == info.boot_owner);
        for index in (0..controllers.len()).map(|step| match owner {
            Some(i) if step == 0 => i,
            Some(i) if step <= i => step - 1,
            _ => step,
        }) {
            let c = controllers[index];
            crate::debug(b"MUSHA: XHCI_SCAN BDF=");
            crate::debug(&crate::cpu::hex(c.bdf() as u64));
            crate::debug(b"\n");
            match probe(info, c, index, true, tick) {
                Ok(report) => {
                    success = true;
                    attempted |= report.attempted;
                    safe_empty[index] = !report.attempted;
                    if report.pointer && pointer.is_none() {
                        pointer = Some(index);
                    }
                    if report.storage > 0 && storage.is_none() {
                        storage = Some(index);
                    }
                    if report.readable_storage > 0 && readable_storage.is_none() {
                        readable_storage = Some(index);
                    }
                    found_device |= report.keyboard || report.storage > 0;
                    if report.keyboard && keyboard.is_none() {
                        keyboard = Some(index);
                    }
                }
                Err(error @ ("QUIESCE FAILED" | "APP INPUT ERROR" | "APP FILE CACHE")) => {
                    return Err(error);
                }
                Err(error) => {
                    last_error = error;
                    attempted = true;
                }
            }
        }
        // All first-pass hosts are halted with BME=0. Only untouched-slot
        // controllers may reinitialize rings without HCRST; PP is preserved.
        if musha_xhci::superspeed::rescan_allowed(
            keyboard.is_some() || pointer.is_some(),
            readable_storage.is_some(),
            attempted,
            0,
        ) {
            crate::timing::set_rescan(1);
            crate::diagnostics::observation(format_args!("RESCAN 1/1 START MAX 1000 MS"));
            crate::debug(b"MUSHA: RESCAN 1/1 START\n");
            let mut clock = acpi::Time::new(info.timer)?;
            let start = clock.now()?;
            let epoch_end = crate::bootstate::epoch_now()
                .ok_or("NO TIMER")?
                .saturating_add(1000);
            let mut rescanned = [false; 16];
            for _ in 0..=50 {
                if clock.now()?.saturating_sub(start) >= 1000 {
                    break;
                }
                for (index, c) in controllers.iter().copied().enumerate() {
                    if clock.now()?.saturating_sub(start) >= 1000 {
                        break;
                    }
                    if !safe_empty[index] || rescanned[index] {
                        continue;
                    }
                    let regs = Registers {
                        base: c.base,
                        bytes: c.bytes,
                    };
                    let op = (regs.read(0)? & 255) as usize;
                    let ports = (regs.read(4)? >> 24) as usize;
                    let connected = (1..=ports).try_fold(false, |found, port| {
                        regs.read(op + 0x400 + (port - 1) * 16)
                            .map(|v| found || v & 1 != 0)
                    })?;
                    if !connected {
                        continue;
                    }
                    rescanned[index] = true;
                    crate::diagnostics::begin_controller(c);
                    RESCAN_DEADLINE.store(epoch_end, core::sync::atomic::Ordering::Relaxed);
                    let report = command_probe(info, c, index, true, false, 0, tick);
                    RESCAN_DEADLINE.store(0, core::sync::atomic::Ordering::Relaxed);
                    match report {
                        Ok(report) => {
                            attempted |= report.attempted;
                            found_device |= report.keyboard || report.storage > 0;
                            if report.keyboard {
                                keyboard = Some(index);
                            }
                            if report.storage > 0 {
                                storage = Some(index);
                            }
                            if report.readable_storage > 0 {
                                readable_storage = Some(index);
                            }
                        }
                        Err(error @ ("QUIESCE FAILED" | "APP INPUT ERROR" | "APP FILE CACHE")) => {
                            return Err(error);
                        }
                        Err(error) => {
                            last_error = error;
                            attempted = true;
                        }
                    }
                }
                if attempted {
                    break;
                }
                let until = clock
                    .now()?
                    .saturating_add(20)
                    .min(start.saturating_add(1000));
                for _ in 0..5_000_000 {
                    if clock.now()? >= until {
                        break;
                    }
                    core::hint::spin_loop();
                }
            }
            crate::diagnostics::observation(format_args!(
                "RESCAN 1/1 RESULT K{} S{}",
                keyboard.is_some() as u8,
                readable_storage.is_some() as u8
            ));
            crate::debug(b"MUSHA: RESCAN 1/1 DONE\n");
        }
        crate::debug(b"MUSHA: XHCI_SCAN_COMPLETE\n");
        if let Some(k) = keyboard {
            crate::debug(b"MUSHA: XHCI_INPUT BDF=");
            crate::debug(&crate::cpu::hex(controllers[k].bdf() as u64));
            crate::debug(b"\n");
        }
        if let Some(index) = readable_storage.or(keyboard).or(pointer).or(storage) {
            live_probe(info, index, keyboard.or(pointer), tick).map(|_| ())
        } else {
            tick(AppEvent::Ready(false))?;
            if success { Ok(()) } else { Err(last_error) }
        }
    })();
    #[cfg(feature = "diagnostics-detail")]
    crate::diagnostics::observation(format_args!(
        "RUN SESSION RESULT {}",
        result.err().unwrap_or("OK")
    ));
    if crate::app::VIEWER {
        return result;
    }
    clear_line(info.framebuffer, grid::KEYBOARD);
    unsafe {
        info.framebuffer.text_grid(
            if result.is_ok() {
                if found_device {
                    "USB DEVICE READY"
                } else {
                    "USB NO KBD/STORAGE"
                }
            } else {
                "XHCI FAILED"
            },
            grid::LABEL,
            grid::KEYBOARD,
            info.framebuffer
                .color(if result.is_ok() { 0 } else { 255 }, 240, 100),
        );
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
    event_dispatch(
        regs,
        clock,
        base,
        cursor,
        runtime,
        ac64,
        expected,
        ports,
        expected_slot,
        transfer,
        timeout_ms,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
    )
}
fn event_dispatch(
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
    deferred: Option<(usize, u8, u8)>,
    mut saved: Option<&mut Option<[u32; 4]>>,
    bulk: Option<(usize, u8, u8)>,
    mut bulk_saved: Option<&mut Option<[u32; 4]>>,
    mouse: Option<(usize, u8, u8)>,
    mut mouse_saved: Option<&mut Option<[u32; 4]>>,
    short: Option<(usize, u32)>,
    mut residual: Option<&mut Option<u32>>,
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
        rescan_budget(clock)?;
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
                        crate::debug(b"MUSHA: COMMAND_CC=");
                        crate::debug(&crate::cpu::hex((words[2] >> 24) as u64));
                        crate::debug(b"\n");
                        crate::diagnostics::observation(format_args!(
                            "COMMAND CC {} SLOT {} EXPECT {}",
                            words[2] >> 24,
                            words[3] >> 24,
                            expected_slot
                        ));
                        crate::diagnostics::observation(format_args!(
                            "COMMAND RAW {:08X} {:08X}",
                            words[0], words[1]
                        ));
                        crate::diagnostics::observation(format_args!(
                            "COMMAND RAW {:08X} {:08X}",
                            words[2], words[3]
                        ));
                        crate::diagnostics::observation(format_args!(
                            "COMMAND EXPECT TRB {:016X}",
                            expected
                        ));
                        if transfer == 0
                            && musha_xhci::recoverable_command_error(words, expected, expected_slot)
                        {
                            // Consume the correlated error once before issuing Disable Slot.
                            cursor.advance();
                            regs.address(
                                runtime + 0x38,
                                (base + cursor.index * 16) as u64 | 8,
                                ac64,
                            )?;
                            regs.write(runtime + 0x20, (regs.read(runtime + 0x20)? & !3) | 1)?;
                            return Err("COMMAND REJECTED");
                        }
                        return Err("BAD COMPLETION");
                    }
                    true
                }
                32 => {
                    if let (Some((data, requested)), Some(residual)) =
                        (short, residual.as_deref_mut())
                        && let Some(remaining) = musha_xhci::control::short_residual(
                            words,
                            data,
                            expected_slot,
                            requested,
                        )
                    {
                        if residual.is_some() {
                            return Err("DUPLICATE SHORT EVENT");
                        }
                        *residual = Some(remaining);
                        false
                    } else if transfer == 0
                        || !musha_xhci::endpoint_completion(
                            words,
                            expected,
                            expected_slot,
                            transfer,
                        )
                    {
                        let stored = saved.as_deref_mut().is_some_and(|saved| {
                            musha_xhci::defer_completion(words, deferred, saved)
                        }) || bulk_saved
                            .as_deref_mut()
                            .is_some_and(|saved| musha_xhci::defer_completion(words, bulk, saved))
                            || mouse_saved.as_deref_mut().is_some_and(|saved| {
                                musha_xhci::defer_completion(words, mouse, saved)
                            });
                        if !stored {
                            crate::diagnostics::transfer_failure(
                                words,
                                expected,
                                expected_slot,
                                transfer,
                            );
                            return Err("BAD TRANSFER");
                        }
                        false
                    } else {
                        true
                    }
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
    controller: pci::Controller,
    index: usize,
    discovery: bool,
    routed: bool,
    route_epoch: u64,
    tick: &mut dyn FnMut(AppEvent<'_>) -> Result<bool, &'static str>,
) -> Result<Probe, &'static str> {
    use musha_xhci::{Cursor, Pool};
    let regs = Registers {
        base: controller.base,
        bytes: controller.bytes,
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
    let dma_base = musha_platform::controllers::dma_slice(
        info.dma_base,
        info.dma_bytes,
        index,
        info.xhcis.count,
    )
    .ok_or("DMA RANGE")?;
    let mut pool = Pool::new(dma_base, DMA_SLICE).ok_or("DMA POOL")?;
    // Also guard the no-HCRST rescan path before reusing this DMA slice.
    if regs.read(op + 4)? & 1 == 0
        || unsafe { pci::read(controller.bus, controller.device, controller.function, 4) } & 4 != 0
    {
        return Err("QUIESCE FAILED");
    }
    // Reset/halt and PCI BME=0 were verified before any allocation/initialization.
    unsafe {
        core::ptr::write_bytes(dma_base as *mut u8, 0, DMA_SLICE);
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
            pci::enable_dma(controller);
        }
        if unsafe { pci::read(controller.bus, controller.device, controller.function, 4) } & 4 == 0
        {
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
            bdf: controller.bdf(),
            routed,
            route_epoch,
            attempted: false,
            discovery,
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
            storage_matches: 0,
            readable_storage: 0,
            control_cursors: [musha_xhci::control::Cursor::NEW; 9],
            keyboard_pending: None,
            keyboard_saved: None,
            mouse_pending: None,
            mouse_saved: None,
            bulk_pending: None,
            bulk_saved: None,
        };
        let (keyboard, pointer) = usb::enumerate(&mut host, &mut pool, dcbaa, discovery, tick)?;
        Ok(Probe {
            attempted: host.attempted,
            keyboard,
            pointer,
            storage: host.storage_matches,
            readable_storage: host.readable_storage,
        })
    })();
    if let Err(error) = result {
        crate::diagnostics::usb_failure(error);
    }
    RESCAN_DEADLINE.store(0, core::sync::atomic::Ordering::Relaxed);
    let stop_time = crate::timing::Span::new(controller.bdf(), "QUIESCE");
    let halted = (|| {
        let command = regs.read(op)?;
        regs.write(op, command & !0xd)?;
        wait(&regs, &mut clock, op + 4, 1, 1, 100)
    })();
    unsafe {
        pci::disable_dma(controller);
    }
    let disabled =
        unsafe { pci::read(controller.bus, controller.device, controller.function, 4) } & 4 == 0;
    stop_time.finish(if !disabled {
        "DMA STILL ENABLED"
    } else {
        halted.as_ref().err().copied().unwrap_or("OK")
    });
    if halted.is_err() || !disabled {
        return Err("QUIESCE FAILED");
    }
    crate::diagnostics::usb_stage("STOPPED DMA DISABLED");
    crate::debug(b"MUSHA: XHCI_QUIESCED DMA_DISABLED\n");
    result
}

struct Host<'a> {
    bdf: u16,
    routed: bool,
    route_epoch: u64,
    attempted: bool,
    discovery: bool,
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
    storage_matches: usize,
    readable_storage: usize,
    control_cursors: [musha_xhci::control::Cursor; 9],
    keyboard_pending: Option<(usize, u8, u8)>,
    keyboard_saved: Option<[u32; 4]>,
    mouse_pending: Option<(usize, u8, u8)>,
    mouse_saved: Option<[u32; 4]>,
    bulk_pending: Option<(usize, u8, u8)>,
    bulk_saved: Option<[u32; 4]>,
}
impl Host<'_> {
    fn command(
        &mut self,
        parameter: usize,
        control: u32,
        slot: u8,
    ) -> Result<[u32; 4], &'static str> {
        rescan_budget(self.clock)?;
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
        #[cfg(feature = "diagnostics-detail")]
        crate::diagnostics::observation(format_args!(
            "SUBMIT CMD {} SLOT {} TRB {:016X}",
            (control >> 10) & 63,
            slot,
            address
        ));
        self.regs.write(self.doorbell, 0)?;
        let result = event_dispatch(
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
            self.keyboard_pending,
            Some(&mut self.keyboard_saved),
            self.bulk_pending,
            Some(&mut self.bulk_saved),
            self.mouse_pending,
            Some(&mut self.mouse_saved),
            None,
            None,
        );
        if result.is_ok() || result == Err("COMMAND REJECTED") {
            self.producer.advance();
        }
        result
    }
    fn delay(&mut self, ms: u64) -> Result<(), &'static str> {
        let end = self.clock.now()?.checked_add(ms).ok_or("CLOCK OVERFLOW")?;
        for _ in 0..5_000_000 {
            rescan_budget(self.clock)?;
            if self.clock.now()? >= end {
                return Ok(());
            }
            core::hint::spin_loop();
        }
        Err("CLOCK STALLED")
    }
}

fn clear_line(fb: musha_framebuffer::Framebuffer, row: usize) {
    let m = grid::Metrics::new(fb.width, fb.height);
    let y = m.y(row);
    for row in y..(y + m.line_pitch).min(fb.height) {
        for x in m.x(grid::LABEL)..m.x(m.status_end_column()).min(fb.width) {
            unsafe {
                fb.pixel(x, row, fb.color(12, 20, 32));
            }
        }
    }
}

/// Terminal application shutdown. Only the input loan publishes this hook;
/// all controllers were acquired by run_session before the loan. Reuse requires
/// true plus normal return through every enclosing session cleanup scope.
pub(crate) fn quiesce_app(info: &BootInfo) -> bool {
    let mut ok = true;
    for c in &info.xhcis.entries[..info.xhcis.count] {
        let bme = unsafe { pci::read(c.bus, c.device, c.function, 4) } & 4 != 0;
        if !bme {
            continue;
        }
        let regs = Registers {
            base: c.base,
            bytes: c.bytes,
        };
        let halted = (|| {
            let op = (regs.read(0)? & 255) as usize;
            regs.write(op, regs.read(op)? & !0xd)?;
            let mut time = acpi::Time::new(info.timer)?;
            wait(&regs, &mut time, op + 4, 1, 1, 100)
        })();
        unsafe {
            pci::disable_dma(*c);
        }
        ok &= halted.is_ok() && unsafe { pci::read(c.bus, c.device, c.function, 4) } & 4 == 0;
    }
    ok
}

/// # Safety
/// No callback/Host/file-session references remain and DMA stop is confirmed.
pub(crate) unsafe fn release_files_stopped() {
    unsafe {
        usb::release_files_stopped();
    }
}
