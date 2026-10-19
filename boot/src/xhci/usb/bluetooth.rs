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
    let plan = musha_platform::bluetooth::usb_context::plan(
        transport,
        interval,
        [ring as u64, 0, 0],
        true,
    )
    .map_err(|_| "BT CONTEXT PLAN")?;
    unsafe {
        core::ptr::write_bytes(input as *mut u8, 0, 4096);
        dma_word(input + 4, plan.add_flags);
        for i in 0..3 {
            dma_word(input + stride + i * 4, read_word(output + i * 4));
        }
        let old = read_word(input + stride);
        dma_word(input + stride, (old & !(31 << 27)) | ((dci as u32) << 27));
        let ep = input + (dci as usize + 1) * stride;
        for (i, word) in plan.endpoints[0].words.into_iter().enumerate() {
            dma_word(ep + i * 4, word);
        }
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
    // Per-TD records are bounded; scanning repeats them many times per second.
    logged: u8,
    completions: u32,
    last_cc: u8,
    last_bytes: u32,
}
const PIPE_LOG_LIMIT: u8 = 6;
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
        if self.logged < PIPE_LOG_LIMIT {
            crate::bootlog::device(
                "bluetooth",
                format_args!(
                    "event/ACL queued slot={} dci={} length={} IOC=1 ISP={}",
                    slot,
                    self.dci,
                    bytes,
                    u8::from(data.is_none())
                ),
            );
        }
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
        self.completions = self.completions.saturating_add(1);
        self.last_cc = (words[2] >> 24) as u8;
        self.last_bytes = bytes as u32;
        if self.logged < PIPE_LOG_LIMIT {
            self.logged += 1;
            crate::bootlog::device(
                "bluetooth",
                format_args!(
                    "TransferEvent slot={} dci={} cc={} length={}{}",
                    slot,
                    self.dci,
                    words[2] >> 24,
                    bytes,
                    if self.logged == PIPE_LOG_LIMIT {
                        "; later TDs of this pipe are not logged individually"
                    } else {
                        ""
                    }
                ),
            );
        }
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
    entropy_version: bool,
    entropy_retry: bool,
    entropy_events: u8,
    initialize_deadline: u64,
    scan_command: Option<(u16, u64)>,
    slot: u8,
    adapter_id: (u16, u16),
    adapter_route: u32,
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
    connection_number: u32,
    host_sample: u64,
    started: Option<u64>,
    advertisements: usize,
    advertising: musha_platform::bluetooth::stage::Advertising,
    advertising_budget: musha_platform::bluetooth::advertising_diagnostic::Budget,
    scan_logged: bool,
    unsupported_logged: bool,
    acl_logged: [bool; 2],
    channels_logged: u8,
    att_skipped: u32,
    // Pairing Request time, and whether the no-reply guidance was shown.
    smp_started: u64,
    smp_hinted: bool,
    encryption_observation: musha_platform::bluetooth::connection_diagnostic::Wait,
    rx_observation: musha_platform::bluetooth::rx_diagnostic::Once,
    completion_baseline: [u32; 3],
    acl_in_context: usize,
    // Per connection: packets the controller reported done for our handle
    // (Number of Completed Packets) and HCI events this driver ignores.
    completed_packets: u16,
    ignored_events: u16,
    directed_logged: bool,
    retry: musha_platform::bluetooth::retry::Retry,
    security_request: bool,
    disconnect_deadline: u64,
    // Own address for the current attempt: 0 = controller public, 1 = random
    // static (fresh per new pairing, or the stored bond's on reconnect), and
    // whether LE_Set_Random_Address has applied it.
    own_type: u8,
    own_random: [u8; 6],
    random_set: bool,
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
    // Short SDUs (signaling, SMP, ATT) queued while `tx` is in flight, in order.
    deferred: [Option<(u16, [u8; 72], usize)>; 2],
    signaling: musha_platform::bluetooth::signaling::Signaling,
    update: Option<musha_platform::bluetooth::signaling::Parameters>,
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
        for (_, bytes, _) in self.deferred.iter_mut().flatten() {
            for b in bytes {
                unsafe {
                    core::ptr::write_volatile(b, 0);
                }
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
    route: u32,
    adapter_id: (u16, u16),
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
    // From here every unclaimed Transfer Event of this slot is a Bluetooth fault.
    host.bt_mailbox.own(slot)?;
    let header = musha_platform::bluetooth::usb_context::plan(transport, interval, [0; 3], false)
        .map_err(|_| "BT CONTEXT PLAN")?;
    let eps = [transport.event, transport.acl_in, transport.acl_out];
    let event_bytes =
        musha_platform::bluetooth::usb_context::event_transfer_bytes(transport.event.max_packet);
    let mut pipes = core::array::from_fn(|i| Pipe {
        ring: 0,
        buffer: 0,
        dci: (eps[i].address & 15) * 2 + u8::from(eps[i].address & 0x80 != 0),
        index: i,
        cursor: musha_xhci::Cursor::new(TRBS - 1).unwrap(),
        pending: false,
        bytes: if i == 0 { event_bytes } else { 2048 },
        logged: 0,
        completions: 0,
        last_cc: 0,
        last_bytes: 0,
    });
    unsafe {
        core::ptr::write_bytes(input as *mut u8, 0, 4096);
        dma_word(input + 4, header.add_flags);
        for i in 0..3 {
            dma_word(input + stride + i * 4, read_word(output + i * 4));
        }
        let old = read_word(input + stride);
        let max = header.context_entries;
        dma_word(input + stride, (old & !(31 << 27)) | ((max as u32) << 27));
        for (i, p) in pipes.iter_mut().enumerate() {
            p.ring = pool.allocate(4096, 4096).ok_or("DMA FULL")?;
            p.buffer = pool.allocate(2048, 2048).ok_or("DMA FULL")?;
            core::ptr::write_bytes(p.ring as *mut u8, 0, 4096);
            let plan = musha_platform::bluetooth::usb_context::plan(
                transport,
                interval,
                core::array::from_fn(|j| if j == i { p.ring as u64 } else { 0 }),
                false,
            )
            .map_err(|_| "BT CONTEXT PLAN")?;
            let context = plan.endpoints[i];
            let ep = input + (p.dci as usize + 1) * stride;
            for (j, word) in context.words.into_iter().enumerate() {
                dma_word(ep + j * 4, word);
            }
            crate::bootlog::device(
                "bluetooth",
                format_args!(
                    "endpoint slot={} dci={} type={} maxpacket={} interval={} addflags={:02x} entries={}",
                    slot,
                    p.dci,
                    (context.words[1] >> 3) & 7,
                    eps[i].max_packet,
                    interval,
                    plan.add_flags,
                    plan.context_entries
                ),
            );
            publish(
                p.ring + (TRBS - 1) * 16,
                [p.ring as u32, 0, 0, (6 << 10) | 3],
            );
        }
        core::arch::asm!("mfence", options(nostack));
    }
    let completion = host
        .command(input, (12 << 10) | ((slot as u32) << 24), slot)
        .map_err(|_| "BT CONTEXT COMMAND")?;
    crate::bootlog::device(
        "bluetooth",
        format_args!("ConfigureEndpoint slot={} cc={}", slot, completion[2] >> 24),
    );
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
    crate::bootlog::device(
        "bluetooth",
        format_args!(
            "HCI initial credits=1; SET_CONFIGURATION={} complete; no SET_INTERFACE; event TD={} bytes (maxpacket {})",
            configuration, event_bytes, transport.event.max_packet
        ),
    );
    let device = unsafe {
        (&mut *DEVICE.0.get()).write(Device {
            entropy_samples: musha_platform::bluetooth::sources::HciRand::new(),
            entropy_reset: false,
            entropy_version: false,
            entropy_retry: false,
            entropy_events: 0,
            initialize_deadline: host.bt_deadline.ok_or("BT INITIALIZATION DEADLINE")?,
            scan_command: None,
            slot,
            adapter_id,
            adapter_route: route,
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
            connection_number: 0,
            host_sample: 0,
            started: None,
            advertisements: 0,
            advertising: musha_platform::bluetooth::stage::Advertising::new(),
            advertising_budget: musha_platform::bluetooth::advertising_diagnostic::Budget::new(),
            scan_logged: false,
            unsupported_logged: false,
            acl_logged: [false; 2],
            channels_logged: 0,
            att_skipped: 0,
            smp_started: 0,
            smp_hinted: false,
            encryption_observation: musha_platform::bluetooth::connection_diagnostic::Wait::new(),
            rx_observation: musha_platform::bluetooth::rx_diagnostic::Once::new(),
            completion_baseline: [0; 3],
            acl_in_context: output + ((transport.acl_in.address & 15) * 2 + 1) as usize * stride,
            completed_packets: 0,
            ignored_events: 0,
            directed_logged: false,
            retry: musha_platform::bluetooth::retry::Retry::new(),
            security_request: false,
            disconnect_deadline: 0,
            own_type: 0,
            own_random: [0; 6],
            random_set: false,
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
            deferred: [None; 2],
            signaling: musha_platform::bluetooth::signaling::Signaling::new(),
            update: None,
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
        observe_hci(bytes)?;
        if !self.entropy_samples.done() {
            let event = Event::parse(bytes).map_err(|_| "BT ENTROPY EVENT")?;
            crate::bootlog::device(
                "bluetooth",
                format_args!(
                    "HCI warm-up event code={:02x} param-len={}",
                    event.code,
                    event.parameters.len()
                ),
            );
            let (opcode, deadline) = self.command.ok_or("BT ENTROPY NO COMMAND")?;
            let actual_now = host.clock.now()?;
            isolation::remaining(self.initialize_deadline, actual_now)?;
            let (credits, result) = match isolation::warmup_event(
                bytes,
                opcode,
                deadline,
                actual_now,
                &mut self.entropy_events,
            )? {
                isolation::WarmupEvent::Complete { credits, result } => (credits, result),
                isolation::WarmupEvent::LateReset { credits } => {
                    isolation::retry_after_reset(
                        opcode,
                        deadline,
                        actual_now,
                        &mut self.entropy_retry,
                    )?;
                    crate::bootlog::device(
                        "bluetooth",
                        format_args!(
                            "HCI late Reset CC; retry opcode={:04x} retry=1 deadline-unchanged=true",
                            opcode
                        ),
                    );
                    self.command = None;
                    self.command_credits = credits;
                    let outer_deadline = host.bt_deadline;
                    host.bt_deadline = Some(self.initialize_deadline.min(deadline));
                    let sent = self.send_command(host, actual_now, opcode, &[]);
                    host.bt_deadline = outer_deadline;
                    self.command = Some((opcode, deadline));
                    sent?;
                    return Ok(());
                }
                isolation::WarmupEvent::Pending(kind) => {
                    crate::bootlog::device(
                        "bluetooth",
                        format_args!(
                            "HCI warm-up pending={} expected={:04x} events={}/64",
                            kind, opcode, self.entropy_events
                        ),
                    );
                    return Ok(());
                }
            };
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
            if !self.entropy_version {
                if opcode != 0x1001 || !isolation::valid_version(result) || credits == 0 {
                    return Err("BT INITIAL VERSION REJECTED");
                }
                crate::bootlog::device(
                    "bluetooth",
                    format_args!(
                        "HCI synchronization ReadLocalVersion status=00 HCI={:02x} manufacturer={:04x}",
                        result[1],
                        u16::from_le_bytes([result[5], result[6]])
                    ),
                );
                crate::bootlog::device(
                    "bluetooth",
                    format_args!(
                        "LIVE adapter bdf={:04x} root={} route={:05x} slot={} {:04x}:{:04x} manufacturer={:04x}",
                        host.bdf,
                        self.port,
                        self.adapter_route,
                        self.slot,
                        self.adapter_id.0,
                        self.adapter_id.1,
                        u16::from_le_bytes([result[5], result[6]])
                    ),
                );
                self.command = None;
                self.command_credits = credits;
                self.entropy_version = true;
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
            if bytes.first() == Some(&0x0e) {
                self.scan_command = None;
            }
            if !self.scan_logged
                && self.scan.state() == musha_platform::bluetooth::scan::State::Scanning
            {
                self.scan_logged = true;
                crate::bootlog::device(
                    "bluetooth",
                    format_args!(
                        "LE scan enabled: passive 10ms/10ms, duplicates filtered; reports aggregated per second"
                    ),
                );
            }
        }
        let event =
            musha_platform::bluetooth::Event::parse(bytes).map_err(|_| "BT EVENT FORMAT")?;
        if self.phase == 0 && event.code == 0x3e && event.parameters.first() == Some(&2) {
            self.advertising.event(now);
            let mut unsupported = None;
            let mut directed = None;
            musha_platform::bluetooth::advertisements(event, |advert| {
                self.advertisements = self.advertisements.saturating_add(1);
                self.advertising.report(&advert.address);
                let classification = musha_platform::bluetooth::advertising_diagnostic::classify(advert.event_type, advert.data);
                let admitted = !self.pairing || self.phase != 0 || classification.candidate.is_none()
                    || self.retry.admits(&advert.address, now);
                if self.pairing && self.advertising_budget.admits(classification) {
                    let reason = if self.phase != 0 { "already-selected" }
                        else if classification.candidate.is_some() && !admitted { "deferred" }
                        else { classification.reason };
                    let tail = musha_platform::bluetooth::stage::address_tail(&advert.address);
                    crate::bootlog::device("bluetooth", format_args!(
                        "ADV ..:{:02x}:{:02x} type={} hid={} app={:04x} valid={} reason={} flags={} L={} G={}",
                        tail[0], tail[1], advert.event_type, classification.hid,
                        classification.appearance.unwrap_or(0),
                        classification.ad_valid, reason,classification.flags,classification.flags.limited().map_or("?",|v|if v {"1"}else{"0"}),classification.flags.general().map_or("?",|v|if v {"1"}else{"0"})));
                }
                // A controller that still holds a bond reconnects with directed
                // advertising and no HID data; new pairing needs pairing mode.
                if self.pairing && advert.event_type == 1 {
                    directed.get_or_insert(advert.address);
                }
                if self.phase == 0 && matches!(advert.event_type, 0 | 1) {
                    if !self.pairing {
                        if crate::bluetooth::stored_connectable(
                            advert.event_type,
                            advert.address_type,
                            &advert.address,
                        ) {
                            self.peer = advert.address;
                            self.peer_type = advert.address_type;
                            // Reconnect with the own address the bond was made with.
                            let (kind, own) = crate::bluetooth::stored_own_address(
                                advert.address_type,
                                &advert.address,
                            )
                            .unwrap_or((0, [0; 6]));
                            self.own_type = kind;
                            self.own_random = own;
                            self.random_set = false;
                            self.phase = 1;
                        }
                        return;
                    }
                    if classification.ad_valid && classification.hid {
                        self.method = match classification.candidate {
                            Some(musha_platform::bluetooth::advertising_diagnostic::Candidate::Keyboard) => musha_platform::bluetooth::sc::Method::DisplayPasskey,
                            Some(musha_platform::bluetooth::advertising_diagnostic::Candidate::Gamepad) => musha_platform::bluetooth::sc::Method::JustWorks,
                            None => {
                                self.advertising.unsupported();
                                unsupported.get_or_insert((classification.appearance, advert.address));
                                return;
                            }
                        };
                        if !admitted {
                            self.advertising.deferred();
                            return;
                        }
                        self.advertising.hid();
                        self.peer = advert.address;
                        self.peer_type = advert.address_type;
                        self.phase = 1;
                    }
                }
            })
            .map_err(|_| "BT ADVERTISING")?;
            crate::debug(b"MUSHA: BT_LE_ADVERTISING_VALIDATED\n");
            if let Some(address) = directed.filter(|_| !self.directed_logged) {
                self.directed_logged = true;
                let tail = musha_platform::bluetooth::stage::address_tail(&address);
                crate::bootlog::device(
                    "bluetooth",
                    format_args!(
                        "directed advertising ..:{:02x}:{:02x} ignored for new pairing; the controller may hold an old bond (use its pair button)",
                        tail[0], tail[1]
                    ),
                );
            }
            if let Some((appearance, address)) = unsupported.filter(|_| !self.unsupported_logged) {
                self.unsupported_logged = true;
                let tail = musha_platform::bluetooth::stage::address_tail(&address);
                match appearance {
                    Some(value) => crate::bootlog::device(
                        "bluetooth",
                        format_args!(
                            "HID advertisement ..:{:02x}:{:02x} ignored: appearance={:04x}; pairing accepts 03c1 keyboard or 03c4 gamepad",
                            tail[0], tail[1], value
                        ),
                    ),
                    None => crate::bootlog::device(
                        "bluetooth",
                        format_args!(
                            "HID advertisement ..:{:02x}:{:02x} ignored: no appearance field; pairing accepts 03c1 keyboard or 03c4 gamepad",
                            tail[0], tail[1]
                        ),
                    ),
                }
            }
            if self.phase == 1 {
                self.log_advertising(now, true);
                let tail = musha_platform::bluetooth::stage::address_tail(&self.peer);
                let own = musha_platform::bluetooth::stage::address_tail(&self.own_random);
                let class = address_class(self.peer_type, &self.peer);
                if self.own_type == 1 {
                    crate::bluetooth::set_peer(Some(format_args!(
                        "BT peer ..:{:02x}:{:02x} addr-type={} ({}); own random ..:{:02x}:{:02x}",
                        tail[0], tail[1], self.peer_type, class, own[0], own[1]
                    )));
                } else {
                    crate::bluetooth::set_peer(Some(format_args!(
                        "BT peer ..:{:02x}:{:02x} addr-type={} ({}); own public",
                        tail[0], tail[1], self.peer_type, class
                    )));
                }
                crate::bootlog::device(
                    "bluetooth",
                    format_args!(
                        "HID candidate ..:{:02x}:{:02x} addr-type={} ({}{}) method={}; connecting",
                        tail[0],
                        tail[1],
                        self.peer_type,
                        address_class(self.peer_type, &self.peer),
                        if !self.pairing
                            && crate::bluetooth::stored_resolved(self.peer_type, &self.peer)
                        {
                            ", stored bond via IRK"
                        } else {
                            ""
                        },
                        match (self.pairing, self.method) {
                            (false, _) => "stored-reconnect",
                            (true, musha_platform::bluetooth::sc::Method::JustWorks) => {
                                "JustWorks gamepad"
                            }
                            (true, _) => "DisplayPasskey keyboard",
                        }
                    ),
                );
            }
        } else if self.phase != 0 {
            self.handle_event(host, now, event)?;
        }
        Ok(())
    }
    /// A fresh random static own address from the DRBG for a new pairing, so a
    /// controller that kept an old bond with this host sees an unknown host.
    fn new_own_address(&mut self) -> Result<(), &'static str> {
        use musha_platform::bluetooth::entropy::Random;
        for _ in 0..4 {
            let mut bytes = [0; 6];
            crate::entropy::BootRandom
                .fill(&mut bytes)
                .map_err(|_| "BT OWN ADDRESS ENTROPY")?;
            if let Some(address) = musha_platform::bluetooth::stage::random_static_from(bytes) {
                let tail = musha_platform::bluetooth::stage::address_tail(&address);
                crate::bootlog::device(
                    "bluetooth",
                    format_args!(
                        "own address random static ..:{:02x}:{:02x} (new for this pairing)",
                        tail[0], tail[1]
                    ),
                );
                self.own_type = 1;
                self.own_random = address;
                self.random_set = false;
                return Ok(());
            }
        }
        Err("BT OWN ADDRESS")
    }
    fn log_advertising(&mut self, now: u64, force: bool) {
        let summary = if force {
            self.advertising.flush(now)
        } else {
            self.advertising.due(now)
        };
        if let Some(a) = summary {
            let (high, low) = a.last.map_or((0, 0), |t| (t[0], t[1]));
            crate::bootlog::device(
                "bluetooth",
                format_args!(
                    "advertising window events={} reports={} hid-candidates={} hid-unsupported={} deferred={} last=..:{:02x}:{:02x} adv-suppressed={}",
                    a.events,
                    a.reports,
                    a.hid,
                    a.unsupported,
                    a.deferred,
                    high,
                    low,
                    self.advertising_budget.suppressed()
                ),
            );
        }
    }
    fn poll(&mut self, host: &mut Host<'_>) -> Result<(), &'static str> {
        // An unclaimed event of this slot was consumed by another poller.
        if host.bt_mailbox.faulted() {
            return Err("BT TRANSFER FAULT");
        }
        let now = host.clock.now()?;
        self.host_sample = now;
        if let Some(wait) = self.encryption_observation.due(
            now,
            [
                self.event.completions,
                self.acl_in.completions,
                self.acl_out.completions,
            ],
        ) {
            self.log_encryption_wait(wait, "still waiting for Encryption Change");
            if self.rx_observation.due(true, wait.elapsed) {
                self.observe_acl_receive(host, wait.elapsed, "enc-5s");
            }
        }
        self.event_stream
            .check(self.generation, now)
            .map_err(|_| "BT EVENT STREAM TIMEOUT")?;
        self.acl_stream
            .check(self.generation, now)
            .map_err(|_| "BT ACL STREAM TIMEOUT")?;
        if !self.entropy_samples.done() {
            isolation::remaining(self.initialize_deadline, now)?;
            if let Some((opcode, deadline)) = self.command {
                if now >= deadline {
                    timeout(opcode, now.saturating_sub(deadline.saturating_sub(1000)));
                    return Err("BT ENTROPY COMMAND TIMEOUT");
                }
            } else {
                self.entropy_retry = false;
                let opcode = if !self.entropy_reset {
                    0x0c03
                } else if !self.entropy_version {
                    0x1001
                } else {
                    0x2018
                };
                self.send_command(host, now, opcode, &[])?;
            }
            // Match the proven probe order: finish the EP0 command TD first,
            // then arm the interrupt IN TD. USB device event FIFO retains CC.
            self.event.arm(host, self.slot, None)?;
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
        if crate::bluetooth::take_input_overflow() {
            return Err("BT INPUT OVERFLOW");
        }
        if self.started.is_some() && crate::bluetooth::take_request() {
            match musha_platform::bluetooth::repair::action(true, self.pairing, self.subscribed) {
                musha_platform::bluetooth::repair::Action::RestartFresh => {
                    if crate::bluetooth::restart_after_stop(true) {
                        super::super::request_session_stop(
                            "fresh pairing requested during stored reconnect",
                        );
                        return Err("BT REPAIR REQUEST");
                    }
                    crate::bluetooth::status(
                        "Bluetooth restart budget exhausted; restart Musha-OS.",
                    );
                }
                _ => crate::bluetooth::status("Bluetooth fresh pairing already in progress."),
            }
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
            if self.pairing {
                self.new_own_address()?;
            }
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
            self.log_advertising(now, false);
            match self.scan.next(self.generation, now) {
                Ok(Some(command)) => {
                    let bytes = &command.bytes[..command.length];
                    let opcode = u16::from_le_bytes([bytes[0], bytes[1]]);
                    self.scan_command = Some((opcode, now));
                    crate::bootlog::device(
                        "bluetooth",
                        format_args!("HCI TX opcode={:04x}", opcode),
                    );
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
                Err(_) => {
                    if let Some((opcode, started)) = self.scan_command {
                        timeout(opcode, now.saturating_sub(started));
                    }
                    return Err("BT SCAN STATE");
                }
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
        // Phases 9/10 are our own disconnect and rescan: no ACL is serviced.
        if self.phase == 8 {
            self.poll_acl(host, now)?;
        }
        Ok(())
    }
}
pub(super) use isolation::local_failure;
pub(in crate::xhci) use isolation::remaining;
/// LE address class from the type and the two top bits (HCI order).
fn address_class(kind: u8, address: &[u8; 6]) -> &'static str {
    match (kind, address[5] >> 6) {
        (0, _) => "public",
        (_, 3) => "random static",
        (_, 1) => "resolvable private",
        _ => "non-resolvable private",
    }
}
/// Completion codes only; transfer buffers are never read for a fault.
fn log_fault(host: &mut Host<'_>) {
    if let Some((words, count)) = host.bt_mailbox.take_fault() {
        let cc = words[2] >> 24;
        crate::bootlog::device(
            "bluetooth",
            format_args!(
                "transfer fault slot={} dci={} cc={} ({}) residual={} events={}",
                words[3] >> 24,
                (words[3] >> 16) & 31,
                cc,
                match cc {
                    1 => "success on an unknown TD",
                    3 => "babble",
                    4 => "USB transaction error",
                    6 => "stall",
                    13 => "short packet on an unknown TD",
                    26 => "stopped",
                    _ => "other",
                },
                words[2] & 0xffffff,
                count
            ),
        );
    }
}
pub(super) fn retire(host: &mut Host<'_>, slot: u8) -> Result<(), &'static str> {
    log_fault(host);
    let control = host.bt_control_pending.filter(|(s, _, _)| *s == slot);
    let addresses = control.as_ref().map_or(&[][..], |(_, a, n)| &a[..*n]);
    host.bt_mailbox.retire_slot(slot, addresses)
}
pub(super) fn unavailable(reason: &'static str) {
    crate::bluetooth::isolated();
    crate::bootlog::event(
        musha_platform::boot_log::Level::Ready,
        "bluetooth",
        format_args!("unavailable: {}", reason),
    );
    // Viewer status lines are static; the prefix is joined at compile time.
    macro_rules! prefixed {
        ($($known:literal),* $(,)?) => {
            match reason {
                $($known => concat!("Bluetooth unavailable: ", $known),)*
                _ => reason,
            }
        };
    }
    match reason {
        "BT PAIRING NO RESPONSE" => {
            return crate::bluetooth::status(musha_platform::bluetooth::stage::PAIRING_TIMED_OUT);
        }
        _ => {}
    }
    crate::bluetooth::status(prefixed!(
        "BT ENTROPY COMMAND TIMEOUT",
        "BT INITIALIZATION TIMEOUT",
        "BT COMMAND TIMEOUT",
        "TRANSFER TIMEOUT",
        "BT COMMAND REJECTED",
        "BT ENTROPY RESET",
        "BT TRANSFER FAULT",
        "BT INPUT OVERFLOW",
        "BT BOND SAVE UNKNOWN",
        "BT BOND SAVE LAYOUT",
        "BT BOND SAVE BUDGET",
        "BT BOND SAVE NO SETTINGS",
        "BT BOND SAVE SETTINGS FULL",
        "BT BOND SAVE NO SPACE",
        "BT BOND SAVE PROTECTED",
        "BT BOND SAVE CAPACITY",
        "BT BOND SAVE FILESYSTEM",
        "BT BOND SAVE UNOWNED",
        "BT BOND SAVE IO",
        "BT BOND SAVE RECOVERY REQUIRED",
        "BT BOND SAVE FULL",
        "BT BOND SAVE WEAKER KEY",
        "BT BOND SAVE RECORD",
        "BT ADVERTISING",
        "BT EVENT FORMAT",
        "BT EVENT STATE",
        "BT EVENT STREAM",
        "BT SCAN STATE",
        "BT COMMAND STATUS",
        "BT ASYNC COMMAND",
        "BT SYNC COMMAND",
        "BT CONNECTION IDENTITY",
        "BT CONNECTION LOST",
        "BT HARDWARE ERROR",
        "BT ACL TX TIMEOUT",
        "BT ACL REASSEMBLY",
        "BT SMP TIMEOUT",
        "BT SC AUTHENTICATION",
        "BT ENCRYPTION REJECTED",
        "BT PAIR TIMEOUT",
        "BT DISCONNECT TIMEOUT",
        "BT OWN ADDRESS",
        "BT OWN ADDRESS ENTROPY",
        "BT DISCONNECT FAILED",
        "BT CCCD WRITE REJECTED",
        "BT PAIRING FAILED PASSKEY ENTRY",
        "BT PAIRING FAILED OOB NOT AVAILABLE",
        "BT PAIRING FAILED AUTHENTICATION REQUIREMENTS",
        "BT PAIRING FAILED CONFIRM VALUE",
        "BT PAIRING FAILED NOT SUPPORTED",
        "BT PAIRING FAILED ENCRYPTION KEY SIZE",
        "BT PAIRING FAILED COMMAND NOT SUPPORTED",
        "BT PAIRING FAILED UNSPECIFIED",
        "BT PAIRING FAILED REPEATED ATTEMPTS",
        "BT PAIRING FAILED INVALID PARAMETERS",
        "BT PAIRING FAILED DHKEY CHECK",
        "BT PAIRING FAILED NUMERIC COMPARISON",
        "BT PAIRING FAILED BR/EDR IN PROGRESS",
        "BT PAIRING FAILED CROSS-TRANSPORT KEY",
        "BT PAIRING FAILED KEY REJECTED",
        "BT PAIRING FAILED OTHER",
        "BT HOGP SERVICES REJECTED",
        "BT HOGP CHARACTERISTICS REJECTED",
        "BT HOGP REPORT MAP REJECTED",
        "BT HOGP DESCRIPTORS REJECTED",
        "BT HOGP REFERENCE OR MAP BINDING REJECTED",
        "BT L2CAP SIGNALING",
        "BT DISCONNECTED",
        "SHORT CONTROL WRITE",
    ));
}
fn timeout(opcode: u16, elapsed: u64) {
    crate::bootlog::device(
        "bluetooth",
        format_args!("HCI timeout opcode={:04x} after={}ms", opcode, elapsed),
    );
}
fn observe_hci(bytes: &[u8]) -> Result<(), &'static str> {
    let Some((kind, credits, opcode, status)) = isolation::command_event(bytes)? else {
        return Ok(());
    };
    match status {
        Some(status) => crate::bootlog::device(
            "bluetooth",
            format_args!(
                "HCI {} opcode={:04x} status={:02x} credits={}",
                kind, opcode, status, credits
            ),
        ),
        None => crate::bootlog::device(
            "bluetooth",
            format_args!(
                "HCI {} opcode={:04x} status=n/a credits={}",
                kind, opcode, credits
            ),
        ),
    }
    Ok(())
}
pub(super) fn poll(
    host: &mut Host<'_>,
    _tick: &mut super::super::Tick<'_>,
) -> Result<(), &'static str> {
    let Some(mut device) = host.bluetooth.take() else {
        return Ok(());
    };
    let initializing = !device.entropy_samples.done();
    if initializing {
        host.bt_deadline = Some(device.initialize_deadline);
    }
    let result = device.poll(host);
    host.bt_deadline = None;
    if let Err(reason) = result {
        let counts = [
            device.event.completions,
            device.acl_in.completions,
            device.acl_out.completions,
        ];
        if let Some(wait) = device.encryption_observation.finish_observed(counts) {
            device.log_encryption_wait(wait, reason);
        }
        if device.handle != 0 && device.rx_observation.terminal() {
            device.observe_acl_receive(
                host,
                device.host_sample.saturating_sub(device.smp_started),
                "end-last-sample",
            );
        }
        if matches!(reason, "TRANSFER TIMEOUT" | "BT INITIALIZATION TIMEOUT") {
            if let Some((opcode, deadline)) = device.command {
                timeout(
                    opcode,
                    host.clock
                        .now()?
                        .saturating_sub(deadline.saturating_sub(1000)),
                );
            }
        }
        if !local_failure(reason) {
            log_fault(host);
            crate::bootlog::device(
                "bluetooth",
                format_args!("non-local failure: {}; USB session stops", reason),
            );
            host.bluetooth = Some(device);
            return Err(reason);
        }
        // Known allocations remain reserved; exact old TDs may finish while the
        // correlated Disable Slot command drains endpoints. Never reset the
        // shared event cursor or another device's saved completions.
        let slot = device.slot;
        let isolated = isolation::disable(&mut Stop(host), slot);
        if let Err(error) = isolated {
            crate::bootlog::device(
                "bluetooth",
                format_args!(
                    "isolation failed: {} after {}; USB session stops",
                    error, reason
                ),
            );
            host.bluetooth = Some(device);
            return Err(error);
        }
        drop(device); // retires CPU lease; does NOT free lease or DMA allocations.
        if isolated == Ok(isolation::Isolation::Quarantined) {
            crate::bootlog::device(
                "bluetooth",
                format_args!(
                    "Disable Slot refused; slot={} quarantined with context and DMA retained",
                    slot
                ),
            );
        }
        unavailable(reason);
        crate::debug(b"MUSHA: BT_ISOLATED USB_SESSION_CONTINUES\n");
        return Ok(());
    }
    host.bluetooth = Some(device);
    Ok(())
}

mod connection;

struct Stop<'h, 'a>(&'h mut Host<'a>);
impl isolation::SlotStop for Stop<'_, '_> {
    fn retire(&mut self, slot: u8) -> Result<(), &'static str> {
        retire(self.0, slot)
    }
    fn disable(&mut self, slot: u8) -> Result<(), &'static str> {
        self.0
            .command(0, (10 << 10) | ((slot as u32) << 24), slot)
            .map(|_| ())
    }
    fn detach(&mut self, slot: u8) {
        unsafe {
            super::dma_u64(self.0.dcbaa + slot as usize * 8, 0);
            core::arch::asm!("mfence", options(nostack));
        }
        self.0.bt_control_pending = None;
        self.0.bt_disabled = true;
        self.0.bt_mailbox.disown();
    }
    fn quarantine(&mut self, _slot: u8) {
        // The slot may still be enabled: its DCBAA entry must stay valid and the
        // mailbox keeps absorbing its events. Nothing re-arms its endpoints.
        self.0.bt_control_pending = None;
        self.0.bt_disabled = true;
    }
}
pub(super) fn disable_failed_prepare(host: &mut Host<'_>, slot: u8) -> Result<(), &'static str> {
    isolation::disable(&mut Stop(host), slot).map(|_| ())
}
mod isolation;

pub(super) fn warm_up(host: &mut Host<'_>, device: &mut DeviceRef) -> Result<(), &'static str> {
    // Keep a quiet outstanding IN loan armed: EP0 uses a distinct buffer. Never
    // clear pending/cursor/mailbox on quiet timeout or reuse its DMA storage.
    let slot = device.slot;
    let start = host.clock.now()?;
    let mut drain = isolation::Drain::new(start);
    let mut stream =
        musha_platform::bluetooth::stream::Stream::<257>::new(device.generation, false);
    loop {
        let now = host.clock.now()?;
        isolation::remaining(device.initialize_deadline, now)?;
        if drain.finished(now)? {
            break;
        }
        device.event.arm(host, slot, None)?;
        if let Some(n) = device.event.poll(host, slot)? {
            let mut bytes = [0; 257];
            if n > bytes.len() {
                return Err("BT DRAIN LENGTH");
            }
            for (i, b) in bytes[..n].iter_mut().enumerate() {
                *b = unsafe { (device.event.buffer as *const u8).add(i).read_volatile() };
            }
            let mut input = &bytes[..n];
            while !input.is_empty() {
                let Some(frame) = stream
                    .next(device.generation, now, &mut input)
                    .map_err(|_| "BT DRAIN STREAM")?
                else {
                    break;
                };
                let event = Event::parse(frame).map_err(|_| "BT DRAIN EVENT")?;
                observe_hci(frame)?;
                crate::bootlog::device(
                    "bluetooth",
                    format_args!(
                        "HCI drain event code={:02x} param-len={}",
                        event.code,
                        event.parameters.len()
                    ),
                );
                // Validate command/known-event forms without changing live command state.
                let _ = isolation::warmup_event(
                    frame,
                    0x0c03,
                    device.initialize_deadline,
                    now,
                    &mut device.entropy_events,
                )?;
                drain.event(now)?;
                stream.release();
            }
        }
        core::hint::spin_loop();
    }
    // Preserve any incomplete framed event rather than silently dropping it.
    device.event_stream = stream;
    crate::bootlog::device(
        "bluetooth",
        format_args!(
            "HCI drain complete count={} elapsed={}ms pending-IN={}",
            drain.count(),
            host.clock.now()?.saturating_sub(start),
            device.event.pending
        ),
    );
    while !device.entropy_samples.done() {
        device.poll(host)?;
    }
    Ok(())
}
