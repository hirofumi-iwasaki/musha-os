// Copyright 2026 Hirofumi Iwasaki
// SPDX-License-Identifier: Apache-2.0
use musha_framebuffer::metrics as grid;
mod file;
mod hub;
mod storage;
use super::{Host, Registers, TRBS, dma_u64, publish, wait};

#[derive(Clone, Copy)]
struct Protocol {
    major: u8,
    slot: u8,
    offset: usize,
}
fn protocols(regs: &Registers, ports: u32) -> Result<[Protocol; 256], &'static str> {
    let mut result = [Protocol {
        major: 0,
        slot: 0,
        offset: 0,
    }; 256];
    let mut next = ((regs.read(0x10)? >> 16) as usize) * 4;
    let mut visited = 0;
    while next != 0 {
        visited += 1;
        if visited > 256 || next < 0x20 {
            return Err("EXT CAPABILITY");
        }
        let header = regs.read(next)?;
        if header & 255 == 2 {
            let major = (header >> 24) as u8;
            if !matches!(major, 2 | 3) || regs.read(next + 4)? != 0x20425355 {
                return Err("USB PROTOCOL");
            }
            let range = regs.read(next + 8)?;
            let start = (range & 255) as usize;
            let count = ((range >> 8) & 255) as usize;
            let slot = (regs.read(next + 12)? & 31) as u8;
            if start == 0 || count == 0 || start + count > ports as usize + 1 {
                return Err("PROTOCOL PORTS");
            }
            let psi = ((range >> 28) & 15) as usize;
            if psi != 0 {
                regs.read(next + 12 + psi * 4)?;
            }
            for port in start..start + count {
                if result[port].major != 0 {
                    return Err("PROTOCOL OVERLAP");
                }
                result[port] = Protocol {
                    major,
                    slot,
                    offset: next,
                };
            }
        }
        let delta = ((header >> 8) & 255) as usize * 4;
        if delta == 0 {
            break;
        }
        next = next.checked_add(delta).ok_or("EXT CAPABILITY")?;
    }
    Ok(result)
}
fn speed(regs: &Registers, protocol: Protocol, id: u8) -> Result<musha_xhci::Speed, &'static str> {
    use musha_xhci::Speed;
    let count = (regs.read(protocol.offset + 8)? >> 28) & 15;
    if count == 0 {
        return match (protocol.major, id) {
            (2, 1) => Ok(Speed::Full),
            (2, 2) => Ok(Speed::Low),
            (2, 3) => Ok(Speed::High),
            (3, 4) => Ok(Speed::Super),
            _ => Err("PORT SPEED"),
        };
    }
    let mut found = None;
    for i in 0..count {
        let psi = regs.read(protocol.offset + 16 + i as usize * 4)?;
        if psi & 15 == id as u32 {
            // Reject asymmetric speed definitions until RX/TX pair handling is
            // implemented. Symmetric PSI supplies a single usable line rate.
            if psi & 0xc0 != 0 || found.is_some() {
                return Err("ASYMMETRIC SPEED");
            }
            let multiplier = match (psi >> 4) & 3 {
                0 => 1,
                1 => 1000,
                2 => 1_000_000,
                _ => 1_000_000_000,
            };
            found = Speed::from_rate(protocol.major, (psi >> 16) as u64 * multiplier);
        }
    }
    found.ok_or("PORT SPEED")
}
fn port_reset(
    host: &mut Host<'_>,
    port: usize,
    protocol: Protocol,
) -> Result<(u8, musha_xhci::Speed), &'static str> {
    let op = (host.regs.read(0)? & 255) as usize;
    let offset = op + 0x400 + (port - 1) * 16;
    let original = host.regs.read(offset)?;
    if original & 1 == 0 {
        return Err("DISCONNECTED");
    }
    if original & (1 << 3) != 0 {
        return Err("OVERCURRENT");
    }
    // Debounce: require continuous connection for 100ms before USB reset.
    let until = host.clock.now()?.checked_add(100).ok_or("CLOCK OVERFLOW")?;
    let mut stable = false;
    for _ in 0..5_000_000 {
        super::rescan_budget(host.clock)?;
        if host.regs.read(offset)? & 1 == 0 {
            return Err("DISCONNECTED");
        }
        if host.clock.now()? >= until {
            stable = true;
            break;
        }
        core::hint::spin_loop();
    }
    if !stable {
        return Err("CLOCK STALLED");
    }
    let status = host.regs.read(offset)?;
    host.regs.write(
        offset,
        musha_xhci::port_control(status) | (status & 0xfe0000),
    )?;
    let reset = if protocol.major == 2 { 1 << 4 } else { 1 << 31 };
    let change = if protocol.major == 2 {
        1 << 21
    } else {
        1 << 19
    };
    host.regs.write(
        offset,
        musha_xhci::port_control(host.regs.read(offset)?) | reset,
    )?;
    wait(host.regs, host.clock, offset, reset | change, change, 200)?;
    let status = host.regs.read(offset)?;
    if status & 3 != 3 || status & (15 << 5) != 0 || status & (1 << 3) != 0 {
        return Err("PORT DISABLED");
    }
    host.regs.write(
        offset,
        musha_xhci::port_control(status) | (status & 0xfe0000),
    )?;
    host.delay(10)?; // USB reset recovery before default-control traffic
    let id = ((status >> 10) & 15) as u8;
    let speed = speed(host.regs, protocol, id)?;
    crate::diagnostics::observation(format_args!(
        "RESET ROOT {} BEFORE {:08X} AFTER {:08X}",
        port, original, status
    ));
    crate::diagnostics::observation(format_args!(
        "RESET ROOT {} SPEED ID {} NAME {:?}",
        port, id, speed
    ));
    crate::debug(b"MUSHA: USB_PORT_RESET_OK PORT=");
    crate::debug(&crate::cpu::hex(port as u64));
    crate::debug(b" SPEED=");
    crate::debug(&crate::cpu::hex(id as u64));
    crate::debug(b"\n");
    Ok((id, speed))
}
unsafe fn dma_word(address: usize, value: u32) {
    unsafe {
        (address as *mut u32).write_volatile(value);
    }
}
unsafe fn read_word(address: usize) -> u32 {
    unsafe { (address as *const u32).read_volatile() }
}
fn get_descriptor(
    host: &mut Host<'_>,
    slot: u8,
    ring: usize,
    buffer: usize,
    bytes: u32,
) -> Result<(), &'static str> {
    control(host, slot, ring, buffer, bytes, 0x01000680, 0)
}
fn control(
    host: &mut Host<'_>,
    slot: u8,
    ring: usize,
    buffer: usize,
    bytes: u32,
    request: u32,
    interface: u16,
) -> Result<(), &'static str> {
    let actual = control_read(host, slot, ring, buffer, bytes, request, interface)?;
    if actual != bytes {
        return Err("SHORT CONTROL READ");
    }
    Ok(())
}
fn control_read(
    host: &mut Host<'_>,
    slot: u8,
    ring: usize,
    buffer: usize,
    bytes: u32,
    request: u32,
    interface: u16,
) -> Result<u32, &'static str> {
    if !(1..=8).contains(&slot) || bytes > 1024 || (bytes != 0 && request & 0x80 == 0) {
        return Err("TRANSFER RANGE");
    }
    crate::diagnostics::control_request(slot, request, interface, bytes);
    let plan = host.control_cursors[slot as usize].plan(bytes != 0);
    let status = plan.count - 1;
    unsafe {
        core::ptr::write_bytes(buffer as *mut u8, 0, bytes as usize);
        // One TD in flight. Publish the Link before Setup exposes this TD.
        if let Some(cycle) = plan.link_cycle {
            publish(
                ring + (TRBS - 1) * 16,
                [ring as u32, 0, 0, (6 << 10) | 2 | cycle],
            );
        }
        publish(
            ring + plan.indices[status] * 16,
            [
                0,
                0,
                0,
                (4 << 10) | (1 << 5) | if bytes == 0 { 1 << 16 } else { 0 } | plan.cycles[status],
            ],
        );
        if bytes != 0 {
            publish(
                ring + plan.indices[1] * 16,
                [
                    buffer as u32,
                    0,
                    bytes,
                    (3 << 10) | (1 << 16) | (1 << 2) | plan.cycles[1],
                ],
            );
        }
        publish(
            ring + plan.indices[0] * 16,
            [
                request,
                (bytes << 16) | interface as u32,
                8,
                (2 << 10) | (1 << 6) | if bytes == 0 { 0 } else { 3 << 16 } | plan.cycles[0],
            ],
        );
    }
    if !(cfg!(feature = "usb-descriptor-timeout") && request == 0x01000680 && bytes == 18) {
        host.regs.write(host.doorbell + slot as usize * 4, 1)?;
    }
    let mut residual = None;
    super::event_dispatch(
        host.regs,
        host.clock,
        host.events,
        host.consumer,
        host.runtime,
        host.ac64,
        ring + plan.indices[status] * 16,
        host.ports,
        slot,
        1,
        1000,
        host.keyboard_pending,
        Some(&mut host.keyboard_saved),
        host.bulk_pending,
        Some(&mut host.bulk_saved),
        host.mouse_pending,
        Some(&mut host.mouse_saved),
        if bytes == 0 {
            None
        } else {
            Some((ring + plan.indices[1] * 16, bytes))
        },
        Some(&mut residual),
    )?;
    host.control_cursors[slot as usize] = plan.next;
    if plan.link_cycle.is_some() {
        crate::debug(b"MUSHA: EP0_RING_WRAP_OK\n");
    }
    Ok(bytes - residual.unwrap_or(0))
}
// Capture transitions, not every unchanged poll; bound journal usage per root.
fn ss_trace(port: usize, elapsed: u64, status: u32, label: &str, attempt: u8) {
    crate::diagnostics::observation(format_args!(
        "SS ROOT {} T {} {} TRY {} PORTSC {:08X}",
        port, elapsed, label, attempt, status
    ));
    crate::diagnostics::observation(format_args!(
        "CCS {} PED {} PP {} SPEED {} PLS {}",
        status & 1,
        (status >> 1) & 1,
        (status >> 9) & 1,
        (status >> 10) & 15,
        (status >> 5) & 15
    ));
}
fn settle_superspeed(
    host: &mut Host<'_>,
    protocols: &[Protocol; 256],
    op: usize,
    power_start: u64,
    grace: u64,
) -> Result<(), &'static str> {
    use musha_xhci::superspeed::{self as ss, Action, Attach};
    let ss_time = crate::timing::Span::new(host.bdf, "SS WAIT");
    let mut states = [Attach::new(); 256];
    let mut previous = [u32::MAX; 256];
    let mut changes = [0u8; 256];
    let start = host.clock.now()?;
    // Shared two-second wall-clock deadline and finite polling bound.
    for poll in 0..=ss::BUDGET_MS / ss::POLL_MS {
        let elapsed = host
            .clock
            .now()?
            .checked_sub(start)
            .ok_or("CLOCK BACKWARD")?;
        let expired = elapsed >= ss::BUDGET_MS || poll == ss::BUDGET_MS / ss::POLL_MS;
        let mut pending = false;
        for port in 1..=host.ports as usize {
            if protocols[port].major != 3 {
                continue;
            }
            let offset = op + 0x400 + (port - 1) * 16;
            let status = host.regs.read(offset)?;
            if status != previous[port] && changes[port] < 8 {
                ss_trace(port, elapsed, status, "SAMPLE", states[port].attempts);
                ss_trace(
                    port,
                    crate::bootstate::epoch_now()
                        .unwrap_or(host.route_epoch)
                        .saturating_sub(host.route_epoch),
                    status,
                    "ROUTE+ MS",
                    states[port].attempts,
                );
                changes[port] += 1;
            }
            previous[port] = status;
            let action = states[port].sample(
                status,
                if expired { ss::BUDGET_MS } else { elapsed },
                host.clock.now()?.saturating_sub(power_start),
                grace,
            );
            match action {
                Action::Ready | Action::Empty => {}
                Action::WarmReset => {
                    ss_trace(port, elapsed, status, "WPR", states[port].attempts);
                    host.regs.write(offset, ss::warm_reset(status))?;
                    ss_trace(
                        port,
                        elapsed,
                        host.regs.read(offset)?,
                        "WPR READBACK",
                        states[port].attempts,
                    );
                    pending = true;
                }
                Action::Wait => pending = true,
                Action::Expired => {}
            }
        }
        if expired || !pending {
            crate::timing::record(format_args!(
                "BDF {:04X} WPR COUNT {}",
                host.bdf,
                states.iter().map(|s| s.attempts as usize).sum::<usize>()
            ));
            ss_time.finish(if expired { "BUDGET" } else { "READY" });
            for port in 1..=host.ports as usize {
                if protocols[port].major == 3 {
                    let status = host.regs.read(op + 0x400 + (port - 1) * 16)?;
                    ss_trace(
                        port,
                        elapsed,
                        status,
                        if ss::empty(status) {
                            "EMPTY"
                        } else if ss::ready(status) {
                            "U0"
                        } else {
                            "TIMEOUT"
                        },
                        states[port].attempts,
                    );
                }
            }
            return Ok(());
        }
        host.delay(ss::POLL_MS)?;
    }
    Ok(())
}
pub(super) fn enumerate(
    host: &mut Host<'_>,
    pool: &mut musha_xhci::Pool,
    dcbaa: usize,
    discovery: bool,
    tick: &mut dyn FnMut(super::AppEvent<'_>) -> Result<bool, &'static str>,
) -> Result<(bool, bool), &'static str> {
    let protocols = protocols(host.regs, host.ports)?;
    let stride = if host.regs.read(0x10)? & 4 != 0 {
        64
    } else {
        32
    };
    let op = (host.regs.read(0)? & 255) as usize;
    let power_start = host.clock.now()?;
    // During rescan PP has remained on throughout the first pass and grace.
    let grace = if super::RESCAN_DEADLINE.load(core::sync::atomic::Ordering::Relaxed) != 0 {
        0
    } else {
        musha_xhci::superspeed::grace_ms(host.routed)
    };
    // Ensure port power before sampling attach; no change/status bits replayed.
    for port in 1..=host.ports as usize {
        if protocols[port].major == 0 {
            continue;
        }
        let offset = op + 0x400 + (port - 1) * 16;
        let status = host.regs.read(offset)?;
        if status & (1 << 9) == 0 {
            host.regs
                .write(offset, musha_xhci::port_control(status) | (1 << 9))?;
        }
    }
    // One shared upper bound for all ports. Observe CCS/PLS after power and
    // routing changes rather than declaring absent after one 20 ms sample.
    // No PORTSC writes here: change bits and reset policy remain untouched.
    let attach_time = crate::timing::Span::new(host.bdf, "ATTACH WAIT");
    crate::debug(b"MUSHA: ATTACH WAIT START\n");
    let mut waited = 0;
    let mut last = [u32::MAX; 256];
    let mut changes = [0u8; 256];
    for _ in 0..musha_xhci::intel_ports::ATTACH_POLLS {
        waited = host.clock.now()?.saturating_sub(power_start);
        if waited >= 1000 {
            break;
        }
        let mut pending = false;
        for port in 1..=host.ports as usize {
            let status = host.regs.read(op + 0x400 + (port - 1) * 16)?;
            if protocols[port].major == 3 && status != last[port] && changes[port] < 8 {
                let route_ms = crate::bootstate::epoch_now()
                    .unwrap_or(host.route_epoch)
                    .saturating_sub(host.route_epoch);
                ss_trace(port, route_ms, status, "ROUTE+ MS", 0);
                changes[port] += 1;
            }
            last[port] = status;
            pending |= musha_xhci::intel_ports::attach_pending(
                status,
                protocols[port].major,
                waited,
                grace,
            );
        }
        if !pending {
            break;
        }
        host.delay(musha_xhci::intel_ports::ATTACH_POLL_MS)?;
        waited += musha_xhci::intel_ports::ATTACH_POLL_MS;
    }
    crate::diagnostics::observation(format_args!("ATTACH WAIT {} MS MAX 1000", waited));
    attach_time.finish(if waited >= 1000 { "BUDGET" } else { "READY" });
    settle_superspeed(host, &protocols, op, power_start, grace)?;
    let enum_time = crate::timing::Span::new(host.bdf, "ENUMERATE");
    let mut count = 0u64;
    let mut active_keyboard = None;
    let mut active_mouse = None;
    let mut active_storage = None;
    let mut targets = [None; 32];
    let mut tail = 0;
    let mut retained_hubs = [None; 8];
    let mut hub_count = 0;
    for port in 1..=host.ports as usize {
        let status = host.regs.read(op + 0x400 + (port - 1) * 16)?;
        crate::diagnostics::observation(format_args!(
            "ROOT {} USB{} PORTSC {:08X}",
            port, protocols[port].major, status
        ));
        crate::diagnostics::observation(format_args!(
            "CCS {} PED {} PP {} SPEED {} PLS {}",
            status & 1,
            (status >> 1) & 1,
            (status >> 9) & 1,
            (status >> 10) & 15,
            (status >> 5) & 15
        ));
        crate::diagnostics::observation(format_args!(
            "ROOT {} {}",
            port,
            if protocols[port].major == 0 {
                "SKIP NO PROTOCOL"
            } else if status & 1 == 0 {
                "SKIP DISCONNECTED"
            } else if protocols[port].major == 3 && !musha_xhci::superspeed::ready(status) {
                "SKIP SS NOT U0"
            } else {
                "QUEUE ENUMERATION"
            }
        ));
        if protocols[port].major != 0
            && status & 1 != 0
            && (protocols[port].major != 3 || musha_xhci::superspeed::ready(status))
        {
            if tail == targets.len() {
                return Err("DEVICE LIMIT");
            }
            targets[tail] = Some(hub::Target {
                path: musha_xhci::hub::Path::root(port as u8).ok_or("HUB ROUTE")?,
                port: port as u8,
                protocol: protocols[port],
                parent: None,
                ancestors: [None; 5],
            });
            tail += 1;
        }
    }
    let mut head = 0;
    while head < tail {
        let target = targets[head].take().ok_or("HUB QUEUE")?;
        head += 1;
        let protocol = target.protocol;
        let port = target.path.root_port as usize;
        crate::diagnostics::observation(format_args!("ENUM ROOT {} PORT RESET", port));
        crate::diagnostics::usb_stage("PORT RESET");
        let (speed_id, speed, path) = if let Some(parent) = target.parent {
            let speed = hub::reset_port(host, parent, target.port)?;
            let speed_id = hub::speed_id(host.regs, protocol, speed)?;
            let path = parent
                .path
                .child(target.port, parent.slot, parent.speed, speed, false)
                .ok_or("HUB ROUTE")?;
            (speed_id, speed, path)
        } else {
            let (id, speed) = port_reset(host, port, protocol)?;
            (id, speed, target.path)
        };
        let output = pool.allocate(4096, 4096).ok_or("DMA FULL")?;
        let input = pool.allocate(4096, 4096).ok_or("DMA FULL")?;
        let ring = pool.allocate(4096, 4096).ok_or("DMA FULL")?;
        let buffer = pool.allocate(1024, 64).ok_or("DMA FULL")?;
        host.attempted = true;
        let enabled = host.command(0, (9 << 10) | ((protocol.slot as u32) << 16), 255)?;
        let slot = (enabled[3] >> 24) as u8;
        if slot as u32 > (host.regs.read(4)? & 255).min(8) {
            return Err("SLOT RANGE");
        }
        crate::diagnostics::set(
            23,
            format_args!(
                "TARGET ROOT {:02X} RT{:05X} SLOT {} SPEED {:?}",
                port, path.route, slot, speed
            ),
        );
        host.control_cursors[slot as usize] = musha_xhci::control::Cursor::NEW;
        let (slot_words, ep_words) = path
            .slot_words(speed_id, speed.packet(), ring)
            .ok_or("CONTEXT")?;
        unsafe {
            dma_u64(dcbaa + slot as usize * 8, output as u64);
            dma_word(input + 4, 3); // Add Slot and EP0, no drops
            for (i, word) in slot_words.iter().enumerate() {
                dma_word(input + stride + i * 4, *word);
            }
            for (i, word) in ep_words.iter().enumerate() {
                dma_word(input + 2 * stride + i * 4, *word);
            }
            publish(ring + (TRBS - 1) * 16, [ring as u32, 0, 0, (6 << 10) | 3]);
            core::arch::asm!("mfence", options(nostack));
        }
        crate::diagnostics::usb_stage("SLOT / ADDRESS");
        crate::diagnostics::observation(format_args!(
            "ADDRESS ROOT {} SLOT {} SPEED ID {} {:?}",
            port, slot, speed_id, speed
        ));
        crate::diagnostics::observation(format_args!(
            "SLOT SPEED ID {} EP0 PACKET {}",
            (slot_words[0] >> 20) & 15,
            speed.packet()
        ));
        #[cfg(feature = "diagnostics-detail")]
        {
            crate::diagnostics::observation(format_args!("INPUT {:016X} STRIDE {}", input, stride));
            for (i, word) in slot_words.iter().enumerate() {
                crate::diagnostics::observation(format_args!("SLOT DW{} {:08X}", i, word));
            }
            for (i, word) in ep_words.iter().enumerate() {
                crate::diagnostics::observation(format_args!("EP0 DW{} {:08X}", i, word));
            }
        }
        #[cfg(feature = "usb-address-reject")]
        if head == 1 {
            // The xHC must return an error: required Add Context flags are absent.
            unsafe {
                dma_word(input + 4, 0);
                core::arch::asm!("mfence", options(nostack));
            }
            crate::debug(b"MUSHA: ADDRESS_REJECT_INJECTED\n");
        }
        // BSR=0: xHC sends SET_ADDRESS. Only a consumed, correlated device
        // error is recoverable; timeout/malformed events still stop all DMA.
        if let Err(error) = host.command(input, (11 << 10) | ((slot as u32) << 24), slot) {
            crate::diagnostics::observation(format_args!(
                "ADDRESS FAILED ROOT {} SLOT {} {:?} {}",
                port, slot, speed, error
            ));
            crate::debug(b"MUSHA: ADDRESS_FAILED ROOT=");
            crate::debug(&crate::cpu::hex(port as u64));
            crate::debug(b"\n");
            if error != "COMMAND REJECTED" {
                return Err(error);
            }
            host.command(0, (10 << 10) | ((slot as u32) << 24), slot)?;
            unsafe {
                dma_u64(dcbaa + slot as usize * 8, 0);
                core::arch::asm!("mfence", options(nostack));
            }
            // Keep failed-device allocations reserved until controller teardown.
            crate::diagnostics::observation(format_args!("ROOT {} SLOT DISABLED / CONTINUE", port));
            crate::debug(b"MUSHA: ADDRESS_SLOT_DISABLED_CONTINUE\n");
            continue;
        }
        let state = unsafe { read_word(output + 12) };
        if state >> 27 != 2 || state & 255 == 0 || state & 255 > 127 {
            return Err("ADDRESS STATE");
        }
        crate::diagnostics::usb_stage("DEVICE DESCRIPTOR");
        get_descriptor(host, slot, ring, buffer, 8)?;
        let mut prefix = [0u8; 8];
        for (i, b) in prefix.iter_mut().enumerate() {
            *b = unsafe { (buffer as *const u8).add(i).read_volatile() };
        }
        if prefix[0] != 18 || prefix[1] != 1 {
            return Err("DEVICE DESCRIPTOR");
        }
        let packet = speed.descriptor_packet(prefix[7]).ok_or("EP0 PACKET")?;
        if packet != speed.packet() {
            unsafe {
                dma_word(input + 4, 2); // Evaluate only EP0
                for i in 0..stride / 4 {
                    dma_word(
                        input + 2 * stride + i * 4,
                        read_word(output + stride + i * 4),
                    );
                }
                let previous = read_word(input + 2 * stride + 4);
                dma_word(
                    input + 2 * stride + 4,
                    (previous & 0xffff) | ((packet as u32) << 16),
                );
                core::arch::asm!("mfence", options(nostack));
            }
            host.command(input, (13 << 10) | ((slot as u32) << 24), slot)?;
        }
        crate::diagnostics::usb_stage("DEVICE DESCRIPTOR");
        get_descriptor(host, slot, ring, buffer, 18)?;
        let mut descriptor = [0u8; 18];
        for (i, b) in descriptor.iter_mut().enumerate() {
            *b = unsafe { (buffer as *const u8).add(i).read_volatile() };
        }
        let (vendor, product) =
            musha_xhci::descriptor(&descriptor, speed).ok_or("DEVICE DESCRIPTOR")?;
        if descriptor[7] != prefix[7] {
            return Err("DESCRIPTOR CHANGED");
        }
        #[cfg(feature = "usb-control-probe")]
        {
            for _ in 0..300 {
                get_descriptor(host, slot, ring, buffer, 18)?;
            }
            let actual = control_read(host, slot, ring, buffer, 64, 0x01000680, 0)?;
            if actual != 18 {
                return Err("EP0 SHORT PROBE");
            }
            crate::debug(b"MUSHA: EP0_SHORT_OK BYTES=18 STATUS_COMPLETE\n");
        }
        crate::diagnostics::usb(
            port,
            path.route,
            slot,
            vendor,
            product,
            match speed {
                musha_xhci::Speed::Low => "LOW",
                musha_xhci::Speed::Full => "FULL",
                musha_xhci::Speed::High => "HIGH",
                musha_xhci::Speed::Super => "SUPER",
            },
            [descriptor[4], descriptor[5], descriptor[6]],
        );
        crate::debug(b"MUSHA: USB_DEVICE PORT=");
        crate::debug(&crate::cpu::hex(port as u64));
        crate::debug(b" ROUTE=");
        crate::debug(&crate::cpu::hex(path.route as u64));
        crate::debug(b" SLOT=");
        crate::debug(&crate::cpu::hex(slot as u64));
        crate::debug(b" ADDRESS=");
        crate::debug(&crate::cpu::hex((state & 255) as u64));
        crate::debug(b" VID=");
        crate::debug(&crate::cpu::hex(vendor as u64));
        crate::debug(b" PID=");
        crate::debug(&crate::cpu::hex(product as u64));
        crate::debug(b"\n");
        crate::diagnostics::usb_stage("CONFIGURATION");
        let is_hub = descriptor[4] == 9;
        if is_hub && !musha_xhci::hub::device_supported(&descriptor, speed) {
            crate::diagnostics::unsupported_hub(u16::from_le_bytes([descriptor[2], descriptor[3]]));
            host.command(0, (10 << 10) | ((slot as u32) << 24), slot)?;
            unsafe {
                dma_u64(dcbaa + slot as usize * 8, 0);
                core::arch::asm!("mfence", options(nostack));
            }
            count += 1;
            continue;
        }
        let configured = configure(
            host, pool, slot, input, output, stride, ring, buffer, speed, port, descriptor, tick,
        )?;
        if let Configured::Hub(configuration) = configured {
            let hub = hub::setup(
                host,
                pool,
                slot,
                input,
                output,
                stride,
                ring,
                buffer,
                speed,
                path,
                configuration,
            )?;
            if hub_count == retained_hubs.len() {
                return Err("HUB LIMIT");
            }
            retained_hubs[hub_count] = Some(hub);
            hub_count += 1;
            for child_port in 1..=hub.ports {
                let status = hub::status(host, hub, child_port)?;
                if status.overcurrent() {
                    return Err("HUB OVERCURRENT");
                }
                if status.connected() {
                    if tail == targets.len() {
                        return Err("DEVICE LIMIT");
                    }
                    if path.depth >= 5 {
                        return Err("HUB DEPTH LIMIT");
                    }
                    let mut ancestors = target.ancestors;
                    ancestors[path.depth as usize] = Some((hub, child_port));
                    targets[tail] = Some(hub::Target {
                        path,
                        port: child_port,
                        protocol,
                        parent: Some(hub),
                        ancestors,
                    });
                    tail += 1;
                }
            }
            count += 1;
            continue;
        }
        if let Configured::Storage(mut device) = configured {
            if !discovery
                && active_storage.is_none()
                && (device.prepared.is_some() || cfg!(feature = "live-storage-probe"))
            {
                device.ancestors = target.ancestors;
                active_storage = Some(device);
                crate::debug(b"MUSHA: STORAGE_RETAINED\n");
                count += 1;
                continue;
            }
        } else if let Configured::Keyboard(mut keyboard) = configured {
            keyboard.ancestors = target.ancestors;
            if keyboard.mouse.is_some() && active_mouse.is_none() {
                active_mouse = Some(keyboard);
                count += 1;
                continue;
            }
            if keyboard.mouse.is_none() && active_keyboard.is_none() {
                active_keyboard = Some(keyboard);
                count += 1;
                continue;
            }
            crate::debug(b"MUSHA: HID_ADDITIONAL_UNSUPPORTED\n");
        }
        // Release slots not retained for the live loop only after a
        // Disable Slot completion. The DMA allocation itself is never reused.
        host.command(0, (10 << 10) | ((slot as u32) << 24), slot)?;
        if host
            .keyboard_pending
            .is_some_and(|(_, pending, _)| pending == slot)
        {
            host.keyboard_pending = None;
            host.keyboard_saved = None;
        }
        unsafe {
            dma_u64(dcbaa + slot as usize * 8, 0);
            core::arch::asm!("mfence", options(nostack));
        }
        count += 1;
    }
    enum_time.finish("OK");
    crate::debug(b"MUSHA: USB_ENUMERATION_OK COUNT=");
    crate::debug(&crate::cpu::hex(count));
    crate::debug(b"\nMUSHA: USB_DISCOVERY_DONE\n");
    let found_keyboard = active_keyboard.is_some();
    let found_mouse = active_mouse.is_some();
    if !discovery && !found_keyboard && active_storage.is_none() {
        tick(super::AppEvent::Ready(found_keyboard))?;
    }
    if !discovery && !found_keyboard && active_mouse.is_none() {
        if let Some(device) = active_storage.as_mut() {
            let mut session = device.prepared.take().map(file::Session::new).transpose()?;
            let mut poll = |tick: &mut super::Tick<'_>| {
                if let Some(session) = session.as_mut() {
                    session.step(host, device, tick)
                } else {
                    Ok(false)
                }
            };
            tick(super::AppEvent::Retained(&mut poll))?;
        }
    }
    if let Some(keyboard) = active_keyboard {
        let slot = keyboard.slot;
        crate::diagnostics::usb_stage("KEYBOARD POLLING");
        if !discovery {
            run_keyboard(
                host,
                keyboard,
                active_mouse.as_ref(),
                active_storage.as_mut(),
                tick,
            )?;
        }
        host.command(0, (10 << 10) | ((slot as u32) << 24), slot)?;
        if host
            .keyboard_pending
            .is_some_and(|(_, pending, _)| pending == slot)
        {
            host.keyboard_pending = None;
            host.keyboard_saved = None;
        }
        unsafe {
            dma_u64(dcbaa + slot as usize * 8, 0);
            core::arch::asm!("mfence", options(nostack));
        }
    }
    if let Some(mouse) = active_mouse {
        if !discovery && !found_keyboard {
            run_pointer(host, &mouse, active_storage.as_mut(), tick)?;
        }
        host.command(0, (10 << 10) | ((mouse.slot as u32) << 24), mouse.slot)?;
        host.mouse_pending = None;
        host.mouse_saved = None;
        unsafe {
            dma_u64(dcbaa + mouse.slot as usize * 8, 0);
        }
    }
    if let Some(mut device) = active_storage {
        // Disable Slot completion is the ownership boundary. On any error the
        // outer controller stop disables all DMA before this arena is reused.
        let slot = device.slot();
        host.command(0, (10 << 10) | ((slot as u32) << 24), slot)?;
        device.stopped(host);
        unsafe {
            dma_u64(dcbaa + slot as usize * 8, 0);
        }
        crate::debug(b"MUSHA: STORAGE_RELEASED\n");
    }
    for hub in retained_hubs[..hub_count].iter().rev().flatten() {
        host.command(0, (10 << 10) | ((hub.slot as u32) << 24), hub.slot)?;
        unsafe {
            dma_u64(dcbaa + hub.slot as usize * 8, 0);
            core::arch::asm!("mfence", options(nostack));
        }
    }
    Ok((found_keyboard, found_mouse))
}

struct Keyboard {
    ancestors: [Option<(hub::Hub, u8)>; 5],
    slot: u8,
    endpoint: u8,
    ring: usize,
    report: usize,
    port: usize,
    mouse: Option<musha_xhci::mouse::Layout>,
}
enum Configured {
    Other,
    Storage(storage::Device),
    Keyboard(Keyboard),
    Hub(musha_xhci::hub::Configuration),
}
fn configure(
    host: &mut Host<'_>,
    pool: &mut musha_xhci::Pool,
    slot: u8,
    input: usize,
    output: usize,
    stride: usize,
    control_ring: usize,
    buffer: usize,
    speed: musha_xhci::Speed,
    port: usize,
    descriptor: [u8; 18],
    tick: &mut dyn FnMut(super::AppEvent<'_>) -> Result<bool, &'static str>,
) -> Result<Configured, &'static str> {
    // Only configuration zero is inspected at this diagnostic stage.
    control(host, slot, control_ring, buffer, 9, 0x02000680, 0)?;
    let mut bytes = [0u8; 1024];
    for (i, b) in bytes[..9].iter_mut().enumerate() {
        *b = unsafe { (buffer as *const u8).add(i).read_volatile() };
    }
    let length = u16::from_le_bytes([bytes[2], bytes[3]]) as usize;
    if bytes[0] != 9 || bytes[1] != 2 || !(9..=1024).contains(&length) {
        return Err("CONFIG HEADER");
    }
    control(
        host,
        slot,
        control_ring,
        buffer,
        length as u32,
        0x02000680,
        0,
    )?;
    for (i, b) in bytes[..length].iter_mut().enumerate() {
        *b = unsafe { (buffer as *const u8).add(i).read_volatile() };
    }
    crate::diagnostics::set(
        22,
        format_args!(
            "CONFIG S{} VALUE {} IFACES {} BYTES {}",
            slot, bytes[5], bytes[4], length
        ),
    );
    let kind = musha_platform::diagnostics::configuration_kind(&bytes[..length])
        .ok_or("CONFIG DIAGNOSTIC CHAIN")?;
    crate::diagnostics::usb_kind(kind);
    crate::debug(b"MUSHA: USB_INTERFACE_KIND ");
    crate::debug(kind.as_bytes());
    crate::debug(b"\n");
    if let Some(configuration) = musha_xhci::hub::configuration(&bytes[..length], speed)? {
        if !musha_xhci::hub::device_supported(&descriptor, speed) {
            crate::diagnostics::unsupported_hub(u16::from_le_bytes([descriptor[2], descriptor[3]]));
            return Ok(Configured::Other);
        }
        return Ok(Configured::Hub(configuration));
    }
    if let Some(disk) = musha_xhci::storage::configuration(&bytes[..length], speed)? {
        crate::diagnostics::usb_kind("BOT STORAGE MATCH / CHECK READ");
        crate::diagnostics::supported_device(false);
        host.storage_matches += 1;
        return storage::probe(
            host,
            pool,
            slot,
            input,
            output,
            stride,
            control_ring,
            buffer,
            disk,
            tick,
        )
        .map(|device| Configured::Storage(device.with_port(port)));
    }
    let keyboard = musha_xhci::keyboard::configuration(&bytes[..length], speed)?;
    let pointer = if keyboard.is_none() {
        musha_xhci::mouse::configuration(&bytes[..length], speed)?
    } else {
        None
    };
    let Some(kbd) = keyboard.or(pointer) else {
        return Ok(Configured::Other);
    };
    let mouse = pointer.is_some();
    crate::diagnostics::usb_kind(if mouse {
        "BOOT MOUSE MATCH / CHECK RELATIVE"
    } else {
        "BOOT KEYBOARD MATCH / CHECK INPUT"
    });
    if !mouse {
        crate::diagnostics::supported_device(true);
    }
    let endpoint = kbd.endpoint * 2 + 1;
    let ring = pool.allocate(4096, 4096).ok_or("DMA FULL")?;
    let report = pool.allocate(64, 64).ok_or("DMA FULL")?;
    unsafe {
        core::ptr::write_bytes(input as *mut u8, 0, 4096);
        dma_word(input + 4, 1 | (1 << endpoint));
        for i in 0..3 {
            dma_word(input + stride + i * 4, read_word(output + i * 4));
        }
        let previous = read_word(input + stride);
        dma_word(
            input + stride,
            (previous & !(31 << 27)) | ((endpoint as u32) << 27),
        );
        let ep = input + (endpoint as usize + 1) * stride;
        dma_word(ep, (kbd.interval as u32) << 16);
        dma_word(ep + 4, (3 << 1) | (7 << 3) | ((kbd.packet as u32) << 16));
        dma_word(ep + 8, ring as u32 | 1);
        let payload = if mouse { 4 } else { 8 };
        dma_word(ep + 16, payload | (payload << 16));
        publish(ring + (TRBS - 1) * 16, [ring as u32, 0, 0, (6 << 10) | 3]);
        core::arch::asm!("mfence", options(nostack));
    }
    host.command(input, (12 << 10) | ((slot as u32) << 24), slot)?;
    control(
        host,
        slot,
        control_ring,
        buffer,
        0,
        ((kbd.configuration as u32) << 16) | 0x0900,
        0,
    )?;
    control(
        host,
        slot,
        control_ring,
        buffer,
        0,
        0x0b21,
        kbd.interface as u16,
    )?;
    let mut layout = if mouse {
        Some(musha_xhci::mouse::Layout {
            bytes: 3,
            wheel: false,
            buttons: 3,
        })
    } else {
        None
    };
    if mouse {
        let mut at = 9;
        let mut selected = false;
        let mut report_length = 0usize;
        while at < length {
            let n = bytes[at] as usize;
            if bytes[at + 1] == 4 {
                selected = bytes[at + 2] == kbd.interface;
            }
            if selected && bytes[at + 1] == 0x21 {
                report_length = u16::from_le_bytes([bytes[at + 7], bytes[at + 8]]) as usize;
            }
            at += n;
        }
        if (1..=256).contains(&report_length) {
            let got = control_read(
                host,
                slot,
                control_ring,
                buffer,
                report_length as u32,
                0x22000681,
                kbd.interface as u16,
            )?;
            let mut descriptor = [0; 256];
            for (i, byte) in descriptor[..got as usize].iter_mut().enumerate() {
                *byte = unsafe { (buffer as *const u8).add(i).read_volatile() };
            }
            if let Some(parsed) = musha_xhci::mouse::layout(&descriptor[..got as usize]) {
                control(
                    host,
                    slot,
                    control_ring,
                    buffer,
                    0,
                    0x00010b21,
                    kbd.interface as u16,
                )?;
                layout = Some(parsed);
            }
        }
        crate::debug(b"MUSHA: MOUSE_RELATIVE_LAYOUT BYTES/WHEEL=");
        let l = layout.unwrap();
        crate::debug(&crate::cpu::hex(l.bytes as u64));
        crate::debug(&crate::cpu::hex(l.wheel as u64));
        crate::debug(b"\n");
    }
    Ok(Configured::Keyboard(Keyboard {
        mouse: layout,
        ancestors: [None; 5],
        slot,
        endpoint,
        ring,
        report,
        port,
    }))
}
fn run_keyboard(
    host: &mut Host<'_>,
    keyboard: Keyboard,
    mouse: Option<&Keyboard>,
    mut storage: Option<&mut storage::Device>,
    tick: &mut dyn FnMut(super::AppEvent<'_>) -> Result<bool, &'static str>,
) -> Result<(), &'static str> {
    let Keyboard {
        ancestors,
        slot,
        endpoint,
        ring,
        report,
        port,
        mouse: _,
    } = keyboard;
    crate::debug(
        b"MUSHA: HID_READY
",
    );
    unsafe {
        host.framebuffer.text_grid(
            "KEYBOARD READY",
            grid::LABEL,
            grid::KEYBOARD,
            host.framebuffer.color(0, 240, 100),
        );
    }

    let mut file_session = match storage.as_deref_mut().and_then(|d| d.prepared.take()) {
        Some(prepared) => Some(file::Session::new(prepared)?),
        None => None,
    };
    tick(super::AppEvent::Ready(true))?;
    let op = (host.regs.read(0)? & 255) as usize;
    let deadline = if cfg!(feature = "qemu-debug") && !cfg!(feature = "input-persistent") {
        host.clock
            .now()?
            .checked_add(10000)
            .ok_or("CLOCK OVERFLOW")?
    } else {
        u64::MAX
    };
    let mut state = musha_xhci::keyboard::State::default();
    let mut producer = musha_xhci::Cursor::new(TRBS - 1).ok_or("CURSOR")?;
    let mut pending = false;
    let mut reports = 0;
    let mut check_at = host.clock.now()?;
    let mut mouse_state = PointerState::default();
    let mut poll = |tick: &mut super::Tick<'_>| -> Result<bool, &'static str> {
        if host.regs.read(op + 0x400 + (port - 1) * 16)? & 1 == 0 {
            return Err("KEYBOARD DISCONNECTED");
        }
        if host.clock.now()? >= deadline {
            return Ok(true);
        }
        if host.clock.now()? >= check_at {
            for (hub, port) in ancestors.iter().flatten().copied() {
                let status = hub::status(host, hub, port)?;
                if status.overcurrent() {
                    return Err("HUB OVERCURRENT");
                }
                if !status.connected()
                    || status.ready_speed().is_none()
                    || status.connection_changed()
                    || status.configuration_error()
                {
                    return Err("KEYBOARD DISCONNECTED");
                }
            }
            check_at = host.clock.now()?.checked_add(100).ok_or("CLOCK OVERFLOW")?;
        }
        if !pending {
            unsafe {
                core::ptr::write_bytes(report as *mut u8, 0, 64);
                if producer.index == TRBS - 2 {
                    publish(
                        ring + (TRBS - 1) * 16,
                        [ring as u32, 0, 0, (6 << 10) | 2 | producer.cycle],
                    );
                }
                publish(
                    ring + producer.index * 16,
                    [report as u32, 0, 8, (1 << 10) | (1 << 5) | producer.cycle],
                );
            }
            host.regs
                .write(host.doorbell + slot as usize * 4, endpoint as u32)?;
            pending = true;
            host.keyboard_pending = Some((ring + producer.index * 16, slot, endpoint));
        }
        let result = if let Some(words) = host.keyboard_saved.take() {
            Ok(words)
        } else {
            super::event_dispatch(
                host.regs,
                host.clock,
                host.events,
                host.consumer,
                host.runtime,
                host.ac64,
                ring + producer.index * 16,
                host.ports,
                slot,
                endpoint,
                0,
                None,
                None,
                host.bulk_pending,
                Some(&mut host.bulk_saved),
                host.mouse_pending,
                Some(&mut host.mouse_saved),
                None,
                None,
            )
        };
        let mut transitions = [(0u8, false); 20];
        let mut count = 0;
        match result {
            Err("EVENT PENDING") => {}
            Err(error) => return Err(error),
            Ok(_) => {
                let mut data = [0u8; 8];
                for (i, b) in data.iter_mut().enumerate() {
                    *b = unsafe { (report as *const u8).add(i).read_volatile() };
                }
                if !state.update(data, |key, down| {
                    transitions[count] = (key, down);
                    count += 1;
                    crate::debug(if down {
                        b"MUSHA: HID_KEY_DOWN="
                    } else {
                        b"MUSHA: HID_KEY_UP="
                    });
                    crate::debug(&crate::cpu::hex(key as u64));
                    crate::debug(b"\n");
                }) {
                    crate::debug(b"MUSHA: HID_ROLLOVER\n");
                }
                pending = false;
                host.keyboard_pending = None;
                producer.advance();
                if producer.index == 0 {
                    crate::debug(b"MUSHA: HID_RING_WRAP_OK\n");
                }
                reports += 1;
            }
        }
        if let (Some(session), Some(device)) = (file_session.as_mut(), storage.as_deref_mut()) {
            if session.step(host, device, tick)? {
                return Ok(true);
            }
        }
        if cfg!(feature = "live-storage-probe") && !cfg!(feature = "live-file-probe") {
            if let Some(device) = storage.as_deref_mut() {
                device.probe_step(host)?;
            }
        }
        if let Some(mouse) = mouse {
            poll_mouse(host, mouse, &mut mouse_state, tick)?;
        }
        // Hardware poll never waits for a key. Each iteration also advances the
        // application, including while the device NAKs an outstanding transfer.
        if tick(super::AppEvent::Keys(&transitions[..count]))? {
            return Ok(true);
        }
        Ok(false)
    };
    tick(super::AppEvent::InputPoll(&mut poll))?;
    while !poll(tick)? {}
    drop(poll);
    crate::debug(b"MUSHA: HID_DIAGNOSTIC_OK REPORTS=");
    crate::debug(&crate::cpu::hex(reports));
    crate::debug(
        b"
",
    );
    Ok(())
}

struct PointerState {
    producer: musha_xhci::Cursor,
    pending: bool,
    buttons: u32,
    check_at: u64,
}
impl Default for PointerState {
    fn default() -> Self {
        Self {
            producer: musha_xhci::Cursor::new(TRBS - 1).unwrap(),
            pending: false,
            buttons: 0,
            check_at: 0,
        }
    }
}
fn poll_mouse(
    host: &mut Host<'_>,
    mouse: &Keyboard,
    state: &mut PointerState,
    tick: &mut super::Tick<'_>,
) -> Result<(), &'static str> {
    let layout = mouse.mouse.ok_or("MOUSE FORMAT")?;
    let op = (host.regs.read(0)? & 255) as usize;
    if host.regs.read(op + 0x400 + (mouse.port - 1) * 16)? & 1 == 0 {
        return Err("MOUSE DISCONNECTED");
    }
    if host.clock.now()? >= state.check_at {
        for (hub, port) in mouse.ancestors.iter().flatten().copied() {
            let status = hub::status(host, hub, port)?;
            if status.overcurrent()
                || !status.connected()
                || status.ready_speed().is_none()
                || status.connection_changed()
                || status.configuration_error()
            {
                return Err("MOUSE DISCONNECTED");
            }
        }
        state.check_at = host.clock.now()?.checked_add(100).ok_or("CLOCK OVERFLOW")?;
    }
    if !state.pending {
        unsafe {
            core::ptr::write_bytes(mouse.report as *mut u8, 0, 64);
            if state.producer.index == TRBS - 2 {
                publish(
                    mouse.ring + (TRBS - 1) * 16,
                    [
                        mouse.ring as u32,
                        0,
                        0,
                        (6 << 10) | 2 | state.producer.cycle,
                    ],
                );
            }
            publish(
                mouse.ring + state.producer.index * 16,
                [
                    mouse.report as u32,
                    0,
                    layout.bytes as u32,
                    (1 << 10) | (1 << 5) | state.producer.cycle,
                ],
            );
        }
        host.regs.write(
            host.doorbell + mouse.slot as usize * 4,
            mouse.endpoint as u32,
        )?;
        host.mouse_pending = Some((
            mouse.ring + state.producer.index * 16,
            mouse.slot,
            mouse.endpoint,
        ));
        state.pending = true;
    }
    let result = if let Some(words) = host.mouse_saved.take() {
        Ok(words)
    } else {
        super::event_dispatch(
            host.regs,
            host.clock,
            host.events,
            host.consumer,
            host.runtime,
            host.ac64,
            mouse.ring + state.producer.index * 16,
            host.ports,
            mouse.slot,
            mouse.endpoint,
            0,
            host.keyboard_pending,
            Some(&mut host.keyboard_saved),
            host.bulk_pending,
            Some(&mut host.bulk_saved),
            None,
            None,
            None,
            None,
        )
    };
    match result {
        Err("EVENT PENDING") => {}
        Err(error) => return Err(error),
        Ok(_) => {
            let mut bytes = [0; 4];
            for (i, byte) in bytes[..layout.bytes].iter_mut().enumerate() {
                *byte = unsafe { (mouse.report as *const u8).add(i).read_volatile() };
            }
            let (values, buttons) =
                musha_xhci::mouse::decode(layout, &bytes[..layout.bytes]).ok_or("MOUSE REPORT")?;
            host.mouse_pending = None;
            state.pending = false;
            state.producer.advance();
            if values != [0; 3] || buttons != state.buttons {
                tick(super::AppEvent::Pointer(values, buttons))?;
            }
            state.buttons = buttons;
        }
    }
    Ok(())
}

fn run_pointer(
    host: &mut Host<'_>,
    mouse: &Keyboard,
    mut storage: Option<&mut storage::Device>,
    tick: &mut super::Tick<'_>,
) -> Result<(), &'static str> {
    let mut files = storage
        .as_deref_mut()
        .and_then(|d| d.prepared.take())
        .map(file::Session::new)
        .transpose()?;
    let mut state = PointerState::default();
    tick(super::AppEvent::Ready(false))?; // Mouse is never reported as a keyboard.
    crate::debug(b"MUSHA: HID_MOUSE_READY NO_KEYBOARD\n");
    let mut poll = |tick: &mut super::Tick<'_>| {
        poll_mouse(host, mouse, &mut state, tick)?;
        if let (Some(session), Some(device)) = (files.as_mut(), storage.as_deref_mut()) {
            if session.step(host, device, tick)? {
                return Ok(true);
            }
        }
        tick(super::AppEvent::Keys(&[]))
    };
    tick(super::AppEvent::InputPoll(&mut poll))?;
    while !poll(tick)? {}
    Ok(())
}
