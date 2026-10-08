// Copyright 2026 Hirofumi Iwasaki
// SPDX-License-Identifier: Apache-2.0
//! Bounded, polling-only diagnostic for the QEMU 82574. No I218/I219 support.
use crate::{BootInfo, acpi, pci};
use core::{
    arch::asm,
    sync::atomic::{AtomicU32, Ordering, compiler_fence},
};
use musha_net::{BUFFER_BYTES, DMA_BYTES, RX_BUFFERS, RX_COUNT, TX_BUFFERS, TX_COUNT, TX_RING};
static BDF: AtomicU32 = AtomicU32::new(u32::MAX);
unsafe extern "C" {
    fn musha_lwip_init(mac: *const u8) -> i32;
    fn musha_lwip_poll(ms: u32);
    fn musha_lwip_input(bytes: *const u8, length: u16) -> i32;
    fn musha_lwip_tx(bytes: *mut u8, capacity: u16) -> i32;
    fn musha_lwip_counters(out: *mut u32);
}
#[unsafe(no_mangle)]
extern "C" fn musha_lwip_assert() -> ! {
    let b = BDF.load(Ordering::Relaxed);
    if b != u32::MAX {
        unsafe {
            pci::disable_dma(pci::Controller {
                base: 0,
                bytes: 0,
                bus: (b >> 8) as u8,
                device: ((b >> 3) & 31) as u8,
                function: (b & 7) as u8,
            });
        }
    }
    crate::debug(b"MUSHA: NET_FAILED LWIP ASSERT DMA_DISABLED\n");
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
        self.write(0xd8, u32::MAX);
        self.write(0x100, 0);
        self.write(0x400, 0);
        unsafe {
            pci::disable_dma(self.c);
        }
        unsafe { pci::read(self.c.bus, self.c.device, self.c.function, 4) & 4 == 0 }
    }
    fn run(&self, info: &BootInfo) -> Result<(), &'static str> {
        let mut t = acpi::Time::new(info.timer)?;
        self.write(0xd8, u32::MAX);
        self.write(0x100, 0);
        self.write(0x400, 0);
        self.write(0, self.read(0) | (1 << 26));
        self.wait(&mut t, 0, 1 << 26, 0, 100)?;
        self.wait(&mut t, 0x10, 1 << 9, 1 << 9, 1000)?;
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
        self.write(0, self.read(0) | (1 << 6));
        self.wait(&mut t, 8, 2, 2, 3000)?;
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
        let start = t.now()?;
        let (mut rx, mut tx, mut rx_wrap, mut tx_wrap, mut drops) =
            (0usize, 0usize, 0u64, 0u64, 0u64);
        let mut bytes = [0u8; 2048];
        let mut expired = false;
        for _ in 0..100_000_000 {
            let now = t.now()?;
            if now - start >= 10000 {
                expired = true;
                break;
            }
            if self.read(8) & 2 == 0 {
                return Err("LINK DOWN");
            }
            unsafe {
                musha_lwip_poll((now - start) as u32);
            }
            for _ in 0..8 {
                let d = self.dma + rx * 16;
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
                            *b = ((self.dma + RX_BUFFERS + rx * BUFFER_BYTES + i) as *const u8)
                                .read_volatile();
                        }
                        musha_lwip_input(bytes.as_ptr(), len as u16);
                    }
                } else {
                    drops += 1;
                }
                unsafe {
                    ((d + 12) as *mut u32).write_volatile(0);
                    asm!("mfence", options(nostack));
                }
                compiler_fence(Ordering::Release);
                self.write(0x2818, rx as u32);
                rx = (rx + 1) % RX_COUNT;
                if rx == 0 {
                    rx_wrap += 1;
                }
            }
            for _ in 0..8 {
                let len = unsafe { musha_lwip_tx(bytes.as_mut_ptr(), bytes.len() as u16) };
                if len == 0 {
                    break;
                }
                if !(14..=1514).contains(&len) {
                    return Err("TX LENGTH");
                }
                let d = self.dma + TX_RING + tx * 16;
                let buf = self.dma + TX_BUFFERS + tx * BUFFER_BYTES;
                unsafe {
                    for (i, b) in bytes[..len as usize].iter().enumerate() {
                        ((buf + i) as *mut u8).write_volatile(*b);
                    }
                    (d as *mut u64).write_volatile(buf as u64);
                    ((d + 8) as *mut u64)
                        .write_volatile(musha_net::tx_word(len as usize).ok_or("TX LENGTH")?);
                    asm!("mfence", options(nostack));
                }
                compiler_fence(Ordering::Release);
                let next = (tx + 1) % TX_COUNT;
                self.write(0x3818, next as u32);
                let end = t.now()? + 100;
                let mut done = false;
                for _ in 0..5_000_000 {
                    let status = unsafe { ((d + 12) as *const u8).read_volatile() };
                    if status & 1 != 0 {
                        if status & 14 != 0 {
                            return Err("TX ERROR");
                        }
                        done = true;
                        break;
                    }
                    if t.now()? >= end {
                        break;
                    }
                }
                if !done {
                    return Err("TX TIMEOUT");
                }
                compiler_fence(Ordering::Acquire);
                unsafe {
                    asm!("lfence", options(nostack));
                }
                tx = next;
                if tx == 0 {
                    tx_wrap += 1;
                }
            }
        }
        if !expired {
            return Err("POLL LIMIT");
        }
        let mut counters = [0u32; 5];
        unsafe {
            musha_lwip_counters(counters.as_mut_ptr());
        }
        for (label, value) in [
            (b"MUSHA: NET_ARP=".as_slice(), counters[0] as u64),
            (b" ICMP=", counters[1] as u64),
            (b" UDP=", counters[2] as u64),
            (b" RX_WRAP=", rx_wrap),
            (b" TX_WRAP=", tx_wrap),
            (b" RX_DROP=", drops),
            (b" TX_QUEUE_DROP=", counters[3] as u64),
            (b" LWIP_DROP=", counters[4] as u64),
        ] {
            crate::debug(label);
            crate::debug(&crate::cpu::hex(value));
        }
        crate::debug(b"\n");
        Ok(())
    }
}
pub(crate) fn diagnose(info: &BootInfo) {
    let c = info.nic;
    if c.base == 0 {
        crate::debug(b"MUSHA: NET_UNSUPPORTED\n");
        return;
    }
    if c.bytes < 0x6000
        || unsafe { pci::read(c.bus, c.device, c.function, 0) } != 0x10d38086
        || unsafe { pci::read(c.bus, c.device, c.function, 4) } & 2 == 0
        || !musha_net::valid_dma(info.net_dma_base, info.net_dma_bytes)
    {
        crate::debug(b"MUSHA: NET_FAILED RESOURCE\n");
        return;
    }
    BDF.store(
        ((c.bus as u32) << 8) | ((c.device as u32) << 3) | c.function as u32,
        Ordering::Relaxed,
    );
    unsafe {
        pci::disable_dma(c);
    }
    let nic = Nic {
        c,
        dma: info.net_dma_base,
    };
    let result = nic.run(info);
    let stopped = nic.quiesce();
    if result.is_ok() && info.framebuffer.height >= 540 {
        unsafe {
            info.framebuffer.text(
                "NET TEST OK",
                24,
                516,
                info.framebuffer.color(0, 220, 240),
            );
        }
    }
    if let Err(error) = result {
        crate::debug(b"MUSHA: NET_FAILED ");
        crate::debug(error.as_bytes());
        crate::debug(b"\n");
    }
    if stopped {
        crate::debug(b"MUSHA: NET_QUIESCED DMA_DISABLED\n");
        BDF.store(u32::MAX, Ordering::Relaxed);
    } else {
        crate::debug(b"MUSHA: NET_FAILED DMA STOP\n");
        crate::stop();
    }
}
