// Copyright 2026 Hirofumi Iwasaki
// SPDX-License-Identifier: Apache-2.0
//! Exclusive, idle BOT only. CPU workspace contains no escaping references and
//! remains image-owned through host quiescence. No CONFIG write capability.
use super::*;
use musha_fs::settings::{self, Disk};
struct Image<'a, 'h> {
    host: &'a mut Host<'h>,
    device: &'a mut Device,
    // First USB transport failure; a pure filesystem refusal leaves it None.
    transport: Option<&'static str>,
}
impl Image<'_, '_> {
    fn io<T>(&mut self, result: Result<T, &'static str>) -> Result<T, musha_fs::Error> {
        result.map_err(|reason| {
            self.transport.get_or_insert(reason);
            musha_fs::Error::Io
        })
    }
}
impl Disk for Image<'_, '_> {
    fn read(&mut self, lba: u64, out: &mut [u8]) -> Result<(), musha_fs::Error> {
        let connected = self.device.check_connected(self.host);
        self.io(connected)?;
        let begun = self.device.begin_read(self.host, lba);
        self.io(begun)?;
        loop {
            let done = self.device.poll_read(self.host, out);
            if self.io(done)? {
                return Ok(());
            }
            core::hint::spin_loop();
        }
    }
    fn write(&mut self, lba: u64, b: &[u8]) -> Result<(), musha_fs::Error> {
        let begun = self.device.begin_write(self.host, lba, b);
        self.io(begun)?;
        loop {
            let done = self.device.poll_write(self.host);
            match self.io(done)? {
                Some(true) => return Ok(()),
                Some(false) => {
                    self.transport.get_or_insert("BOT WRITE STATUS");
                    return Err(musha_fs::Error::Io);
                }
                None => core::hint::spin_loop(),
            }
        }
    }
    fn sync(&mut self) -> Result<(), musha_fs::Error> {
        let begun = self.device.begin_sync(self.host);
        self.io(begun)?;
        loop {
            let done = self.device.poll_write(self.host);
            match self.io(done)? {
                Some(true) => return Ok(()),
                Some(false) => {
                    self.transport.get_or_insert("BOT SYNC STATUS");
                    return Err(musha_fs::Error::Io);
                }
                None => core::hint::spin_loop(),
            }
        }
    }
    fn protected(&self) -> bool {
        self.device.write_protected
    }
}
struct Shared(core::cell::UnsafeCell<settings::Workspace>);
unsafe impl Sync for Shared {} // BSP; called only with exclusive idle Device/Host.
static WORK: Shared = Shared(core::cell::UnsafeCell::new(settings::Workspace::new()));
fn old(image: &mut Image<'_, '_>, out: &mut [u8; 4096]) -> Result<bool, &'static str> {
    let size = image.device.size as usize;
    let blocks = image.device.blocks;
    let plan = settings::preflight(
        blocks,
        size,
        settings::OWNED_NAME,
        4096,
        false,
        &mut |l, b| image.read(l, b),
    )
    .map_err(|_| "BT SETTINGS PREFLIGHT")?;
    if plan.created {
        return Ok(false);
    }
    for (i, b) in out.chunks_mut(size).enumerate() {
        image
            .read(plan.data_sector(i).ok_or("BT SETTINGS RANGE")?, b)
            .map_err(|_| "BT SETTINGS READ")?;
    }
    if !musha_platform::pairing::recognized(out) {
        return Err("BT SETTINGS UNOWNED");
    }
    Ok(true)
}
impl Device {
    pub(in crate::xhci::usb) fn pairing_step(
        &mut self,
        host: &mut Host<'_>,
    ) -> Result<(), &'static str> {
        if host.bluetooth.is_none() {
            return Ok(());
        }
        if self.bot.pending.is_some() {
            return Ok(());
        }
        let mut image = Image {
            host,
            device: self,
            transport: None,
        };
        let mut record = [0; 4096];
        if !image.device.pairing_checked {
            let loaded = old(&mut image, &mut record);
            // Unchanged policy: an unreadable old record only refuses reconnect.
            image.transport = None;
            match loaded {
                Ok(true) => {
                    if crate::bluetooth::load_record(Some(&record)).is_err() {
                        crate::bluetooth::record_ready(None);
                        crate::bluetooth::status(
                            "Bluetooth saved keys invalid; reconnect refused.",
                        );
                    }
                }
                Ok(false) => {
                    let _ = crate::bluetooth::load_record(None);
                }
                Err(_) => {
                    let _ = crate::bluetooth::load_record(None);
                }
            }
            image.device.pairing_checked = true;
            for b in &mut record {
                unsafe {
                    core::ptr::write_volatile(b, 0);
                }
            }
        }
        let Some((generation, bond)) = crate::bluetooth::pending_save() else {
            return Ok(());
        };
        let loaded = old(&mut image, &mut record);
        let mut records = 0;
        let result = match loaded {
            Ok(exists) => bond.save(
                if exists { Some(&record) } else { None },
                crate::settings::pairing_config_hash(),
                |bytes| {
                    records = musha_platform::pairing::bond_count(bytes).unwrap_or(0);
                    let blocks = image.device.blocks;
                    let size = image.device.size as usize;
                    settings::write_owned(
                        &mut image,
                        blocks,
                        size,
                        settings::OWNED_NAME,
                        bytes,
                        musha_platform::pairing::recognized,
                        unsafe { &mut *WORK.0.get() },
                    )
                    .map(|_| ())
                    .map_err(|_| musha_platform::bluetooth::Error::Invalid)
                },
            ),
            Err(_) => Err(musha_platform::bluetooth::bonding::SaveError::PersistenceUnknown),
        };
        for b in &mut record {
            unsafe {
                core::ptr::write_volatile(b, 0);
            }
        }
        // Workspace snapshots are CPU-only and no references escape the call.
        // Do not touch the BOT DMA buffer after an ambiguous transport failure.
        unsafe {
            let ptr = WORK.0.get().cast::<u8>();
            for i in 0..core::mem::size_of::<settings::Workspace>() {
                ptr.add(i).write_volatile(0);
            }
        }
        let ok = result.is_ok();
        match &result {
            Ok(_) => {
                let (free, mapped) = crate::cpu::stack_headroom().unwrap_or((0, 0));
                crate::bootlog::device(
                    "bluetooth",
                    format_args!(
                        "bond saved record={} file=SETTINGS/PAIRING.DAT; stack never-touched {} of {} bytes",
                        records, free, mapped
                    ),
                )
            }
            Err(e) => crate::bootlog::device(
                "bluetooth",
                format_args!(
                    "bond save failed reason={:?} transport={}",
                    e,
                    image.transport.unwrap_or("ok")
                ),
            ),
        }
        crate::bluetooth::save_result(generation, result);
        if !ok {
            crate::bluetooth::status(
                "Bluetooth pairing not published: record persistence unknown.",
            );
            // A USB transport failure leaves this storage device's state unknown and
            // still ends the session. A filesystem refusal keeps storage usable; the
            // failed result stops only Bluetooth (BT BOND SAVE UNKNOWN, isolated).
            if let Some(reason) = image.transport {
                crate::bootlog::device(
                    "bluetooth",
                    format_args!("pairing record transport failure: {}", reason),
                );
                return Err("BT RECORD PERSISTENCE UNKNOWN");
            }
            crate::bootlog::device(
                "bluetooth",
                format_args!("pairing record not written: filesystem refused; storage continues"),
            );
            return Ok(());
        }
        crate::debug(b"MUSHA: BT_PAIRING_RECORD_DURABLE\n");
        Ok(())
    }
}
