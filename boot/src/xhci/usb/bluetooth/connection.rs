// Copyright 2026 Hirofumi Iwasaki
// SPDX-License-Identifier: Apache-2.0
//! BSP-only live HCI connection and SC path. All failures propagate to the
//! existing host halt/BME-off boundary; no DMA allocation is reclaimed here.
use super::*;
use musha_platform::bluetooth::{self as bt, sc::Action};
impl Device {
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
            security
                .check(self.generation, now)
                .map_err(|_| "BT SMP TIMEOUT")?;
        }
        if self.command.is_some() || self.command_credits == 0 {
            return Ok(());
        }
        match self.phase {
            1 => self.send_command(host, now, 0x0c01, &[0x90, 0xe0, 4, 0, 0, 0, 0, 0x20]),
            2 => self.send_command(host, now, 0x2001, &[0x13, 0, 0, 0, 0, 0, 0, 0]),
            3 => self.send_command(host, now, 0x1009, &[]),
            4 => self.send_command(
                host,
                now,
                if self.shared_capacity { 0x1005 } else { 0x2002 },
                &[],
            ),
            5 => self.send_command(host, now, 0x200c, &[0, 0]),
            6 => {
                let mut p = [0; 25];
                p[..4].copy_from_slice(&[0x10, 0, 0x10, 0]);
                p[5] = self.peer_type;
                p[6..12].copy_from_slice(&self.peer);
                p[13..17].copy_from_slice(&[0x18, 0, 0x28, 0]);
                p[19..21].copy_from_slice(&[0xf4, 1]);
                self.send_command(host, now, 0x200d, &p)
            }
            8 if self.encrypt.is_some() => {
                let mut key = self.encrypt.take().unwrap();
                let mut p = [0; 28];
                p[..2].copy_from_slice(&self.handle.to_le_bytes());
                p[12..].copy_from_slice(&key);
                let result = self.send_command(host, now, 0x2019, &p);
                for b in p.iter_mut().chain(key.iter_mut()) {
                    unsafe {
                        core::ptr::write_volatile(b, 0);
                    }
                }
                result
            }
            _ => Ok(()),
        }
    }
    pub(super) fn handle_event(
        &mut self,
        host: &mut Host<'_>,
        now: u64,
        event: Event<'_>,
    ) -> Result<(), &'static str> {
        match event.code {
            0x10 | 5 => {
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
                let (credits, op, result) = if event.code == 0x0e {
                    event.complete().map_err(|_| "BT COMMAND COMPLETE")?
                } else {
                    let p = event.parameters;
                    if p.len() != 4 {
                        return Err("BT COMMAND STATUS");
                    }
                    (p[1], u16::from_le_bytes([p[2], p[3]]), &p[..1])
                };
                self.command_credits = credits;
                if op == 0 && result.is_empty() {
                    return Ok(());
                }
                if self.command.map(|c| c.0) != Some(op) || result.first() != Some(&0) {
                    return Err("BT COMMAND REJECTED");
                }
                self.command = None;
                if matches!(op, 0x200d | 0x2019) {
                    if event.code != 0x0f {
                        return Err("BT ASYNC COMMAND");
                    }
                    if op == 0x200d {
                        self.phase = 7;
                    } else {
                        self.encryption_started = true;
                    }
                    return Ok(());
                }
                if event.code != 0x0e {
                    return Err("BT SYNC COMMAND");
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
                if self.pairing {
                    let security = bt::sc::Central::new(
                        self.generation,
                        self.handle,
                        now,
                        self.method,
                        0,
                        self.own,
                        self.peer_type,
                        self.peer,
                        &mut crate::entropy::BootRandom,
                    )
                    .map_err(|_| "BT SC INIT")?;
                    let request = security.request();
                    self.security = Some(security);
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
            0x13 => self
                .credits
                .as_mut()
                .ok_or("BT ACL CREDITS")?
                .completed(self.generation, event)
                .map_err(|_| "BT ACL COMPLETION")?,
            8 => {
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
            0x3e => {} // Unsupported LE meta events never grant authentication.
            _ => {}
        }
        Ok(())
    }
    fn queue(&mut self, channel: u16, data: &[u8], now: u64) -> Result<(), &'static str> {
        if data.is_empty() {
            return Ok(());
        }
        if self.tx_len != 0 || data.len() > 1024 {
            return Err("BT TX QUEUE FULL");
        }
        self.tx[..2].copy_from_slice(&(data.len() as u16).to_le_bytes());
        self.tx[2..4].copy_from_slice(&channel.to_le_bytes());
        self.tx[4..4 + data.len()].copy_from_slice(data);
        self.tx_len = 4 + data.len();
        self.tx_at = 0;
        self.tx_deadline = now.checked_add(1000).ok_or("BT TIME")?;
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
                    return Err("BT BOND SAVE UNKNOWN");
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
            let bits = self.handle | if self.tx_at == 0 { 0x2000 } else { 0x1000 };
            p[..2].copy_from_slice(&bits.to_le_bytes());
            p[2..4].copy_from_slice(&(n as u16).to_le_bytes());
            p[4..4 + n].copy_from_slice(&self.tx[self.tx_at..self.tx_at + n]);
            self.acl_out.arm(host, self.slot, Some(&p[..4 + n]))?;
            self.tx_at += n;
            for b in &mut p {
                unsafe {
                    core::ptr::write_volatile(b, 0);
                }
            }
        }
        self.acl_in.arm(host, self.slot, None)?;
        if let Some(n) = self.acl_in.poll(host, self.slot)? {
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
                    if channel == 6 {
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
                            .map_err(|_| "BT SC AUTHENTICATION")?;
                        self.action(host, now, action)?;
                    } else if channel == 4 {
                        self.att(now, &sdu[..length])?;
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
            bt::hogp::DiscoveryAction::Send(p, n) => self.queue(4, &p[..n], now)?,
            bt::hogp::DiscoveryAction::Ready(keyboard) => {
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
                self.cccd = keyboard.cccd(0).ok_or("BT CCCD MISSING")?;
                self.keyboard = Some(keyboard);
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
    fn att(&mut self, now: u64, p: &[u8]) -> Result<(), &'static str> {
        if self.subscribed {
            self.keyboard
                .as_mut()
                .ok_or("BT INPUT STATE")?
                .notification(self.generation, self.handle, p, |change| match change {
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
                .map_err(|_| "BT INPUT REJECTED")?;
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
                crate::bluetooth::status("Bluetooth HID connected; stored record validated.");
            }
            return Ok(());
        }
        let bond = self.bond.as_ref().ok_or("BT UNAUTHENTICATED ATT")?;
        let action = self
            .gatt
            .as_mut()
            .ok_or("BT GATT STATE")?
            .receive(p, bond)
            .map_err(|_| "BT HOGP DISCOVERY UNSUPPORTED")?;
        self.gatt_action(now, action)
    }
}
