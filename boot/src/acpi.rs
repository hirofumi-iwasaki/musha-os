// SPDX-License-Identifier: Apache-2.0
use musha_memory::{MemoryMap, Range};
use musha_platform::Timer;
use r_efi::efi;

// Firmware remains active, and the supplied map describes readable table RAM.
// Never expose these borrows after ExitBootServices: only Timer is copied out.
unsafe fn bytes<'a>(map: &MemoryMap<'_>, address: usize, size: usize) -> Option<&'a [u8]> {
    let range = Range::new(address, size).ok()?;
    if address == 0 {
        return None;
    }
    for i in 0..map.count() {
        let r = map.region(i).ok()?;
        if matches!(r.kind, 2 | 4 | 6 | 9 | 10)
            && r.attributes & (1 << 13) == 0
            && r.range.start <= range.start
            && r.range.end >= range.end
        {
            return Some(unsafe { core::slice::from_raw_parts(address as *const u8, size) });
        }
    }
    None
}
unsafe fn table<'a>(map: &MemoryMap<'_>, address: usize) -> Option<&'a [u8]> {
    let head = unsafe { bytes(map, address, 36) }?;
    let size = u32::from_le_bytes(head[4..8].try_into().ok()?) as usize;
    if !(36..=65536).contains(&size) {
        return None;
    }
    let all = unsafe { bytes(map, address, size) }?;
    musha_platform::checksum(all).then_some(all)
}
pub(crate) unsafe fn discover(
    system: *const efi::SystemTable,
    map: &MemoryMap<'_>,
) -> Option<Timer> {
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
    let rsdp = unsafe { bytes(map, address, 20) }?;
    if &rsdp[..8] != b"RSD PTR " || !musha_platform::checksum(rsdp) {
        return None;
    }
    let (root, width) = if rsdp[15] >= 2 {
        let extended = unsafe { bytes(map, address, 36) }?;
        let size = u32::from_le_bytes(extended[20..24].try_into().ok()?) as usize;
        if !(36..=4096).contains(&size) {
            return None;
        }
        if !musha_platform::checksum(unsafe { bytes(map, address, size) }?) {
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
    let root = unsafe { table(map, root) }?;
    let signature = if width == 8 { b"XSDT" } else { b"RSDT" };
    if &root[..4] != signature || (root.len() - 36) % width != 0 || (root.len() - 36) / width > 512
    {
        return None;
    }
    let mut result = None;
    for item in root[36..].chunks_exact(width) {
        let address = if width == 8 {
            usize::try_from(u64::from_le_bytes(item.try_into().ok()?)).ok()?
        } else {
            u32::from_le_bytes(item.try_into().ok()?) as usize
        };
        let header = unsafe { bytes(map, address, 36) }?;
        if &header[..4] == b"FACP" {
            if result.is_some() {
                return None;
            }
            result = Some(musha_platform::fadt(unsafe { table(map, address) }?)?);
        }
    }
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
    pub(crate) fn now(&mut self) -> Result<u64, &'static str> {
        self.clock
            .sample(unsafe { read(self.timer.port) })
            .ok_or("CLOCK OVERFLOW")
    }
}
