// SPDX-License-Identifier: Apache-2.0
use musha_memory::{MemoryMap, Range};
use musha_platform::Timer;
use r_efi::efi;

// BSP-only discovery before runtime; scalar evidence survives in image data.
#[derive(Clone, Copy)]
struct Evidence {
    stage: &'static str,
    reason: &'static str,
    address: usize,
    size: usize,
    kind: u32,
    attr: u64,
    length: usize,
    revision: u8,
    flags: u32,
    legacy: u32,
    legacy_len: u8,
    gas: [u8; 4],
    extended: u64,
}
static mut EVIDENCE: Evidence = Evidence {
    stage: "INITIAL MEMORY MAP",
    reason: "NOT READ",
    address: 0,
    size: 0,
    kind: u32::MAX,
    attr: 0,
    length: 0,
    revision: 0,
    flags: 0,
    legacy: 0,
    legacy_len: 0,
    gas: [0; 4],
    extended: 0,
};
fn checkpoint(stage: &'static str) {
    unsafe {
        EVIDENCE.stage = stage;
    }
}
fn failure(reason: &'static str) {
    unsafe {
        EVIDENCE.reason = reason;
    }
}
fn show_evidence() {
    let e = unsafe { EVIDENCE };
    crate::diagnostics::set(10, format_args!("ACPI STOP {}", e.stage));
    crate::diagnostics::set(
        11,
        format_args!("TABLE {:016X} BYTES {:X}", e.address, e.size),
    );
    crate::diagnostics::set(12, format_args!("MAP TYPE {} ATTR {:016X}", e.kind, e.attr));
    crate::diagnostics::set(
        13,
        format_args!("FADT BYTES {} REV {}", e.length, e.revision),
    );
    crate::diagnostics::set(14, format_args!("FADT FLAGS {:08X}", e.flags));
    crate::diagnostics::set(
        15,
        format_args!("PM LEGACY {:08X} LEN {}", e.legacy, e.legacy_len),
    );
    crate::diagnostics::set(
        16,
        format_args!(
            "GAS SPACE {} WIDTH {} OFFSET {} ACCESS {}",
            e.gas[0], e.gas[1], e.gas[2], e.gas[3]
        ),
    );
    crate::diagnostics::set(17, format_args!("GAS ADDRESS {:016X}", e.extended));
    crate::diagnostics::set(18, format_args!("ACPI RESULT {}", e.reason));
    crate::diagnostics::set(19, format_args!("FIELDS ZERO UNTIL FADT VALIDATED"));
}
// Firmware remains active, and the supplied map describes readable table RAM.
// Never expose these borrows after ExitBootServices: only Timer is copied out.
unsafe fn bytes<'a>(map: &MemoryMap<'_>, address: usize, size: usize) -> Option<&'a [u8]> {
    unsafe {
        EVIDENCE.address = address;
        EVIDENCE.size = size;
        EVIDENCE.kind = u32::MAX;
        EVIDENCE.attr = 0;
    }
    failure("READ RANGE INVALID");
    let range = Range::new(address, size).ok()?;
    if address == 0 {
        return None;
    }
    failure("READ MAP COVERAGE / TYPE");
    for i in 0..map.count() {
        let r = map.region(i).ok()?;
        if r.range.overlaps(range) {
            unsafe {
                EVIDENCE.kind = r.kind;
                EVIDENCE.attr = r.attributes;
            }
        }
        // Attribute RP/RO/XP are capabilities, not current access settings.
        // Consume the firmware table contract before ExitBootServices only.
        if musha_platform::acpi_read::readable_region(
            r.kind,
            r.range.start,
            r.range.end,
            address,
            size,
        ) {
            failure("READ OK");
            return Some(unsafe { core::slice::from_raw_parts(address as *const u8, size) });
        }
    }
    None
}
unsafe fn table<'a>(map: &MemoryMap<'_>, address: usize) -> Option<&'a [u8]> {
    let head = unsafe { bytes(map, address, 36) }?;
    let size = u32::from_le_bytes(head[4..8].try_into().ok()?) as usize;
    if !(36..=65536).contains(&size) {
        failure("TABLE LENGTH");
        return None;
    }
    let all = unsafe { bytes(map, address, size) }?;
    if !musha_platform::checksum(all) {
        failure("TABLE CHECKSUM");
        return None;
    }
    failure("TABLE VALIDATED");
    Some(all)
}
pub(crate) unsafe fn discover(
    system: *const efi::SystemTable,
    map: &MemoryMap<'_>,
) -> Option<Timer> {
    checkpoint("CONFIGURATION TABLE");
    let system = unsafe { &*system };
    if system.number_of_table_entries > 1024 || system.configuration_table.is_null() {
        return None;
    }
    let entries = unsafe {
        core::slice::from_raw_parts(system.configuration_table, system.number_of_table_entries)
    };
    let entry = entries
        .iter()
        .find(|e| e.vendor_guid == efi::ACPI_20_TABLE_GUID)
        .or_else(|| {
            entries
                .iter()
                .find(|e| e.vendor_guid == efi::ACPI_10_TABLE_GUID)
        })?;
    let address = entry.vendor_table as usize;
    checkpoint("RSDP READ");
    let rsdp = unsafe { bytes(map, address, 20) }?;
    checkpoint("RSDP SIGNATURE / CHECKSUM");
    if let Some(reason) = musha_platform::acpi_read::rsdp_error(rsdp) {
        failure(reason);
        return None;
    }
    failure("RSDP VALIDATED");
    let (root, width) = if rsdp[15] >= 2 {
        checkpoint("RSDP EXTENDED");
        let extended = unsafe { bytes(map, address, 36) }?;
        let size = u32::from_le_bytes(extended[20..24].try_into().ok()?) as usize;
        if !(36..=4096).contains(&size) {
            failure("RSDP EXTENDED LENGTH");
            return None;
        }
        if !musha_platform::checksum(unsafe { bytes(map, address, size) }?) {
            failure("RSDP EXTENDED CHECKSUM");
            return None;
        }
        let xsdt = u64::from_le_bytes(extended[24..32].try_into().ok()?);
        if xsdt != 0 {
            (usize::try_from(xsdt).ok()?, 8)
        } else {
            (
                u32::from_le_bytes(rsdp[16..20].try_into().ok()?) as usize,
                4,
            )
        }
    } else {
        (
            u32::from_le_bytes(rsdp[16..20].try_into().ok()?) as usize,
            4,
        )
    };
    checkpoint("ROOT READ / CHECKSUM");
    let root = unsafe { table(map, root) }?;
    checkpoint("ROOT LAYOUT");
    let signature = if width == 8 { b"XSDT" } else { b"RSDT" };
    if &root[..4] != signature || (root.len() - 36) % width != 0 || (root.len() - 36) / width > 512
    {
        failure("ROOT SIGNATURE / LAYOUT");
        return None;
    }
    let mut result = None;
    for item in root[36..].chunks_exact(width) {
        let address = if width == 8 {
            usize::try_from(u64::from_le_bytes(item.try_into().ok()?)).ok()?
        } else {
            u32::from_le_bytes(item.try_into().ok()?) as usize
        };
        checkpoint("CHILD HEADER READ");
        let header = unsafe { bytes(map, address, 36) }?;
        if &header[..4] == b"FACP" {
            checkpoint("DUPLICATE FADT");
            if result.is_some() {
                failure("DUPLICATE FADT");
                return None;
            }
            checkpoint("FADT READ / CHECKSUM");
            let fadt = unsafe { table(map, address) }?;
            unsafe {
                EVIDENCE.length = fadt.len();
                EVIDENCE.revision = fadt[8];
                if fadt.len() >= 116 {
                    EVIDENCE.flags = u32::from_le_bytes(fadt[112..116].try_into().ok()?);
                    EVIDENCE.legacy = u32::from_le_bytes(fadt[76..80].try_into().ok()?);
                    EVIDENCE.legacy_len = fadt[91];
                }
                if fadt.len() >= 220 {
                    EVIDENCE.gas = fadt[208..212].try_into().ok()?;
                    EVIDENCE.extended = u64::from_le_bytes(fadt[212..220].try_into().ok()?);
                }
            }
            checkpoint("FADT TIMER POLICY");
            failure("FADT TIMER POLICY");
            result = Some(musha_platform::fadt(fadt)?);
        }
    }
    failure(if result.is_some() {
        "TIMER FOUND"
    } else {
        "FADT ABSENT"
    });
    checkpoint(if result.is_some() {
        "TIMER FOUND"
    } else {
        "FADT ABSENT"
    });
    result
}
unsafe fn read(port: u16) -> u32 {
    let raw: u32;
    unsafe {
        core::arch::asm!("in eax, dx",in("dx") port,out("eax") raw,options(nomem,nostack));
    }
    raw
}
pub(crate) fn diagnose(timer: Timer, fb: musha_framebuffer::Framebuffer) {
    if timer.port == 0 {
        show_evidence();
        super::debug(b"MUSHA: ACPI_TIMER_UNAVAILABLE\n");
        unsafe {
            fb.text("TIMER UNAVAILABLE", 24, 260, fb.color(255, 180, 0));
        }
        return;
    }
    let mut clock = musha_platform::Clock::new(timer, unsafe { read(timer.port) }).unwrap();
    // Bounded probe: a stopped clock cannot hang boot forever. This iteration
    // limit is diagnostic, not a time-calibrated timeout for future drivers.
    for _ in 0..2_000_000 {
        if let Some(ms) = clock.sample(unsafe { read(timer.port) }) {
            if ms >= 100 {
                super::debug(b"MUSHA: ACPI_TIMER_OK MS=");
                super::debug(&super::cpu::hex(ms));
                super::debug(b"\n");
                unsafe {
                    fb.text("TIMER READY", 24, 260, fb.color(0, 240, 100));
                }
                return;
            }
        } else {
            break;
        }
    }
    super::debug(b"MUSHA: ACPI_TIMER_STALLED\n");
    unsafe {
        fb.text("TIMER STALLED", 24, 260, fb.color(255, 0, 0));
    }
}

// The runtime owns this clock. All driver waits and app steps poll it on BSP.
pub(crate) struct Time {
    timer: Timer,
    clock: musha_platform::Clock,
}
impl Time {
    pub(crate) fn new(timer: Timer) -> Result<Self, &'static str> {
        if timer.port == 0 {
            return Err("NO TIMER");
        }
        let clock =
            musha_platform::Clock::new(timer, unsafe { read(timer.port) }).ok_or("TIMER WIDTH")?;
        Ok(Self { timer, clock })
    }
    #[cfg(feature = "i218-phy-probe")]
    pub(crate) fn delay_us(&mut self, us: u32) -> Result<(), &'static str> {
        let start = unsafe { read(self.timer.port) };
        self.clock.sample(start).ok_or("CLOCK OVERFLOW")?;
        let delay = musha_platform::ShortDelay::new(self.timer, start, us).ok_or("DELAY RANGE")?;
        for _ in 0..2_000_000 {
            let raw = unsafe { read(self.timer.port) };
            self.clock.sample(raw).ok_or("CLOCK OVERFLOW")?;
            if delay.complete(raw) {
                return Ok(());
            }
            core::hint::spin_loop();
        }
        Err("CLOCK STALLED")
    }
    pub(crate) fn now(&mut self) -> Result<u64, &'static str> {
        self.clock
            .sample(unsafe { read(self.timer.port) })
            .ok_or("CLOCK OVERFLOW")
    }
}
