// SPDX-License-Identifier: Apache-2.0
//! Bounded USB2 and USB3.0/5Gbps boot-time hub traversal. All requests use the owning slot EP0.
use super::{Host, Protocol, Registers, control, dma_word, read_word};
use musha_xhci::{
    Speed,
    hub::{Descriptor, Kind, Path, PortStatus},
};
#[derive(Clone, Copy)]
pub(super) struct Target {
    pub path: Path,
    pub port: u8,
    pub protocol: Protocol,
    pub parent: Option<Hub>,
    pub ancestors: [Option<(Hub, u8)>; 5],
}
#[derive(Clone, Copy)]
pub(super) struct Hub {
    pub slot: u8,
    pub ring: usize,
    pub buffer: usize,
    pub speed: Speed,
    pub path: Path,
    pub ports: u8,
    pub kind: Kind,
}
pub(super) fn speed_id(
    regs: &Registers,
    protocol: Protocol,
    wanted: Speed,
) -> Result<u8, &'static str> {
    for id in 1..=15 {
        if super::speed(regs, protocol, id).ok() == Some(wanted) {
            return Ok(id);
        }
    }
    Err("HUB SPEED ID")
}
pub(super) fn status(host: &mut Host<'_>, hub: Hub, port: u8) -> Result<PortStatus, &'static str> {
    control(host, hub.slot, hub.ring, hub.buffer, 4, 0xa3, port as u16)?;
    let mut bytes = [0; 4];
    for (i, b) in bytes.iter_mut().enumerate() {
        *b = unsafe { (hub.buffer as *const u8).add(i).read_volatile() };
    }
    crate::diagnostics::observation(format_args!(
        "HUB S{} P{} STATUS {:04X} CHANGE {:04X}",
        hub.slot,
        port,
        u16::from_le_bytes([bytes[0], bytes[1]]),
        u16::from_le_bytes([bytes[2], bytes[3]])
    ));
    PortStatus::parse(&bytes, hub.kind).ok_or("HUB STATUS")
}
fn feature(
    host: &mut Host<'_>,
    hub: Hub,
    port: u8,
    value: u16,
    set: bool,
) -> Result<(), &'static str> {
    crate::diagnostics::observation(format_args!(
        "HUB S{} P{} {} FEATURE {}",
        hub.slot,
        port,
        if set { "SET" } else { "CLEAR" },
        value
    ));
    control(
        host,
        hub.slot,
        hub.ring,
        hub.buffer,
        0,
        ((value as u32) << 16) | if set { 0x0323 } else { 0x0123 },
        port as u16,
    )
}
pub(super) fn setup(
    host: &mut Host<'_>,
    pool: &mut musha_xhci::Pool,
    slot: u8,
    input: usize,
    output: usize,
    stride: usize,
    ring: usize,
    buffer: usize,
    speed: Speed,
    path: Path,
    configuration: musha_xhci::hub::Configuration,
) -> Result<Hub, &'static str> {
    if !matches!(speed, Speed::Full | Speed::High | Speed::Super) {
        return Err("HUB SPEED");
    }
    crate::diagnostics::usb_stage("HUB CONFIGURATION");
    control(
        host,
        slot,
        ring,
        buffer,
        0,
        ((configuration.value as u32) << 16) | 0x0900,
        0,
    )?;
    let kind = configuration.kind;
    if (kind == Kind::Super) != (speed == Speed::Super) {
        return Err("HUB KIND");
    }
    if kind == Kind::Super && kind.depth_request(path.depth).is_none() {
        return Err("HUB DEPTH LIMIT");
    }
    let request = ((kind.descriptor_type() as u32) << 24) | 0x000006a0;
    let length = if kind == Kind::Super {
        12
    } else {
        control(host, slot, ring, buffer, 7, request, 0)?;
        let n = unsafe { (buffer as *const u8).read_volatile() } as usize;
        if !(9..=11).contains(&n) {
            return Err("HUB DESCRIPTOR");
        }
        n
    };
    control(host, slot, ring, buffer, length as u32, request, 0)?;
    let mut bytes = [0; 12];
    for (i, b) in bytes[..length].iter_mut().enumerate() {
        *b = unsafe { (buffer as *const u8).add(i).read_volatile() };
    }
    let descriptor = Descriptor::parse(&bytes[..length], kind).ok_or("HUB DESCRIPTOR")?;
    if configuration.packet < (descriptor.ports as u16 + 1).div_ceil(8) {
        return Err("HUB STATUS PACKET");
    }
    // Hub fields are initialized by the first Configure Endpoint command,
    // not Evaluate Context. Configure an empty status ring but never arm it.
    let endpoint = configuration.endpoint * 2 + 1;
    let status_ring = pool.allocate(4096, 4096).ok_or("DMA FULL")?;
    unsafe {
        core::ptr::write_bytes(input as *mut u8, 0, 4096);
        dma_word(input + 4, 1 | (1 << endpoint));
        for i in 0..stride / 4 {
            dma_word(input + stride + i * 4, read_word(output + i * 4));
        }
        dma_word(
            input + stride,
            (read_word(input + stride) & !(31 << 27)) | (1 << 26) | ((endpoint as u32) << 27),
        );
        dma_word(
            input + stride + 4,
            (read_word(input + stride + 4) & 0x00ffffff) | ((descriptor.ports as u32) << 24),
        );
        dma_word(
            input + stride + 8,
            read_word(input + stride + 8)
                | if speed == Speed::High {
                    (descriptor.tt_think_time as u32) << 16
                } else {
                    0
                },
        );
        let ep = input + (endpoint as usize + 1) * stride;
        dma_word(ep, (configuration.interval as u32) << 16);
        dma_word(
            ep + 4,
            (3 << 1) | (7 << 3) | ((configuration.packet as u32) << 16),
        );
        dma_word(ep + 8, status_ring as u32 | 1);
        dma_word(
            ep + 16,
            configuration.packet as u32 | ((configuration.packet as u32) << 16),
        );
        super::publish(
            status_ring + (super::TRBS - 1) * 16,
            [status_ring as u32, 0, 0, (6 << 10) | 3],
        );
        core::arch::asm!("mfence", options(nostack));
    }
    host.command(input, (12 << 10) | ((slot as u32) << 24), slot)?;
    if unsafe { read_word(output + 12) } >> 27 != 3 {
        return Err("HUB CONTEXT STATE");
    }
    // QEMU's Configure Endpoint currently updates Context Entries and Slot State
    // without copying the hub fields. A successful command is authoritative;
    // report incomplete readback rather than modifying the output DMA context.
    if unsafe { read_word(output) } & (1 << 26) == 0
        || unsafe { read_word(output + 4) } >> 24 != descriptor.ports as u32
    {
        crate::debug(b"MUSHA: HUB_CONTEXT_FIELDS_NOT_REPORTED\n");
    }
    if let Some(request) = kind.depth_request(path.depth) {
        crate::diagnostics::usb_stage("HUB DEPTH");
        control(host, slot, ring, buffer, 0, request, 0)?;
    }
    let hub = Hub {
        slot,
        ring,
        buffer,
        speed,
        path,
        ports: descriptor.ports,
        kind,
    };
    for port in 1..=hub.ports {
        feature(host, hub, port, 8, true)?;
    }
    host.delay(descriptor.power_good_ms as u64 + 20)?;
    match kind {
        Kind::Usb2 => {
            crate::diagnostics::usb_kind("USB2 HUB / BOOT ENUMERATION");
            crate::debug(b"MUSHA: USB2_HUB_READY\n");
        }
        Kind::Super => {
            crate::diagnostics::usb_kind("SS HUB / BOOT ENUMERATION UNVERIFIED");
            crate::debug(b"MUSHA: SS_HUB_READY UNVERIFIED\n");
        }
    }
    Ok(hub)
}
pub(super) fn reset_port(host: &mut Host<'_>, hub: Hub, port: u8) -> Result<Speed, &'static str> {
    crate::diagnostics::usb_stage("HUB PORT RESET");
    // Debounce without resetting other occupied ports. Bounded 100ms interval.
    for _ in 0..10 {
        let s = status(host, hub, port)?;
        if !s.connected() {
            return Err("HUB DISCONNECTED");
        }
        if s.overcurrent() {
            return Err("HUB OVERCURRENT");
        }
        host.delay(10)?;
    }
    let s = status(host, hub, port)?;
    if s.configuration_error() {
        return Err("HUB PORT CONFIG ERROR");
    }
    clear_changes(host, hub, port, s)?;
    feature(host, hub, port, hub.kind.reset_feature(), true)?;
    for _ in 0..50 {
        host.delay(10)?;
        let s = status(host, hub, port)?;
        if !s.connected() {
            return Err("HUB DISCONNECTED");
        }
        if s.overcurrent() {
            return Err("HUB OVERCURRENT");
        }
        if s.configuration_error() {
            return Err("HUB PORT CONFIG ERROR");
        }
        if s.reset_complete() {
            let speed = s.ready_speed().ok_or("HUB PORT DISABLED")?;
            clear_changes(host, hub, port, s)?;
            host.delay(10)?;
            crate::debug(b"MUSHA: HUB_CHILD_RESET_OK\n");
            return Ok(speed);
        }
    }
    Err("HUB RESET TIMEOUT")
}

fn clear_changes(
    host: &mut Host<'_>,
    hub: Hub,
    port: u8,
    status: PortStatus,
) -> Result<(), &'static str> {
    for selector in status.change_features().into_iter().flatten() {
        feature(host, hub, port, selector, false)?;
    }
    Ok(())
}
