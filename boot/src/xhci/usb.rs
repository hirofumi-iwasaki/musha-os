// Copyright 2026 Hirofumi Iwasaki
// SPDX-License-Identifier: Apache-2.0
mod storage;
use super::{Host, Registers, TRBS, dma_u64, event, publish, wait};

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
    index: usize,
    buffer: usize,
    bytes: u32,
) -> Result<(), &'static str> {
    control(host, slot, ring, index, buffer, bytes, 0x01000680, 0)
}
fn control(
    host: &mut Host<'_>,
    slot: u8,
    ring: usize,
    index: usize,
    buffer: usize,
    bytes: u32,
    request: u32,
    interface: u16,
) -> Result<(), &'static str> {
    if index + 3 >= TRBS || bytes > 1024 {
        return Err("TRANSFER RANGE");
    }
    unsafe {
        core::ptr::write_bytes(buffer as *mut u8, 0, bytes as usize);
        // Publish backward so the first Setup cycle is visible only once the
        // complete Setup/Data/Status sequence exists. Only one TD is in flight.
        let status_index = index + if bytes == 0 { 1 } else { 2 };
        publish(
            ring + status_index * 16,
            [
                0,
                0,
                0,
                (4 << 10) | (1 << 5) | if bytes == 0 { 1 << 16 } else { 0 } | 1,
            ],
        );
        if bytes != 0 {
            publish(
                ring + (index + 1) * 16,
                [buffer as u32, 0, bytes, (3 << 10) | (1 << 16) | 1],
            );
        }
        publish(
            ring + index * 16,
            [
                request,
                (bytes << 16) | interface as u32,
                8,
                (2 << 10) | (1 << 6) | if bytes == 0 { 0 } else { 3 << 16 } | 1,
            ],
        );
    }
    if !(cfg!(feature = "usb-descriptor-timeout") && request == 0x01000680 && bytes == 18) {
        host.regs.write(host.doorbell + slot as usize * 4, 1)?;
    }
    event(
        host.regs,
        host.clock,
        host.events,
        host.consumer,
        host.runtime,
        host.ac64,
        ring + (index + if bytes == 0 { 1 } else { 2 }) * 16,
        host.ports,
        slot,
        1,
        1000,
    )?;
    Ok(())
}
pub(super) fn enumerate(
    host: &mut Host<'_>,
    pool: &mut musha_xhci::Pool,
    dcbaa: usize,
    tick: &mut dyn FnMut(super::AppEvent<'_>) -> Result<bool, &'static str>,
) -> Result<(), &'static str> {
    let protocols = protocols(host.regs, host.ports)?;
    let stride = if host.regs.read(0x10)? & 4 != 0 {
        64
    } else {
        32
    };
    let op = (host.regs.read(0)? & 255) as usize;
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
    host.delay(20)?;
    let mut count = 0u64;
    for port in 1..=host.ports as usize {
        if protocols[port].major == 0 {
            continue;
        }
        if host.regs.read(op + 0x400 + (port - 1) * 16)? & 1 == 0 {
            continue;
        }
        if count >= 8 {
            return Err("DEVICE LIMIT");
        }
        let protocol = protocols[port];
        if protocol.major == 0 {
            return Err("MISSING PROTOCOL");
        }
        let (speed_id, speed) = port_reset(host, port, protocol)?;
        let output = pool.allocate(4096, 4096).ok_or("DMA FULL")?;
        let input = pool.allocate(4096, 4096).ok_or("DMA FULL")?;
        let ring = pool.allocate(4096, 4096).ok_or("DMA FULL")?;
        let buffer = pool.allocate(1024, 64).ok_or("DMA FULL")?;
        let enabled = host.command(0, (9 << 10) | ((protocol.slot as u32) << 16), 255)?;
        let slot = (enabled[3] >> 24) as u8;
        if slot as u32 > (host.regs.read(4)? & 255).min(8) {
            return Err("SLOT RANGE");
        }
        let (slot_words, ep_words) =
            musha_xhci::initial_context(port as u8, speed_id, speed.packet(), ring)
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
        host.command(input, (11 << 10) | ((slot as u32) << 24), slot)?; // BSR=0: xHC sends SET_ADDRESS
        let state = unsafe { read_word(output + 12) };
        if state >> 27 != 2 || state & 255 == 0 || state & 255 > 127 {
            return Err("ADDRESS STATE");
        }
        get_descriptor(host, slot, ring, 0, buffer, 8)?;
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
        get_descriptor(host, slot, ring, 3, buffer, 18)?;
        let mut descriptor = [0u8; 18];
        for (i, b) in descriptor.iter_mut().enumerate() {
            *b = unsafe { (buffer as *const u8).add(i).read_volatile() };
        }
        let (vendor, product) =
            musha_xhci::descriptor(&descriptor, speed).ok_or("DEVICE DESCRIPTOR")?;
        if descriptor[7] != prefix[7] {
            return Err("DESCRIPTOR CHANGED");
        }
        crate::debug(b"MUSHA: USB_DEVICE PORT=");
        crate::debug(&crate::cpu::hex(port as u64));
        crate::debug(b" SLOT=");
        crate::debug(&crate::cpu::hex(slot as u64));
        crate::debug(b" ADDRESS=");
        crate::debug(&crate::cpu::hex((state & 255) as u64));
        crate::debug(b" VID=");
        crate::debug(&crate::cpu::hex(vendor as u64));
        crate::debug(b" PID=");
        crate::debug(&crate::cpu::hex(product as u64));
        crate::debug(b"\n");
        keyboard(
            host, pool, slot, input, output, stride, ring, buffer, speed, tick,
        )?;
        // Diagnostic enumeration releases each hardware slot only after a
        // Disable Slot completion. The DMA allocation itself is never reused.
        host.command(0, (10 << 10) | ((slot as u32) << 24), slot)?;
        unsafe {
            dma_u64(dcbaa + slot as usize * 8, 0);
            core::arch::asm!("mfence", options(nostack));
        }
        count += 1;
    }
    crate::debug(b"MUSHA: USB_ENUMERATION_OK COUNT=");
    crate::debug(&crate::cpu::hex(count));
    crate::debug(b"\n");
    Ok(())
}

fn keyboard(
    host: &mut Host<'_>,
    pool: &mut musha_xhci::Pool,
    slot: u8,
    input: usize,
    output: usize,
    stride: usize,
    control_ring: usize,
    buffer: usize,
    speed: musha_xhci::Speed,
    tick: &mut dyn FnMut(super::AppEvent<'_>) -> Result<bool, &'static str>,
) -> Result<(), &'static str> {
    // Only configuration zero is inspected at this diagnostic stage.
    control(host, slot, control_ring, 6, buffer, 9, 0x02000680, 0)?;
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
        9,
        buffer,
        length as u32,
        0x02000680,
        0,
    )?;
    for (i, b) in bytes[..length].iter_mut().enumerate() {
        *b = unsafe { (buffer as *const u8).add(i).read_volatile() };
    }
    if let Some(disk) = musha_xhci::storage::configuration(&bytes[..length], speed)? {
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
        );
    }
    let Some(kbd) = musha_xhci::keyboard::configuration(&bytes[..length], speed)? else {
        return Ok(());
    };
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
        dma_word(ep + 16, 8 | (8 << 16));
        publish(ring + (TRBS - 1) * 16, [ring as u32, 0, 0, (6 << 10) | 3]);
        core::arch::asm!("mfence", options(nostack));
    }
    host.command(input, (12 << 10) | ((slot as u32) << 24), slot)?;
    control(
        host,
        slot,
        control_ring,
        12,
        buffer,
        0,
        ((kbd.configuration as u32) << 16) | 0x0900,
        0,
    )?;
    control(
        host,
        slot,
        control_ring,
        14,
        buffer,
        0,
        0x0b21,
        kbd.interface as u16,
    )?;
    crate::debug(
        b"MUSHA: HID_READY
",
    );
    unsafe {
        host.framebuffer.text(
            "KEYBOARD READY",
            24,
            292,
            host.framebuffer.color(0, 240, 100),
        );
    }
    let deadline = if cfg!(feature = "qemu-debug") && !cfg!(feature = "input-persistent") {
        host.clock
            .now()?
            .checked_add(5000)
            .ok_or("CLOCK OVERFLOW")?
    } else {
        u64::MAX
    };
    let mut state = musha_xhci::keyboard::State::default();
    let mut producer = musha_xhci::Cursor::new(TRBS - 1).ok_or("CURSOR")?;
    let mut pending = false;
    let mut reports = 0;
    loop {
        if host.clock.now()? >= deadline {
            break;
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
        }
        let result = event(
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
        );
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
                producer.advance();
                if producer.index == 0 {
                    crate::debug(b"MUSHA: HID_RING_WRAP_OK\n");
                }
                reports += 1;
            }
        }
        // Hardware poll never waits for a key. Each iteration also advances the
        // application, including while the device NAKs an outstanding transfer.
        if tick(super::AppEvent::Keys(&transitions[..count]))? {
            break;
        }
    }
    crate::debug(b"MUSHA: HID_DIAGNOSTIC_OK REPORTS=");
    crate::debug(&crate::cpu::hex(reports));
    crate::debug(
        b"
",
    );
    Ok(())
}
