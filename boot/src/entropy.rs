// Copyright 2026 Hirofumi Iwasaki
// SPDX-License-Identifier: Apache-2.0
//! BSP-owned cryptographic DRBG. Firmware access ends before the final map/EBS.
use core::{cell::UnsafeCell, fmt::Write};
use musha_platform::bluetooth::{
    Error,
    drbg::{Drbg, Sources, healthy},
    entropy::Random,
    sources,
};
use r_efi::{efi, protocols::rng};
struct Shared(UnsafeCell<Drbg>);
unsafe impl Sync for Shared {} // BSP only; no borrow crosses firmware/USB callbacks.
static RNG: Shared = Shared(UnsafeCell::new(Drbg::new()));
static STRONG: core::sync::atomic::AtomicU8 = core::sync::atomic::AtomicU8::new(0);
static FAILURE: core::sync::atomic::AtomicU8 = core::sync::atomic::AtomicU8::new(1);
fn erase(bytes: &mut [u8]) {
    for b in bytes {
        unsafe {
            core::ptr::write_volatile(b, 0);
        }
    }
}
const REPORT_LIMIT: usize = 80;
struct Reports {
    rows: [[u8; 192]; REPORT_LIMIT],
    lengths: [u8; REPORT_LIMIT],
    count: usize,
    overflow: bool,
}
struct SharedReports(UnsafeCell<Reports>);
unsafe impl Sync for SharedReports {} // BSP only, no firmware callback retains a borrow.
static REPORTS: SharedReports = SharedReports(UnsafeCell::new(Reports {
    rows: [[0; 192]; REPORT_LIMIT],
    lengths: [0; REPORT_LIMIT],
    count: 0,
    overflow: false,
}));
static PUBLISHED: core::sync::atomic::AtomicBool = core::sync::atomic::AtomicBool::new(false);
struct Writer<'a> {
    row: &'a mut [u8; 192],
    used: usize,
}
impl core::fmt::Write for Writer<'_> {
    fn write_str(&mut self, text: &str) -> core::fmt::Result {
        let end = self
            .used
            .checked_add(text.len())
            .filter(|n| *n <= self.row.len())
            .ok_or(core::fmt::Error)?;
        self.row[self.used..end].copy_from_slice(text.as_bytes());
        self.used = end;
        Ok(())
    }
}
fn report(args: core::fmt::Arguments<'_>) {
    if PUBLISHED.load(core::sync::atomic::Ordering::Relaxed) {
        crate::bootlog::device("entropy", args);
        return;
    }
    // Preserve the mandatory first starting line: firmware diagnostics are collected
    // before GOP exists, then published to the ordinary ring immediately after bind.
    unsafe {
        let reports = &mut *REPORTS.0.get();
        if reports.count == REPORT_LIMIT {
            reports.overflow = true;
            return;
        }
        let index = reports.count;
        let mut writer = Writer {
            row: &mut reports.rows[index],
            used: 0,
        };
        if writer.write_fmt(args).is_err() {
            reports.overflow = true;
            return;
        }
        reports.lengths[index] = writer.used as u8;
        reports.count += 1;
    }
}
pub fn publish() {
    PUBLISHED.store(true, core::sync::atomic::Ordering::Relaxed);
    let count = unsafe { (*REPORTS.0.get()).count };
    for i in 0..count {
        let (row, len) = unsafe {
            let reports = &*REPORTS.0.get();
            (reports.rows[i], reports.lengths[i] as usize)
        };
        if let Ok(text) = core::str::from_utf8(&row[..len]) {
            crate::bootlog::device("entropy", format_args!("{}", text));
        }
    }
    if unsafe { (*REPORTS.0.get()).overflow } {
        crate::bootlog::device(
            "entropy",
            format_args!("diagnostic buffer limit={} exceeded", REPORT_LIMIT),
        );
    }
}
fn label(a: &efi::Guid) -> &'static str {
    if *a == rng::ALGORITHM_RAW {
        "raw"
    } else if *a == rng::ALGORITHM_SP800_90_HASH_256_GUID {
        "hash256"
    } else if *a == rng::ALGORITHM_SP800_90_HMAC_256_GUID {
        "hmac256"
    } else if *a == rng::ALGORITHM_SP800_90_CTR_256_GUID {
        "ctr256"
    } else {
        "advertised"
    }
}
unsafe fn firmware(bs: *mut efi::BootServices, out: &mut [u8; 64]) -> bool {
    let mut ptr = core::ptr::null_mut();
    let mut guid = rng::PROTOCOL_GUID;
    let status = unsafe { ((*bs).locate_protocol)(&mut guid, core::ptr::null_mut(), &mut ptr) };
    report(format_args!(
        "UEFI locate status={:?} present={}",
        status,
        !ptr.is_null()
    ));
    if status.is_error() || ptr.is_null() {
        return false;
    }
    let protocol = ptr as *mut rng::Protocol;
    const MAX: usize = 32;
    let mut algorithms = [rng::ALGORITHM_RAW; MAX];
    let mut size = core::mem::size_of_val(&algorithms);
    let info = unsafe { ((*protocol).get_info)(protocol, &mut size, algorithms.as_mut_ptr()) };
    let count = if !info.is_error() && size <= core::mem::size_of_val(&algorithms) && size % 16 == 0
    {
        size / 16
    } else {
        0
    };
    report(format_args!(
        "UEFI GetInfo status={:?} bytes={} count={} limit={}",
        info, size, count, MAX
    ));
    if size > MAX * 16 {
        report(format_args!(
            "UEFI algorithm list overflow; advertised list not read"
        ));
    }
    for (i, a) in algorithms[..count].iter().enumerate() {
        let b = a.as_bytes();
        report(format_args!(
            "UEFI algorithm {} {} guid-first8={:016x}",
            i,
            label(a),
            u64::from_le_bytes(b[..8].try_into().unwrap())
        ));
    }
    let mut selected = false;
    // Publicly advertised GUIDs, followed by explicit NULL/RAW/SP800-90 attempts.
    // The fixed array and explicit overflow report prevent unbounded firmware data.
    for index in 0..count + 5 {
        let mut explicit = match index.checked_sub(count) {
            Some(1) => rng::ALGORITHM_RAW,
            Some(2) => rng::ALGORITHM_SP800_90_HASH_256_GUID,
            Some(3) => rng::ALGORITHM_SP800_90_HMAC_256_GUID,
            _ => rng::ALGORITHM_SP800_90_CTR_256_GUID,
        };
        let (name, algorithm) = if index < count {
            (label(&algorithms[index]), &mut algorithms[index] as *mut _)
        } else if index == count {
            ("default", core::ptr::null_mut())
        } else {
            (label(&explicit), &mut explicit as *mut _)
        };
        let mut bytes = [0; 64];
        let status =
            unsafe { ((*protocol).get_rng)(protocol, algorithm, bytes.len(), bytes.as_mut_ptr()) };
        let good = !status.is_error() && healthy(&bytes);
        report(format_args!(
            "UEFI GetRng {} index={} status={:?} bytes={} health={}",
            name,
            index,
            status,
            if status.is_error() { 0 } else { 64 },
            good
        ));
        if good && !selected {
            out.copy_from_slice(&bytes);
            selected = true;
        }
        erase(&mut bytes);
    }
    selected
}
fn cpu(seed: bool) -> Option<u64> {
    let value: u64;
    let ok: u8;
    unsafe {
        if seed {
            core::arch::asm!("rdseed {value}","setc {ok}",value=out(reg)value,ok=out(reg_byte)ok,options(nomem,nostack));
        } else {
            core::arch::asm!("rdrand {value}","setc {ok}",value=out(reg)value,ok=out(reg_byte)ok,options(nomem,nostack));
        }
    }
    if ok != 0 { Some(value) } else { None }
}
pub unsafe fn initialize(bs: *mut efi::BootServices) {
    let mut fw = [0; 64];
    let fw_good = unsafe { firmware(bs, &mut fw) };
    let max = core::arch::x86_64::__cpuid(0).eax;
    let rdseed = max >= 7 && core::arch::x86_64::__cpuid_count(7, 0).ebx & (1 << 18) != 0;
    let rdrand = max >= 1 && core::arch::x86_64::__cpuid(1).ecx & (1 << 30) != 0;
    report(format_args!(
        "CPU RDSEED={} RDRAND={} retry-limits=64/10",
        rdseed, rdrand
    ));
    let seed_result = sources::collect(rdseed, 64, || cpu(true));
    let rand_result = sources::collect(rdrand, 10, || cpu(false));
    report(format_args!(
        "RDSEED acquisition={} RDRAND acquisition={}",
        match &seed_result {
            Ok(_) => "64 bytes",
            Err(sources::Failure::Unsupported) => "unsupported",
            Err(_) => "retry-exhausted",
        },
        match &rand_result {
            Ok(_) => "64 bytes",
            Err(sources::Failure::Unsupported) => "unsupported",
            Err(_) => "retry-exhausted",
        }
    ));
    let mut seed = seed_result.ok();
    let mut rand = rand_result.ok();
    if seed.as_ref().is_some_and(|s| !healthy(s)) {
        report(format_args!("RDSEED health rejected"));
        if let Some(s) = seed.as_mut() {
            erase(s);
        }
        seed = None;
    }
    if rand.as_ref().is_some_and(|s| !healthy(s)) {
        report(format_args!("RDRAND health rejected"));
        if let Some(s) = rand.as_mut() {
            erase(s);
        }
        rand = None;
    }
    let strong = fw_good as u8 + seed.is_some() as u8;
    report(format_args!(
        "sources firmware={} RDSEED={} RDRAND={} strong={}",
        fw_good,
        seed.is_some(),
        rand.is_some(),
        strong
    ));
    let result = unsafe {
        (&mut *RNG.0.get()).instantiate(Sources {
            firmware: fw_good.then_some(fw.as_slice()),
            rdseed: seed.as_ref().map(|a| a.as_slice()),
            rdrand: rand.as_ref().map(|a| a.as_slice()),
            hci: None,
        })
    };
    FAILURE.store(
        if result.is_ok() {
            0
        } else if strong == 0 {
            1
        } else {
            2
        },
        core::sync::atomic::Ordering::Relaxed,
    );
    STRONG.store(
        if result.is_ok() { strong } else { 0 },
        core::sync::atomic::Ordering::Relaxed,
    );
    report(format_args!(
        "DRBG instantiated={} strong={}",
        result.is_ok(),
        STRONG.load(core::sync::atomic::Ordering::Relaxed)
    ));
    erase(&mut fw);
    if let Some(s) = seed.as_mut() {
        erase(s);
    }
    if let Some(s) = rand.as_mut() {
        erase(s);
    }
}
pub fn fill(out: &mut [u8]) -> Result<(), Error> {
    let result = unsafe { (&mut *RNG.0.get()).generate(out) };
    if result.is_err() {
        FAILURE.store(3, core::sync::atomic::Ordering::Relaxed);
        STRONG.store(0, core::sync::atomic::Ordering::Relaxed);
    }
    result.map_err(|_| Error::Entropy)
}
pub fn available() -> bool {
    STRONG.load(core::sync::atomic::Ordering::Relaxed) > 0 && unsafe { (&*RNG.0.get()).is_ready() }
}
pub fn reason() -> &'static str {
    match FAILURE.load(core::sync::atomic::Ordering::Relaxed) {
        3 => "Bluetooth pairing unavailable: DRBG request limit/error.",
        2 => "Bluetooth pairing unavailable: DRBG initialization rejected.",
        _ => "Bluetooth pairing unavailable: no healthy firmware/RDSEED source.",
    }
}
pub fn hci(bytes: &mut [u8]) -> bool {
    let good = healthy(bytes);
    let result = if good && available() {
        unsafe { (&mut *RNG.0.get()).reseed_auxiliary(bytes) }
    } else {
        Err(musha_platform::bluetooth::drbg::Error::Health)
    };
    report(format_args!(
        "HCI auxiliary bytes={} health={} mixed={} strong={}",
        bytes.len(),
        good,
        result.is_ok(),
        STRONG.load(core::sync::atomic::Ordering::Relaxed)
    ));
    erase(bytes);
    result.is_ok()
}
pub struct BootRandom;
impl Random for BootRandom {
    fn fill(&mut self, out: &mut [u8]) -> Result<(), Error> {
        fill(out)
    }
}
