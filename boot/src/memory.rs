// Copyright 2026 Hirofumi Iwasaki
// SPDX-License-Identifier: Apache-2.0
use crate::BootInfo;
use core::arch::{asm, x86_64::__cpuid};
use musha_memory::{Error, Image, MemoryMap, PAGE, Range};
static mut PROBE_TABLES: Option<Tables> = None;
const NX: u64 = 1 << 63;
const ADDRESS: u64 = 0x000f_ffff_ffff_f000;
struct Tables {
    base: usize,
    used: usize,
    pages: usize,
    wb: u8,
    uc: u8,
}
impl Tables {
    unsafe fn allocate(&mut self) -> Result<usize, Error> {
        if self.used >= self.pages {
            return Err(Error::NoMemory);
        }
        let address = self.base + self.used * PAGE;
        self.used += 1;
        unsafe {
            core::ptr::write_bytes(address as *mut u8, 0, PAGE);
        }
        Ok(address)
    }
    unsafe fn map(
        &mut self,
        range: Range,
        write: bool,
        execute: bool,
        cache: u8,
    ) -> Result<(), Error> {
        crate::diagnostics::set(
            15,
            format_args!("MAPPING {:016X}-{:016X}", range.start, range.end),
        );
        let range = range.aligned()?;
        for address in (range.start..range.end).step_by(PAGE) {
            if address == 0 {
                return Err(Error::Invalid);
            }
            let mut table = self.base;
            for shift in [39, 30, 21] {
                let slot = (table as *mut u64).wrapping_add((address >> shift) & 511);
                let mut value = unsafe { slot.read() };
                if value & 1 == 0 {
                    value = unsafe { self.allocate()? } as u64 | 3;
                    unsafe {
                        slot.write(value);
                    }
                }
                if value & (1 << 7) != 0 {
                    return Err(Error::Invalid);
                }
                table = (value & ADDRESS) as usize;
            }
            let cache_bits = ((cache & 1) as u64) << 3
                | (((cache >> 1) & 1) as u64) << 4
                | (((cache >> 2) & 1) as u64) << 7;
            let leaf = address as u64
                | 1
                | if write { 2 } else { 0 }
                | if execute { 0 } else { NX }
                | cache_bits;
            unsafe {
                (table as *mut u64).add((address >> 12) & 511).write(leaf);
            }
        }
        Ok(())
    }
}
unsafe fn rdmsr(index: u32) -> u64 {
    let low: u32;
    let high: u32;
    unsafe {
        asm!("rdmsr",in("ecx") index,out("eax") low,out("edx") high,options(nomem,nostack));
    }
    (low as u64) | ((high as u64) << 32)
}
unsafe fn wrmsr(index: u32, value: u64) {
    unsafe {
        asm!("wrmsr",in("ecx") index,in("eax") value as u32,in("edx") (value>>32) as u32,options(nomem,nostack));
    }
}
// SAFETY: all pointers are permanent firmware LoaderData/LoadedImage mappings;
// Called exactly once, on BSP with interrupts disabled and self-owned IDT.
// The returned arena is the sole mutable application view of chosen RAM.
pub unsafe fn initialize<'a>(info: &'a BootInfo) -> Result<&'a mut [u8], Error> {
    unsafe {
        crate::diagnostics::set(10, format_args!("MEM CHECK CPU / PAT"));
        let leaf = __cpuid(1);
        if leaf.edx & (1 << 16) == 0
            || __cpuid(0x80000000).eax < 0x80000008
            || __cpuid(0x80000001).edx & (1 << 20) == 0
        {
            return Err(Error::Unsupported);
        }
        let physical_bits = __cpuid(0x80000008).eax & 255;
        if !(36..=52).contains(&physical_bits) {
            return Err(Error::Unsupported);
        }
        let physical_limit = 1usize << physical_bits;
        let cr4: usize;
        asm!("mov {}, cr4",out(reg) cr4,options(nomem,nostack));
        if cr4 & ((1 << 12) | (1 << 17)) != 0 {
            return Err(Error::Unsupported);
        }
        let pat = rdmsr(0x277);
        let find = |kind: u8| {
            (0u8..8)
                .find(|i| ((pat >> (*i as usize * 8)) & 255) as u8 == kind)
                .ok_or(Error::Unsupported)
        };
        // Preserve firmware PAT/MTRRs; select existing WB and UC entries.
        let wb = find(6)?;
        let uc = find(0)?;
        crate::diagnostics::set(10, format_args!("MEM CHECK BOOTINFO"));
        crate::diagnostics::set(
            11,
            format_args!(
                "MAP BYTES {} STRIDE {} VERSION {}",
                info.map_size, info.descriptor_size, info.descriptor_version
            ),
        );
        crate::diagnostics::set(
            12,
            format_args!("IMAGE {:016X} SIZE {:X}", info.image_base, info.image_bytes),
        );
        crate::diagnostics::set(
            13,
            format_args!("TABLE {:016X} SIZE {:X}", info.table_base, info.table_bytes),
        );
        if info.map_size > 128 * 1024 || info.image_base % PAGE != 0 || info.table_base % PAGE != 0
        {
            return Err(Error::Invalid);
        }
        let bytes = core::slice::from_raw_parts(info.map_base as *const u8, info.map_size);
        crate::diagnostics::set(10, format_args!("MEM CHECK UEFI MAP"));
        let map = MemoryMap::new(bytes, info.descriptor_size, info.descriptor_version).map_err(
            |error| {
                // Inspect only bounded, complete descriptors for an offending record.
                if info.descriptor_size >= 40 && info.descriptor_size <= bytes.len() {
                    for (index, record) in bytes.chunks_exact(info.descriptor_size).enumerate() {
                        if MemoryMap::new(record, info.descriptor_size, info.descriptor_version)
                            .is_err()
                        {
                            let u64at =
                                |n| u64::from_le_bytes(record[n..n + 8].try_into().unwrap());
                            crate::diagnostics::set(
                                14,
                                format_args!(
                                    "BAD MAP INDEX {} TYPE {}",
                                    index,
                                    u32::from_le_bytes(record[..4].try_into().unwrap())
                                ),
                            );
                            crate::diagnostics::set(
                                15,
                                format_args!("START {:016X} PAGES {:X}", u64at(8), u64at(24)),
                            );
                            crate::diagnostics::set(16, format_args!("ATTR {:016X}", u64at(32)));
                            break;
                        }
                    }
                }
                error
            },
        )?;
        crate::diagnostics::set(10, format_args!("MEM CHECK RESERVED RANGES"));
        let reserved = [
            Range::new(info.image_base, info.image_bytes)?.aligned()?,
            Range::new(info.stack_base, info.stack_bytes)?,
            Range::new(info.map_base, 128 * 1024)?,
            Range::new(info as *const BootInfo as usize, PAGE)?,
            Range::new(info.emergency_base, 32768)?,
            Range::new(info.table_base, info.table_bytes)?,
            Range::new(info.framebuffer.base, info.framebuffer.bytes)?.aligned()?,
            Range::new(info.dma_base, info.dma_bytes)?,
            Range::new(info.net_dma_base, info.net_dma_bytes)?,
        ];
        for (i, a) in reserved.iter().enumerate() {
            crate::diagnostics::set(
                14,
                format_args!("RESERVED {} {:016X}-{:016X}", i, a.start, a.end),
            );
            if a.end > physical_limit {
                return Err(Error::Invalid);
            }
            for b in &reserved[..i] {
                if a.overlaps(*b) {
                    return Err(Error::Overlap);
                }
            }
        }
        crate::diagnostics::set(10, format_args!("MEM CHECK DMA LOADER COVERAGE"));
        for (dma_index, dma) in reserved[7..9].iter().enumerate() {
            crate::diagnostics::set(
                15,
                format_args!("DMA OWNER {}", if dma_index == 0 { "USB" } else { "LAN" }),
            );
            crate::diagnostics::set(14, format_args!("DMA {:016X}-{:016X}", dma.start, dma.end));
            if dma.start % PAGE != 0 || dma.end % PAGE != 0 || dma.end > 1usize << 32 {
                return Err(Error::Invalid);
            }
            if let Err(error) = map.loader_coverage(*dma) {
                // Explain the exact coverage rejection without changing policy.
                let mut cursor = dma.start;
                for _ in 0..map.count() {
                    let mut found = None;
                    for i in 0..map.count() {
                        let region = map.region(i)?;
                        if region.range.start <= cursor && cursor < region.range.end {
                            found = Some((i, region));
                            break;
                        }
                    }
                    let Some((index, region)) = found else {
                        crate::diagnostics::set(
                            16,
                            format_args!("DMA REJECT GAP AT {:016X}", cursor),
                        );
                        break;
                    };
                    let rejected = region.attributes & musha_memory::RUNTIME_ATTRIBUTE;
                    if region.kind != 2 || rejected != 0 {
                        crate::diagnostics::set(
                            16,
                            format_args!(
                                "DMA REJECT MAP {} {}",
                                index,
                                if region.kind != 2 {
                                    "TYPE"
                                } else {
                                    "ATTR POLICY"
                                }
                            ),
                        );
                        crate::diagnostics::set(39, format_args!("REJECT MASK {:016X}", rejected));
                        break;
                    }
                    cursor = region.range.end.min(dma.end);
                    if cursor == dma.end {
                        break;
                    }
                }
                let mut shown = 0;
                for i in 0..map.count() {
                    let region = map.region(i)?;
                    if !region.range.overlaps(*dma) {
                        continue;
                    }
                    if shown < 5 {
                        let row = 18 + shown * 4;
                        crate::diagnostics::set(
                            row,
                            format_args!(
                                "MAP {} TYPE {} ATTR {:016X}",
                                i, region.kind, region.attributes
                            ),
                        );
                        crate::diagnostics::set(
                            row + 1,
                            format_args!("START {:016X}", region.range.start),
                        );
                        crate::diagnostics::set(
                            row + 2,
                            format_args!("END   {:016X}", region.range.end),
                        );
                        crate::diagnostics::set(
                            row + 3,
                            format_args!(
                                "FLAGS RT {} RP {} RO {} XP {} WB {}",
                                (region.attributes >> 63) & 1,
                                (region.attributes >> 13) & 1,
                                (region.attributes >> 17) & 1,
                                (region.attributes >> 14) & 1,
                                (region.attributes >> 3) & 1
                            ),
                        );
                    }
                    shown += 1;
                }
                crate::diagnostics::set(
                    40,
                    format_args!("DMA OVERLAP RECORDS {} FIRST 5 SHOWN", shown),
                );
                crate::diagnostics::set(41, format_args!("REQUIRE TYPE 2 / NO GAP / NO RUNTIME"));
                return Err(error);
            }
        }
        crate::diagnostics::set(10, format_args!("MEM CHECK MMIO"));
        let mut mmio = [None; crate::pci::MAX_CONTROLLERS + 3];
        for (index, controller) in info.xhcis.entries[..info.xhcis.count]
            .iter()
            .chain(core::iter::once(&info.nic))
            .chain(core::iter::once(&info.nic_flash))
            .chain(core::iter::once(&info.realtek_diag.controller))
            .enumerate()
        {
            if controller.bytes == 0 {
                continue;
            }
            crate::diagnostics::set(
                14,
                format_args!(
                    "MMIO {} BASE {:016X} SIZE {:X}",
                    index, controller.base, controller.bytes
                ),
            );
            let optional = index == info.xhcis.count + 2;
            let range = Range::new(controller.base, controller.bytes)?;
            if range.start % PAGE != 0
                || range.end % PAGE != 0
                || range.end > physical_limit
                || reserved.iter().any(|r| r.overlaps(range))
                || mmio.iter().flatten().any(|r: &Range| r.overlaps(range))
            {
                if optional {
                    continue;
                }
                return Err(Error::Overlap);
            }
            let mut valid = true;
            for i in 0..map.count() {
                let region = map.region(i)?;
                if range.overlaps(region.range) && !matches!(region.kind, 0 | 11 | 12) {
                    if optional {
                        valid = false;
                        break;
                    }
                    return Err(Error::Overlap);
                }
            }
            if !valid {
                continue;
            }
            mmio[index] = Some(range);
        }
        crate::diagnostics::set(10, format_args!("MEM CHECK ARENA"));
        let arena = map.arena(&reserved, 16 * 1024 * 1024, 64 * 1024 * 1024)?;
        // One-shot user execution needs vacant low virtual addresses. Keep its
        // backing arena outside the candidate image, record and stack windows.
        let arena = {
            let start = arena.start.max(0x0200_0000);
            let end = arena.end.min(0x1000_0000);
            if end <= start || end - start < 1024 * 1024 {
                return Err(Error::NoMemory);
            }
            Range::new(start, end - start)?
        };
        #[cfg(external_app)]
        {
            let staging = crate::external::staging_range()?;
            if staging.start < reserved[0].start
                || staging.end > reserved[0].end
                || staging.overlaps(arena)
                || reserved[1..].iter().any(|r| staging.overlaps(*r))
            {
                return Err(Error::Overlap);
            }
            #[cfg(feature = "qemu-debug")]
            for (name, range) in [
                (b"IMAGE" as &[u8], reserved[0]),
                (b"STAGING", staging),
                (b"ARENA", arena),
                (b"USB_DMA", reserved[7]),
                (b"NET_DMA", reserved[8]),
            ] {
                crate::debug(b"MUSHA: EXTERNAL_MEMORY ");
                crate::debug(name);
                crate::debug(b" ");
                crate::debug(&crate::cpu::hex(range.start as u64));
                crate::debug(b" ");
                crate::debug(&crate::cpu::hex(range.end as u64));
                crate::debug(b"\n");
            }
        }
        if arena.end > physical_limit {
            return Err(Error::Invalid);
        }
        let image_bytes =
            core::slice::from_raw_parts(info.image_base as *const u8, info.image_bytes);
        crate::diagnostics::set(10, format_args!("MEM CHECK PE IMAGE"));
        let image = Image::new(image_bytes)?;
        let mut tables = Tables {
            base: info.table_base,
            used: 0,
            pages: info.table_bytes / PAGE,
            wb,
            uc,
        };
        crate::diagnostics::set(10, format_args!("MEM CHECK PAGE TABLES"));
        tables.allocate()?;
        tables.map(reserved[0], false, false, tables.wb)?;
        for i in 0..image.count() {
            let section = image.section(i)?;
            if section.bytes > 0 {
                tables.map(
                    Range::new(info.image_base + section.offset, section.bytes)?,
                    section.writable,
                    section.executable,
                    tables.wb,
                )?;
            }
        }
        // The first page of the ordinary stack is deliberately left unmapped.
        tables.map(
            Range::new(info.stack_base + PAGE, info.stack_bytes - PAGE)?,
            true,
            false,
            tables.wb,
        )?;
        for range in &reserved[2..6] {
            tables.map(*range, true, false, tables.wb)?;
        }
        tables.map(reserved[6], true, false, tables.uc)?;
        // Dedicated DMA pages use UC, including non-snooping scratchpads.
        tables.map(reserved[7], true, false, tables.uc)?;
        tables.map(reserved[8], true, false, tables.uc)?;
        tables.map(arena, true, false, tables.wb)?;
        let realtek_mapped = mmio[info.xhcis.count + 2].is_some();
        for range in mmio.into_iter().flatten() {
            let diagnostic_readonly =
                realtek_mapped && range.start == info.realtek_diag.controller.base;
            tables.map(range, !diagnostic_readonly, false, tables.uc)?;
        }
        // No global mappings survive from firmware. Preserve MTRR and PAT values.
        let efer = rdmsr(0xc0000080);
        wrmsr(0xc0000080, efer | (1 << 11));
        asm!("mov cr4, {}",in(reg) cr4 & !(1<<7),options(nostack));
        let cr0: usize;
        asm!("mov {}, cr0",out(reg) cr0,options(nomem,nostack));
        asm!("mov cr0, {}",in(reg) cr0|(1<<16),options(nostack));
        // Old firmware mappings may have cached these newly allocated pages as
        // WB. Flush dirty lines before changing DMA/GOP pages to UC, then flush
        // translations via CR3. No DMA-pool access occurs between these steps.
        asm!("mfence", "wbinvd", options(nostack));
        asm!("mov cr3, {}",in(reg) tables.base,options(nostack));
        let active: usize;
        asm!("mov {}, cr3",out(reg) active,options(nomem,nostack));
        crate::diagnostics::set(10, format_args!("MEM CHECK ACTIVE CR3"));
        if active != tables.base {
            return Err(Error::Invalid);
        }
        if realtek_mapped {
            crate::pci::realtek::mapped();
        }
        {
            core::ptr::addr_of_mut!(PROBE_TABLES).write(Some(tables));
        }
        // SAFETY: conventional RAM was validated, reserved regions subtracted, and
        // mapped RW/NX. Only this slice is created; allocator retains no Rust borrow.
        Ok(core::slice::from_raw_parts_mut(
            arena.start as *mut u8,
            arena.len(),
        ))
    }
}

/// One-shot BSP probe only. New user mappings never replace a present leaf.
/// Caller owns the physical page, supplies a low canonical virtual page and
/// retains backing until execution has ended. C3 restores its transaction
/// before wiping/reusing backing; C1/C2 retain it until the probe stops.
pub unsafe fn probe_user_page(
    va: usize,
    pa: usize,
    writable: bool,
    executable: bool,
) -> Result<(), Error> {
    if va == 0 || va >= 1usize << 47 || va % PAGE != 0 || pa % PAGE != 0 || writable && executable {
        return Err(Error::Invalid);
    }
    unsafe {
        let tables = (&mut *core::ptr::addr_of_mut!(PROBE_TABLES))
            .as_mut()
            .ok_or(Error::Invalid)?;
        let mut table = tables.base;
        for shift in [39, 30, 21] {
            let slot = (table as *mut u64).add((va >> shift) & 511);
            let mut value = slot.read();
            if value & 1 == 0 {
                value = tables.allocate()? as u64 | 3;
            }
            if value & (1 << 7) != 0 {
                return Err(Error::Invalid);
            }
            // Old kernel leaves retain U/S=0 even when a shared ancestor is U/S=1.
            slot.write(value | 4);
            table = (value & ADDRESS) as usize;
        }
        let slot = (table as *mut u64).add((va >> 12) & 511);
        if slot.read() & 1 != 0 {
            return Err(Error::Overlap);
        }
        let cache = tables.wb as u64;
        slot.write(
            pa as u64
                | 5
                | if writable { 2 } else { 0 }
                | if executable { 0 } else { NX }
                | ((cache & 1) << 3)
                | ((cache & 2) << 3)
                | ((cache & 4) << 5),
        );
        asm!("invlpg [{}]",in(reg) va,options(nostack));
        Ok(())
    }
}
/// Remove the probe's supervisor stack guard from the existing identity map.
pub unsafe fn probe_guard(va: usize) -> Result<(), Error> {
    if va % PAGE != 0 {
        return Err(Error::Invalid);
    }
    unsafe {
        let tables = (&*core::ptr::addr_of!(PROBE_TABLES))
            .as_ref()
            .ok_or(Error::Invalid)?;
        let mut table = tables.base;
        for shift in [39, 30, 21] {
            let v = (table as *const u64).add((va >> shift) & 511).read();
            if v & 1 == 0 || v & 128 != 0 {
                return Err(Error::Invalid);
            }
            table = (v & ADDRESS) as usize;
        }
        let slot = (table as *mut u64).add((va >> 12) & 511);
        if slot.read() & ADDRESS != va as u64 || slot.read() & 4 != 0 {
            return Err(Error::Invalid);
        }
        slot.write(0);
        asm!("invlpg [{}]",in(reg) va,options(nostack));
        Ok(())
    }
}

/// Read-only collision check for the one-shot loader, including guard pages.
pub unsafe fn probe_vacant(va: usize, bytes: usize) -> Result<(), Error> {
    if va == 0 || va % PAGE != 0 || bytes == 0 || bytes % PAGE != 0 {
        return Err(Error::Invalid);
    }
    let end = va
        .checked_add(bytes)
        .filter(|e| *e <= 1usize << 47)
        .ok_or(Error::Invalid)?;
    let tables = unsafe {
        (&*core::ptr::addr_of!(PROBE_TABLES))
            .as_ref()
            .ok_or(Error::Invalid)?
    };
    for address in (va..end).step_by(PAGE) {
        let mut table = tables.base;
        for shift in [39, 30, 21, 12] {
            let value = unsafe { (table as *const u64).add((address >> shift) & 511).read() };
            if value & 1 == 0 {
                break;
            }
            if shift == 12 || value & 128 != 0 {
                return Err(Error::Overlap);
            }
            table = (value & ADDRESS) as usize;
        }
    }
    Ok(())
}

// C3 BSP-only transaction: snapshot the bounded table pool before any mapping
// changes. No concurrent page-table writers or global mappings are permitted.
#[cfg(any(feature = "ring3-lifecycle-probe", external_app))]
#[repr(C, align(4096))]
pub struct ProbeSnapshot {
    bytes: [u8; 256 * PAGE],
    used: usize,
    pages: usize,
}
#[cfg(any(feature = "ring3-lifecycle-probe", external_app))]
impl ProbeSnapshot {
    pub const fn new() -> Self {
        Self {
            bytes: [0; 256 * PAGE],
            used: 0,
            pages: 0,
        }
    }
    pub unsafe fn save(&mut self) -> Result<(), Error> {
        let t = unsafe {
            (&*core::ptr::addr_of!(PROBE_TABLES))
                .as_ref()
                .ok_or(Error::Invalid)?
        };
        if t.pages > 256 || t.used > t.pages {
            return Err(Error::Invalid);
        }
        self.used = t.used;
        self.pages = t.pages;
        unsafe {
            core::ptr::copy_nonoverlapping(
                t.base as *const u8,
                self.bytes.as_mut_ptr(),
                t.used * PAGE,
            );
        }
        Ok(())
    }
    pub unsafe fn restore(&self) -> Result<(), Error> {
        let t = unsafe {
            (&mut *core::ptr::addr_of_mut!(PROBE_TABLES))
                .as_mut()
                .ok_or(Error::Invalid)?
        };
        if t.pages != self.pages || t.used < self.used {
            return Err(Error::Invalid);
        }
        unsafe {
            core::ptr::copy_nonoverlapping(
                self.bytes.as_ptr(),
                t.base as *mut u8,
                self.used * PAGE,
            );
            // Clear newly allocated tables and reset the allocator watermark.
            core::ptr::write_bytes(
                (t.base + self.used * PAGE) as *mut u8,
                0,
                (t.used - self.used) * PAGE,
            );
            t.used = self.used;
            asm!("mov cr3, {}",in(reg) t.base,options(nostack));
        }
        Ok(())
    }
}
