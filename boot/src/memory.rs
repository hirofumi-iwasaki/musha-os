// Copyright 2026 Hirofumi Iwasaki
// SPDX-License-Identifier: Apache-2.0
use crate::BootInfo;
use core::arch::{asm, x86_64::__cpuid};
use musha_memory::{Error, Image, MemoryMap, PAGE, Range};
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
        if info.map_size > 128 * 1024 || info.image_base % PAGE != 0 || info.table_base % PAGE != 0
        {
            return Err(Error::Invalid);
        }
        let bytes = core::slice::from_raw_parts(info.map_base as *const u8, info.map_size);
        let map = MemoryMap::new(bytes, info.descriptor_size, info.descriptor_version)?;
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
            if a.end > physical_limit {
                return Err(Error::Invalid);
            }
            for b in &reserved[..i] {
                if a.overlaps(*b) {
                    return Err(Error::Overlap);
                }
            }
        }
        for dma in &reserved[7..9] {
            if dma.start % PAGE != 0 || dma.end % PAGE != 0 || dma.end > 1usize << 32 {
                return Err(Error::Invalid);
            }
            let mut valid = false;
            for i in 0..map.count() {
                let region = map.region(i)?;
                if region.kind == 2
                    && region.range.start <= dma.start
                    && region.range.end >= dma.end
                    && region.attributes & ((1 << 13) | (1 << 17)) == 0
                {
                    valid = true;
                }
            }
            if !valid {
                return Err(Error::Invalid);
            }
        }
        let mut mmio = [None; 2];
        for (index, controller) in [info.xhci, info.nic].iter().enumerate() {
            if controller.bytes == 0 {
                continue;
            }
            let range = Range::new(controller.base, controller.bytes)?;
            if range.start % PAGE != 0
                || range.end % PAGE != 0
                || range.end > physical_limit
                || reserved.iter().any(|r| r.overlaps(range))
                || mmio.iter().flatten().any(|r: &Range| r.overlaps(range))
            {
                return Err(Error::Overlap);
            }
            for i in 0..map.count() {
                let region = map.region(i)?;
                if range.overlaps(region.range) && !matches!(region.kind, 0 | 11 | 12) {
                    return Err(Error::Overlap);
                }
            }
            mmio[index] = Some(range);
        }
        let arena = map.arena(&reserved, 16 * 1024 * 1024, 64 * 1024 * 1024)?;
        if arena.end > physical_limit {
            return Err(Error::Invalid);
        }
        let image_bytes =
            core::slice::from_raw_parts(info.image_base as *const u8, info.image_bytes);
        let image = Image::new(image_bytes)?;
        let mut tables = Tables {
            base: info.table_base,
            used: 0,
            pages: info.table_bytes / PAGE,
            wb,
            uc,
        };
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
        for range in mmio.into_iter().flatten() {
            tables.map(range, true, false, tables.uc)?;
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
        if active != tables.base {
            return Err(Error::Invalid);
        }
        // SAFETY: conventional RAM was validated, reserved regions subtracted, and
        // mapped RW/NX. Only this slice is created; allocator retains no Rust borrow.
        Ok(core::slice::from_raw_parts_mut(
            arena.start as *mut u8,
            arena.len(),
        ))
    }
}
