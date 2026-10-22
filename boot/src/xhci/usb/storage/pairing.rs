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
    writes_completed: u32,
    logical_barriers_completed: u32,
    recovery_blocked: bool,
    recovery_attempted: bool,
}
impl Image<'_, '_> {
    fn failed_write(&mut self, reason: &'static str) -> musha_fs::Error {
        self.recovery_blocked = true;
        if !self.recovery_attempted && self.device.bot.pending.is_some() {
            self.recovery_attempted = true;
            match self.device.recover_retired_write(self.host) {
                Ok(()) => {
                    self.recovery_blocked = false;
                    crate::debug(b"MUSHA: PAIRING_BOT_RECOVERED\n");
                }
                Err(cause) => {
                    crate::bootlog::device(
                        "storage",
                        format_args!("BOT recovery refused: {}", cause),
                    );
                }
            }
        }
        self.io::<()>(Err(reason)).unwrap_err()
    }
    fn io<T>(&mut self, result: Result<T, &'static str>) -> Result<T, musha_fs::Error> {
        result.map_err(|reason| {
            let first = self.transport.is_none();
            self.transport.get_or_insert(reason);
            if first {
                if let Some(f) = self.device.write_failure {
                    crate::bootlog::device("bluetooth",format_args!("bond storage failure opcode={:02x} lba={:?} bytes={} tag={} CSW-status={} residue={}", f.opcode,f.lba,f.bytes,f.tag,f.csw_status,f.residue));
                    if let Some(sense) = f.sense {
                        crate::bootlog::device("bluetooth",format_args!("bond storage sense key={:02x} ASC={:02x} ASCQ={:02x}; completed writes={} logical-barriers={}; SCSI sync omitted; no retry",sense.key,sense.asc,sense.ascq,self.writes_completed,self.logical_barriers_completed));
                    } else {
                        crate::bootlog::device("bluetooth",format_args!("bond storage sense unavailable; completed writes={} logical-barriers={}; SCSI sync omitted; no retry",self.writes_completed,self.logical_barriers_completed));
                    }
                }
            }
            musha_fs::Error::Io
        })
    }
}
impl Disk for Image<'_, '_> {
    fn read(&mut self, lba: u64, out: &mut [u8]) -> Result<(), musha_fs::Error> {
        if self.recovery_blocked {
            return Err(musha_fs::Error::Io);
        }
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
        if self.recovery_blocked {
            return Err(musha_fs::Error::Io);
        }
        let begun = self.device.begin_write(self.host, lba, b);
        if let Err(reason) = begun {
            return Err(self.failed_write(reason));
        }
        loop {
            let done = match self.device.poll_write(self.host) {
                Ok(done) => done,
                Err(reason) => return Err(self.failed_write(reason)),
            };
            match done {
                Some(true) => {
                    self.writes_completed = self.writes_completed.saturating_add(1);
                    return Ok(());
                }
                Some(false) => {
                    self.transport.get_or_insert("BOT WRITE STATUS");
                    return Err(musha_fs::Error::Io);
                }
                None => core::hint::spin_loop(),
            }
        }
    }
    fn sync(&mut self) -> Result<(), musha_fs::Error> {
        if self.recovery_blocked {
            return Err(musha_fs::Error::Io);
        }
        let begun = self.device.begin_sync(self.host);
        self.io(begun)?;
        loop {
            let done = self.device.poll_write(self.host);
            match self.io(done)? {
                Some(true) => {
                    self.logical_barriers_completed =
                        self.logical_barriers_completed.saturating_add(1);
                    return Ok(());
                }
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
/// A refused or failed record access, for diagnostics. Never key material.
#[derive(Clone, Copy)]
enum Failure {
    Refused(settings::Refusal),
    Write(settings::WriteError),
    Range,
    Unowned,
    Io,
}
impl Failure {
    /// Bluetooth failure reason; distinct per kind so the viewer shows it.
    fn reason(self) -> &'static str {
        use settings::{Refusal as R, WriteError as W};
        match self {
            Self::Refused(r) | Self::Write(W::Refused(r)) => match r {
                R::Layout => "BT BOND SAVE LAYOUT",
                R::Budget => "BT BOND SAVE BUDGET",
                R::MissingDirectory => "BT BOND SAVE NO SETTINGS",
                R::FullDirectory => "BT BOND SAVE SETTINGS FULL",
                R::NoSpace => "BT BOND SAVE NO SPACE",
                R::Protected => "BT BOND SAVE PROTECTED",
                R::Capacity => "BT BOND SAVE CAPACITY",
                R::Name => "BT BOND SAVE NAME",
                R::Filesystem(_) => "BT BOND SAVE FILESYSTEM",
            },
            Self::Write(W::Unowned) | Self::Unowned => "BT BOND SAVE UNOWNED",
            Self::Write(W::Io(_)) | Self::Io => "BT BOND SAVE IO",
            Self::Write(W::RecoveryRequired) => "BT BOND SAVE RECOVERY REQUIRED",
            Self::Range => "BT BOND SAVE RANGE",
        }
    }
}
impl core::fmt::Debug for Failure {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Refused(r) => write!(f, "{:?}", r),
            Self::Write(w) => write!(f, "{:?}", w),
            Self::Range => f.write_str("Range"),
            Self::Unowned => f.write_str("Unowned"),
            Self::Io => f.write_str("Io"),
        }
    }
}
fn old(
    image: &mut Image<'_, '_>,
    out: &mut [u8; 4096],
    t: &mut settings::Trace,
) -> Result<bool, Failure> {
    let size = image.device.size as usize;
    let blocks = image.device.blocks;
    let plan = settings::preflight_traced(
        blocks,
        size,
        settings::OWNED_NAME,
        4096,
        false,
        &mut |l, b| image.read(l, b),
        t,
    )
    .map_err(Failure::Refused)?;
    if plan.created {
        return Ok(false);
    }
    t.step = "read-old";
    for (i, b) in out.chunks_mut(size).enumerate() {
        image
            .read(plan.data_sector(i).ok_or(Failure::Range)?, b)
            .map_err(|_| Failure::Io)?;
    }
    if !musha_platform::pairing::recognized(out) {
        t.detail = "existing PAIRING.DAT not an OS record";
        return Err(Failure::Unowned);
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
            writes_completed: 0,
            logical_barriers_completed: 0,
            recovery_blocked: false,
            recovery_attempted: false,
        };
        let mut record = [0; 4096];
        if !image.device.pairing_checked {
            let mut trace = settings::Trace::default();
            let loaded = old(&mut image, &mut record, &mut trace);
            if let Err(failure) = loaded {
                crate::bootlog::device(
                    "bluetooth",
                    format_args!(
                        "pairing record not loaded reason={:?} step={} detail={} value=0x{:x}",
                        failure, trace.step, trace.detail, trace.value
                    ),
                );
            }
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
        let started = crate::bootstate::epoch_now().unwrap_or(0);
        let mut old_trace = settings::Trace::default();
        let mut trace = settings::Trace::default();
        let mut failure = None;
        let loaded = old(&mut image, &mut record, &mut old_trace);
        let mut records = 0;
        let result = match loaded {
            Ok(exists) => bond.save(
                if exists { Some(&record) } else { None },
                crate::settings::pairing_config_hash(),
                |bytes| {
                    records = musha_platform::pairing::bond_count(bytes).unwrap_or(0);
                    let blocks = image.device.blocks;
                    let size = image.device.size as usize;
                    settings::write_owned_traced(
                        &mut image,
                        blocks,
                        size,
                        settings::OWNED_NAME,
                        bytes,
                        musha_platform::pairing::recognized,
                        unsafe { &mut *WORK.0.get() },
                        &mut trace,
                    )
                    .map(|_| ())
                    .map_err(|e| {
                        failure = Some(Failure::Write(e));
                        musha_platform::bluetooth::Error::Invalid
                    })
                },
            ),
            Err(e) => {
                failure = Some(e);
                trace = old_trace;
                trace.step = match trace.step {
                    "preflight" => "old-preflight",
                    step => step,
                };
                Err(musha_platform::bluetooth::bonding::SaveError::PersistenceUnknown)
            }
        };
        let elapsed = crate::bootstate::epoch_now()
            .unwrap_or(started)
            .saturating_sub(started);
        for warning in [old_trace.warning, trace.warning] {
            if !warning.is_empty() {
                crate::bootlog::device("bluetooth", format_args!("SETTINGS warning: {}", warning));
            }
        }
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
        match (&result, failure) {
            (Ok(_), _) => {
                let (free, mapped) = crate::cpu::stack_headroom().unwrap_or((0, 0));
                crate::bootlog::device(
                    "bluetooth",
                    format_args!(
                        "bond saved record={} file=SETTINGS/PAIRING.DAT reads old={} preflight={} audit={} elapsed={}ms; stack never-touched {} of {} bytes",
                        records,
                        old_trace.preflight_reads,
                        trace.preflight_reads,
                        trace.audit_reads,
                        elapsed,
                        free,
                        mapped
                    ),
                )
            }
            (Err(_), Some(f)) => {
                crate::bootlog::device(
                    "bluetooth",
                    format_args!(
                        "bond save failed reason={:?} step={} detail={} value=0x{:x} reads old={} preflight={} audit={} elapsed={}ms transport={}",
                        f,
                        trace.step,
                        trace.detail,
                        trace.value,
                        old_trace.preflight_reads,
                        trace.preflight_reads,
                        trace.audit_reads,
                        elapsed,
                        image.transport.unwrap_or("ok")
                    ),
                );
                crate::bluetooth::save_failed(f.reason());
            }
            // Refused by the bond merge itself (full store, weaker key, record).
            (Err(e), None) => {
                crate::bootlog::device(
                    "bluetooth",
                    format_args!("bond save failed reason={:?} step=merge", e),
                );
                crate::bluetooth::save_failed(match e {
                    musha_platform::bluetooth::bonding::SaveError::Full => "BT BOND SAVE FULL",
                    musha_platform::bluetooth::bonding::SaveError::Security => {
                        "BT BOND SAVE WEAKER KEY"
                    }
                    _ => "BT BOND SAVE RECORD",
                });
            }
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
