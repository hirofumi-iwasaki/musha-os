// Copyright 2026 Hirofumi Iwasaki
// SPDX-License-Identifier: Apache-2.0
use super::{Host, TRBS, control, dma_word, event, publish, read_word};
use musha_xhci::storage::{self, Command, Endpoint, Storage};
struct Pipe {
    ring: usize,
    dci: u8,
    cursor: musha_xhci::Cursor,
}
impl Pipe {
    fn transfer(
        &mut self,
        host: &mut Host<'_>,
        slot: u8,
        buffer: usize,
        bytes: usize,
        hold: bool,
    ) -> Result<(), &'static str> {
        if bytes == 0 || bytes > 4096 {
            return Err("BULK SIZE");
        }
        let address = self.ring + self.cursor.index * 16;
        unsafe {
            if self.cursor.index == TRBS - 2 {
                publish(
                    self.ring + (TRBS - 1) * 16,
                    [self.ring as u32, 0, 0, (6 << 10) | 2 | self.cursor.cycle],
                );
            }
            publish(
                address,
                [
                    buffer as u32,
                    0,
                    bytes as u32,
                    (1 << 10) | (1 << 5) | self.cursor.cycle,
                ],
            );
        }
        if !hold {
            host.regs
                .write(host.doorbell + slot as usize * 4, self.dci as u32)?;
        }
        event(
            host.regs,
            host.clock,
            host.events,
            host.consumer,
            host.runtime,
            host.ac64,
            address,
            host.ports,
            slot,
            self.dci,
            if hold { 20 } else { 1000 },
        )?;
        self.cursor.advance();
        Ok(())
    }
}
struct Bot {
    slot: u8,
    input: Pipe,
    output: Pipe,
    cbw: usize,
    data: usize,
    csw: usize,
    tag: u32,
}
impl Bot {
    fn command(&mut self, host: &mut Host<'_>, command: Command) -> Result<bool, &'static str> {
        self.tag = self.tag.checked_add(1).ok_or("BOT TAG")?;
        let (cbw, bytes) = storage::cbw(self.tag, command).ok_or("SCSI COMMAND")?;
        unsafe {
            core::ptr::write_bytes(self.data as *mut u8, 0, 4096);
            core::ptr::write_bytes(self.csw as *mut u8, 0, 64);
            for (i, b) in cbw.iter().enumerate() {
                (self.cbw as *mut u8).add(i).write_volatile(*b);
            }
        }
        self.output.transfer(host, self.slot, self.cbw, 31, false)?;
        if bytes != 0 {
            let hold = cfg!(feature = "storage-timeout") && matches!(command, Command::Read { .. });
            self.input
                .transfer(host, self.slot, self.data, bytes, hold)?;
        }
        self.input.transfer(host, self.slot, self.csw, 13, false)?;
        let mut status = [0u8; 13];
        copy(self.csw, &mut status);
        storage::csw(&status, self.tag).ok_or("BOT STATUS")
    }
    fn read(
        &mut self,
        host: &mut Host<'_>,
        lba: u64,
        blocks: u64,
        size: u32,
    ) -> Result<u64, &'static str> {
        storage::read(self.tag.checked_add(1).ok_or("BOT TAG")?, lba, blocks, size)
            .ok_or("LBA RANGE")?;
        if !self.command(
            host,
            Command::Read {
                lba: lba as u32,
                sector_bytes: size,
            },
        )? {
            return Err("SCSI READ");
        }
        let mut bytes = [0u8; 4096];
        copy(self.data, &mut bytes[..size as usize]);
        Ok(storage::hash(&bytes[..size as usize]))
    }
}
fn copy(base: usize, out: &mut [u8]) {
    for (i, b) in out.iter_mut().enumerate() {
        *b = unsafe { (base as *const u8).add(i).read_volatile() };
    }
}
fn endpoint(input: usize, stride: usize, ep: Endpoint, dci: u8, ring: usize, incoming: bool) {
    unsafe {
        let base = input + (dci as usize + 1) * stride;
        dma_word(
            base + 4,
            (3 << 1)
                | ((if incoming { 6 } else { 2 }) << 3)
                | ((ep.burst as u32) << 8)
                | ((ep.packet as u32) << 16),
        );
        dma_word(base + 8, ring as u32 | 1);
        dma_word(base + 16, 512);
        publish(ring + (TRBS - 1) * 16, [ring as u32, 0, 0, (6 << 10) | 3]);
    }
}
pub(super) fn probe(
    host: &mut Host<'_>,
    pool: &mut musha_xhci::Pool,
    slot: u8,
    input: usize,
    output: usize,
    stride: usize,
    control_ring: usize,
    buffer: usize,
    disk: Storage,
    tick: &mut dyn FnMut(super::super::AppEvent<'_>) -> Result<bool, &'static str>,
) -> Result<(), &'static str> {
    crate::diagnostics::usb_stage("STORAGE BOT / CAPACITY");
    let in_ring = pool.allocate(4096, 4096).ok_or("DMA FULL")?;
    let out_ring = pool.allocate(4096, 4096).ok_or("DMA FULL")?;
    let cbw = pool.allocate(64, 64).ok_or("DMA FULL")?;
    let csw = pool.allocate(64, 64).ok_or("DMA FULL")?;
    let data = pool.allocate(4096, 4096).ok_or("DMA FULL")?;
    let indci = disk.input.number * 2 + 1;
    let outdci = disk.output.number * 2;
    unsafe {
        core::ptr::write_bytes(input as *mut u8, 0, 4096);
        dma_word(input + 4, 1 | (1 << indci) | (1 << outdci));
        for i in 0..3 {
            dma_word(input + stride + i * 4, read_word(output + i * 4));
        }
        let previous = read_word(input + stride);
        dma_word(
            input + stride,
            (previous & !(31 << 27)) | ((indci.max(outdci) as u32) << 27),
        );
        endpoint(input, stride, disk.input, indci, in_ring, true);
        endpoint(input, stride, disk.output, outdci, out_ring, false);
        core::arch::asm!("mfence", options(nostack));
    }
    host.command(input, (12 << 10) | ((slot as u32) << 24), slot)?;
    control(
        host,
        slot,
        control_ring,
        buffer,
        0,
        ((disk.configuration as u32) << 16) | 0x0900,
        0,
    )?;
    let mut bot = Bot {
        slot,
        input: Pipe {
            ring: in_ring,
            dci: indci,
            cursor: musha_xhci::Cursor::new(TRBS - 1).ok_or("CURSOR")?,
        },
        output: Pipe {
            ring: out_ring,
            dci: outdci,
            cursor: musha_xhci::Cursor::new(TRBS - 1).ok_or("CURSOR")?,
        },
        cbw,
        data,
        csw,
        tag: 0,
    };
    let mut ready = false;
    for _ in 0..3 {
        if bot.command(host, Command::Ready)? {
            ready = true;
            break;
        }
        if !bot.command(host, Command::Sense)? {
            return Err("SCSI SENSE");
        }
        let mut sense = [0u8; 18];
        copy(data, &mut sense);
        if !matches!(sense[0] & 127, 0x70 | 0x71) || !matches!(sense[2] & 15, 2 | 6) {
            return Err("SCSI NOT READY");
        }
        host.delay(100)?;
    }
    if !ready {
        return Err("SCSI NOT READY");
    }
    if !bot.command(host, Command::Capacity)? {
        return Err("SCSI CAPACITY");
    }
    let mut cap = [0u8; 8];
    copy(data, &mut cap);
    let (blocks, size) = storage::capacity(&cap).ok_or("CAPACITY RANGE")?;
    crate::diagnostics::set(
        36,
        format_args!("MEDIA SLOT {:02X} BLOCKS {:016X}", slot, blocks),
    );
    crate::diagnostics::set(37, format_args!("SECTOR BYTES {:08X}", size));
    crate::diagnostics::usb_stage("STORAGE SECTOR READ");
    crate::debug(b"MUSHA: STORAGE_CAPACITY BLOCKS=");
    crate::debug(&crate::cpu::hex(blocks));
    crate::debug(b" SECTOR=");
    crate::debug(&crate::cpu::hex(size as u64));
    crate::debug(b"\n");
    for lba in [0, blocks - 1] {
        let hash = bot.read(host, lba, blocks, size)?;
        crate::debug(b"MUSHA: STORAGE_READ_OK LBA=");
        crate::debug(&crate::cpu::hex(lba));
        crate::debug(b" HASH=");
        crate::debug(&crate::cpu::hex(hash));
        crate::debug(b"\n");
    }
    let mut file = [0u8; 4096];
    crate::diagnostics::usb_stage("FAT32 FILE READ");
    let file_result = musha_fs::read_root(
        blocks,
        size as usize,
        b"MUSHA   TXT",
        &mut file,
        &mut |lba, out| {
            bot.read(host, lba, blocks, size)
                .map_err(|_| musha_fs::Error::Io)?;
            copy(data, out);
            Ok(())
        },
    );
    match file_result {
        Ok(bytes) => {
            crate::diagnostics::set(38, format_args!("FILE MUSHA.TXT BYTES {:08X}", bytes));
            crate::diagnostics::set(
                39,
                format_args!("FILE HASH {:016X}", storage::hash(&file[..bytes])),
            );
            tick(super::super::AppEvent::File(&file[..bytes]))?;
            crate::debug(b"MUSHA: FAT32_FILE_OK BYTES=");
            crate::debug(&crate::cpu::hex(bytes as u64));
            crate::debug(b" HASH=");
            crate::debug(&crate::cpu::hex(storage::hash(&file[..bytes])));
            crate::debug(b"\n");
            if host.framebuffer.height >= 476 {
                super::super::clear_line(host.framebuffer, 452);
                unsafe {
                    host.framebuffer.text(
                        "FAT32 READ OK",
                        24,
                        452,
                        host.framebuffer.color(0, 240, 100),
                    );
                }
            }
        }
        Err(musha_fs::Error::Unsupported) => {
            crate::diagnostics::set(38, format_args!("FILE FORMAT UNSUPPORTED"));
            crate::diagnostics::set(39, format_args!("FILE HASH UNKNOWN"));
            tick(super::super::AppEvent::FileError(
                musha_api::Error::Unsupported,
            ))?;
            crate::debug(b"MUSHA: FAT32_UNSUPPORTED\n");
        }
        Err(musha_fs::Error::NotFound) => {
            crate::diagnostics::set(38, format_args!("FILE MUSHA.TXT MISSING"));
            crate::diagnostics::set(39, format_args!("FILE HASH UNKNOWN"));
            tick(super::super::AppEvent::FileError(
                musha_api::Error::NotFound,
            ))?;
            crate::debug(b"MUSHA: FAT32_FILE_MISSING\n");
        }
        Err(_) => return Err("FAT32 READ"),
    }
    if host.framebuffer.height >= 444 {
        super::super::clear_line(host.framebuffer, 388);
        super::super::clear_line(host.framebuffer, 420);
        unsafe {
            host.framebuffer
                .text("USB READ OK", 24, 388, host.framebuffer.color(0, 240, 100));
            host.framebuffer
                .text("USB BYTES", 24, 420, host.framebuffer.color(0, 220, 240));
            host.framebuffer.text(
                core::str::from_utf8(&crate::cpu::hex(blocks * size as u64)).unwrap(),
                240,
                420,
                host.framebuffer.color(0, 220, 240),
            );
        }
    }
    crate::debug(b"MUSHA: STORAGE_PROBE_OK READ_ONLY\n");
    Ok(())
}
