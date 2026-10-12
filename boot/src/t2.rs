// Copyright 2026 Hirofumi Iwasaki
// SPDX-License-Identifier: Apache-2.0
//! T2-D1: read-only segment-zero PCI and pre-EBS SMBIOS evidence.
//! No PCI config writes, BAR sizing, MMIO, DMA allocations or firmware commands.
use musha_memory::MemoryMap;
use musha_platform::t2_diagnostics::{BCE_ID, capabilities, smbios_entry, smbios_product};
use r_efi::efi;

#[derive(Clone, Copy)]
struct Device {
    bdf: u16,
    header: [u32; 16],
}
const EMPTY: Device = Device {
    bdf: 0,
    header: [0; 16],
};
struct Evidence {
    devices: [Device; 64],
    count: usize,
    omitted: usize,
    bce: [u16; 2],
    bce_count: usize,
    bce_omitted: usize,
    phases: [[[u32; 64]; 2]; 3],
    model: [u8; 48],
    model_len: usize,
    model_status: &'static str,
}
// Boot CPU only; scalar copies survive EBS. No firmware pointer is retained.
static mut EVIDENCE: Evidence = Evidence {
    devices: [EMPTY; 64],
    count: 0,
    omitted: 0,
    bce: [0; 2],
    bce_count: 0,
    bce_omitted: 0,
    phases: [[[0; 64]; 2]; 3],
    model: [0; 48],
    model_len: 0,
    model_status: "NOT CAPTURED",
};
fn read(bdf: u16, offset: u8) -> u32 {
    unsafe {
        crate::pci::read(
            (bdf >> 8) as u8,
            ((bdf >> 3) & 31) as u8,
            (bdf & 7) as u8,
            offset,
        )
    }
}
pub fn discover() {
    // SAFETY: called once on the boot CPU, no callbacks or references escape.
    let e = unsafe { &mut *core::ptr::addr_of_mut!(EVIDENCE) };
    for bus in 0..=255u16 {
        for device in 0..32u16 {
            let first = (bus << 8) | (device << 3);
            if read(first, 0) as u16 == 0xffff {
                continue;
            }
            let functions = if read(first, 0x0c) & 0x0080_0000 != 0 {
                8
            } else {
                1
            };
            for function in 0..functions {
                let bdf = first | function;
                let id = read(bdf, 0);
                if id as u16 == 0xffff {
                    continue;
                }
                if e.count < e.devices.len() {
                    let d = &mut e.devices[e.count];
                    d.bdf = bdf;
                    // Read standard headers only; no device-specific registers.
                    for (i, w) in d.header.iter_mut().enumerate() {
                        *w = read(bdf, (i * 4) as u8);
                    }
                    e.count += 1;
                } else {
                    e.omitted += 1;
                }
                if id == BCE_ID {
                    if e.bce_count < e.bce.len() {
                        e.bce[e.bce_count] = bdf;
                        e.bce_count += 1;
                    } else {
                        e.bce_omitted += 1;
                    }
                }
            }
        }
    }
    capture(0);
}
/// Configuration reads only, including immediately around ExitBootServices.
pub fn capture(phase: usize) {
    if phase >= 3 {
        return;
    }
    let e = unsafe { &mut *core::ptr::addr_of_mut!(EVIDENCE) };
    for i in 0..e.bce_count {
        for (j, w) in e.phases[phase][i].iter_mut().enumerate() {
            *w = read(e.bce[i], (j * 4) as u8);
        }
    }
}
// Firmware mappings are still active; require one readable RAM descriptor.
unsafe fn bytes<'a>(map: &MemoryMap<'_>, address: usize, size: usize) -> Option<&'a [u8]> {
    if size > 65536 {
        return None;
    }
    for i in 0..map.count() {
        let r = map.region(i).ok()?;
        if musha_platform::acpi_read::readable_region(
            r.kind,
            r.range.start,
            r.range.end,
            address,
            size,
        ) {
            return Some(unsafe { core::slice::from_raw_parts(address as *const u8, size) });
        }
    }
    None
}
pub unsafe fn model(system: *const efi::SystemTable, map: &MemoryMap<'_>) {
    let e = unsafe { &mut *core::ptr::addr_of_mut!(EVIDENCE) };
    e.model_status = "SMBIOS UNAVAILABLE";
    let s = unsafe { &*system };
    if s.number_of_table_entries > 1024 || s.configuration_table.is_null() {
        return;
    }
    let entries =
        unsafe { core::slice::from_raw_parts(s.configuration_table, s.number_of_table_entries) };
    for v3 in [true, false] {
        let guid = if v3 {
            efi::SMBIOS3_TABLE_GUID
        } else {
            efi::SMBIOS_TABLE_GUID
        };
        for entry in entries.iter().filter(|x| x.vendor_guid == guid) {
            e.model_status = "SMBIOS ENTRY / MAP INVALID";
            let addr = entry.vendor_table as usize;
            let Some(head) = (unsafe { bytes(map, addr, if v3 { 24 } else { 31 }) }) else {
                continue;
            };
            let len = head[if v3 { 6 } else { 5 }] as usize;
            if len > 32 {
                continue;
            }
            let Some(head) = (unsafe { bytes(map, addr, len) }) else {
                continue;
            };
            let Some((address, size)) = smbios_entry(head, v3) else {
                continue;
            };
            e.model_status = "SMBIOS TABLE MAP INVALID";
            let Some(table) = (unsafe { bytes(map, address as usize, size) }) else {
                continue;
            };
            match smbios_product(table) {
                Ok(product) => {
                    e.model[..product.len()].copy_from_slice(product.as_bytes());
                    e.model_len = product.len();
                    e.model_status = "OK";
                    return;
                }
                Err(error) => e.model_status = error,
            }
        }
    }
}
fn emit(args: core::fmt::Arguments<'_>) {
    use core::fmt::Write;
    let mut line = musha_platform::diagnostics::Line::EMPTY;
    let _ = line.write_fmt(args);
    crate::debug(b"MUSHA: ");
    crate::debug(line.as_str().as_bytes());
    crate::debug(b"\n");
    crate::diagnostics::observation(format_args!("{}", line.as_str()));
}
pub fn report() {
    let e = unsafe { &*core::ptr::addr_of!(EVIDENCE) };
    emit(format_args!("T2-D1 READ ONLY / NO MMIO / NO DMA"));
    emit(format_args!(
        "T2 MODEL {}",
        core::str::from_utf8(&e.model[..e.model_len]).unwrap_or("?")
    ));
    emit(format_args!("T2 SMBIOS {}", e.model_status));
    emit(format_args!(
        "T2 PCI SEG 0 COUNT {} OMIT {}",
        e.count, e.omitted
    ));
    for d in &e.devices[..e.count] {
        emit(format_args!(
            "T2 PCI {:04X} ID {:08X} CLASS {:06X}",
            d.bdf,
            d.header[0],
            d.header[2] >> 8
        ));
        if (d.header[3] >> 16) & 0x7f == 1 {
            emit(format_args!(
                "T2 BRIDGE {:04X} CMD {:04X} BUS {:08X}",
                d.bdf, d.header[1] as u16, d.header[6]
            ));
            emit(format_args!(
                "T2 BR MEM {:08X} PREF {:08X}",
                d.header[8], d.header[9]
            ));
            emit(format_args!(
                "T2 BR PREF UPPER {:08X} {:08X}",
                d.header[10], d.header[11]
            ));
        }
    }
    emit(format_args!(
        "T2 BCE COUNT {} OMIT {}",
        e.bce_count, e.bce_omitted
    ));
    for i in 0..e.bce_count {
        for (phase, name) in ["EARLY", "PRE EBS", "POST EBS"].iter().enumerate() {
            let c = &e.phases[phase][i];
            emit(format_args!(
                "T2 {} BDF {:04X} ID {:08X}",
                name, e.bce[i], c[0]
            ));
            emit(format_args!(
                "T2 CMD {:04X} STATUS {:04X} HDR {:02X}",
                c[1] as u16,
                c[1] >> 16,
                (c[3] >> 16) as u8
            ));
            if c[0] != BCE_ID {
                emit(format_args!("T2 ID CHANGED / CONFIG UNAVAILABLE"));
                continue;
            }
            let bars: [u32; 6] = c[4..10].try_into().unwrap();
            let layout = musha_platform::bar_layout((c[3] >> 16) as u8, bars);
            for j in 0..6 {
                emit(format_args!("T2 BAR{} {:08X} {}", j, bars[j], layout[j]));
            }
            match capabilities(c) {
                Ok(caps) => {
                    for (kind, value) in [
                        ("PMCSR", caps.pm),
                        ("MSI CTRL", caps.msi),
                        ("MSIX CTRL", caps.msix),
                    ] {
                        if let Some((at, value)) = value {
                            emit(format_args!(
                                "T2 {} CAP {:02X} VALUE {:04X}",
                                kind, at, value
                            ));
                        } else {
                            emit(format_args!("T2 {} ABSENT", kind));
                        }
                    }
                }
                Err(error) => emit(format_args!("T2 CAP ERROR {}", error)),
            }
        }
    }
    emit(format_args!(
        "T2 BAR RANGES UNVALIDATED / DRIVER NOT STARTED"
    ));
    emit(format_args!("T2_DIAGNOSTICS_COMPLETE"));
}
