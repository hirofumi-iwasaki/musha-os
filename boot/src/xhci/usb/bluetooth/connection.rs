// Copyright 2026 Hirofumi Iwasaki
// SPDX-License-Identifier: Apache-2.0
//! BSP-only live HCI connection and SC path. All failures propagate to the
//! existing host halt/BME-off boundary; no DMA allocation is reclaimed here.
use super::*;
use musha_platform::bluetooth::{self as bt, sc::Action};
impl Device {
    pub(super) fn log_encryption_wait(
        &self,
        w: bt::connection_diagnostic::WaitSnapshot,
        why: &str,
    ) {
        crate::bootlog::device(
            "bluetooth",
            format_args!(
                "ENC wait mode={} handle={:04x} elapsed={}ms accepted={}",
                w.mode.name(),
                w.handle,
                w.elapsed,
                w.accepted
            ),
        );
        crate::bootlog::device(
            "bluetooth",
            format_args!(
                "ENC local={} deadline={} reason={}",
                w.sampled_at, w.deadline, why
            ),
        );
        crate::bootlog::device(
            "bluetooth",
            format_args!(
                "ENC completions event={} in={} out={} (since request)",
                w.completions[0], w.completions[1], w.completions[2]
            ),
        );
    }
    pub(super) fn send_command(
        &mut self,
        host: &mut Host<'_>,
        now: u64,
        opcode: u16,
        parameters: &[u8],
    ) -> Result<(), &'static str> {
        if let Some(reconnect) = self.reconnect.as_mut() {
            reconnect
                .check(self.generation, now)
                .map_err(|_| "BT RECONNECT TIMEOUT")?;
        }
        if self.command.is_some() || self.command_credits == 0 {
            return Err("BT COMMAND CREDIT");
        }
        let mut bytes = [0; 35];
        if parameters.len() > 32 {
            return Err("BT COMMAND SIZE");
        }
        bytes[..2].copy_from_slice(&opcode.to_le_bytes());
        bytes[2] = parameters.len() as u8;
        bytes[3..3 + parameters.len()].copy_from_slice(parameters);
        let setup = self
            .transport
            .command_setup(&bytes[..3 + parameters.len()], self.composite)
            .map_err(|_| "BT COMMAND SETUP")?;
        crate::bootlog::device(
            "bluetooth",
            format_args!("HCI TX intent opcode={:04x}", opcode),
        );
        self.command = Some((opcode, now.checked_add(1000).ok_or("BT TIME")?));
        self.command_credits -= 1;
        let result = control_write(
            host,
            self.slot,
            self.control_ring,
            self.buffer,
            &bytes[..3 + parameters.len()],
            u32::from_le_bytes(setup[..4].try_into().unwrap()),
            u16::from_le_bytes(setup[4..6].try_into().unwrap()),
        );
        // Command CPU/DMA scratch may contain LTK. Volatile clearing survives DSE.
        for b in &mut bytes {
            unsafe {
                core::ptr::write_volatile(b, 0);
            }
        }
        // A failed transfer can still own this DMA buffer. Leave it quarantined
        // until the outer host confirms halt/BME-off; never scrub in flight.
        if result.is_ok() {
            unsafe {
                core::ptr::write_bytes(self.buffer as *mut u8, 0, 35);
            }
        }
        result
    }
    pub(super) fn drive(&mut self, host: &mut Host<'_>, now: u64) -> Result<(), &'static str> {
        if let Some((opcode, deadline)) = self.command {
            if now >= deadline {
                timeout(opcode, now.saturating_sub(deadline.saturating_sub(1000)));
                return Err("BT COMMAND TIMEOUT");
            }
        }
        if let Some(reconnect) = self.reconnect.as_mut() {
            reconnect
                .check(self.generation, now)
                .map_err(|_| "BT RECONNECT TIMEOUT")?;
        }
        if let Some(security) = self.security.as_mut() {
            let awaiting = security.awaiting_response();
            security.check(self.generation, now).map_err(|_| {
                if awaiting {
                    crate::bootlog::device(
                        "bluetooth",
                        format_args!("SMP timeout with no Pairing Response"),
                    );
                    "BT PAIRING NO RESPONSE"
                } else {
                    "BT SMP TIMEOUT"
                }
            })?;
        }
        // Our own Disconnect must complete within 1 s, or only Bluetooth stops.
        if self.phase == 9 && now >= self.disconnect_deadline {
            return Err("BT DISCONNECT TIMEOUT");
        }
        if self.command.is_some() || self.command_credits == 0 {
            return Ok(());
        }
        // A silent connection to a controller that keeps an old bond: give up
        // on it after 5 s (or at once on a Security Request) and rescan.
        if self.phase == 8
            && self
                .security
                .as_ref()
                .is_some_and(|s| s.awaiting_response())
        {
            let waited = now.saturating_sub(self.smp_started);
            if self.rx_observation.due(true, waited) {
                self.observe_acl_receive(host, waited, "smp-5s");
            }
            match self.retry.decide(waited, self.security_request) {
                bt::retry::Decision::Wait => {}
                bt::retry::Decision::Disconnect => return self.give_up_connection(host, now),
                bt::retry::Decision::Exhausted => {
                    if !self.smp_hinted {
                        self.smp_hinted = true;
                        crate::bootlog::device(
                            "bluetooth",
                            format_args!(
                                "no Pairing Response after {}ms and {} attempts; waiting for the SMP timeout; completed-packets={} ignored-events={}",
                                waited,
                                self.retry.attempts(),
                                self.completed_packets,
                                self.ignored_events
                            ),
                        );
                        crate::bluetooth::status(bt::stage::PAIRING_NO_REPLY);
                    }
                }
            }
        }
        if let Some(stage) = match self.phase {
            1 => Some("set event mask"),
            2 => Some("LE set event mask"),
            3 => Some("read BD_ADDR"),
            4 if self.shared_capacity => Some("read shared ACL buffer size"),
            4 => Some("LE read buffer size"),
            5 => Some("LE scan disable"),
            6 if self.own_type == 1 && !self.random_set => Some("LE set random address"),
            6 => Some("LE create connection"),
            8 if self.encrypt.is_some() => Some("LE start encryption"),
            8 if self.update.is_some() && self.tx_len == 0 => Some("LE connection update"),
            10 => Some("LE scan re-enable"),
            _ => None,
        } {
            crate::bootlog::device(
                "bluetooth",
                format_args!("connect stage={} phase={}", stage, self.phase),
            );
        }
        match self.phase {
            1 => self.send_command(host, now, 0x0c01, &[0x90, 0xe0, 4, 0, 0, 0, 0, 0x20]),
            // Connection Complete, Advertising Report, Connection Update
            // Complete and Long Term Key Request.
            2 => self.send_command(host, now, 0x2001, &[0x17, 0, 0, 0, 0, 0, 0, 0]),
            3 => self.send_command(host, now, 0x1009, &[]),
            4 => self.send_command(
                host,
                now,
                if self.shared_capacity { 0x1005 } else { 0x2002 },
                &[],
            ),
            5 => self.send_command(host, now, 0x200c, &[0, 0]),
            // Scanning is off here, so the random address may be changed.
            6 if self.own_type == 1 && !self.random_set => {
                let address = self.own_random;
                self.send_command(host, now, 0x2005, &address)
            }
            6 => {
                let p = bt::stage::create_connection(self.peer_type, &self.peer, self.own_type);
                self.send_command(host, now, 0x200d, &p)
            }
            8 if self.encrypt.is_some() => {
                let mut key = self.encrypt.take().unwrap();
                let mut p = [0; 28];
                p[..2].copy_from_slice(&self.handle.to_le_bytes());
                p[12..].copy_from_slice(&key);
                let deadline = self
                    .reconnect
                    .as_ref()
                    .map(|r| r.deadline_ms())
                    .or_else(|| self.security.as_ref().map(|s| s.deadline_ms()))
                    .unwrap_or(now);
                let mode = bt::connection_diagnostic::Mode::from_pairing(self.pairing);
                self.encryption_observation.begin(
                    mode,
                    self.handle,
                    now,
                    deadline,
                    [
                        self.event.completions,
                        self.acl_in.completions,
                        self.acl_out.completions,
                    ],
                );
                crate::bootlog::device(
                    "bluetooth",
                    format_args!(
                        "ENC request mode={} stored-key={} handle={:04x} deadline={} (existing)",
                        mode.name(),
                        mode.stored_key(),
                        self.handle,
                        deadline
                    ),
                );
                let result = self.send_command(host, now, 0x2019, &p);
                for b in p.iter_mut().chain(key.iter_mut()) {
                    unsafe {
                        core::ptr::write_volatile(b, 0);
                    }
                }
                result
            }
            // After the accepted signaling response has left the TX slot.
            8 if self.update.is_some() && self.tx_len == 0 => {
                let parameters = self.update.take().unwrap();
                self.send_command(host, now, 0x2013, &parameters.command(self.handle))
            }
            // Disconnected by us: scanning resumes with the pairing request kept.
            10 => self.send_command(host, now, 0x200c, &[1, 1]),
            _ => Ok(()),
        }
    }
    /// Observe only allocated endpoint context and CPU-owned completion metadata.
    /// No endpoint commands, ring consumption, payload reads or DMA reclamation.
    pub(super) fn observe_acl_receive(&self, host: &Host<'_>, waited: u64, boundary: &str) {
        let endpoint = bt::rx_diagnostic::Endpoint::decode(unsafe {
            [
                read_word(self.acl_in_context),
                read_word(self.acl_in_context + 4),
            ]
        });
        let trb = self.acl_in.ring + self.acl_in.cursor.index * 16;
        // Both reads remain within this allocated output endpoint context.
        // DMA can advance between samples, so report equality rather than assume a snapshot.
        let first = unsafe {
            [
                read_word(self.acl_in_context + 8),
                read_word(self.acl_in_context + 12),
            ]
        };
        let second = unsafe {
            [
                read_word(self.acl_in_context + 8),
                read_word(self.acl_in_context + 12),
            ]
        };
        // Only the owned ring's TRB control word is read, never its buffer contents.
        let control = unsafe { read_word(trb + 12) };
        let position = bt::rx_diagnostic::Position::decode(
            first,
            second,
            self.acl_in.ring as u64,
            TRBS,
            trb as u64,
            self.acl_in.cursor.cycle as u8,
            control,
        );
        let event_trb = self.event.ring + self.event.cursor.index * 16;
        let (event_owned, event_saved) =
            host.bt_mailbox
                .observation(self.event.index, event_trb, self.slot, self.event.dci);
        crate::bootlog::device(
            "bluetooth",
            format_args!(
                "RX pos gen={} conn={} bdf={:04x} root={} slot={} h={:04x} at={}",
                self.generation,
                self.connection_number,
                host.bdf,
                self.port,
                self.slot,
                self.handle,
                boundary
            ),
        );
        crate::bootlog::device(
            "bluetooth",
            format_args!(
                "RX local={} waited={}ms (last poll sample)",
                self.host_sample, waited
            ),
        );
        crate::bootlog::device(
            "bluetooth",
            format_args!(
                "ACL IN observed deq={:016x} DCS={} idx={:?} relation={} stable={} (position only)",
                position.dequeue,
                position.dcs,
                position.ring_index,
                position.relation(),
                position.stable
            ),
        );
        crate::bootlog::device(
            "bluetooth",
            format_args!(
                "ACL IN expected td={:016x} ring={:016x} idx={} cycle={} ctrl={:08x}",
                position.expected,
                self.acl_in.ring,
                self.acl_in.cursor.index,
                position.cycle,
                position.control
            ),
        );
        crate::bootlog::device(
            "bluetooth",
            format_args!(
                "EV IN slot={} dci={} pending={} owned={} saved={} td={:016x} idx={} cycle={} ER idx={} cycle={}",
                self.slot,
                self.event.dci,
                self.event.pending,
                event_owned,
                event_saved,
                event_trb,
                self.event.cursor.index,
                self.event.cursor.cycle,
                host.consumer.index,
                host.consumer.cycle
            ),
        );
        let (owned, saved) =
            host.bt_mailbox
                .observation(self.acl_in.index, trb, self.slot, self.acl_in.dci);
        let counts = bt::rx_diagnostic::counts(
            [
                self.event.completions,
                self.acl_in.completions,
                self.acl_out.completions,
            ],
            self.completion_baseline,
        );
        crate::bootlog::device(
            "bluetooth",
            format_args!(
                "ACL RX wait={}ms slot={} handle={:04x} ep={:02x} dci={} state={} type={} maxpacket={}",
                waited,
                self.slot,
                self.handle,
                self.transport.acl_in.address,
                self.acl_in.dci,
                endpoint.state,
                endpoint.kind,
                endpoint.max_packet
            ),
        );
        crate::bootlog::device(
            "bluetooth",
            format_args!(
                "ACL RX pending={} owned={} saved={} td={} cycle={} bytes={}",
                self.acl_in.pending,
                owned,
                saved,
                self.acl_in.cursor.index,
                self.acl_in.cursor.cycle,
                self.acl_in.bytes
            ),
        );
        crate::bootlog::device(
            "bluetooth",
            format_args!(
                "ACL RX completions event={} in={} out={} last-in-cc={} last-in-bytes={}",
                counts[0], counts[1], counts[2], self.acl_in.last_cc, self.acl_in.last_bytes
            ),
        );
    }
    /// HCI Disconnect (reason 0x13, remote user terminated) of a connection
    /// whose peer does not answer pairing; its address is deferred for 10 s.
    fn give_up_connection(&mut self, host: &mut Host<'_>, now: u64) -> Result<(), &'static str> {
        let tail = bt::stage::address_tail(&self.peer);
        crate::bootlog::device(
            "bluetooth",
            format_args!(
                "{} (attempt {}/{}); completed-packets={} ignored-events={}; disconnecting ..:{:02x}:{:02x} and deferring it for {}ms",
                if self.security_request {
                    "SMP Security Request: the controller holds an old bond"
                } else {
                    "no Pairing Response after 5000ms; the controller may hold an old bond"
                },
                self.retry.attempts() + 1,
                bt::retry::MAX_ATTEMPTS,
                self.completed_packets,
                self.ignored_events,
                tail[0],
                tail[1],
                bt::retry::DEFER_MS
            ),
        );
        crate::bluetooth::status(bt::stage::PAIRING_NO_REPLY);
        let mut p = [0; 3];
        p[..2].copy_from_slice(&self.handle.to_le_bytes());
        p[2] = 0x13;
        self.send_command(host, now, 0x0406, &p)?;
        self.retry.disconnected(self.peer, now);
        // The SMP attempt ends with the link; its 30 s timer must not fire.
        self.security = None;
        self.phase = 9;
        self.disconnect_deadline = now.checked_add(1000).ok_or("BT TIME")?;
        Ok(())
    }
    /// Release per-connection state after our own Disconnect completed.
    fn reset_connection(&mut self) {
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
        self.deferred = [None; 2];
        self.tx_len = 0;
        self.tx_at = 0;
        self.handle = 0;
        self.credits = None;
        self.reassembly = None;
        self.acl_stream = bt::stream::Stream::new(self.generation, true);
        self.security = None;
        self.encryption_observation.clear();
        self.security_request = false;
        self.update = None;
        self.signaling = bt::signaling::Signaling::new();
        self.acl_logged = [false; 2];
        self.channels_logged = 0;
        self.smp_hinted = false;
        self.completed_packets = 0;
        self.ignored_events = 0;
        crate::bluetooth::set_peer(None);
    }
    pub(super) fn handle_event(
        &mut self,
        host: &mut Host<'_>,
        now: u64,
        event: Event<'_>,
    ) -> Result<(), &'static str> {
        match event.code {
            // Completion of our own Disconnect: release the link and rescan.
            5 if self.phase == 9 => {
                let p = event.parameters;
                if p.len() != 4 || u16::from_le_bytes([p[1], p[2]]) != self.handle {
                    return Err("BT DISCONNECT IDENTITY");
                }
                if p[0] != 0 {
                    return Err("BT DISCONNECT FAILED");
                }
                crate::bootlog::device(
                    "bluetooth",
                    format_args!(
                        "Disconnection Complete status=00 handle={:04x} reason={:02x}; rescanning",
                        self.handle, p[3]
                    ),
                );
                self.reset_connection();
                self.phase = 10;
            }
            0x10 | 5 => {
                let p = event.parameters;
                if event.code == 5 && p.len() == 4 {
                    crate::bootlog::device(
                        "bluetooth",
                        format_args!(
                            "Disconnection Complete status={:02x} handle={:04x} reason={:02x}",
                            p[0],
                            u16::from_le_bytes([p[1], p[2]]),
                            p[3]
                        ),
                    );
                } else if event.code == 0x10 && p.len() == 1 {
                    crate::bootlog::device(
                        "bluetooth",
                        format_args!("Hardware Error code={:02x}", p[0]),
                    );
                }
                let loss = bt::lifecycle::loss(event, self.handle)
                    .map_err(|_| "BT LOSS IDENTITY")?
                    .ok_or("BT LOSS STATE")?;
                if loss == bt::lifecycle::Loss::Hardware || self.subscribed {
                    crate::bluetooth::restart_after_stop(false);
                }
                return Err(if loss == bt::lifecycle::Loss::Hardware {
                    "BT HARDWARE ERROR"
                } else {
                    "BT CONNECTION LOST"
                });
            }
            0x0e | 0x0f => {
                let p = event.parameters;
                // A refused LE Connection Update keeps the current parameters.
                if event.code == 0x0f
                    && p.len() == 4
                    && p[0] != 0
                    && self.command.map(|c| c.0) == Some(0x2013)
                    && u16::from_le_bytes([p[2], p[3]]) == 0x2013
                {
                    crate::bootlog::device(
                        "bluetooth",
                        format_args!(
                            "LE connection update refused CS status={:02x}; connection kept",
                            p[0]
                        ),
                    );
                    self.command_credits = p[1];
                    self.command = None;
                    return Ok(());
                }
                let (op, result) = match isolation::command_reply(
                    event.code,
                    event.parameters,
                    self.command.map(|c| c.0),
                )? {
                    isolation::Reply::Credits(credits) => {
                        self.command_credits = credits;
                        return Ok(());
                    }
                    // Command Status: only the controller's acceptance; the
                    // connection or encryption result arrives as its own event.
                    isolation::Reply::Accepted { credits, opcode } => {
                        self.command_credits = credits;
                        self.command = None;
                        match opcode {
                            0x200d => {
                                crate::bootlog::device(
                                    "bluetooth",
                                    format_args!(
                                        "LE create connection accepted (CS 00); awaiting LE Connection Complete"
                                    ),
                                );
                                self.phase = 7;
                            }
                            0x2013 => crate::bootlog::device(
                                "bluetooth",
                                format_args!(
                                    "LE connection update accepted (CS 00); awaiting update complete"
                                ),
                            ),
                            0x0406 => crate::bootlog::device(
                                "bluetooth",
                                format_args!(
                                    "disconnect accepted (CS 00); awaiting Disconnection Complete"
                                ),
                            ),
                            0x2019 => {
                                self.encryption_started = true;
                                self.encryption_observation.accepted();
                                crate::bootlog::device(
                                    "bluetooth",
                                    format_args!(
                                        "ENC command accepted handle={:04x}; awaiting Encryption Change",
                                        self.handle
                                    ),
                                );
                            }
                            _ => self.encryption_started = true,
                        }
                        return Ok(());
                    }
                    isolation::Reply::Complete {
                        credits,
                        opcode,
                        result,
                    } => {
                        self.command_credits = credits;
                        self.command = None;
                        (opcode, result)
                    }
                };
                if op == 0x2005 {
                    if result.len() != 1 {
                        return Err("BT COMMAND LENGTH");
                    }
                    crate::bootlog::device(
                        "bluetooth",
                        format_args!("LE random address set (CC 00)"),
                    );
                    self.random_set = true;
                    return Ok(());
                }
                if op == 0x200c && self.phase == 10 {
                    if result.len() != 1 {
                        return Err("BT COMMAND LENGTH");
                    }
                    crate::bootlog::device(
                        "bluetooth",
                        format_args!("LE scan re-enabled; pairing request kept"),
                    );
                    self.phase = 0;
                    return Ok(());
                }
                if op == 0x1009 {
                    if result.len() != 7 {
                        return Err("BT ADDRESS");
                    }
                    self.own.copy_from_slice(&result[1..]);
                } else if matches!(op, 0x2002 | 0x1005) {
                    match bt::acl::capacity(op, result).map_err(|_| "BT ACL CAPACITY")? {
                        None if op == 0x2002 => {
                            self.shared_capacity = true;
                            return Ok(()); // stay at phase4, issue Read Buffer Size.
                        }
                        Some((bytes, packets)) => {
                            crate::bootlog::device(
                                "bluetooth",
                                format_args!(
                                    "ACL buffer bytes={} packets={} source={}",
                                    bytes,
                                    packets,
                                    if op == 0x2002 { "LE" } else { "shared BR/EDR" }
                                ),
                            );
                            self.packet_bytes = bytes;
                            self.packets = packets;
                        }
                        _ => return Err("BT SHARED ACL CAPACITY"),
                    }
                } else if result.len() != 1 {
                    return Err("BT COMMAND LENGTH");
                }
                self.phase += 1;
            }
            0x3e if event.parameters.first() == Some(&1) => {
                let p = event.parameters;
                match bt::stage::connection_complete(p) {
                    Some(c) => crate::bootlog::device(
                        "bluetooth",
                        format_args!(
                            "LE Connection Complete status={:02x} handle={:04x} role={} peer=..:{:02x}:{:02x} type={} interval={}x1.25ms latency={} timeout={}x10ms",
                            c.status,
                            c.handle,
                            c.role,
                            c.peer_tail[0],
                            c.peer_tail[1],
                            c.peer_type,
                            c.interval,
                            c.latency,
                            c.timeout
                        ),
                    ),
                    None => crate::bootlog::device(
                        "bluetooth",
                        format_args!("LE Connection Complete malformed param-len={}", p.len()),
                    ),
                }
                if self.phase != 7
                    || p.len() != 19
                    || p[1] != 0
                    || p[4] != 0
                    || p[5] != self.peer_type
                    || p[6..12] != self.peer
                {
                    return Err("BT CONNECTION IDENTITY");
                }
                self.handle = u16::from_le_bytes([p[2], p[3]]);
                self.credits = Some(
                    bt::acl::Credits::new(
                        self.generation,
                        self.handle,
                        self.packets,
                        self.packet_bytes,
                    )
                    .map_err(|_| "BT ACL CREDITS")?,
                );
                self.reassembly = Some(
                    bt::acl::Reassembly::new(self.generation, self.handle)
                        .map_err(|_| "BT ACL HANDLE")?,
                );
                self.phase = 8;
                self.connection_number = self.connection_number.saturating_add(1);
                self.rx_observation = bt::rx_diagnostic::Once::new();
                // Origin for the final position sample in either fresh or stored mode.
                self.smp_started = now;
                let tail = bt::stage::address_tail(&self.peer);
                let mode = bt::connection_diagnostic::Mode::from_pairing(self.pairing);
                crate::bootlog::device(
                    "bluetooth",
                    format_args!(
                        "BT mode={} stored-key={} peer=..:{:02x}:{:02x} type={} slot={}",
                        mode.name(),
                        mode.stored_key(),
                        tail[0],
                        tail[1],
                        self.peer_type,
                        self.slot
                    ),
                );
                if self.pairing {
                    let security = bt::sc::Central::new(
                        self.generation,
                        self.handle,
                        now,
                        self.method,
                        self.own_type,
                        if self.own_type == 1 {
                            self.own_random
                        } else {
                            self.own
                        },
                        self.peer_type,
                        self.peer,
                        &mut crate::entropy::BootRandom,
                    )
                    .map_err(|_| "BT SC INIT")?;
                    let request = security.request();
                    self.security = Some(security);
                    self.smp_started = now;
                    self.rx_observation = bt::rx_diagnostic::Once::new();
                    self.completion_baseline = [
                        self.event.completions,
                        self.acl_in.completions,
                        self.acl_out.completions,
                    ];
                    self.acl_in.last_cc = 0;
                    self.acl_in.last_bytes = 0;
                    self.smp_hinted = false;
                    self.security_request = false;
                    self.phase = 8;
                    self.queue(6, &request, now)?;
                } else {
                    let mut pending = crate::bluetooth::reconnect(
                        self.generation,
                        self.handle,
                        now,
                        self.peer_type,
                        self.peer,
                    )
                    .map_err(|_| "BT STORED IDENTITY REJECTED")?;
                    let (kind, tail) = pending.identity_tail();
                    crate::bootlog::device(
                        "bluetooth",
                        format_args!(
                            "stored identity type={} ..:{:02x}:{:02x}; key selected (value hidden)",
                            kind, tail[0], tail[1]
                        ),
                    );
                    let key = pending
                        .command_key(self.generation, now)
                        .map_err(|_| "BT STORED KEY")?;
                    key.use_with(|key| self.encrypt = Some(*key));
                    self.reconnect = Some(pending);
                    self.reused = true;
                    crate::bluetooth::status("Bluetooth reconnecting with stored SC key.");
                }
                crate::debug(if self.pairing {
                    b"MUSHA: BT_SC_PAIRING_BEGIN\n"
                } else {
                    b"MUSHA: BT_STORED_RECONNECT_BEGIN\n"
                });
            }
            0x13 => {
                self.credits
                    .as_mut()
                    .ok_or("BT ACL CREDITS")?
                    .completed(self.generation, event)
                    .map_err(|_| "BT ACL COMPLETION")?;
                let done = bt::stage::completed_packets(event.parameters, self.handle);
                if done != 0 && self.completed_packets == 0 {
                    crate::bootlog::device(
                        "bluetooth",
                        format_args!(
                            "ACL controller buffer completion: Number of Completed Packets handle={:04x} count={}",
                            self.handle, done
                        ),
                    );
                }
                self.completed_packets = self.completed_packets.saturating_add(done);
            }
            8 => {
                if let Some(meta) = bt::connection_diagnostic::event_metadata(event) {
                    crate::bootlog::device(
                        "bluetooth",
                        format_args!(
                            "Encryption Change status={:02x} handle={:04x} enabled={:02x}",
                            meta.status, meta.handle, meta.detail
                        ),
                    );
                    if let Some(wait) = self.encryption_observation.finish(
                        now,
                        [
                            self.event.completions,
                            self.acl_in.completions,
                            self.acl_out.completions,
                        ],
                    ) {
                        self.log_encryption_wait(wait, "Encryption Change received");
                    }
                }
                if !self.encryption_started {
                    return Err("BT ENCRYPTION BEFORE COMMAND ACCEPT");
                }
                self.encryption_started = false;
                let action = if let Some(reconnect) = self.reconnect.as_mut() {
                    let bond = reconnect
                        .encryption(self.generation, now, event)
                        .map_err(|_| "BT STORED KEY REJECTED")?;
                    self.reconnect = None;
                    Action::Bond(bond)
                } else {
                    self.security
                        .as_mut()
                        .ok_or("BT SMP STATE")?
                        .encryption(self.generation, now, event)
                        .map_err(|_| "BT ENCRYPTION REJECTED")?
                };
                self.action(host, now, action)?;
            }
            0x3e if event.parameters.first() == Some(&3) => {
                // Metadata only; a failed update keeps the existing connection.
                match bt::signaling::update_complete(event.parameters) {
                    Some(u) => crate::bootlog::device(
                        "bluetooth",
                        format_args!(
                            "connection update status={:02x} handle={:04x} interval={}x1.25ms latency={} timeout={}x10ms",
                            u.status, u.handle, u.interval, u.latency, u.timeout
                        ),
                    ),
                    None => crate::bootlog::device(
                        "bluetooth",
                        format_args!(
                            "connection update malformed param-len={}",
                            event.parameters.len()
                        ),
                    ),
                }
            }
            // Unsupported LE meta events never grant authentication. Name the
            // first few per connection: a peer that never answers pairing may
            // be waiting on a procedure this host does not run.
            code => {
                self.ignored_events = self.ignored_events.saturating_add(1);
                if self.ignored_events <= 4 {
                    crate::bootlog::device(
                        "bluetooth",
                        format_args!(
                            "HCI event ignored code={:02x} sub={:02x} len={} phase={}",
                            code,
                            if code == 0x3e {
                                event.parameters.first().copied().unwrap_or(0)
                            } else {
                                0
                            },
                            event.parameters.len(),
                            self.phase
                        ),
                    );
                }
            }
        }
        Ok(())
    }
    fn queue(&mut self, channel: u16, data: &[u8], now: u64) -> Result<(), &'static str> {
        if data.is_empty() {
            return Ok(());
        }
        if data.len() > 1024 {
            return Err("BT TX QUEUE FULL");
        }
        self.log_l2cap("TX", channel, data);
        if self.tx_len != 0 || self.deferred[0].is_some() {
            // Signaling can interleave with SMP/ATT; keep their order.
            let slot = self
                .deferred
                .iter_mut()
                .find(|d| d.is_none())
                .filter(|_| data.len() <= 72)
                .ok_or("BT TX QUEUE FULL")?;
            let mut bytes = [0; 72];
            bytes[..data.len()].copy_from_slice(data);
            *slot = Some((channel, bytes, data.len()));
            return Ok(());
        }
        self.load(channel, data, now)
    }
    fn load(&mut self, channel: u16, data: &[u8], now: u64) -> Result<(), &'static str> {
        self.tx[..2].copy_from_slice(&(data.len() as u16).to_le_bytes());
        self.tx[2..4].copy_from_slice(&channel.to_le_bytes());
        self.tx[4..4 + data.len()].copy_from_slice(data);
        self.tx_len = 4 + data.len();
        self.tx_at = 0;
        self.tx_deadline = now.checked_add(1000).ok_or("BT TIME")?;
        Ok(())
    }
    /// Channel and SMP code only. SMP values (keys, confirm, random, DHKey
    /// check) are never formatted; Pairing Failed carries a public reason code.
    fn log_l2cap(&mut self, direction: &str, channel: u16, sdu: &[u8]) {
        let bit = match channel {
            4 => 1,
            6 => 2,
            5 => 8,
            _ => 4,
        };
        if self.channels_logged & bit == 0 {
            self.channels_logged |= bit;
            crate::bootlog::device(
                "bluetooth",
                format_args!(
                    "L2CAP first {} cid={:04x} ({}) length={}",
                    direction,
                    channel,
                    match channel {
                        4 => "ATT",
                        5 => "LE signaling",
                        6 => "SMP",
                        _ => "unexpected",
                    },
                    sdu.len()
                ),
            );
        }
        if channel != 6 {
            return;
        }
        let code = sdu.first().copied().unwrap_or(0);
        if code == 0x05 && sdu.len() == 2 {
            crate::bootlog::device(
                "bluetooth",
                format_args!(
                    "SMP {} Pairing Failed reason={:02x} length=2",
                    direction, sdu[1]
                ),
            );
        } else {
            crate::bootlog::device(
                "bluetooth",
                format_args!(
                    "SMP {} {} code={:02x} length={}",
                    direction,
                    bt::stage::smp_name(code),
                    code,
                    sdu.len()
                ),
            );
        }
    }
    /// LE signaling (CID 5): answer Connection Parameter Update, reject other
    /// requests, never answer responses. The connection is kept either way.
    fn l2cap_signal(&mut self, now: u64, sdu: &[u8]) -> Result<(), &'static str> {
        let signal = self
            .signaling
            .receive(sdu)
            .map_err(|_| "BT L2CAP SIGNALING")?;
        match signal {
            bt::signaling::Signal::Update {
                identifier,
                parameters: p,
                response,
            }
            | bt::signaling::Signal::Rejected {
                identifier,
                parameters: p,
                response,
            } => {
                let accepted = matches!(signal, bt::signaling::Signal::Update { .. });
                crate::bootlog::device(
                    "bluetooth",
                    format_args!(
                        "LE signaling Connection Parameter Update Request id={:02x} interval={}-{}x1.25ms latency={} timeout={}x10ms {}",
                        identifier,
                        p.min,
                        p.max,
                        p.latency,
                        p.timeout,
                        if accepted {
                            "accepted"
                        } else {
                            "rejected (out of range)"
                        }
                    ),
                );
                self.queue(bt::signaling::CHANNEL, &response, now)?;
                if accepted {
                    self.update = Some(p);
                }
            }
            bt::signaling::Signal::Reject {
                identifier,
                code,
                response,
            } => {
                crate::bootlog::device(
                    "bluetooth",
                    format_args!(
                        "LE signaling code={:02x} id={:02x} unsupported; Command Reject sent",
                        code, identifier
                    ),
                );
                self.queue(bt::signaling::CHANNEL, &response, now)?;
            }
            bt::signaling::Signal::Ignore {
                identifier,
                code,
                duplicate,
            } => crate::bootlog::device(
                "bluetooth",
                format_args!(
                    "LE signaling code={:02x} id={:02x} ignored{}",
                    code,
                    identifier,
                    if duplicate {
                        " (repeated identifier)"
                    } else {
                        ""
                    }
                ),
            ),
        }
        Ok(())
    }
    fn action(
        &mut self,
        _host: &mut Host<'_>,
        now: u64,
        action: Action,
    ) -> Result<(), &'static str> {
        match action {
            Action::Send(p, n) => self.queue(6, &p[..n], now)?,
            Action::Display(value) => {
                crate::bluetooth::display(self.generation, value);
                self.waiting_display = true;
            }
            Action::Enter => {
                crate::bluetooth::enter(self.generation);
                self.waiting_entry = true;
            }
            Action::Encrypt(key) => self.encrypt = Some(key),
            Action::Bond(bond) => {
                let b = bond.bond();
                let tail = bt::stage::address_tail(&b.address);
                crate::bootlog::device(
                    "bluetooth",
                    format_args!(
                        "bond identity addr-type={} ({}) ..:{:02x}:{:02x} irk={}",
                        b.address_type,
                        super::address_class(b.address_type, &b.address),
                        tail[0],
                        tail[1],
                        if b.irk == [0; 16] { "no" } else { "yes" }
                    ),
                );
                self.bond = Some(bond);
                self.security = None;
                crate::bluetooth::status("Bluetooth encrypted; discovering HID service.");
                self.gatt = Some(bt::hogp::Discovery::new());
                self.gatt_action(now, self.gatt.as_ref().unwrap().request())?;
            }
        }
        Ok(())
    }
    pub(super) fn poll_acl(&mut self, host: &mut Host<'_>, now: u64) -> Result<(), &'static str> {
        if self.tx_len != 0 && now >= self.tx_deadline {
            return Err("BT ACL TX TIMEOUT");
        }
        if !self.saved && self.keyboard.is_some() {
            if let Some(ok) = crate::bluetooth::saved(self.generation) {
                if !ok {
                    return Err(crate::bluetooth::save_failure());
                }
                self.saved = true;
                let mut p = [0x12, 0, 0, 1, 0];
                p[1..3].copy_from_slice(&self.cccd.to_le_bytes());
                self.queue(4, &p, now)?;
            }
        }
        if self.waiting_display && crate::bluetooth::displayed(self.generation) {
            self.waiting_display = false;
            let action = self
                .security
                .as_mut()
                .ok_or("BT SC STATE")?
                .displayed(self.generation, now, &mut crate::entropy::BootRandom)
                .map_err(|_| "BT PASSKEY DISPLAY")?;
            self.action(host, now, action)?;
        }
        if self.waiting_entry {
            if let Some(value) = crate::bluetooth::take_entry(self.generation) {
                self.waiting_entry = false;
                let Some(value) = value else {
                    return Err("BT PASSKEY CANCELLED");
                };
                let action = value.use_with(|value| {
                    self.security
                        .as_mut()
                        .ok_or("BT SC STATE")?
                        .entered(self.generation, now, value, &mut crate::entropy::BootRandom)
                        .map_err(|_| "BT PASSKEY ENTRY")
                })?;
                self.action(host, now, action)?;
            }
        }
        if self.acl_out.pending {
            if self.acl_out.poll(host, self.slot)?.is_some() && self.tx_at == self.tx_len {
                for b in &mut self.tx {
                    unsafe {
                        core::ptr::write_volatile(b, 0);
                    }
                }
                self.tx_len = 0;
                self.tx_at = 0;
            }
        }
        if self.tx_len == 0 {
            if let Some((channel, mut bytes, n)) = self.deferred[0].take() {
                self.deferred[0] = self.deferred[1].take();
                let loaded = self.load(channel, &bytes[..n], now);
                for b in &mut bytes {
                    unsafe {
                        core::ptr::write_volatile(b, 0);
                    }
                }
                loaded?;
            }
        }
        if self.tx_len != 0 && !self.acl_out.pending {
            if now >= self.tx_deadline {
                return Err("BT ACL TX TIMEOUT");
            }
            let n = (self.tx_len - self.tx_at).min(self.packet_bytes as usize);
            match self
                .credits
                .as_mut()
                .ok_or("BT CREDIT STATE")?
                .reserve(self.generation, n)
            {
                Ok(()) => {}
                Err(bt::Error::Busy) => return Ok(()),
                Err(_) => return Err("BT CREDIT RESERVE"),
            }
            let mut p = [0; 1028];
            let header = bt::acl::le_tx_header(self.handle, self.tx_at != 0, n as u16);
            p[..4].copy_from_slice(&header);
            p[4..4 + n].copy_from_slice(&self.tx[self.tx_at..self.tx_at + n]);
            self.acl_out.arm(host, self.slot, Some(&p[..4 + n]))?;
            if !self.acl_logged[0] {
                self.acl_logged[0] = true;
                crate::bootlog::device(
                    "bluetooth",
                    format_args!(
                        "ACL first TX dci={} handle={:04x} PB={:02x} length={}",
                        self.acl_out.dci,
                        self.handle,
                        header[1] >> 4,
                        4 + n
                    ),
                );
            }
            self.tx_at += n;
            for b in &mut p {
                unsafe {
                    core::ptr::write_volatile(b, 0);
                }
            }
        }
        self.acl_in.arm(host, self.slot, None)?;
        if let Some(n) = self.acl_in.poll(host, self.slot)? {
            if !self.acl_logged[1] {
                self.acl_logged[1] = true;
                crate::bootlog::device(
                    "bluetooth",
                    format_args!("ACL first RX dci={} length={}", self.acl_in.dci, n),
                );
            }
            if n > 1028 {
                return Err("BT ACL RX SIZE");
            }
            let mut incoming = [0; 1028];
            for (i, b) in incoming[..n].iter_mut().enumerate() {
                *b = unsafe { (self.acl_in.buffer as *const u8).add(i).read_volatile() };
            }
            let mut input = &incoming[..n];
            while !input.is_empty() {
                let Some(packet) = self
                    .acl_stream
                    .next(self.generation, now, &mut input)
                    .map_err(|_| "BT ACL STREAM")?
                else {
                    break;
                };

                let mut sdu = [0; 1024];
                let mut channel = 0;
                let mut length = 0;
                self.reassembly
                    .as_mut()
                    .ok_or("BT RX STATE")?
                    .receive(self.generation, now, packet, |c, data| {
                        channel = c;
                        length = data.len();
                        sdu[..length].copy_from_slice(data)
                    })
                    .map_err(|_| "BT ACL REASSEMBLY")?;
                self.acl_stream.release();
                if length != 0 {
                    self.log_l2cap("RX", channel, &sdu[..length]);
                    if channel == 6
                        && sdu[0] == 0x0b
                        && self
                            .security
                            .as_ref()
                            .is_some_and(|s| s.awaiting_response())
                    {
                        // The peer believes it is bonded; this host has no bond for
                        // it. Not an error: show the guidance and give up at once.
                        self.security_request = true;
                        crate::bluetooth::status(bt::stage::PAIRING_NO_REPLY);
                    } else if channel == 6 {
                        let action = self
                            .security
                            .as_mut()
                            .ok_or("BT UNEXPECTED SMP")?
                            .receive(
                                self.generation,
                                now,
                                &sdu[..length],
                                &mut crate::entropy::BootRandom,
                            )
                            .map_err(|e| match e {
                                // Pairing Failed from the peer: name its reason code.
                                bt::Error::Controller(code) if sdu[0] == 0x05 => {
                                    bt::stage::pairing_failed(code)
                                }
                                _ => "BT SC AUTHENTICATION",
                            })?;
                        self.action(host, now, action)?;
                    } else if channel == 4 {
                        self.att(now, &sdu[..length])?;
                    } else if channel == bt::signaling::CHANNEL {
                        self.l2cap_signal(now, &sdu[..length])?;
                    }
                }
                for b in sdu.iter_mut() {
                    unsafe {
                        core::ptr::write_volatile(b, 0);
                    }
                }
            }
            for b in &mut incoming {
                unsafe {
                    core::ptr::write_volatile(b, 0);
                }
            }
        }
        Ok(())
    }
    fn gatt_action(
        &mut self,
        now: u64,
        action: bt::hogp::DiscoveryAction,
    ) -> Result<(), &'static str> {
        match action {
            bt::hogp::DiscoveryAction::Send(p, n) => {
                crate::bootlog::device(
                    "bluetooth",
                    format_args!(
                        "ATT queue intent request={:02x} handle={:04x} len={}",
                        p[0],
                        u16::from_le_bytes([p[1], p[2]]),
                        n
                    ),
                );
                self.queue(4, &p[..n], now)?;
            }
            bt::hogp::DiscoveryAction::Ignore => {}
            bt::hogp::DiscoveryAction::Confirm => self.queue(4, &[0x1e], now)?,
            // Discovery already bound the reports into self.keyboard in place.
            bt::hogp::DiscoveryAction::Ready => {
                let keyboard = self.keyboard.as_ref().ok_or("BT INPUT STATE")?;
                if self.reused {
                    self.method = if keyboard
                        .single_kind()
                        .map_err(|_| "BT DEVICE KIND AMBIGUOUS")?
                    {
                        bt::sc::Method::JustWorks
                    } else {
                        bt::sc::Method::DisplayPasskey
                    };
                } else if !keyboard.supports(self.method == bt::sc::Method::JustWorks) {
                    return Err("BT DEVICE KIND MISMATCH");
                }
                let (free, mapped) = crate::cpu::stack_headroom().unwrap_or((0, 0));
                crate::bootlog::device(
                    "bluetooth",
                    format_args!(
                        "HOGP bound input-reports={} unmapped={}; stack never-touched {} of {} bytes",
                        keyboard.count(),
                        keyboard.unmapped(),
                        free,
                        mapped
                    ),
                );
                self.cccd = keyboard.cccd(0).ok_or("BT CCCD MISSING")?;
                self.gatt = None;
                if self.reused {
                    self.bond.take();
                    crate::bluetooth::reused(self.generation);
                } else {
                    crate::bluetooth::save(
                        self.generation,
                        self.bond.take().ok_or("BT BOND STATE")?,
                    )?;
                }
                crate::bluetooth::status("Bluetooth HID validated; saving pairing record.");
            }
        }
        Ok(())
    }
    /// Phase name for skipped-PDU diagnostics.
    fn att_phase(&self) -> &'static str {
        if let Some(gatt) = self.gatt.as_ref() {
            gatt.stage_name()
        } else if self.bond.is_none() && self.keyboard.is_none() {
            "pairing"
        } else if !self.subscribed {
            "subscription"
        } else {
            "connected"
        }
    }
    /// First skipped PDU is logged with its opcode and length; later ones are
    /// only counted. Report contents are never formatted.
    fn att_skip(&mut self, what: &str, p: &[u8]) {
        self.att_skipped = self.att_skipped.saturating_add(1);
        if self.att_skipped == 1 {
            crate::bootlog::device(
                "bluetooth",
                format_args!(
                    "ATT {} opcode={:02x} len={} during stage={}",
                    what,
                    p.first().copied().unwrap_or(0),
                    p.len(),
                    self.att_phase()
                ),
            );
        }
    }
    fn att(&mut self, now: u64, p: &[u8]) -> Result<(), &'static str> {
        // A peer request needs an answer in every phase, or it times out and
        // disconnects. This central has no GATT database.
        if let Some((reply, n)) = bt::att::peer_request_reply(p) {
            crate::bootlog::device(
                "bluetooth",
                format_args!(
                    "ATT peer request opcode={:02x} answered opcode={:02x} during stage={}",
                    p[0],
                    reply[0],
                    self.att_phase()
                ),
            );
            return self.queue(4, &reply[..n], now);
        }
        // Indications are confirmed and never delivered as input.
        if p.first() == Some(&0x1d) {
            self.att_skip("indication confirmed", p);
            return self.queue(4, &[0x1e], now);
        }
        if self.subscribed {
            let observed = self
                .keyboard
                .as_mut()
                .ok_or("BT INPUT STATE")?
                .notification_observed(self.generation, self.handle, p, |change| match change {
                    bt::hogp::Change::Key(key, down)
                        if self.method != bt::sc::Method::JustWorks =>
                    {
                        crate::bluetooth::key(self.generation, key, down)
                    }
                    bt::hogp::Change::Gamepad(change)
                        if self.method == bt::sc::Method::JustWorks =>
                    {
                        crate::bluetooth::gamepad(change)
                    }
                    _ => {}
                })
                .map_err(|error| {
                    crate::bootlog::device(
                        "bluetooth",
                        format_args!(
                            "HID input rejected reason={:?} ATT-op={:02x} len={}",
                            error,
                            p.first().copied().unwrap_or(0),
                            p.len()
                        ),
                    );
                    "BT INPUT REJECTED"
                })?;
            if let Some(info) = observed {
                crate::bluetooth::input_accepted(info);
            }
            return Ok(());
        }
        // Notifications before discovery (SMP/encryption) or while CCCDs are
        // being written are dropped; discovery skips its own below.
        if p.first() == Some(&0x1b) && self.gatt.is_none() {
            self.att_skip("ignored", p);
            return Ok(());
        }
        if self.saved && p == [0x13] {
            self.cccd_index += 1;
            if let Some(next) = self.keyboard.as_ref().and_then(|r| r.cccd(self.cccd_index)) {
                self.cccd = next;
                let mut bytes = [0x12, 0, 0, 1, 0];
                bytes[1..3].copy_from_slice(&next.to_le_bytes());
                self.queue(4, &bytes, now)?;
            } else {
                self.subscribed = true;
                crate::bootlog::device(
                    "bluetooth",
                    format_args!(
                        "HID subscribed; ATT PDUs skipped before input={}",
                        self.att_skipped
                    ),
                );
                crate::bluetooth::status("Bluetooth HID connected; stored record validated.");
            }
            return Ok(());
        }
        // An Error Response while a CCCD write is pending: name the refusal.
        if self.saved && !self.subscribed && p.len() == 5 && p[0] == 0x01 {
            crate::bootlog::device(
                "bluetooth",
                format_args!(
                    "CCCD write rejected request={:02x} handle={:04x} code={:02x}",
                    p[1],
                    u16::from_le_bytes([p[2], p[3]]),
                    p[4]
                ),
            );
            return Err("BT CCCD WRITE REJECTED");
        }
        let bond = self.bond.as_ref().ok_or("BT UNAUTHENTICATED ATT")?;
        let gatt = self.gatt.as_mut().ok_or("BT GATT STATE")?;
        let diagnostic = gatt.diagnostic(p);
        let action = match gatt.receive_into(p, bond, &mut self.keyboard) {
            Ok(
                action @ (bt::hogp::DiscoveryAction::Ignore | bt::hogp::DiscoveryAction::Confirm),
            ) => {
                self.att_skip("ignored", p);
                return self.gatt_action(now, action);
            }
            Ok(action) => {
                crate::bootlog::device("bluetooth", format_args!("{}", diagnostic));
                crate::bootlog::device(
                    "bluetooth",
                    format_args!("HOGP stage={} accepted", diagnostic.stage),
                );
                action
            }
            Err(error) => {
                crate::bootlog::device("bluetooth", format_args!("{}", diagnostic));
                crate::bootlog::device(
                    "bluetooth",
                    format_args!(
                        "HOGP stage={} rejected error={:?} reason={}",
                        diagnostic.stage,
                        error,
                        diagnostic.reason(error)
                    ),
                );
                return Err(match diagnostic.stage {
                    "services" => "BT HOGP SERVICES REJECTED",
                    "characteristics" => "BT HOGP CHARACTERISTICS REJECTED",
                    "report-map" => "BT HOGP REPORT MAP REJECTED",
                    "descriptors" => "BT HOGP DESCRIPTORS REJECTED",
                    "reference" => "BT HOGP REFERENCE OR MAP BINDING REJECTED",
                    _ => "BT HOGP DISCOVERY REJECTED",
                });
            }
        };
        self.gatt_action(now, action)
    }
}
