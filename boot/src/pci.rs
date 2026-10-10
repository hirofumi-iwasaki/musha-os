// Copyright 2026 Hirofumi Iwasaki
// SPDX-License-Identifier: Apache-2.0
//! PCI configuration mechanism 1: segment zero, firmware-configured buses.
use core::arch::asm;
use musha_framebuffer::Framebuffer;

// The boot CPU is the sole caller; interrupts stay disabled. CF8/CFC transactions
// cannot interleave. Discovery does not size BARs or reconfigure buses.
// The driver ownership helpers below separately change PCI bus mastering.
pub(crate) unsafe fn read(bus: u8, device: u8, function: u8, register: u8) -> u32 {
    let address = 0x8000_0000u32
        | ((bus as u32) << 16)
        | ((device as u32) << 11)
        | ((function as u32) << 8)
        | (register as u32 & 0xfc);
    let value: u32;
    unsafe {
        asm!("out dx, eax", in("dx") 0xcf8u16, in("eax") address, options(nomem, nostack));
        asm!("in eax, dx", in("dx") 0xcfcu16, out("eax") value, options(nomem, nostack));
    }
    value
}
fn field(label: &[u8], value: u64) {
    super::debug(label);
    super::debug(&super::cpu::hex(value));
}
pub(crate) fn diagnose(fb: Framebuffer, selected: Controller) {
    let mut count = 0u64;
    let mut xhci = 0u64;
    let mut intel_lan = 0u64;
    for bus in 0..=255u8 {
        for device in 0..32u8 {
            // PCI functions are present only if function zero exists.
            let id = unsafe { read(bus, device, 0, 0) };
            if id as u16 == 0xffff {
                continue;
            }
            let header = unsafe { read(bus, device, 0, 0x0c) };
            let functions = if header & 0x0080_0000 != 0 { 8 } else { 1 };
            for function in 0..functions {
                let id = unsafe { read(bus, device, function, 0) };
                if id as u16 == 0xffff {
                    continue;
                }
                let class = unsafe { read(bus, device, function, 8) } >> 8;
                count += 1;
                if class == 0x0c0330 {
                    let bdf = ((bus as u16) << 8) | ((device as u16) << 3) | function as u16;
                    let active = selected.base != 0
                        && selected.bus == bus
                        && selected.device == device
                        && selected.function == function;
                    crate::diagnostics::controller(xhci as usize, id, bdf, active);
                    if active {
                        crate::diagnostics::pci(2, id, bdf);
                    }
                    xhci += 1;
                }
                if class >> 8 == 0x0200 && id as u16 == 0x8086 {
                    if intel_lan == 0 {
                        crate::diagnostics::pci(
                            3,
                            id,
                            ((bus as u16) << 8) | ((device as u16) << 3) | function as u16,
                        );
                    }
                    intel_lan += 1;
                }
                field(
                    b"MUSHA: PCI BDF=",
                    ((bus as u64) << 8) | ((device as u64) << 3) | function as u64,
                );
                field(b" ID=", id as u64);
                field(b" CLASS=", class as u64);
                super::debug(b"\n");
            }
        }
    }
    crate::diagnostics::inventory_total(xhci as usize);
    field(b"MUSHA: PCI_COUNT=", count);
    field(b" XHCI=", xhci);
    field(b" INTEL_LAN=", intel_lan);
    super::debug(b"\nMUSHA: PCI_ENUMERATION_OK\n");
    // Discovery is diagnostic: zero devices in a class is reported, not treated
    // as proof of a working driver or as a reason to stop unrelated hardware.
    unsafe {
        fb.text("PCI DEVICES", 24, 164, fb.color(0, 220, 240));
        let digits = super::cpu::hex(count);
        fb.text(
            core::str::from_utf8_unchecked(&digits),
            240,
            164,
            fb.color(0, 220, 240),
        );
        fb.text("XHCI", 24, 196, fb.color(0, 220, 240));
        let digits = super::cpu::hex(xhci);
        fb.text(
            core::str::from_utf8_unchecked(&digits),
            240,
            196,
            fb.color(0, 220, 240),
        );
        fb.text("INTEL LAN", 24, 228, fb.color(0, 220, 240));
        let digits = super::cpu::hex(intel_lan);
        fb.text(
            core::str::from_utf8_unchecked(&digits),
            240,
            228,
            fb.color(0, 220, 240),
        );
    }
}

pub(crate) use musha_platform::controllers::{Controller, Controllers, MAX_CONTROLLERS};
pub(crate) unsafe fn discover(bs: *mut r_efi::efi::BootServices, nic: bool) -> Controller {
    unsafe { discover_all(bs, nic).entries[0] }
}
// EFI protocol/resource pointers are trusted firmware allocations, used and
// freed while Boot Services is live. Copy only scalar BAR and BDF information.
pub(crate) unsafe fn discover_all(bs: *mut r_efi::efi::BootServices, nic: bool) -> Controllers {
    use r_efi::{efi, protocols::pci_io};
    let mut guid = pci_io::PROTOCOL_GUID;
    let mut count = 0;
    let mut handles = core::ptr::null_mut();
    unsafe {
        let status = ((*bs).locate_handle_buffer)(
            efi::BY_PROTOCOL,
            &mut guid,
            core::ptr::null_mut(),
            &mut count,
            &mut handles,
        );
        if status.is_error() || handles.is_null() {
            return Controllers::EMPTY;
        }
        let mut found = Controllers::EMPTY;
        if count <= 1024 {
            for handle in core::slice::from_raw_parts(handles, count) {
                let mut protocol = core::ptr::null_mut();
                if ((*bs).handle_protocol)(*handle, &mut guid, &mut protocol).is_error()
                    || protocol.is_null()
                {
                    continue;
                }
                let p = protocol as *mut pci_io::Protocol;
                let mut class = 0u32;
                if ((*p).pci.read)(
                    p,
                    pci_io::WIDTH_UINT32,
                    8,
                    1,
                    (&mut class as *mut u32).cast(),
                )
                .is_error()
                    || class >> 8 != if nic { 0x020000 } else { 0x0c0330 }
                {
                    continue;
                }
                if nic {
                    let mut id = 0u32;
                    // Only the explicit probe build selects NUC5's expected
                    // I218-V3 candidate; other I218 variants remain excluded.
                    let eligible = |id: u32| {
                        id == 0x10d38086 || (cfg!(feature = "i218-phy-probe") && id == 0x15a38086)
                    };
                    if ((*p).pci.read)(p, pci_io::WIDTH_UINT32, 0, 1, (&mut id as *mut u32).cast())
                        .is_error()
                        || !eligible(id)
                    {
                        continue;
                    }
                }
                let (mut segment, mut bus, mut device, mut function) = (0, 0, 0, 0);
                if ((*p).get_location)(p, &mut segment, &mut bus, &mut device, &mut function)
                    .is_error()
                    || segment != 0
                    || bus > 255
                    || device > 31
                    || function > 7
                {
                    continue;
                }
                let mut resources = core::ptr::null_mut();
                if ((*p).get_bar_attributes)(p, 0, core::ptr::null_mut(), &mut resources).is_error()
                    || resources.is_null()
                {
                    continue;
                }
                // UEFI specifies one QWORD resource descriptor + end tag.
                let descriptor = core::slice::from_raw_parts(resources as *const u8, 48);
                let range = musha_platform::bar_resource(descriptor);
                ((*bs).free_pool)(resources);
                if let Some((base, bytes)) = range {
                    let c = Controller {
                        base,
                        bytes,
                        bus: bus as u8,
                        device: device as u8,
                        function: function as u8,
                    };
                    if let Err(error) = found.insert(c) {
                        crate::debug(b"MUSHA: PCI_RESOURCE_REJECTED ");
                        crate::debug(error.as_bytes());
                        crate::debug(b"\n");
                        continue;
                    }
                    if nic {
                        break;
                    }
                }
            }
        }
        ((*bs).free_pool)(handles.cast());
        found.entries[..found.count].sort_unstable_by_key(|c| c.bdf());
        found
    }
}
/// Disable bus mastering after ownership/halt. A 16-bit write avoids clearing
/// adjacent PCI Status W1C bits. Keep memory decoding for reset diagnostics.
pub(crate) unsafe fn disable_dma(c: Controller) {
    let command = unsafe { read(c.bus, c.device, c.function, 4) } as u16;
    let address = 0x80000004u32
        | ((c.bus as u32) << 16)
        | ((c.device as u32) << 11)
        | ((c.function as u32) << 8);
    unsafe {
        asm!("out dx, eax",in("dx") 0xcf8u16,in("eax") address,options(nomem,nostack));
        asm!("out dx, ax",in("dx") 0xcfcu16,in("ax") command & !4,options(nomem,nostack));
    }
}

/// Enable only when every DMA pointer and ring is ready, after the owning controller is initialized.
pub(crate) unsafe fn enable_dma(c: Controller) {
    let command = unsafe { read(c.bus, c.device, c.function, 4) } as u16;
    let address = 0x80000004u32
        | ((c.bus as u32) << 16)
        | ((c.device as u32) << 11)
        | ((c.function as u32) << 8);
    unsafe {
        asm!("out dx, eax",in("dx") 0xcf8u16,in("eax") address,options(nomem,nostack));
        asm!("out dx, ax",in("dx") 0xcfcu16,in("ax") command | 4,options(nomem,nostack));
    }
}

fn pm_state(c: Controller) -> u32 {
    let r = |reg| unsafe { read(c.bus, c.device, c.function, reg) };
    if r(4) & (1 << 20) == 0 {
        return u32::MAX;
    }
    let mut next = (r(0x34) & 0xfc) as u8;
    let mut seen = [false; 64];
    for _ in 0..48 {
        if next == 0 {
            return u32::MAX;
        }
        if next < 0x40 || seen[(next / 4) as usize] {
            return u32::MAX - 1;
        }
        seen[(next / 4) as usize] = true;
        let cap = r(next);
        if cap & 255 == 1 {
            return if next <= 0xf8 {
                r(next + 4) & 0xffff
            } else {
                u32::MAX - 1
            };
        }
        next = ((cap >> 8) & 0xfc) as u8;
    }
    u32::MAX - 1
}
// Read-only, bounded capability walk. Never enable decoding or size BARs here.
pub(crate) fn controller_evidence(c: Controller) {
    let read = |r| unsafe { self::read(c.bus, c.device, c.function, r) };
    crate::diagnostics::observation(format_args!(
        "PCI {:04X} CMD {:04X} STATUS {:04X}",
        c.bdf(),
        read(4) & 0xffff,
        read(4) >> 16
    ));
    crate::diagnostics::observation(format_args!(
        "BAR0 {:08X} BAR1 {:08X}",
        read(0x10),
        read(0x14)
    ));
    crate::diagnostics::observation(format_args!(
        "BAR RESOURCE {:016X} BYTES {:X}",
        c.base, c.bytes
    ));
    if read(4) & (1 << 20) == 0 {
        crate::diagnostics::observation(format_args!("PCI PM CAP ABSENT"));
        return;
    }
    let mut next = (read(0x34) & 0xfc) as u8;
    let mut visited = [false; 64];
    for _ in 0..48 {
        if next == 0 {
            crate::diagnostics::observation(format_args!("PCI PM CAP ABSENT"));
            return;
        }
        if next < 0x40 || visited[(next / 4) as usize] {
            crate::diagnostics::observation(format_args!("PCI CAP CHAIN INVALID {:02X}", next));
            return;
        }
        visited[(next / 4) as usize] = true;
        let cap = read(next);
        if cap & 255 == 1 {
            if next > 0xf8 {
                crate::diagnostics::observation(format_args!("PCI PM CAP RANGE"));
                return;
            }
            let pmcsr = read(next + 4) & 0xffff;
            crate::diagnostics::observation(format_args!(
                "PCI PMCSR {:04X} POWER D{}",
                pmcsr,
                pmcsr & 3
            ));
            return;
        }
        next = ((cap >> 8) & 0xfc) as u8;
    }
    crate::diagnostics::observation(format_args!("PCI CAP CHAIN LIMIT"));
}

#[derive(Clone, Copy)]
pub(crate) struct Snapshot {
    values: [u32; 5],
}
impl Snapshot {
    pub const EMPTY: Self = Self { values: [0; 5] };
}
pub(crate) fn snapshots(cs: &Controllers) -> [Snapshot; MAX_CONTROLLERS] {
    let mut out = [Snapshot::EMPTY; MAX_CONTROLLERS];
    for (i, c) in cs.entries[..cs.count].iter().enumerate() {
        out[i].values = unsafe {
            [
                read(c.bus, c.device, c.function, 0),
                read(c.bus, c.device, c.function, 4),
                read(c.bus, c.device, c.function, 0x10),
                read(c.bus, c.device, c.function, 0x14),
                pm_state(*c),
            ]
        };
    }
    out
}
pub(crate) fn report_snapshots(cs: &Controllers, phases: &[(&str, &[Snapshot; MAX_CONTROLLERS])]) {
    for (i, c) in cs.entries[..cs.count].iter().enumerate() {
        for (name, snapshots) in phases {
            let v = snapshots[i].values;
            crate::diagnostics::observation(format_args!("{} PMCSR {:08X}", name, v[4]));
            crate::diagnostics::observation(format_args!(
                "{} PCI {:04X} ID {:08X} CMD {:04X}",
                name,
                c.bdf(),
                v[0],
                v[1] & 0xffff
            ));
            crate::diagnostics::observation(format_args!(
                "{} STATUS {:04X} BAR {:08X} {:08X}",
                name,
                v[1] >> 16,
                v[2],
                v[3]
            ));
        }
    }
}
#[derive(Clone, Copy)]
pub(crate) struct BootPath {
    bytes: [u8; 512],
    len: usize,
    complete: bool,
}
// Firmware protocol pointer is consumed before ExitBootServices. Retain bytes only.
pub(crate) unsafe fn boot_path(
    bs: *mut r_efi::efi::BootServices,
    handle: r_efi::efi::Handle,
) -> BootPath {
    let mut out = BootPath {
        bytes: [0; 512],
        len: 0,
        complete: false,
    };
    let mut guid = r_efi::protocols::device_path::PROTOCOL_GUID;
    let mut ptr = core::ptr::null_mut();
    unsafe {
        if ((*bs).handle_protocol)(handle, &mut guid, &mut ptr).is_error() || ptr.is_null() {
            return out;
        }
        let ptr = ptr as *const u8;
        for _ in 0..64 {
            if out.len + 4 > 512 {
                break;
            }
            let head = core::slice::from_raw_parts(ptr.add(out.len), 4);
            let n = u16::from_le_bytes([head[2], head[3]]) as usize;
            if n < 4 || out.len + n > 512 {
                break;
            }
            let end = head[0] == 0x7f && head[1] == 0xff;
            out.bytes[out.len..out.len + n]
                .copy_from_slice(core::slice::from_raw_parts(ptr.add(out.len), n));
            out.len += n;
            if end {
                out.complete = n == 4;
                break;
            }
        }
    }
    out
}
pub(crate) fn report_boot_path(path: &BootPath) {
    crate::diagnostics::observation(format_args!(
        "BOOT PATH BYTES {} COMPLETE {}",
        path.len, path.complete as u8
    ));
    let mut offset = 0;
    while offset + 4 <= path.len {
        let b = &path.bytes[offset..];
        let n = u16::from_le_bytes([b[2], b[3]]) as usize;
        if n < 4 || offset + n > path.len {
            break;
        }
        crate::diagnostics::observation(format_args!(
            "BOOT NODE {:02X}/{:02X} LEN {}",
            b[0], b[1], n
        ));
        if b[0] == 1 && b[1] == 1 && n == 6 {
            crate::diagnostics::observation(format_args!(
                "BOOT PCI DEVICE {} FUNCTION {}",
                b[5], b[4]
            ));
        } else if b[0] == 3 && b[1] == 5 && n == 6 {
            crate::diagnostics::observation(format_args!(
                "BOOT USB PORT {} INTERFACE {}",
                b[4], b[5]
            ));
        } else {
            for (i, chunk) in b[4..n].chunks(8).enumerate() {
                let mut value = 0u64;
                for byte in chunk {
                    value = (value << 8) | *byte as u64;
                }
                crate::diagnostics::observation(format_args!(
                    "BOOT DATA {} {:016X} BYTES {}",
                    i,
                    value,
                    chunk.len()
                ));
            }
        }
        offset += n;
    }
}

pub(crate) unsafe fn boot_owner(
    bs: *mut r_efi::efi::BootServices,
    cs: &Controllers,
    path: &BootPath,
) -> u16 {
    use r_efi::{efi, protocols::pci_io};
    if !path.complete {
        return u16::MAX;
    }
    let mut guid = pci_io::PROTOCOL_GUID;
    let mut count = 0;
    let mut handles = core::ptr::null_mut();
    let mut owner = u16::MAX;
    unsafe {
        if ((*bs).locate_handle_buffer)(
            efi::BY_PROTOCOL,
            &mut guid,
            core::ptr::null_mut(),
            &mut count,
            &mut handles,
        )
        .is_error()
            || handles.is_null()
        {
            return owner;
        }
        if count <= 1024 {
            for handle in core::slice::from_raw_parts(handles, count) {
                let mut ptr = core::ptr::null_mut();
                if ((*bs).handle_protocol)(*handle, &mut guid, &mut ptr).is_error() || ptr.is_null()
                {
                    continue;
                }
                let p = ptr as *mut pci_io::Protocol;
                let (mut segment, mut bus, mut dev, mut fun) = (0, 0, 0, 0);
                if ((*p).get_location)(p, &mut segment, &mut bus, &mut dev, &mut fun).is_error()
                    || segment != 0
                    || bus > 255
                    || dev > 31
                    || fun > 7
                {
                    continue;
                }
                let bdf = ((bus as u16) << 8) | ((dev as u16) << 3) | fun as u16;
                if !cs.entries[..cs.count].iter().any(|c| c.bdf() == bdf) {
                    continue;
                }
                let cp = boot_path(bs, *handle);
                if cp.complete
                    && cp.len > 4
                    && path.len >= cp.len
                    && path.bytes[..cp.len - 4] == cp.bytes[..cp.len - 4]
                {
                    if owner != u16::MAX {
                        owner = u16::MAX - 1;
                        break;
                    }
                    owner = bdf;
                }
            }
        }
        ((*bs).free_pool)(handles.cast());
    }
    owner
}

const INSPECT_OFFSETS: [u8; 10] = [0, 4, 8, 12, 16, 20, 24, 28, 32, 36];
#[derive(Clone, Copy)]
struct ConfigInspection {
    values: [u32; 10],
    statuses: [usize; 10],
}
impl ConfigInspection {
    const EMPTY: Self = Self {
        values: [0; 10],
        statuses: [usize::MAX; 10],
    };
    fn stable(self, other: Self) -> bool {
        self.statuses.iter().all(|s| *s == 0)
            && other.statuses.iter().all(|s| *s == 0)
            && self.values == other.values
    }
}
unsafe fn inspect_config(p: *mut r_efi::protocols::pci_io::Protocol) -> ConfigInspection {
    let mut result = ConfigInspection::EMPTY;
    for (i, offset) in INSPECT_OFFSETS.iter().enumerate() {
        result.statuses[i] = unsafe {
            ((*p).pci.read)(
                p,
                r_efi::protocols::pci_io::WIDTH_UINT32,
                *offset as u32,
                1,
                (&mut result.values[i] as *mut u32).cast(),
            )
        }
        .as_usize();
    }
    result
}
// Single boot CPU, written only before EBS; scalar evidence survives EBS.
#[derive(Clone, Copy)]
struct BoundedPeer {
    peer: [usize; 4],
    config: ConfigInspection,
    slot: usize,
    live: (u64, u64),
    firmware: (u64, u64),
}
impl BoundedPeer {
    const EMPTY: Self = Self {
        peer: [0; 4],
        config: ConfigInspection::EMPTY,
        slot: 0,
        live: (0, 0),
        firmware: (0, 0),
    };
}
struct BoundStore(core::cell::UnsafeCell<([BoundedPeer; 16], usize)>);
unsafe impl Sync for BoundStore {}
static BOUNDS: BoundStore = BoundStore(core::cell::UnsafeCell::new(([BoundedPeer::EMPTY; 16], 0)));
fn config_matches_cf8(peer: [usize; 4], config: ConfigInspection) -> bool {
    if peer[0] != 0
        || peer[1] > 255
        || peer[2] > 31
        || peer[3] > 7
        || config.statuses.iter().any(|s| *s != 0)
    {
        return false;
    }
    INSPECT_OFFSETS.iter().enumerate().all(|(i, off)| unsafe {
        read(peer[1] as u8, peer[2] as u8, peer[3] as u8, *off) == config.values[i]
    })
}
fn recheck_bounded_peers(c: Controller) -> Result<(), &'static str> {
    let (records, count) = unsafe { &*BOUNDS.0.get() };
    for record in &records[..*count] {
        if !config_matches_cf8(record.peer, record.config) {
            return Err("PEER STATE CHANGED");
        }
        let own = record.peer == [0, c.bus as usize, c.device as usize, c.function as usize];
        let target = (c.base as u64, c.bytes as u64);
        if own
            || !musha_platform::pci_resources::disjoint(record.live, target)
            || !musha_platform::pci_resources::disjoint(record.firmware, target)
        {
            return Err("PEER BOUND CONFLICT");
        }
    }
    Ok(())
}
fn report_bounded_peers() {
    let (records, count) = unsafe { &*BOUNDS.0.get() };
    for r in &records[..*count] {
        crate::diagnostics::observation(format_args!(
            "PEER BOUND {:02X}:{:02X}.{} BAR {}",
            r.peer[1], r.peer[2], r.peer[3], r.slot
        ));
        crate::diagnostics::observation(format_args!(
            "LIVE {:016X} MAX BYTES {:X}",
            r.live.0, r.live.1
        ));
        crate::diagnostics::observation(format_args!(
            "FW {:016X} BYTES {:X}",
            r.firmware.0, r.firmware.1
        ));
    }
}
#[derive(Clone, Copy)]
struct BarInspection {
    present: bool,
    before: ConfigInspection,
    after: ConfigInspection,
    repeat: ConfigInspection,
    cf8: [u32; 10],
    cf8_valid: bool,
    attribute_status: usize,
    attribute_null: bool,
    location_status: usize,
    bar_raw: u32,
}
impl BarInspection {
    const EMPTY: Self = Self {
        present: false,
        before: ConfigInspection::EMPTY,
        after: ConfigInspection::EMPTY,
        repeat: ConfigInspection::EMPTY,
        cf8: [0; 10],
        cf8_valid: false,
        attribute_status: usize::MAX,
        attribute_null: true,
        location_status: usize::MAX,
        bar_raw: 0,
    };
}
// Boot-CPU-only scalar storage, separate from the one-page BootInfo ABI.
struct InspectionStore(core::cell::UnsafeCell<[BarInspection; MAX_CONTROLLERS]>);
// No callbacks use this store; interrupts remain disabled during collection/display.
unsafe impl Sync for InspectionStore {}
static INSPECTIONS: InspectionStore = InspectionStore(core::cell::UnsafeCell::new(
    [BarInspection::EMPTY; MAX_CONTROLLERS],
));
// Immutable pre-EBS evidence. First failure is retained; never relaxes the H14 gate.
#[derive(Clone, Copy)]
pub(crate) struct ResourceEvidence {
    reason: &'static str,
    peer: [usize; 4],
    detail: usize,
    range: [u64; 2],
    descriptor: [u64; 6],
}
impl ResourceEvidence {
    const EMPTY: Self = Self {
        reason: "",
        peer: [usize::MAX; 4],
        detail: 0,
        range: [0; 2],
        descriptor: [0; 6],
    };
    fn reject(&mut self, reason: &'static str, peer: [usize; 4], detail: usize, range: [u64; 2]) {
        if self.reason.is_empty() {
            *self = Self {
                reason,
                peer,
                detail,
                range,
                descriptor: [0; 6],
            };
        }
    }
}
// Check all firmware PCI memory resources, not just other xHCI controllers.
pub(crate) unsafe fn resource_safety(
    bs: *mut r_efi::efi::BootServices,
    cs: &Controllers,
) -> [ResourceEvidence; MAX_CONTROLLERS] {
    use r_efi::{efi, protocols::pci_io};
    let mut safe = [ResourceEvidence::EMPTY; MAX_CONTROLLERS];
    unsafe {
        *INSPECTIONS.0.get() = [BarInspection::EMPTY; MAX_CONTROLLERS];
        *BOUNDS.0.get() = ([BoundedPeer::EMPTY; 16], 0);
    }
    let mut guid = pci_io::PROTOCOL_GUID;
    let mut count = 0;
    let mut handles = core::ptr::null_mut();
    unsafe {
        if ((*bs).locate_handle_buffer)(
            efi::BY_PROTOCOL,
            &mut guid,
            core::ptr::null_mut(),
            &mut count,
            &mut handles,
        )
        .is_error()
            || handles.is_null()
        {
            return [ResourceEvidence {
                reason: "HANDLE ENUMERATION",
                ..ResourceEvidence::EMPTY
            }; MAX_CONTROLLERS];
        }
        if count > 1024 {
            ((*bs).free_pool)(handles.cast());
            return [ResourceEvidence {
                reason: "HANDLE LIMIT",
                ..ResourceEvidence::EMPTY
            }; MAX_CONTROLLERS];
        }
        for handle in core::slice::from_raw_parts(handles, count) {
            let mut peer = [usize::MAX; 4];
            let mut status_code = 0usize;
            let mut resource_range = [0u64; 2];
            let mut ptr = core::ptr::null_mut();
            if ((*bs).handle_protocol)(*handle, &mut guid, &mut ptr).is_error() || ptr.is_null() {
                for evidence in &mut safe {
                    evidence.reject("PROTOCOL LOOKUP", peer, status_code, resource_range);
                }
                break;
            }
            let p = ptr as *mut pci_io::Protocol;
            let (mut seg, mut bus, mut dev, mut fun) = (0, 0, 0, 0);
            let location_status = ((*p).get_location)(p, &mut seg, &mut bus, &mut dev, &mut fun);
            if location_status.is_error() {
                for evidence in &mut safe {
                    evidence.reject("LOCATION READ", peer, status_code, resource_range);
                }
                break;
            }
            let mut head = 0u32;
            if ((*p).pci.read)(
                p,
                pci_io::WIDTH_UINT32,
                0x0c,
                1,
                (&mut head as *mut u32).cast(),
            )
            .is_error()
            {
                for evidence in &mut safe {
                    evidence.reject("HEADER READ", peer, status_code, resource_range);
                }
                break;
            }
            let bars = match (head >> 16) & 0x7f {
                0 => 6,
                1 => 2,
                _ => 0,
            };
            peer = [seg, bus, dev, fun];
            let mut bar = 0;
            let mut resource_index = 0u8;
            while bar < bars {
                resource_range = [0; 2];
                status_code = 0;
                let mut raw = 0u32;
                if ((*p).pci.read)(
                    p,
                    pci_io::WIDTH_UINT32,
                    0x10 + bar * 4,
                    1,
                    (&mut raw as *mut u32).cast(),
                )
                .is_error()
                {
                    for evidence in &mut safe {
                        evidence.reject("BAR READ", peer, status_code, resource_range);
                    }
                    break;
                }
                if raw != 0 && raw & 1 == 0 {
                    let before = inspect_config(p);
                    let was_empty = safe.map(|e| e.reason.is_empty());
                    let mut descriptor = [0u64; 6];
                    let mut resource = core::ptr::null_mut();
                    let status = ((*p).get_bar_attributes)(
                        p,
                        resource_index,
                        core::ptr::null_mut(),
                        &mut resource,
                    );
                    status_code = status.as_usize();
                    let attribute_null = resource.is_null();
                    if status.is_error() || resource.is_null() {
                        for evidence in &mut safe {
                            evidence.reject("BAR ATTRIBUTES", peer, bar as usize, resource_range);
                        }
                    } else {
                        let d = core::slice::from_raw_parts(resource as *const u8, 48);
                        for (word, bytes) in descriptor.iter_mut().zip(d.chunks_exact(8)) {
                            *word = u64::from_le_bytes(bytes.try_into().unwrap());
                        }
                        // Shared decoder; retain raw fields when rejecting so a
                        // real firmware result can be distinguished from a guess.
                        let decoded = musha_platform::memory_bar_descriptor(d);
                        if let Err(reason) = decoded {
                            for evidence in &mut safe {
                                if evidence.reason.is_empty() {
                                    evidence.reject(reason, peer, bar as usize, [0; 2]);
                                    for (word, bytes) in
                                        evidence.descriptor.iter_mut().zip(d.chunks_exact(8))
                                    {
                                        *word = u64::from_le_bytes(bytes.try_into().unwrap());
                                    }
                                }
                            }
                        } else {
                            let (base, len) = decoded.unwrap();
                            let config_base = (raw & !0xf) as u64
                                | if raw & 7 == 4 && bar + 1 < bars {
                                    (before.values[5 + bar as usize] as u64) << 32
                                } else {
                                    0
                                };
                            if base != config_base {
                                let bound = musha_platform::pci_resources::peer_memory32_bound(
                                    raw, base, len,
                                );
                                let coherent = before.values[0] & 0xffff != 0xffff
                                    && before.values[0] & 0xffff != 0
                                    && before.values[1] & 2 != 0
                                    && (before.values[3] >> 16) & 0x7f <= 1
                                    && before.values[4 + bar as usize] == raw
                                    && config_matches_cf8(peer, before);
                                let (records, count) = &mut *BOUNDS.0.get();
                                if let Some(live) =
                                    bound.ok().filter(|_| coherent && *count < records.len())
                                {
                                    records[*count] = BoundedPeer {
                                        peer,
                                        config: before,
                                        slot: bar as usize,
                                        live,
                                        firmware: (base, len),
                                    };
                                    *count += 1;
                                    for (i, c) in cs.entries[..cs.count].iter().enumerate() {
                                        let own = peer
                                            == [
                                                0,
                                                c.bus as usize,
                                                c.device as usize,
                                                c.function as usize,
                                            ];
                                        let target = (c.base as u64, c.bytes as u64);
                                        if own
                                            || !musha_platform::pci_resources::disjoint(
                                                live, target,
                                            )
                                            || !musha_platform::pci_resources::disjoint(
                                                (base, len),
                                                target,
                                            )
                                        {
                                            safe[i].reject(
                                                "PEER BOUND CONFLICT",
                                                peer,
                                                bar as usize,
                                                [base, len],
                                            );
                                        }
                                    }
                                } else {
                                    for evidence in &mut safe {
                                        evidence.reject(
                                            "BAR BASE MISMATCH",
                                            peer,
                                            bar as usize,
                                            [base, len],
                                        );
                                    }
                                }
                            }
                            resource_range = [base, len];
                            if let Some(end) = base.checked_add(len).filter(|_| len > 0) {
                                for (i, c) in cs.entries[..cs.count].iter().enumerate() {
                                    let own = seg == 0
                                        && bus == c.bus as usize
                                        && dev == c.device as usize
                                        && fun == c.function as usize
                                        && bar == 0;
                                    if !own
                                        && base < (c.base + c.bytes) as u64
                                        && (c.base as u64) < end
                                    {
                                        safe[i].reject(
                                            "BAR OVERLAP",
                                            peer,
                                            bar as usize,
                                            resource_range,
                                        );
                                    }
                                }
                            } else {
                                for evidence in &mut safe {
                                    evidence.reject(
                                        "RESOURCE RANGE",
                                        peer,
                                        status_code,
                                        resource_range,
                                    );
                                }
                            }
                        }
                        ((*bs).free_pool)(resource);
                    }
                    let after = inspect_config(p);
                    let layout = musha_platform::bar_layout(
                        (before.values[3] >> 16) as u8,
                        before.values[4..10].try_into().unwrap(),
                    );
                    let consistency = if !before.stable(after) {
                        Some("CONFIG UNSTABLE")
                    } else if before.values[4 + bar as usize] != raw {
                        Some("BAR READ MISMATCH")
                    } else if layout[bar as usize] == "UPPER64"
                        || layout[bar as usize] == "INVALID64"
                        || layout[bar as usize] == "RESERVED"
                    {
                        Some("BAR LAYOUT")
                    } else {
                        None
                    };
                    if let Some(reason) = consistency {
                        for evidence in &mut safe {
                            evidence.reject(reason, peer, bar as usize, [0; 2]);
                        }
                    }
                    if safe
                        .iter()
                        .enumerate()
                        .any(|(i, e)| was_empty[i] && !e.reason.is_empty())
                    {
                        // Exactly one bounded retry for evidence, never a retry-until-success gate.
                        let repeat = inspect_config(p);
                        let cf8_valid = seg == 0 && bus <= 255 && dev <= 31 && fun <= 7;
                        let mut cf8 = [0; 10];
                        if cf8_valid {
                            for (i, offset) in INSPECT_OFFSETS.iter().enumerate() {
                                cf8[i] = read(bus as u8, dev as u8, fun as u8, *offset);
                            }
                        }
                        for (i, evidence) in safe.iter_mut().enumerate() {
                            if was_empty[i] && !evidence.reason.is_empty() {
                                evidence.descriptor = descriptor;
                                (*INSPECTIONS.0.get())[i] = BarInspection {
                                    present: true,
                                    before,
                                    after,
                                    repeat,
                                    cf8,
                                    cf8_valid,
                                    attribute_status: status_code,
                                    attribute_null,
                                    location_status: location_status.as_usize(),
                                    bar_raw: raw,
                                };
                            }
                        }
                    }
                }
                let (next_slot, next_resource) =
                    musha_platform::next_bar_indices(bar as usize, resource_index, raw);
                bar = next_slot as u32;
                resource_index = next_resource;
            }
        }
        ((*bs).free_pool)(handles.cast());
    }
    safe
}
fn state(c: Controller) -> musha_platform::pci_recovery::State {
    Snapshot {
        values: unsafe {
            [
                read(c.bus, c.device, c.function, 0),
                read(c.bus, c.device, c.function, 4),
                read(c.bus, c.device, c.function, 0x10),
                read(c.bus, c.device, c.function, 0x14),
                pm_state(c),
            ]
        },
    }
    .state()
}
impl Snapshot {
    fn state(self) -> musha_platform::pci_recovery::State {
        let v = self.values;
        musha_platform::pci_recovery::State {
            id: v[0],
            command: v[1] as u16,
            bar0: v[2],
            bar1: v[3],
            pm: v[4],
        }
    }
}
unsafe fn write32(c: Controller, reg: u8, value: u32) {
    let address = 0x80000000u32
        | ((c.bus as u32) << 16)
        | ((c.device as u32) << 11)
        | ((c.function as u32) << 8)
        | reg as u32;
    unsafe {
        asm!("out dx, eax",in("dx") 0xcf8u16,in("eax") address,options(nomem,nostack));
        asm!("out dx, eax",in("dx") 0xcfcu16,in("eax") value,options(nomem,nostack));
    }
}
unsafe fn command16(c: Controller, value: u16) {
    let address = 0x80000004u32
        | ((c.bus as u32) << 16)
        | ((c.device as u32) << 11)
        | ((c.function as u32) << 8);
    unsafe {
        asm!("out dx, eax",in("dx") 0xcf8u16,in("eax") address,options(nomem,nostack));
        asm!("out dx, ax",in("dx") 0xcfcu16,in("ax") value,options(nomem,nostack));
    }
}
// Read-only bridge walk; never repair or allocate bridge resources.
fn bridge_route(c: Controller) -> Result<(), &'static str> {
    let mut bus = c.bus;
    let mut seen = [false; 256];
    for _ in 0..8 {
        if bus == 0 {
            return Ok(());
        }
        if seen[bus as usize] {
            return Err("BRIDGE LOOP");
        }
        seen[bus as usize] = true;
        let mut parent = None;
        for b in 0..=255u8 {
            for d in 0..32u8 {
                let id = unsafe { read(b, d, 0, 0) };
                if id as u16 == 0xffff {
                    continue;
                }
                let functions = if unsafe { read(b, d, 0, 0x0c) } & 0x00800000 != 0 {
                    8
                } else {
                    1
                };
                for f in 0..functions {
                    let rd = |r| unsafe { read(b, d, f, r) };
                    if rd(0) as u16 == 0xffff
                        || rd(8) >> 16 != 0x0604
                        || (rd(0x0c) >> 16) & 0x7f != 1
                    {
                        continue;
                    }
                    let buses = rd(0x18);
                    if (buses >> 8) as u8 != bus {
                        continue;
                    }
                    if parent.is_some() {
                        return Err("BRIDGE AMBIGUOUS");
                    }
                    crate::diagnostics::observation(format_args!(
                        "BRIDGE {:02X}:{:02X}.{} CMD {:04X} BUS {:08X} MEM {:08X}",
                        b,
                        d,
                        f,
                        rd(4) & 0xffff,
                        buses,
                        rd(0x20)
                    ));
                    if buses as u8 != b
                        || b == bus
                        || ((buses >> 16) as u8) < c.bus
                        || rd(4) & 2 == 0
                    {
                        return Err("BRIDGE DISABLED");
                    }
                    let mem = rd(0x20);
                    if !musha_platform::pci_recovery::bridge_window(mem, c.base, c.bytes) {
                        return Err("BRIDGE WINDOW");
                    }
                    parent = Some(b);
                }
            }
        }
        bus = parent.ok_or("BRIDGE MISSING")?;
    }
    Err("BRIDGE DEPTH")
}
pub(crate) fn recover(
    info: &crate::BootInfo,
    c: Controller,
    index: usize,
) -> Result<(), &'static str> {
    use musha_platform::pci_recovery::{Decision, decide};
    let now = state(c);
    let plan = decide(
        info.pci_early[index].state(),
        info.pci_pre_exit[index].state(),
        info.pci_post_exit[index].state(),
        now,
        c.base,
        c.bytes,
        info.boot_owner == c.bdf(),
    );
    match plan {
        Decision::Unchanged => return Ok(()),
        Decision::Reject(reason) => {
            crate::diagnostics::observation(format_args!("PCI RECOVERY SKIP {}", reason));
            return Err("MEMORY DISABLED");
        }
        Decision::Restore => {}
    }
    crate::diagnostics::usb_stage("PCI RECOVERY");
    report_bounded_peers();
    let check = (|| {
        let evidence = info.pci_resource_safe[index];
        if !evidence.reason.is_empty() {
            crate::diagnostics::observation(format_args!("RESOURCE REJECT {}", evidence.reason));
            crate::diagnostics::observation(format_args!(
                "PEER SEG {:X} BUS {:X} DEV {:X} FN {:X}",
                evidence.peer[0], evidence.peer[1], evidence.peer[2], evidence.peer[3]
            ));
            crate::diagnostics::observation(format_args!(
                "DETAIL {:X} BASE {:016X}",
                evidence.detail, evidence.range[0]
            ));
            crate::diagnostics::observation(format_args!(
                "LENGTH {:X} TARGET {:016X}",
                evidence.range[1], c.base
            ));
            crate::diagnostics::observation(format_args!("TARGET LENGTH {:X}", c.bytes));
            if evidence.descriptor != [0; 6] {
                for (i, word) in evidence.descriptor.iter().enumerate() {
                    crate::diagnostics::observation(format_args!("DESC RAW {} {:016X}", i, word));
                }
            }
            let trace = unsafe { (*INSPECTIONS.0.get())[index] };
            if trace.present {
                crate::diagnostics::observation(format_args!(
                    "INSPECT BAR {} RAW {:08X}",
                    evidence.detail, trace.bar_raw
                ));
                crate::diagnostics::observation(format_args!(
                    "LOCATION STATUS {:X} ATTR {:X} NULL {}",
                    trace.location_status, trace.attribute_status, trace.attribute_null as u8
                ));
                crate::diagnostics::observation(format_args!("ORDER BEFORE ATTR AFTER REPEAT CF8"));
                crate::diagnostics::observation(format_args!(
                    "PCI READ WIDTH 2 COUNT 1; STATUS PER WORD"
                ));
                for (label, snapshot) in [
                    ("BEFORE", trace.before),
                    ("AFTER", trace.after),
                    ("REPEAT", trace.repeat),
                ] {
                    for (i, offset) in INSPECT_OFFSETS.iter().enumerate() {
                        crate::diagnostics::observation(format_args!(
                            "{} OFF {:02X} {:08X} ST {:X}",
                            label, offset, snapshot.values[i], snapshot.statuses[i]
                        ));
                    }
                }
                crate::diagnostics::observation(format_args!(
                    "STABLE BA {} AR {} CF8 AVAILABLE {}",
                    trace.before.stable(trace.after) as u8,
                    trace.after.stable(trace.repeat) as u8,
                    trace.cf8_valid as u8
                ));
                if trace.cf8_valid {
                    for (i, offset) in INSPECT_OFFSETS.iter().enumerate() {
                        crate::diagnostics::observation(format_args!(
                            "CF8 OFF {:02X} {:08X} MATCH {}",
                            offset,
                            trace.cf8[i],
                            (trace.cf8[i] == trace.repeat.values[i]
                                && trace.repeat.statuses[i] == 0) as u8
                        ));
                    }
                }
                let layout = musha_platform::bar_layout(
                    (trace.before.values[3] >> 16) as u8,
                    trace.before.values[4..10].try_into().unwrap(),
                );
                for (i, kind) in layout.iter().enumerate() {
                    crate::diagnostics::observation(format_args!(
                        "BAR {} {:08X} {} PREFETCH {}",
                        i,
                        trace.before.values[i + 4],
                        kind,
                        ((trace.before.values[i + 4] >> 3) & 1)
                    ));
                }
                let mut bytes = [0u8; 48];
                for (dst, word) in bytes.chunks_exact_mut(8).zip(evidence.descriptor) {
                    dst.copy_from_slice(&word.to_le_bytes());
                }
                match musha_platform::bar_descriptor_fields(&bytes) {
                    Ok(fields) => {
                        for (name, value) in [
                            "TYPE",
                            "GENERAL",
                            "SPECIFIC",
                            "GRANULARITY",
                            "MIN",
                            "MAX",
                            "TRANSLATION",
                            "LENGTH",
                        ]
                        .iter()
                        .zip(fields)
                        {
                            crate::diagnostics::observation(format_args!(
                                "DESC {} {:016X}",
                                name, value
                            ));
                        }
                        crate::diagnostics::observation(format_args!(
                            "DESC STRUCTURE OK; POLICY {}",
                            evidence.reason
                        ));
                    }
                    Err(reason) => {
                        crate::diagnostics::observation(format_args!("DESC STRUCTURE {}", reason))
                    }
                }
            }
            return Err(if evidence.reason == "BAR OVERLAP" {
                "RESOURCE CONFLICT"
            } else {
                "RESOURCE UNKNOWN"
            });
        }
        if unsafe { read(c.bus, c.device, c.function, 8) } >> 8 != 0x0c0330
            || (unsafe { read(c.bus, c.device, c.function, 0x0c) } >> 16) & 0x7f != 0
        {
            return Err("CLASS CHANGED");
        }
        let raw = unsafe { core::slice::from_raw_parts(info.map_base as *const u8, info.map_size) };
        let map = musha_memory::MemoryMap::new(raw, info.descriptor_size, info.descriptor_version)
            .map_err(|_| "MEMORY MAP")?;
        let range = musha_memory::Range::new(c.base, c.bytes).map_err(|_| "BAR RANGE")?;
        for i in 0..map.count() {
            let region = map.region(i).map_err(|_| "MEMORY MAP")?;
            if range.overlaps(region.range)
                && (region.kind != 11 || region.attributes & musha_memory::RUNTIME_ATTRIBUTE != 0)
            {
                return Err("BAR MEMORY CONFLICT");
            }
        }
        bridge_route(c)?;
        recheck_bounded_peers(c)?;
        if state(c) != now {
            return Err("STATE RACE");
        }
        Ok(())
    })();
    if let Err(reason) = check {
        crate::diagnostics::observation(format_args!("PCI RECOVERY REJECT {}", reason));
        return Err(reason);
    }
    let bar = info.pci_pre_exit[index].state().bar0;
    unsafe {
        write32(c, 0x10, bar);
    }
    if unsafe { read(c.bus, c.device, c.function, 0x10) } != bar {
        unsafe {
            command16(c, 0);
        }
        return Err("BAR RESTORE READBACK");
    }
    unsafe {
        command16(c, 2);
    }
    let after = state(c);
    if after.command != 2
        || after.bar0 != bar
        || after.bar1 != 0
        || after.id != now.id
        || after.pm != now.pm
    {
        unsafe {
            command16(c, 0);
        }
        return Err("COMMAND RESTORE READBACK");
    }
    crate::diagnostics::observation(format_args!(
        "PCI RECOVERED {:04X} BAR {:08X} CMD 0002 DMA OFF",
        c.bdf(),
        bar
    ));
    Ok(())
}
