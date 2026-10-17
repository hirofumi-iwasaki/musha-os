// Copyright 2026 Hirofumi Iwasaki
// SPDX-License-Identifier: Apache-2.0
//! Opt-in real USB HCI Reset transport probe. Not a paired/runtime input device.
use super::{Host, TRBS, control, control_write, dma_word, publish, read_word};
use musha_platform::bluetooth::{Event, usb::Transport};
pub(super) fn probe(
    host: &mut Host<'_>,
    pool: &mut musha_xhci::Pool,
    slot: u8,
    input: usize,
    output: usize,
    stride: usize,
    control_ring: usize,
    buffer: usize,
    speed: musha_xhci::Speed,
    transport: Transport,
    configuration: u8,
    composite: bool,
) -> Result<(), &'static str> {
    let interval = match speed {
        musha_xhci::Speed::Full
            if transport.acl_in.max_packet <= 64 && transport.acl_out.max_packet <= 64 =>
        {
            3 + (7 - transport.event.interval.leading_zeros()) as u8
        }
        musha_xhci::Speed::High if (1..=16).contains(&transport.event.interval) => {
            transport.event.interval - 1
        }
        _ => return Err("BT USB SPEED"),
    };
    if configuration == 0 {
        return Err("BT CONFIG");
    }
    let ring = pool.allocate(4096, 4096).ok_or("DMA FULL")?;
    let receive = pool.allocate(512, 512).ok_or("DMA FULL")?;
    let dci = (transport.event.address & 15) * 2 + 1;
    unsafe {
        core::ptr::write_bytes(input as *mut u8, 0, 4096);
        dma_word(input + 4, 1 | (1 << dci));
        for i in 0..3 {
            dma_word(input + stride + i * 4, read_word(output + i * 4));
        }
        let old = read_word(input + stride);
        dma_word(input + stride, (old & !(31 << 27)) | ((dci as u32) << 27));
        let ep = input + (dci as usize + 1) * stride;
        dma_word(ep, (interval as u32) << 16);
        dma_word(
            ep + 4,
            (3 << 1) | (7 << 3) | ((transport.event.max_packet as u32) << 16),
        );
        dma_word(ep + 8, ring as u32 | 1);
        dma_word(ep + 16, 257 | (257 << 16));
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
        ((configuration as u32) << 16) | 0x0900,
        0,
    )?;
    // Five standard commands only, one outstanding command and at most 1s each.
    // Every command uses a new event-ring TRB; storage is retained until host stop.
    for (index, opcode) in musha_platform::bluetooth::probe::COMMANDS
        .into_iter()
        .enumerate()
    {
        let command = [opcode as u8, (opcode >> 8) as u8, 0];
        let setup = transport
            .command_setup(&command, composite)
            .map_err(|_| "BT COMMAND")?;
        control_write(
            host,
            slot,
            control_ring,
            buffer,
            &command,
            u32::from_le_bytes(setup[..4].try_into().unwrap()),
            u16::from_le_bytes(setup[4..6].try_into().unwrap()),
        )?;
        let trb = ring + index * 16;
        unsafe {
            core::ptr::write_bytes(receive as *mut u8, 0, 512);
            publish(
                trb,
                [receive as u32, 0, 257, (1 << 10) | (1 << 5) | (1 << 2) | 1],
            );
        }
        host.regs
            .write(host.doorbell + slot as usize * 4, dci as u32)?;
        let mut residual = None;
        super::super::event_dispatch(
            host.regs,
            host.clock,
            host.events,
            host.consumer,
            host.runtime,
            host.ac64,
            trb,
            host.ports,
            slot,
            dci,
            1000,
            host.keyboard_pending,
            Some(&mut host.keyboard_saved),
            host.bulk_pending,
            Some(&mut host.bulk_saved),
            host.mouse_pending,
            Some(&mut host.mouse_saved),
            Some((trb, 257)),
            Some(&mut residual),
            Some(&mut host.bt_mailbox),
        )?;
        let actual = 257usize
            .checked_sub(residual.ok_or("BT EVENT RESIDUAL")? as usize)
            .ok_or("BT EVENT LENGTH")?;
        let mut bytes = [0; 257];
        for (i, byte) in bytes[..actual].iter_mut().enumerate() {
            *byte = unsafe { (receive as *const u8).add(i).read_volatile() };
        }
        let reply = musha_platform::bluetooth::probe::reply(opcode, &bytes[..actual])
            .map_err(|_| "BT EVENT")?;
        crate::bootlog::device(
            "bluetooth",
            format_args!(
                "HCI CC opcode={:04x} status={:02x} credits={}",
                opcode, reply.status, reply.credits
            ),
        );
        if reply.status == 0 {
            let d = reply.data;
            match opcode {
                0x1001 => crate::bootlog::device(
                    "bluetooth",
                    format_args!(
                        "version HCI={:02x} revision={:04x} LMP={:02x} manufacturer={:04x} subversion={:04x}",
                        d[0],
                        u16::from_le_bytes([d[1], d[2]]),
                        d[3],
                        u16::from_le_bytes([d[4], d[5]]),
                        u16::from_le_bytes([d[6], d[7]])
                    ),
                ),
                0x1003 | 0x2003 => crate::bootlog::device(
                    "bluetooth",
                    format_args!(
                        "{} features={:016x}",
                        if opcode == 0x1003 { "local" } else { "LE" },
                        u64::from_le_bytes(d.try_into().unwrap())
                    ),
                ),
                0x1009 => crate::bootlog::device(
                    "bluetooth",
                    format_args!(
                        "BDADDR={:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
                        d[5], d[4], d[3], d[2], d[1], d[0]
                    ),
                ),
                _ => crate::debug(b"MUSHA: BT_HCI_RESET_TRANSPORT_OK\n"),
            }
        } else if opcode == 0x0c03 {
            return Err("BT RESET FAILED");
        }
        if reply.credits == 0 {
            return Err("BT COMMAND CREDIT ZERO");
        }
    }
    crate::debug(b"MUSHA: BT_HCI_STANDARD_PROBE_COMPLETE\n");
    // The caller disables the slot and outer Host confirms halt/BME-off before
    // pool reuse. A timeout propagates outward with the DMA allocation retained.
    Ok(())
}

struct Pipe {
    ring: usize,
    buffer: usize,
    dci: u8,
    index: usize,
    cursor: musha_xhci::Cursor,
    pending: bool,
    bytes: u32,
}
impl Pipe {
    fn arm(
        &mut self,
        host: &mut Host<'_>,
        slot: u8,
        data: Option<&[u8]>,
    ) -> Result<(), &'static str> {
        if self.pending {
            return Ok(());
        }
        let bytes = data.map_or(self.bytes as usize, |b| b.len());
        if bytes == 0 || bytes > self.bytes as usize {
            return Err("BT TRANSFER SIZE");
        }
        let trb = self.ring + self.cursor.index * 16;
        host.bt_mailbox.arm(
            self.index,
            trb,
            slot,
            self.dci,
            bytes as u32,
            data.is_none(),
        )?;
        unsafe {
            core::ptr::write_bytes(self.buffer as *mut u8, 0, self.bytes as usize);
            if let Some(data) = data {
                for (i, b) in data.iter().enumerate() {
                    (self.buffer as *mut u8).add(i).write_volatile(*b);
                }
            }
            if self.cursor.index == TRBS - 2 {
                publish(
                    self.ring + (TRBS - 1) * 16,
                    [self.ring as u32, 0, 0, (6 << 10) | 2 | self.cursor.cycle],
                );
            }
            publish(
                trb,
                [
                    self.buffer as u32,
                    0,
                    bytes as u32,
                    (1 << 10)
                        | (1 << 5)
                        | (if data.is_none() { 1 << 2 } else { 0 })
                        | self.cursor.cycle,
                ],
            );
        }
        self.pending = true;
        host.regs
            .write(host.doorbell + slot as usize * 4, self.dci as u32)?;
        Ok(())
    }
    fn poll(&mut self, host: &mut Host<'_>, slot: u8) -> Result<Option<usize>, &'static str> {
        if !self.pending {
            return Ok(None);
        }
        let trb = self.ring + self.cursor.index * 16;
        let words = if let Some(w) = host.bt_mailbox.take(self.index) {
            w
        } else {
            let mut residual = None;
            match super::super::event_dispatch(
                host.regs,
                host.clock,
                host.events,
                host.consumer,
                host.runtime,
                host.ac64,
                trb,
                host.ports,
                slot,
                self.dci,
                0,
                host.keyboard_pending,
                Some(&mut host.keyboard_saved),
                host.bulk_pending,
                Some(&mut host.bulk_saved),
                host.mouse_pending,
                Some(&mut host.mouse_saved),
                Some((trb, self.bytes)),
                Some(&mut residual),
                Some(&mut host.bt_mailbox),
            ) {
                Ok(w) => w,
                Err("EVENT PENDING") => return Ok(None),
                Err(e) => return Err(e),
            }
        };
        let bytes = host.bt_mailbox.complete(self.index, words)?;
        self.pending = false;
        self.cursor.advance();
        Ok(Some(bytes))
    }
}
struct Shared<T>(core::cell::UnsafeCell<T>);
unsafe impl<T> Sync for Shared<T> {} // BSP only; lease forbids callbacks/reentry alias.
static LEASE: Shared<musha_xhci::lease::Lease> =
    Shared(core::cell::UnsafeCell::new(musha_xhci::lease::Lease::new()));
static DEVICE: Shared<core::mem::MaybeUninit<Device>> =
    Shared(core::cell::UnsafeCell::new(core::mem::MaybeUninit::uninit()));
pub(in crate::xhci) struct DeviceRef(&'static mut Device);
impl core::ops::Deref for DeviceRef {
    type Target = Device;
    fn deref(&self) -> &Device {
        self.0
    }
}
impl core::ops::DerefMut for DeviceRef {
    fn deref_mut(&mut self) -> &mut Device {
        self.0
    }
}
impl Drop for DeviceRef {
    fn drop(&mut self) {
        unsafe {
            core::ptr::drop_in_place(self.0);
            (&mut *LEASE.0.get()).retire();
        }
    }
}
/// Called only after the enclosing Host/callbacks are gone and this controller's
/// halt plus BME-off were verified. Failure leaves the lease quarantined.
pub(in crate::xhci) unsafe fn release_stopped(owner: u16) -> bool {
    unsafe { (&mut *LEASE.0.get()).stopped(owner) }
}
pub(in crate::xhci) struct Device {
    entropy_samples: musha_platform::bluetooth::sources::HciRand,
    entropy_reset: bool,
    slot: u8,
    port: usize,
    pub(super) ancestors: [Option<(super::hub::Hub, u8)>; 5],
    check_at: u64,
    control_ring: usize,
    buffer: usize,
    transport: Transport,
    composite: bool,
    event: Pipe,
    acl_in: Pipe,
    acl_out: Pipe,
    event_stream: musha_platform::bluetooth::stream::Stream<257>,
    acl_stream: musha_platform::bluetooth::stream::Stream<1028>,
    scan: musha_platform::bluetooth::scan::Scan,
    generation: u64,
    started: Option<u64>,
    advertisements: usize,
    phase: u8,
    command: Option<(u16, u64)>,
    command_credits: u8,
    own: [u8; 6],
    peer: [u8; 6],
    peer_type: u8,
    method: musha_platform::bluetooth::sc::Method,
    shared_capacity: bool,
    packet_bytes: u16,
    packets: u16,
    handle: u16,
    credits: Option<musha_platform::bluetooth::acl::Credits>,
    reassembly: Option<musha_platform::bluetooth::acl::Reassembly>,
    security: Option<musha_platform::bluetooth::sc::Central>,
    tx: [u8; 1028],
    tx_len: usize,
    tx_at: usize,
    tx_deadline: u64,
    encrypt: Option<[u8; 16]>,
    gatt: Option<musha_platform::bluetooth::hogp::Discovery>,
    keyboard: Option<musha_platform::bluetooth::hogp::Reports>,
    cccd: u16,
    cccd_index: usize,
    pairing: bool,
    reused: bool,
    encryption_started: bool,
    reconnect: Option<musha_platform::bluetooth::reconnect::Pending>,
    saved: bool,
    subscribed: bool,
    waiting_display: bool,
    waiting_entry: bool,
    bond: Option<musha_platform::bluetooth::legacy::VerifiedBond>,
}
impl Drop for Device {
    fn drop(&mut self) {
        crate::bluetooth::active(false);
        crate::bluetooth::stopped();
        for b in &mut self.tx {
            unsafe {
                core::ptr::write_volatile(b, 0);
            }
        }
        if let Some(key) = self.encrypt.as_mut() {
            for b in key {
                unsafe {
                    core::ptr::write_volatile(b, 0);
                }
            }
        }
    }
}
static GENERATION: core::sync::atomic::AtomicU64 = core::sync::atomic::AtomicU64::new(1);
pub(super) fn prepare(
    host: &mut Host<'_>,
    pool: &mut musha_xhci::Pool,
    slot: u8,
    input: usize,
    output: usize,
    stride: usize,
    control_ring: usize,
    buffer: usize,
    speed: musha_xhci::Speed,
    transport: Transport,
    configuration: u8,
    composite: bool,
    port: usize,
) -> Result<DeviceRef, &'static str> {
    let interval = match speed {
        musha_xhci::Speed::Full
            if transport.acl_in.max_packet <= 64 && transport.acl_out.max_packet <= 64 =>
        {
            3 + (7 - transport.event.interval.leading_zeros()) as u8
        }
        musha_xhci::Speed::High if (1..=16).contains(&transport.event.interval) => {
            transport.event.interval - 1
        }
        _ => return Err("BT USB SPEED"),
    };
    if configuration == 0 {
        return Err("BT CONFIG");
    }
    let eps = [transport.event, transport.acl_in, transport.acl_out];
    let mut pipes = core::array::from_fn(|i| Pipe {
        ring: 0,
        buffer: 0,
        dci: (eps[i].address & 15) * 2 + u8::from(eps[i].address & 0x80 != 0),
        index: i,
        cursor: musha_xhci::Cursor::new(TRBS - 1).unwrap(),
        pending: false,
        bytes: if i == 0 { 257 } else { 2048 },
    });
    unsafe {
        core::ptr::write_bytes(input as *mut u8, 0, 4096);
        dma_word(
            input + 4,
            1 | pipes.iter().fold(0, |flags, p| flags | (1 << p.dci)),
        );
        for i in 0..3 {
            dma_word(input + stride + i * 4, read_word(output + i * 4));
        }
        let old = read_word(input + stride);
        let max = pipes.iter().map(|p| p.dci).max().unwrap();
        dma_word(input + stride, (old & !(31 << 27)) | ((max as u32) << 27));
        for (i, p) in pipes.iter_mut().enumerate() {
            p.ring = pool.allocate(4096, 4096).ok_or("DMA FULL")?;
            p.buffer = pool.allocate(2048, 2048).ok_or("DMA FULL")?;
            core::ptr::write_bytes(p.ring as *mut u8, 0, 4096);
            let ep = input + (p.dci as usize + 1) * stride;
            dma_word(ep, if i == 0 { (interval as u32) << 16 } else { 0 });
            dma_word(
                ep + 4,
                (3 << 1)
                    | ((if i == 0 {
                        7
                    } else if i == 1 {
                        6
                    } else {
                        2
                    }) << 3)
                    | ((eps[i].max_packet as u32) << 16),
            );
            dma_word(ep + 8, p.ring as u32 | 1);
            dma_word(ep + 16, if i == 0 { 257 | (257 << 16) } else { 512 });
            publish(
                p.ring + (TRBS - 1) * 16,
                [p.ring as u32, 0, 0, (6 << 10) | 3],
            );
        }
        core::arch::asm!("mfence", options(nostack));
    }
    host.command(input, (12 << 10) | ((slot as u32) << 24), slot)?;
    control(
        host,
        slot,
        control_ring,
        buffer,
        0,
        ((configuration as u32) << 16) | 0x0900,
        0,
    )?;
    let generation = GENERATION
        .try_update(
            core::sync::atomic::Ordering::Relaxed,
            core::sync::atomic::Ordering::Relaxed,
            |n| n.checked_add(1),
        )
        .map_err(|_| "BT GENERATION EXHAUSTED")?;
    let [event, acl_in, acl_out] = pipes;
    if !unsafe { (&mut *LEASE.0.get()).acquire(host.bdf) } {
        return Err("BT STATE LEASE BUSY");
    }
    crate::bluetooth::active(true);
    let device = unsafe {
        (&mut *DEVICE.0.get()).write(Device {
            entropy_samples: musha_platform::bluetooth::sources::HciRand::new(),
            entropy_reset: false,
            slot,
            port,
            ancestors: [None; 5],
            check_at: 0,
            control_ring,
            buffer,
            transport,
            composite,
            event,
            acl_in,
            acl_out,
            event_stream: musha_platform::bluetooth::stream::Stream::new(generation, false),
            acl_stream: musha_platform::bluetooth::stream::Stream::new(generation, true),
            scan: musha_platform::bluetooth::scan::Scan::new(generation),
            generation,
            started: None,
            advertisements: 0,
            phase: 0,
            command: None,
            command_credits: 1,
            own: [0; 6],
            peer: [0; 6],
            peer_type: 0,
            method: musha_platform::bluetooth::sc::Method::DisplayPasskey,
            shared_capacity: false,
            packet_bytes: 0,
            packets: 0,
            handle: 0,
            credits: None,
            reassembly: None,
            security: None,
            tx: [0; 1028],
            tx_len: 0,
            tx_at: 0,
            tx_deadline: 0,
            encrypt: None,
            bond: None,
            gatt: None,
            keyboard: None,
            cccd: 0,
            cccd_index: 0,
            pairing: false,
            reused: false,
            encryption_started: false,
            reconnect: None,
            saved: false,
            subscribed: false,
            waiting_display: false,
            waiting_entry: false,
        })
    };
    Ok(DeviceRef(device))
}
impl Device {
    fn hci_event(
        &mut self,
        host: &mut Host<'_>,
        now: u64,
        bytes: &[u8],
    ) -> Result<(), &'static str> {
        if !self.entropy_samples.done() {
            let event = Event::parse(bytes).map_err(|_| "BT ENTROPY EVENT")?;
            let (credits, opcode, result) = event.complete().map_err(|_| "BT ENTROPY COMPLETE")?;
            if self.command.map(|p| p.0) != Some(opcode) || result.is_empty() {
                return Err("BT ENTROPY COMMAND");
            }
            if !self.entropy_reset {
                crate::bootlog::device(
                    "entropy",
                    format_args!(
                        "HCI initial Reset status={:02x} credits={}",
                        result[0], credits
                    ),
                );
                if opcode != 0x0c03 || result != [0] || credits == 0 {
                    return Err("BT ENTROPY RESET");
                }
                self.command = None;
                self.command_credits = credits;
                self.entropy_reset = true;
                return Ok(());
            }
            if opcode != 0x2018 {
                return Err("BT ENTROPY OPCODE");
            }
            self.command = None;
            self.command_credits = credits;
            crate::bootlog::device(
                "entropy",
                format_args!(
                    "HCI LE Rand status={:02x} sample={}/4",
                    result[0],
                    self.entropy_samples.count() + 1
                ),
            );
            if credits == 0 {
                return Err("BT ENTROPY CREDIT");
            }
            match self.entropy_samples.complete(result) {
                Ok(Some(mut bytes)) => {
                    crate::entropy::hci(&mut bytes);
                }
                Ok(None) => {}
                Err(musha_platform::bluetooth::sources::HciFailure::Health) => {
                    crate::bootlog::device(
                        "entropy",
                        format_args!(
                            "HCI auxiliary health rejected; existing strong state retained"
                        ),
                    )
                }
                Err(_) => return Err("BT ENTROPY FORMAT"),
            }
            return Ok(());
        }
        if self.phase == 0 {
            self.scan
                .event(self.generation, host.clock.now()?, bytes)
                .map_err(|_| "BT EVENT STATE")?;
        }
        let event =
            musha_platform::bluetooth::Event::parse(bytes).map_err(|_| "BT EVENT FORMAT")?;
        if self.phase == 0 && event.code == 0x3e && event.parameters.first() == Some(&2) {
            musha_platform::bluetooth::advertisements(event, |advert| {
                self.advertisements = self.advertisements.saturating_add(1);
                if self.phase == 0 && matches!(advert.event_type, 0 | 1) {
                    if !self.pairing {
                        if crate::bluetooth::stored_connectable(
                            advert.event_type,
                            advert.address_type,
                            &advert.address,
                        ) {
                            self.peer = advert.address;
                            self.peer_type = advert.address_type;
                            self.phase = 1;
                        }
                        return;
                    }
                    let mut appearance = None;
                    let mut hid = false;
                    if musha_platform::bluetooth::ad_fields(advert.data, |field| {
                        if field.kind == 0x19 && field.value.len() == 2 {
                            appearance = Some(u16::from_le_bytes(field.value.try_into().unwrap()));
                        }
                        if matches!(field.kind, 2 | 3) && field.value.len() % 2 == 0 {
                            hid = field.value.chunks_exact(2).any(|b| b == [0x12, 0x18]);
                        }
                    })
                    .is_ok()
                        && hid
                    {
                        self.method = match appearance {
                            Some(0x3c1) => musha_platform::bluetooth::sc::Method::DisplayPasskey,
                            Some(0x3c4) => musha_platform::bluetooth::sc::Method::JustWorks,
                            _ => return,
                        };
                        self.peer = advert.address;
                        self.peer_type = advert.address_type;
                        self.phase = 1;
                    }
                }
            })
            .map_err(|_| "BT ADVERTISING")?;
            crate::debug(b"MUSHA: BT_LE_ADVERTISING_VALIDATED\n");
        } else if self.phase != 0 {
            self.handle_event(host, now, event)?;
        }
        Ok(())
    }
    fn poll(&mut self, host: &mut Host<'_>) -> Result<(), &'static str> {
        let now = host.clock.now()?;
        self.event_stream
            .check(self.generation, now)
            .map_err(|_| "BT EVENT STREAM TIMEOUT")?;
        self.acl_stream
            .check(self.generation, now)
            .map_err(|_| "BT ACL STREAM TIMEOUT")?;
        if !self.entropy_samples.done() {
            self.event.arm(host, self.slot, None)?;
            if let Some((_, deadline)) = self.command {
                if now >= deadline {
                    return Err("BT ENTROPY COMMAND TIMEOUT");
                }
            } else {
                self.send_command(
                    host,
                    now,
                    if self.entropy_reset { 0x2018 } else { 0x0c03 },
                    &[],
                )?;
            }
            if let Some(n) = self.event.poll(host, self.slot)? {
                let mut bytes = [0; 257];
                if n > bytes.len() {
                    return Err("BT ENTROPY EVENT LENGTH");
                }
                for (i, b) in bytes[..n].iter_mut().enumerate() {
                    *b = unsafe { (self.event.buffer as *const u8).add(i).read_volatile() };
                }
                let mut input = &bytes[..n];
                while !input.is_empty() {
                    let mut packet = [0; 257];
                    let Some(frame) = self
                        .event_stream
                        .next(self.generation, now, &mut input)
                        .map_err(|_| "BT ENTROPY STREAM")?
                    else {
                        break;
                    };
                    let length = frame.len();
                    packet[..length].copy_from_slice(frame);
                    self.event_stream.release();
                    self.hci_event(host, now, &packet[..length])?;
                }
            }
            return Ok(());
        }
        if self.subscribed && crate::bluetooth::take_request() {
            crate::bluetooth::restart_after_stop(true);
            return Err("BT REPAIR REQUEST");
        }
        if self.started.is_none() {
            self.pairing = crate::bluetooth::take_request();
            if !self.pairing && !crate::bluetooth::stored_any() {
                return Ok(());
            }
            if self.pairing && !crate::entropy::available() {
                crate::debug(b"MUSHA: BT_PAIR_REQUEST_BLOCKED ENTROPY\n");
                return Ok(());
            }
            if self.pairing && !crate::bluetooth::begin_attempt() {
                crate::bluetooth::status(
                    "Bluetooth pairing unavailable: boot attempt budget exhausted.",
                );
                return Ok(());
            }
            self.started = Some(now);
            crate::debug(b"MUSHA: BT_LE_SCAN_BEGIN\n");
        }
        let op = (host.regs.read(0)? & 255) as usize;
        if host.regs.read(op + 0x400 + (self.port - 1) * 16)? & 1 == 0 {
            return Err("BT DISCONNECTED");
        }
        if now >= self.check_at {
            for (hub, port) in self.ancestors.iter().flatten().copied() {
                let status = super::hub::status(host, hub, port)?;
                if status.overcurrent()
                    || !status.connected()
                    || status.ready_speed().is_none()
                    || status.connection_changed()
                    || status.configuration_error()
                {
                    return Err("BT HUB DISCONNECTED");
                }
            }
            self.check_at = now.checked_add(100).ok_or("BT CLOCK")?;
        }
        if !self.subscribed && now.saturating_sub(self.started.unwrap()) >= 120000 {
            return Err("BT PAIR TIMEOUT");
        }
        self.event.arm(host, self.slot, None)?;
        if self.phase == 0 {
            match self.scan.next(self.generation, now) {
                Ok(Some(command)) => {
                    let bytes = &command.bytes[..command.length];
                    let setup = self
                        .transport
                        .command_setup(bytes, self.composite)
                        .map_err(|_| "BT COMMAND")?;
                    control_write(
                        host,
                        self.slot,
                        self.control_ring,
                        self.buffer,
                        bytes,
                        u32::from_le_bytes(setup[..4].try_into().unwrap()),
                        u16::from_le_bytes(setup[4..6].try_into().unwrap()),
                    )?;
                }
                Ok(None) | Err(musha_platform::bluetooth::Error::Busy) => {}
                Err(_) => return Err("BT SCAN STATE"),
            }
        } else {
            self.drive(host, now)?;
        }
        if let Some(n) = self.event.poll(host, self.slot)? {
            let mut bytes = [0; 257];
            if n > bytes.len() {
                return Err("BT EVENT LENGTH");
            }
            for (i, b) in bytes[..n].iter_mut().enumerate() {
                *b = unsafe { (self.event.buffer as *const u8).add(i).read_volatile() };
            }
            let mut input = &bytes[..n];
            while !input.is_empty() {
                let mut packet = [0; 257];
                let Some(frame) = self
                    .event_stream
                    .next(self.generation, now, &mut input)
                    .map_err(|_| "BT EVENT STREAM")?
                else {
                    break;
                };
                let length = frame.len();
                packet[..length].copy_from_slice(frame);
                self.event_stream.release();
                self.hci_event(host, now, &packet[..length])?;
            }
        }
        if self.phase >= 8 {
            self.poll_acl(host, now)?;
        }
        Ok(())
    }
}
pub(super) fn poll(
    host: &mut Host<'_>,
    _tick: &mut super::super::Tick<'_>,
) -> Result<(), &'static str> {
    if let Some(mut device) = host.bluetooth.take() {
        let result = device.poll(host);
        host.bluetooth = Some(device);
        result
    } else {
        Ok(())
    }
}

mod connection;
