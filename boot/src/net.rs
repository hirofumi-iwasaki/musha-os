// Copyright 2026 Hirofumi Iwasaki
// SPDX-License-Identifier: Apache-2.0
//! Cooperative, polling-only session for the QEMU 82574. No I218/I219 support.
use crate::{BootInfo, acpi, pci};
use core::{
    arch::asm,
    sync::atomic::{AtomicU32, Ordering, compiler_fence},
};
use musha_net::{BUFFER_BYTES, DMA_BYTES, RX_BUFFERS, RX_COUNT, TX_BUFFERS, TX_COUNT, TX_RING};
static BDF: AtomicU32 = AtomicU32::new(u32::MAX);
static USB_BDF: AtomicU32 = AtomicU32::new(u32::MAX);
unsafe extern "C" {
    fn musha_lwip_init(mac: *const u8) -> i32;
    fn musha_lwip_poll(ms: u32);
    fn musha_lwip_input(bytes: *const u8, length: u16) -> i32;
    fn musha_lwip_tx(bytes: *mut u8, capacity: u16) -> i32;
    fn musha_lwip_counters(out: *mut u32);
}
#[unsafe(no_mangle)]
extern "C" fn musha_lwip_assert() -> ! {
    let mut disabled = true;
    for b in [BDF.load(Ordering::Relaxed), USB_BDF.load(Ordering::Relaxed)] {
        if b != u32::MAX {
            unsafe {
                pci::disable_dma(pci::Controller {
                    base: 0,
                    bytes: 0,
                    bus: (b >> 8) as u8,
                    device: ((b >> 3) & 31) as u8,
                    function: (b & 7) as u8,
                });
                disabled &=
                    pci::read((b >> 8) as u8, ((b >> 3) & 31) as u8, (b & 7) as u8, 4) & 4 == 0;
            }
        }
    }
    crate::debug(if disabled {
        b"MUSHA: NET_FAILED LWIP ASSERT DMA_DISABLED\n"
    } else {
        b"MUSHA: NET_FAILED LWIP ASSERT DMA_STOP_FAILED\n"
    });
    crate::stop()
}
struct Nic {
    c: pci::Controller,
    dma: usize,
}
impl Nic {
    fn read(&self, r: usize) -> u32 {
        assert!(r % 4 == 0 && r + 4 <= self.c.bytes);
        unsafe { ((self.c.base + r) as *const u32).read_volatile() }
    }
    fn write(&self, r: usize, v: u32) {
        assert!(r % 4 == 0 && r + 4 <= self.c.bytes);
        unsafe {
            ((self.c.base + r) as *mut u32).write_volatile(v);
        }
        self.read(8);
    }
    fn wait(
        &self,
        t: &mut acpi::Time,
        r: usize,
        mask: u32,
        value: u32,
        ms: u64,
    ) -> Result<(), &'static str> {
        let end = t.now()?.checked_add(ms).ok_or("CLOCK")?;
        for _ in 0..5_000_000 {
            if self.read(r) & mask == value {
                return Ok(());
            }
            if t.now()? >= end {
                break;
            }
            core::hint::spin_loop();
        }
        Err("TIMEOUT")
    }
    fn quiesce(&self) -> bool {
        crate::diagnostics::net_stage("STOPPING");
        self.write(0xd8, u32::MAX);
        self.write(0x100, 0);
        self.write(0x400, 0);
        unsafe {
            pci::disable_dma(self.c);
        }
        unsafe { pci::read(self.c.bus, self.c.device, self.c.function, 4) & 4 == 0 }
    }
    fn initialize(&self, t: &mut acpi::Time) -> Result<(), &'static str> {
        crate::diagnostics::net_stage("RESET");
        self.write(0xd8, u32::MAX);
        self.write(0x100, 0);
        self.write(0x400, 0);
        self.write(0, self.read(0) | (1 << 26));
        self.wait(t, 0, 1 << 26, 0, 100)?;
        crate::diagnostics::net_stage("EEPROM / MAC");
        self.wait(t, 0x10, 1 << 9, 1 << 9, 1000)?;
        self.write(0xd8, u32::MAX);
        self.write(0xe0, 0);
        let lo = self.read(0x5400);
        let hi = self.read(0x5404);
        let mac = [
            lo as u8,
            (lo >> 8) as u8,
            (lo >> 16) as u8,
            (lo >> 24) as u8,
            hi as u8,
            (hi >> 8) as u8,
        ];
        if hi & (1 << 31) == 0 || mac[0] & 1 != 0 || mac == [0; 6] {
            return Err("MAC");
        }
        crate::diagnostics::net_stage("LINK");
        self.write(0, self.read(0) | (1 << 6));
        self.wait(t, 8, 2, 2, 3000)?;
        crate::diagnostics::net_stage("DMA / LWIP");
        // DMA storage is permanent, dedicated, below 4GiB and mapped UC/RW/NX.
        // No Rust references are formed to device-owned descriptors or buffers.
        unsafe {
            for i in 0..DMA_BYTES {
                ((self.dma + i) as *mut u8).write_volatile(0);
            }
            for i in 0..RX_COUNT {
                ((self.dma + i * 16) as *mut u64)
                    .write_volatile((self.dma + RX_BUFFERS + i * BUFFER_BYTES) as u64);
            }
        }
        for (r, v) in [
            (0x2800, self.dma as u32),
            (0x2804, 0),
            (0x2808, 256),
            (0x2810, 0),
            (0x2818, 15),
            (0x3800, (self.dma + TX_RING) as u32),
            (0x3804, 0),
            (0x3808, 128),
            (0x3810, 0),
            (0x3818, 0),
            (0x5000, 0),
            (0x410, 10 | (8 << 10) | (6 << 20)),
        ] {
            self.write(r, v);
        }
        self.write(0x2828, 1 << 24); // GRAN=descriptors, WTHRESH=0: immediate writeback.
        for i in 0..128 {
            self.write(0x5200 + i * 4, 0);
        }
        unsafe {
            asm!("mfence", options(nostack));
            pci::enable_dma(self.c);
        }
        if unsafe { pci::read(self.c.bus, self.c.device, self.c.function, 4) } & 4 == 0 {
            return Err("BUS MASTER");
        }
        self.write(0x400, 2 | 8 | (0x10 << 4) | (0x40 << 12));
        self.write(0x100, 2 | (1 << 15) | (1 << 26));
        if unsafe { musha_lwip_init(mac.as_ptr()) } != 0 {
            return Err("LWIP INIT");
        }
        crate::debug(b"MUSHA: NET_READY IP=10.0.2.15 UDP=12345\n");
        Ok(())
    }
}
/// Single-owner session. Drop always disables DMA, including application errors.
pub(crate) struct Session {
    nic: Nic,
    started: u64,
    rx: usize,
    tx: usize,
    rx_wrap: u64,
    tx_wrap: u64,
    drops: u64,
    pending: Option<musha_net::TxFlight>,
    polls: u64,
    stopped: bool,
}
impl Session {
    pub(crate) fn start(
        info: &BootInfo,
        time: &mut acpi::Time,
    ) -> Result<Option<Self>, &'static str> {
        crate::diagnostics::net_stage("RESOURCE / INITIALIZE");
        let c = info.nic;
        if c.base == 0 {
            crate::diagnostics::net_stage("DRIVER UNSUPPORTED / ABSENT");
            crate::debug(b"MUSHA: NET_UNSUPPORTED\n");
            return Ok(None);
        }
        if c.bytes < 0x6000
            || unsafe { pci::read(c.bus, c.device, c.function, 0) } != 0x10d38086
            || unsafe { pci::read(c.bus, c.device, c.function, 4) } & 2 == 0
            || !musha_net::valid_dma(info.net_dma_base, info.net_dma_bytes)
        {
            return Err("RESOURCE");
        }
        let usb = info.xhci;
        if usb.base != 0 {
            USB_BDF.store(
                ((usb.bus as u32) << 8) | ((usb.device as u32) << 3) | usb.function as u32,
                Ordering::Relaxed,
            );
        }
        BDF.store(
            ((c.bus as u32) << 8) | ((c.device as u32) << 3) | c.function as u32,
            Ordering::Relaxed,
        );
        unsafe {
            pci::disable_dma(c);
        }
        let mut session = Self {
            nic: Nic {
                c,
                dma: info.net_dma_base,
            },
            started: time.now()?,
            rx: 0,
            tx: 0,
            rx_wrap: 0,
            tx_wrap: 0,
            drops: 0,
            pending: None,
            polls: 0,
            stopped: false,
        };
        if let Err(error) = session.nic.initialize(time) {
            crate::diagnostics::net_failure(error);
            return Err(error);
        }
        crate::diagnostics::net_stage("POLLING");
        session.started = time.now()?;
        Ok(Some(session))
    }
    pub(crate) fn started(&self) -> u64 {
        self.started
    }
    pub(crate) fn udp_count(&self) -> u32 {
        let mut c = [0u32; 5];
        unsafe {
            musha_lwip_counters(c.as_mut_ptr());
        }
        c[2]
    }
    /// Bounded work: eight RX frames, one TX observation, at most one new TX.
    /// There is no spin loop waiting for a completion here.
    pub(crate) fn poll(&mut self, now: u64) -> Result<(), &'static str> {
        if self.stopped {
            return Err("STOPPED");
        }
        if now < self.started {
            return Err("CLOCK");
        }
        self.polls = self.polls.checked_add(1).ok_or("POLL OVERFLOW")?;
        if self.nic.read(8) & 2 == 0 {
            return Err("LINK DOWN");
        }
        unsafe {
            musha_lwip_poll((now - self.started) as u32);
        }
        let mut bytes = [0u8; BUFFER_BYTES];
        for _ in 0..8 {
            let d = self.nic.dma + self.rx * 16;
            let status = unsafe { ((d + 12) as *const u8).read_volatile() };
            if status & 1 == 0 {
                break;
            }
            compiler_fence(Ordering::Acquire);
            unsafe {
                asm!("lfence", options(nostack));
            }
            let len = unsafe { ((d + 8) as *const u16).read_volatile() } as usize;
            let errors = unsafe { ((d + 13) as *const u8).read_volatile() };
            if musha_net::valid_rx(status, errors, len) {
                unsafe {
                    for (i, b) in bytes[..len].iter_mut().enumerate() {
                        *b = ((self.nic.dma + RX_BUFFERS + self.rx * BUFFER_BYTES + i)
                            as *const u8)
                            .read_volatile();
                    }
                    musha_lwip_input(bytes.as_ptr(), len as u16);
                }
            } else {
                self.drops += 1;
            }
            unsafe {
                ((d + 12) as *mut u32).write_volatile(0);
                asm!("mfence", options(nostack));
            }
            compiler_fence(Ordering::Release);
            self.nic.write(0x2818, self.rx as u32);
            self.rx = (self.rx + 1) % RX_COUNT;
            if self.rx == 0 {
                self.rx_wrap += 1;
            }
        }
        if let Some(flight) = self.pending.as_mut() {
            let d = self.nic.dma + TX_RING + self.tx * 16;
            let status = unsafe { ((d + 12) as *const u8).read_volatile() };
            match flight.observe(status, now) {
                musha_net::TxStatus::Pending => return Ok(()),
                musha_net::TxStatus::Error => return Err("TX ERROR"),
                musha_net::TxStatus::Timeout => return Err("TX TIMEOUT"),
                musha_net::TxStatus::Complete => {
                    compiler_fence(Ordering::Acquire);
                    unsafe {
                        asm!("lfence", options(nostack));
                    }
                    self.pending = None;
                    self.tx = (self.tx + 1) % TX_COUNT;
                    if self.tx == 0 {
                        self.tx_wrap += 1;
                    }
                }
            }
        }
        let len = unsafe { musha_lwip_tx(bytes.as_mut_ptr(), bytes.len() as u16) };
        if len == 0 {
            return Ok(());
        }
        let word = musha_net::tx_word(len as usize).ok_or("TX LENGTH")?;
        let d = self.nic.dma + TX_RING + self.tx * 16;
        let buffer = self.nic.dma + TX_BUFFERS + self.tx * BUFFER_BYTES;
        unsafe {
            for (i, b) in bytes[..len as usize].iter().enumerate() {
                ((buffer + i) as *mut u8).write_volatile(*b);
            }
            (d as *mut u64).write_volatile(buffer as u64);
            ((d + 8) as *mut u64).write_volatile(word);
            asm!("mfence", options(nostack));
        }
        compiler_fence(Ordering::Release);
        self.pending = Some(musha_net::TxFlight::new(now).ok_or("CLOCK")?);
        #[cfg(not(feature = "net-tx-timeout"))]
        self.nic.write(0x3818, ((self.tx + 1) % TX_COUNT) as u32);
        Ok(())
    }
    pub(crate) fn finish(&mut self, fb: musha_framebuffer::Framebuffer) {
        self.report();
        self.stop();
        if fb.height >= 540 {
            unsafe {
                fb.text("NET TEST OK", 24, 516, fb.color(0, 220, 240));
            }
        }
    }
    fn report(&self) {
        let mut c = [0u32; 5];
        unsafe {
            musha_lwip_counters(c.as_mut_ptr());
        }
        crate::diagnostics::set(
            23,
            format_args!("NET UDP {:08X} ARP {:08X} ICMP {:08X}", c[2], c[0], c[1]),
        );
        for (label, value) in [
            (b"MUSHA: NET_ARP=".as_slice(), c[0] as u64),
            (b" ICMP=", c[1] as u64),
            (b" UDP=", c[2] as u64),
            (b" RX_WRAP=", self.rx_wrap),
            (b" TX_WRAP=", self.tx_wrap),
            (b" RX_DROP=", self.drops),
            (b" TX_QUEUE_DROP=", c[3] as u64),
            (b" LWIP_DROP=", c[4] as u64),
            (b" POLLS=", self.polls),
        ] {
            crate::debug(label);
            crate::debug(&crate::cpu::hex(value));
        }
        crate::debug(b"\n");
    }
    fn stop(&mut self) {
        if self.stopped {
            return;
        }
        if !self.nic.quiesce() {
            crate::diagnostics::net_failure("DMA STOP");
            crate::debug(b"MUSHA: NET_FAILED DMA STOP\n");
            crate::stop();
        }
        crate::diagnostics::net_stage("STOPPED DMA DISABLED");
        self.stopped = true;
        BDF.store(u32::MAX, Ordering::Relaxed);
        crate::debug(b"MUSHA: NET_QUIESCED DMA_DISABLED\n");
    }
}
impl Drop for Session {
    fn drop(&mut self) {
        self.stop();
    }
}
pub(crate) fn failed(error: &str) {
    crate::diagnostics::net_failure(error);
    crate::debug(b"MUSHA: NET_FAILED ");
    crate::debug(error.as_bytes());
    crate::debug(b"\n");
}
