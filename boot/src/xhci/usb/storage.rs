// Copyright 2026 Hirofumi Iwasaki
// SPDX-License-Identifier: Apache-2.0
use super::{Host, TRBS, control, dma_word, publish, read_word};
use musha_xhci::storage::transaction::{Phase, Progress, Transaction};
use musha_xhci::storage::{self, Command, Endpoint, Storage};
struct Pipe {
    ring: usize,
    dci: u8,
    cursor: musha_xhci::Cursor,
}
impl Pipe {
    fn submit(
        &mut self,
        host: &mut Host<'_>,
        slot: u8,
        buffer: usize,
        bytes: usize,
        hold: bool,
    ) -> Result<usize, &'static str> {
        if host.bulk_pending.is_some() || host.bulk_saved.is_some() {
            return Err("BULK BUSY");
        }
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
        host.bulk_pending = Some((address, slot, self.dci));
        Ok(address)
    }
    fn poll(
        &mut self,
        host: &mut Host<'_>,
        slot: u8,
        address: usize,
    ) -> Result<bool, &'static str> {
        if host.bulk_pending != Some((address, slot, self.dci)) {
            return Err("BULK OWNER");
        }
        let result = if let Some(words) = host.bulk_saved.take() {
            if !musha_xhci::endpoint_completion(words, address, slot, self.dci) {
                return Err("BAD TRANSFER");
            }
            Ok(words)
        } else {
            super::super::event_dispatch(
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
                0,
                host.keyboard_pending,
                Some(&mut host.keyboard_saved),
                None,
                None,
                None,
                None,
            )
        };
        match result {
            Ok(_) => {
                host.bulk_pending = None;
                self.cursor.advance();
                Ok(true)
            }
            Err("EVENT PENDING") => Ok(false),
            Err(error) => Err(error),
        }
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
    pending: Option<Active>,
}
struct Active {
    transaction: Transaction,
    transfer: Option<(Phase, usize, usize)>,
    hold_data: bool,
    delayed_until: Option<u64>,
}
impl Bot {
    fn begin(&mut self, host: &mut Host<'_>, command: Command) -> Result<(), &'static str> {
        if self.pending.is_some() {
            return Err("BOT BUSY");
        }
        self.tag = self.tag.checked_add(1).ok_or("BOT TAG")?;
        let hold_data =
            cfg!(feature = "storage-timeout") && matches!(command, Command::Read { .. });
        let transaction = Transaction::new(
            self.tag,
            command,
            host.clock.now()?,
            if hold_data { 20 } else { 2000 },
        )
        .map_err(transaction_error)?;
        unsafe {
            core::ptr::write_bytes(self.data as *mut u8, 0, 4096);
            core::ptr::write_bytes(self.csw as *mut u8, 0, 64);
            for (i, b) in transaction.command().iter().enumerate() {
                (self.cbw as *mut u8).add(i).write_volatile(*b);
            }
        }
        self.pending = Some(Active {
            transaction,
            transfer: None,
            hold_data,
            delayed_until: None,
        });
        Ok(())
    }
    /// At most one submission or one event observation; never waits for USB.
    /// An error keeps buffers reserved until outer controller cleanup stops DMA.
    fn poll(&mut self, host: &mut Host<'_>) -> Result<Option<bool>, &'static str> {
        let active = self.pending.as_mut().ok_or("BOT IDLE")?;
        match active
            .transaction
            .poll(host.clock.now()?)
            .map_err(transaction_error)?
        {
            Progress::Complete(ok) => {
                self.pending = None;
                Ok(Some(ok))
            }
            Progress::Submit(phase, bytes) => {
                let (pipe, buffer) = match phase {
                    Phase::Command => (&mut self.output, self.cbw),
                    Phase::Data => (&mut self.input, self.data),
                    Phase::Status => (&mut self.input, self.csw),
                };
                match pipe.submit(
                    host,
                    self.slot,
                    buffer,
                    bytes,
                    phase == Phase::Data && (active.hold_data || active.delayed_until.is_some()),
                ) {
                    Ok(address) => {
                        active.transfer = Some((phase, address, bytes));
                        if phase == Phase::Data && active.delayed_until.is_some() {
                            crate::debug(b"MUSHA: LIVE_STORAGE_WAIT\n");
                        }
                    }
                    Err(error) => {
                        active.transaction.cancel();
                        return Err(error);
                    }
                }
                Ok(None)
            }
            Progress::Pending => {
                let (phase, address, bytes) = active.transfer.ok_or("BOT NO TRANSFER")?;
                if phase == Phase::Data {
                    if let Some(until) = active.delayed_until {
                        if !cfg!(feature = "live-storage-timeout") && host.clock.now()? >= until {
                            host.regs.write(
                                host.doorbell + self.slot as usize * 4,
                                self.input.dci as u32,
                            )?;
                            active.delayed_until = None;
                        }
                    }
                }
                let pipe = if phase == Phase::Command {
                    &mut self.output
                } else {
                    &mut self.input
                };
                match pipe.poll(host, self.slot, address) {
                    Ok(false) => {}
                    Ok(true) => {
                        let mut status = [0; 13];
                        if phase == Phase::Status {
                            copy(self.csw, &mut status);
                        }
                        active
                            .transaction
                            .complete(
                                phase,
                                bytes,
                                if phase == Phase::Status { &status } else { &[] },
                            )
                            .map_err(transaction_error)?;
                        active.transfer = None;
                    }
                    Err(error) => {
                        active.transaction.cancel();
                        return Err(error);
                    }
                }
                Ok(None)
            }
        }
    }
    // Compatibility driver for boot-time discovery. The live scheduler will call
    // begin/poll separately; this wrapper still completes discovery synchronously.
    fn command(&mut self, host: &mut Host<'_>, command: Command) -> Result<bool, &'static str> {
        self.begin(host, command)?;
        loop {
            if let Some(result) = self.poll(host)? {
                return Ok(result);
            }
            core::hint::spin_loop();
        }
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
fn transaction_error(error: musha_xhci::storage::transaction::Error) -> &'static str {
    use musha_xhci::storage::transaction::Error;
    match error {
        Error::Timeout => "TRANSFER TIMEOUT",
        Error::Stalled => "CLOCK STALLED",
        Error::Clock => "CLOCK OVERFLOW",
        Error::Status => "BOT STATUS",
        Error::Invalid => "SCSI COMMAND",
        Error::Transfer => "BAD TRANSFER",
        Error::Canceled => "BOT CANCELED",
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
) -> Result<Device, &'static str> {
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
        pending: None,
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
    let mut first_hash = 0;
    for lba in [0, blocks - 1] {
        let hash = bot.read(host, lba, blocks, size)?;
        if lba == 0 {
            first_hash = hash;
        }
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
    #[cfg(feature = "live-file-probe")]
    let prepared = {
        let bytes = *file_result.as_ref().map_err(|_| "LIVE FILE SNAPSHOT")?;
        let fs = musha_fs::fat32::FileSystem::discover(blocks, size as usize, |lba, out| {
            bot.read(host, lba, blocks, size)
                .map_err(|_| musha_fs::Error::Io)?;
            copy(data, out);
            Ok(())
        })
        .map_err(|_| "LIVE FILE MOUNT")?;
        let index = fs
            .entries()
            .iter()
            .position(|e| e.name() == b"MUSHA   TXT")
            .ok_or("LIVE FILE MISSING")?;
        if fs.entries()[index].len() != bytes as u64 {
            return Err("LIVE FILE LENGTH");
        }
        Some(super::file::Prepared {
            source: fs.source(index).map_err(|_| "LIVE FILE PLAN")?,
            bytes,
            hash: storage::hash(&file[..bytes]),
        })
    };
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
    Ok(Device {
        #[cfg(feature = "live-file-probe")]
        prepared,
        bot,
        blocks,
        size,
        port: 0,
        ancestors: [None; 5],
        first_hash,
        next_probe: 0,
        check_at: 0,
    })
}

/// Retains the configured slot and exclusively owned DMA allocations. Completion
/// copies into CPU memory only after CSW validation; failures reserve all buffers
/// until Disable Slot or outer controller quiescence.
pub(super) struct Device {
    #[cfg(feature = "live-file-probe")]
    pub(super) prepared: Option<super::file::Prepared>,
    bot: Bot,
    blocks: u64,
    size: u32,
    port: usize,
    pub(super) ancestors: [Option<(super::hub::Hub, u8)>; 5],
    first_hash: u64,
    next_probe: u64,
    check_at: u64,
}
impl Device {
    pub(super) fn with_port(mut self, port: usize) -> Self {
        self.port = port;
        self
    }
    pub(super) fn slot(&self) -> u8 {
        self.bot.slot
    }
    pub(super) fn begin_read(&mut self, host: &mut Host<'_>, lba: u64) -> Result<(), &'static str> {
        storage::read(
            self.bot.tag.checked_add(1).ok_or("BOT TAG")?,
            lba,
            self.blocks,
            self.size,
        )
        .ok_or("LBA RANGE")?;
        self.bot.begin(
            host,
            Command::Read {
                lba: lba as u32,
                sector_bytes: self.size,
            },
        )
    }
    pub(super) fn poll_read(
        &mut self,
        host: &mut Host<'_>,
        out: &mut [u8],
    ) -> Result<bool, &'static str> {
        if out.len() != self.size as usize {
            return Err("BULK SIZE");
        }
        match self.bot.poll(host)? {
            None => Ok(false),
            Some(false) => Err("SCSI READ"),
            Some(true) => {
                copy(self.bot.data, out);
                Ok(true)
            }
        }
    }
    #[cfg(feature = "live-file-probe")]
    pub(super) fn delay_read(&mut self, host: &mut Host<'_>) -> Result<(), &'static str> {
        self.bot.pending.as_mut().ok_or("BOT IDLE")?.delayed_until =
            Some(host.clock.now()?.checked_add(200).ok_or("CLOCK OVERFLOW")?);
        Ok(())
    }
    pub(super) fn stopped(&mut self, host: &mut Host<'_>) {
        self.bot.pending = None;
        host.bulk_pending = None;
        host.bulk_saved = None;
    }
    /// QEMU-only live transport exercise; not an application file API request.
    pub(super) fn probe_step(&mut self, host: &mut Host<'_>) -> Result<(), &'static str> {
        let now = host.clock.now()?;
        let op = (host.regs.read(0)? & 255) as usize;
        if host.regs.read(op + 0x400 + (self.port - 1) * 16)? & 1 == 0 {
            return Err("STORAGE DISCONNECTED");
        }
        if now >= self.check_at {
            for (hub, port) in self.ancestors.iter().flatten().copied() {
                let status = super::hub::status(host, hub, port)?;
                if status.overcurrent()
                    || !status.connected()
                    || status.ready_speed().is_none()
                    || status.connection_changed()
                    || status.configuration_error()
                {
                    return Err("STORAGE DISCONNECTED");
                }
            }
            self.check_at = now.checked_add(100).ok_or("CLOCK OVERFLOW")?;
        }
        if self.bot.pending.is_none() {
            if now < self.next_probe {
                return Ok(());
            }
            self.begin_read(host, 0)?;
            self.bot.pending.as_mut().ok_or("BOT IDLE")?.delayed_until =
                Some(now.checked_add(750).ok_or("CLOCK OVERFLOW")?);
            crate::debug(b"MUSHA: LIVE_STORAGE_BEGIN\n");
        } else {
            let mut out = [0u8; 4096];
            if self.poll_read(host, &mut out[..self.size as usize])? {
                if storage::hash(&out[..self.size as usize]) != self.first_hash {
                    return Err("LIVE STORAGE CONTENT");
                }
                crate::debug(b"MUSHA: LIVE_STORAGE_READ_OK\n");
                self.next_probe = now.checked_add(50).ok_or("CLOCK OVERFLOW")?;
            }
        }
        Ok(())
    }
}
