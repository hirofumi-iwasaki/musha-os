// Copyright 2026 Hirofumi Iwasaki
// SPDX-License-Identifier: Apache-2.0
use super::{Host, TRBS, control, dma_word, publish, read_word};
use musha_framebuffer::metrics as grid;
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
        super::super::rescan_budget(host.clock)?;
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
                host.mouse_pending,
                Some(&mut host.mouse_saved),
                None,
                None,
                    Some(&mut host.bt_mailbox),
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
    data_out: bool,
    delayed_until: Option<u64>,
}
impl Bot {
    fn begin(&mut self, host: &mut Host<'_>, command: Command) -> Result<(), &'static str> {
        if self.pending.is_some() {
            return Err("BOT BUSY");
        }
        self.tag = self.tag.checked_add(1).ok_or("BOT TAG")?;
        let hold_data = (cfg!(feature = "storage-timeout")
            && matches!(command, Command::Read { .. }))
            || ((cfg!(feature = "settings-write-timeout")
                || cfg!(feature = "settings-write-cancel"))
                && command.data_out());
        let transaction = Transaction::new(
            self.tag,
            command,
            host.clock.now()?,
            if hold_data && !command.data_out() {
                20
            } else {
                2000
            },
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
            data_out: command.data_out(),
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
                    Phase::Data if active.data_out => (&mut self.output, self.data),
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
                        if cfg!(feature = "settings-write-probe")
                            && phase == Phase::Data
                            && active.data_out
                        {
                            crate::debug(b"MUSHA: SETTINGS_WRITE_DATA_PENDING\n");
                            if cfg!(feature = "settings-write-cancel") {
                                active.transaction.cancel();
                                crate::debug(b"MUSHA: SETTINGS_WRITE_CANCELED\n");
                            }
                        }
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
                                if active.data_out {
                                    self.output.dci
                                } else {
                                    self.input.dci
                                } as u32,
                            )?;
                            active.delayed_until = None;
                        }
                    }
                }
                let pipe = if phase == Phase::Command || (phase == Phase::Data && active.data_out) {
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
    crate::diagnostics::usb_stage(if cfg!(feature = "live-exfat-probe") {
        "EXFAT FILE READ"
    } else {
        "FAT32 FILE READ"
    });
    let mounted = if !host.discovery {
        super::file::prepare(
            blocks,
            size as usize,
            |lba, out| {
                bot.read(host, lba, blocks, size)
                    .map_err(|_| musha_fs::Error::Io)?;
                copy(data, out);
                Ok(())
            },
            &mut file,
        )
    } else {
        let supported = super::file::supported(blocks, size as usize, |lba, out| {
            bot.read(host, lba, blocks, size)
                .map_err(|_| musha_fs::Error::Io)?;
            copy(data, out);
            Ok(())
        });
        match supported {
            Ok(true) => {
                host.readable_storage += 1;
                Ok(None)
            }
            Ok(false) => Ok(None),
            Err(e) => Err(e),
        }
    };
    #[cfg(external_app)]
    if let Err(musha_fs::Error::DuplicateName(name)) = &mounted {
        if !host.discovery {
            crate::external::duplicate_catalog(name, host.framebuffer);
        }
    }
    let skip_snapshot = matches!(mounted, Ok(None));
    let (prepared, file_result) = match mounted {
        Ok(Some((p, bytes))) => (Some(p), bytes.ok_or(musha_fs::Error::NotFound)),
        Ok(None) => (None, Err(musha_fs::Error::Unsupported)),
        Err(e) => (None, Err(e)),
    };
    let exfat = prepared.as_ref().is_some_and(|p| p.is_exfat());
    let format_name = if exfat {
        "exFAT READ OK"
    } else {
        "FAT32 READ OK"
    };
    if !skip_snapshot {
        match file_result {
            Ok(bytes) => {
                if bytes != crate::marker::BYTES
                    || storage::hash(&file[..bytes]) != crate::marker::HASH
                {
                    crate::diagnostics::failure("STARTUP MARKER", "README MISMATCH");
                    return Err("README MARKER");
                }
                crate::diagnostics::set(38, format_args!("FILE README.TXT BYTES {:08X}", bytes));
                crate::diagnostics::set(
                    39,
                    format_args!("FILE HASH {:016X}", storage::hash(&file[..bytes])),
                );
                crate::bootlog::media_ready(exfat);
                tick(super::super::AppEvent::File(&file[..bytes]))?;
                if exfat {
                    crate::debug(b"MUSHA: EXFAT_FILE_OK BYTES=");
                } else {
                    crate::debug(b"MUSHA: FAT32_FILE_OK BYTES=");
                }
                crate::debug(&crate::cpu::hex(bytes as u64));
                crate::debug(b" HASH=");
                crate::debug(&crate::cpu::hex(storage::hash(&file[..bytes])));
                crate::debug(b"\n");
                if grid::Metrics::new(host.framebuffer.width, host.framebuffer.height).rows
                    > grid::FILESYSTEM + grid::MARGIN
                {
                    super::super::clear_line(host.framebuffer, grid::FILESYSTEM);
                    unsafe {
                        host.framebuffer.text_grid(
                            format_name,
                            grid::LABEL,
                            grid::FILESYSTEM,
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
                crate::diagnostics::set(38, format_args!("FILE README.TXT MISSING"));
                crate::diagnostics::set(39, format_args!("FILE HASH UNKNOWN"));
                tick(super::super::AppEvent::FileError(
                    musha_api::Error::NotFound,
                ))?;
                crate::debug(b"MUSHA: FAT32_FILE_MISSING\n");
            }
            Err(_) => return Err("FAT32 READ"),
        }
    }
    if grid::Metrics::new(host.framebuffer.width, host.framebuffer.height).rows
        > grid::USB_BYTES + grid::MARGIN
    {
        super::super::clear_line(host.framebuffer, grid::USB_READ);
        super::super::clear_line(host.framebuffer, grid::USB_BYTES);
        unsafe {
            host.framebuffer.text_grid(
                "USB READ OK",
                grid::LABEL,
                grid::USB_READ,
                host.framebuffer.color(0, 240, 100),
            );
            host.framebuffer.text_grid(
                "USB BYTES",
                grid::LABEL,
                grid::USB_BYTES,
                host.framebuffer.color(0, 220, 240),
            );
            host.framebuffer.text_grid(
                core::str::from_utf8(&crate::cpu::hex(blocks * size as u64)).unwrap(),
                grid::VALUE,
                grid::USB_BYTES,
                host.framebuffer.color(0, 220, 240),
            );
        }
    }
    #[cfg(feature = "settings-write-probe")]
    if !host.discovery {
        settings_probe(host, &mut bot, blocks, size)?;
    }
    crate::debug(b"MUSHA: STORAGE_PROBE_OK READ_ONLY\n");
    Ok(Device {
        prepared,
        bot,
        blocks,
        size,
        port: 0,
        ancestors: [None; 5],
        first_hash,
        next_probe: 0,
        check_at: 0,
        write_protected: false,
        write_checked: false,
        write_sense: false,
        pairing_checked: false,
    })
}

/// Retains the configured slot and exclusively owned DMA allocations. Completion
/// copies into CPU memory only after CSW validation; failures reserve all buffers
/// until Disable Slot or outer controller quiescence.
pub(super) struct Device {
    pub(super) prepared: Option<super::file::Prepared>,
    bot: Bot,
    blocks: u64,
    size: u32,
    port: usize,
    pub(super) ancestors: [Option<(super::hub::Hub, u8)>; 5],
    first_hash: u64,
    next_probe: u64,
    check_at: u64,
    write_protected: bool,
    write_checked: bool,
    write_sense: bool,
    pairing_checked: bool,
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
    /// Internal block transport only: callers must first authorize SETTINGS sectors.
    /// Failure reserves DMA buffers exactly like a read until controller shutdown.
    pub(super) fn begin_write(
        &mut self,
        host: &mut Host<'_>,
        lba: u64,
        bytes: &[u8],
    ) -> Result<(), &'static str> {
        if lba >= self.blocks || lba > u32::MAX as u64 || bytes.len() != self.size as usize {
            return Err("LBA RANGE");
        }
        if !self.write_checked {
            if !self.bot.command(host, Command::ModeSense)? {
                return Err("SCSI MODE SENSE");
            }
            let mut header = [0; 4];
            copy(self.bot.data, &mut header);
            self.write_protected =
                storage::mode_write_protected(&header).ok_or("SCSI MODE HEADER")?;
            self.write_checked = true;
        }
        if self.write_protected {
            return Err("MEDIA READ ONLY");
        }
        self.check_connected(host)?;
        self.bot.begin(
            host,
            Command::Write {
                lba: lba as u32,
                sector_bytes: self.size,
            },
        )?;
        // begin clears the DMA buffer before the OUT payload is published.
        for (i, byte) in bytes.iter().enumerate() {
            unsafe {
                (self.bot.data as *mut u8).add(i).write_volatile(*byte);
            }
        }
        Ok(())
    }
    pub(super) fn begin_sync(&mut self, host: &mut Host<'_>) -> Result<(), &'static str> {
        self.check_connected(host)?;
        if self.write_protected {
            return Err("MEDIA READ ONLY");
        }
        self.bot.begin(host, Command::SynchronizeCache)
    }
    pub(super) fn poll_write(&mut self, host: &mut Host<'_>) -> Result<Option<bool>, &'static str> {
        match self.bot.poll(host)? {
            None => Ok(None),
            Some(ok) if self.write_sense => {
                self.write_sense = false;
                if !ok {
                    return Err("SCSI SENSE");
                }
                let mut bytes = [0; 18];
                copy(self.bot.data, &mut bytes);
                let sense = storage::sense(&bytes).ok_or("SCSI SENSE FORMAT")?;
                if sense.write_protected() {
                    self.write_protected = true;
                    Err("MEDIA READ ONLY")
                } else {
                    Err("SCSI WRITE FAILED")
                }
            }
            Some(false) => {
                self.bot.begin(host, Command::Sense)?;
                self.write_sense = true;
                Ok(None)
            }
            Some(true) => Ok(Some(true)),
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
        self.write_sense = false;
        host.bulk_pending = None;
        host.bulk_saved = None;
    }
    pub(super) fn check_connected(&mut self, host: &mut Host<'_>) -> Result<(), &'static str> {
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
        Ok(())
    }
    /// QEMU-only live transport exercise; not an application file API request.
    pub(super) fn probe_step(&mut self, host: &mut Host<'_>) -> Result<(), &'static str> {
        self.check_connected(host)?;
        let now = host.clock.now()?;
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

#[cfg(feature = "settings-write-probe")]
fn settings_probe(
    host: &mut Host<'_>,
    bot: &mut Bot,
    blocks: u64,
    size: u32,
) -> Result<(), &'static str> {
    struct Image<'a, 'h> {
        host: &'a mut Host<'h>,
        bot: &'a mut Bot,
        blocks: u64,
        size: u32,
        protected: bool,
    }
    impl musha_fs::settings::Disk for Image<'_, '_> {
        fn read(&mut self, lba: u64, b: &mut [u8]) -> Result<(), musha_fs::Error> {
            self.bot
                .read(self.host, lba, self.blocks, self.size)
                .map_err(|_| musha_fs::Error::Io)?;
            copy(self.bot.data, b);
            Ok(())
        }
        fn write(&mut self, lba: u64, b: &[u8]) -> Result<(), musha_fs::Error> {
            if self.protected
                || lba >= self.blocks
                || lba > u32::MAX as u64
                || b.len() != self.size as usize
            {
                return Err(musha_fs::Error::Io);
            }
            self.bot
                .begin(
                    self.host,
                    Command::Write {
                        lba: lba as u32,
                        sector_bytes: self.size,
                    },
                )
                .map_err(|_| musha_fs::Error::Io)?;
            for (i, byte) in b.iter().enumerate() {
                unsafe {
                    (self.bot.data as *mut u8).add(i).write_volatile(*byte);
                }
            }
            loop {
                match self.bot.poll(self.host).map_err(|_| musha_fs::Error::Io)? {
                    None => core::hint::spin_loop(),
                    Some(true) => return Ok(()),
                    Some(false) => break,
                }
            }
            if self
                .bot
                .command(self.host, Command::Sense)
                .map_err(|_| musha_fs::Error::Io)?
            {
                let mut b = [0; 18];
                copy(self.bot.data, &mut b);
                if storage::sense(&b).is_some_and(|s| s.write_protected()) {
                    self.protected = true;
                    crate::debug(b"MUSHA: SETTINGS_WRITE_PROTECTED\n");
                }
            }
            Err(musha_fs::Error::Io)
        }
        fn sync(&mut self) -> Result<(), musha_fs::Error> {
            if self
                .bot
                .command(self.host, Command::SynchronizeCache)
                .map_err(|_| musha_fs::Error::Io)?
            {
                Ok(())
            } else {
                Err(musha_fs::Error::Io)
            }
        }
        fn protected(&self) -> bool {
            self.protected
        }
    }
    // This large bounded workspace is BSP-owned CPU memory, never a DMA buffer.
    struct Shared(core::cell::UnsafeCell<musha_fs::settings::Workspace>);
    unsafe impl Sync for Shared {}
    static WORK: Shared = Shared(core::cell::UnsafeCell::new(
        musha_fs::settings::Workspace::new(),
    ));
    let workspace = unsafe { &mut *WORK.0.get() };
    // Discover write protection without attempting WRITE, since protected USB
    // devices may stall the OUT phase and require BOT reset recovery afterwards.
    if !bot.command(host, Command::ModeSense)? {
        return Err("SETTINGS MODE SENSE");
    }
    let mut mode = [0; 4];
    copy(bot.data, &mut mode);
    let protected = storage::mode_write_protected(&mode).ok_or("SETTINGS MODE HEADER")?;
    if protected {
        crate::debug(b"MUSHA: SETTINGS_WRITE_PROTECTED\n");
    }
    let mut image = Image {
        host,
        bot,
        blocks,
        size,
        protected,
    };
    let mut record = [0; 4096];
    musha_platform::pairing::encode_state(
        &[],
        if cfg!(feature = "settings-write-bounds") {
            None
        } else {
            Some([1; 32])
        },
        &mut record,
    )
    .map_err(|_| "SETTINGS RECORD")?;
    let config = musha_fs::settings::write_owned(
        &mut image,
        blocks,
        size as usize,
        b"CONFIG.TXT",
        &record,
        musha_platform::pairing::recognized,
        workspace,
    );
    if config != Err(musha_fs::settings::WriteError::Unowned) {
        return Err("CONFIG WRITE ALLOWED");
    }
    crate::debug(b"MUSHA: CONFIG_WRITE_REFUSED\n");
    crate::debug(b"MUSHA: SETTINGS_WRITE_BEGIN\n");
    let first = musha_fs::settings::write_owned(
        &mut image,
        blocks,
        size as usize,
        musha_fs::settings::OWNED_NAME,
        &record,
        musha_platform::pairing::recognized,
        workspace,
    );
    settings_debug(format_args!("MUSHA: SETTINGS_WRITE_RESULT {:?}\n", first));
    if first.is_ok() {
        if cfg!(feature = "settings-write-bounds") {
            crate::debug(b"MUSHA: SETTINGS_ZERO_PAYLOAD_OK\n");
            let mut bonds = [musha_platform::pairing::Bond {
                address_type: 0,
                address: [1; 6],
                key_size: 16,
                flags: 2,
                ediv: 0,
                random: [0; 8],
                ltk: [3; 16],
            }; musha_platform::pairing::MAX_BONDS];
            for (i, bond) in bonds.iter_mut().enumerate() {
                bond.address[0] = i as u8;
            }
            musha_platform::pairing::encode_state(&bonds, None, &mut record)
                .map_err(|_| "SETTINGS LIMIT RECORD")?;
        } else {
            musha_platform::pairing::encode_state(&[], Some([2; 32]), &mut record)
                .map_err(|_| "SETTINGS RECORD")?;
        }
        let second = musha_fs::settings::write_owned(
            &mut image,
            blocks,
            size as usize,
            musha_fs::settings::OWNED_NAME,
            &record,
            musha_platform::pairing::recognized,
            workspace,
        );
        settings_debug(format_args!(
            "MUSHA: SETTINGS_OVERWRITE_RESULT {:?}\n",
            second
        ));
        if second != Ok(false) {
            return Err("SETTINGS OVERWRITE");
        }
        let p = musha_fs::settings::preflight(
            blocks,
            size as usize,
            musha_fs::settings::OWNED_NAME,
            4096,
            false,
            &mut |l, b| musha_fs::settings::Disk::read(&mut image, l, b),
        )
        .map_err(|_| "SETTINGS READBACK PLAN")?;
        let mut readback = [0; 4096];
        for (i, b) in readback.chunks_mut(size as usize).enumerate() {
            musha_fs::settings::Disk::read(
                &mut image,
                p.data_sector(i).ok_or("SETTINGS READBACK RANGE")?,
                b,
            )
            .map_err(|_| "SETTINGS READBACK")?;
        }
        if record != readback {
            return Err("SETTINGS CONTENT");
        }
        crate::debug(b"MUSHA: SETTINGS_READBACK_OK\n");
        if cfg!(feature = "settings-write-bounds") {
            crate::debug(b"MUSHA: SETTINGS_MAX_BONDS_OK\n");
            let overflow = musha_fs::settings::write_owned(
                &mut image,
                blocks,
                size as usize,
                musha_fs::settings::OWNED_NAME,
                &[0; 4097],
                musha_platform::pairing::recognized,
                workspace,
            );
            if overflow != Err(musha_fs::settings::WriteError::Unowned) {
                return Err("SETTINGS OVERFLOW ACCEPTED");
            }
            crate::debug(b"MUSHA: SETTINGS_OVERFLOW_REFUSED\n");
        }
    }
    Err("SETTINGS PROBE DONE") // Exercise existing controller DMA shutdown.
}

#[cfg(feature = "settings-write-probe")]
fn settings_debug(args: core::fmt::Arguments<'_>) {
    struct Debug;
    impl core::fmt::Write for Debug {
        fn write_str(&mut self, s: &str) -> core::fmt::Result {
            crate::debug(s.as_bytes());
            Ok(())
        }
    }
    let _ = core::fmt::write(&mut Debug, args);
}

mod pairing;
