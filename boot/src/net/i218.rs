// Copyright 2026 Hirofumi Iwasaki
// SPDX-License-Identifier: Apache-2.0
//! Experimental integration of the BSD PHY backend, with DMA disabled.
use crate::{acpi, pci};
use musha_net::i218::{self, Error, Io};
struct Registers<'a> {
    c: pci::Controller,
    time: &'a mut acpi::Time,
}
impl Registers<'_> {
    fn address(&self, offset: usize) -> Result<*mut u32, Error> {
        if self.c.base == 0
            || self.c.base % 4 != 0
            || offset % 4 != 0
            || offset.checked_add(4).is_none_or(|v| v > self.c.bytes)
        {
            return Err(Error::Resource);
        }
        self.c
            .base
            .checked_add(offset)
            .map(|p| p as *mut u32)
            .ok_or(Error::Resource)
    }
}
impl Io for Registers<'_> {
    fn read(&mut self, offset: usize) -> Result<u32, Error> {
        let p = self.address(offset)?;
        // SAFETY: UEFI BAR was validated and permanently identity-mapped UC.
        Ok(unsafe { p.read_volatile() })
    }
    fn write(&mut self, offset: usize, value: u32) -> Result<(), Error> {
        // PHY probe may issue only MDIO commands and software ownership writes.
        if !matches!(offset, i218::MDIC | i218::EXTCNF_CTRL)
            && !(cfg!(feature = "i218-power-probe") && offset == i218::power::H2ME)
        {
            return Err(Error::Resource);
        }
        let p = self.address(offset)?;
        unsafe {
            p.write_volatile(value);
        }
        // Read the device's STATUS register to flush posted MMIO writes.
        self.read(8)?;
        Ok(())
    }
    fn delay_us(&mut self, us: u32) -> Result<(), Error> {
        self.time.delay_us(us).map_err(|_| Error::Clock)
    }
}
pub(super) fn probe(c: pci::Controller, time: &mut acpi::Time) -> Result<(), &'static str> {
    crate::diagnostics::net_stage("I218 PHY PROBE ONLY");
    let command = unsafe { pci::read(c.bus, c.device, c.function, 4) };
    if c.bytes < 0x6000 || command & 2 == 0 || command & 4 != 0 {
        return Err(Error::Resource.label());
    }
    let revision = unsafe { pci::read(c.bus, c.device, c.function, 8) } as u8;
    crate::diagnostics::set(8, format_args!("I218 PCI REV {:02X} PHY UNKNOWN", revision));
    crate::diagnostics::set(9, format_args!("I218 NETWORK NOT IMPLEMENTED"));
    let mut io = Registers { c, time };
    let status = io.read(8).map_err(Error::label)?;
    let fwsm = io.read(0x5b54).map_err(Error::label)?;
    crate::diagnostics::set(
        23,
        format_args!("I218 STATUS {:08X} FWSM {:08X}", status, fwsm),
    );
    crate::debug(b"MUSHA: I218_PHY_PROBE DMA_DISABLED REV=");
    crate::debug(&crate::cpu::hex(revision as u64));
    crate::debug(b"\n");
    #[cfg(feature = "i218-power-probe")]
    {
        crate::diagnostics::net_stage("I218 ME ULP EXIT / NO DMA");
        i218::power::managed_exit(&mut io, i218::Device::I218V3)
            .map_err(i218::power::Error::label)?;
        crate::debug(b"MUSHA: I218_ME_ULP_EXIT_OK NO_NETWORK DMA_DISABLED\n");
    }
    let (id, rev) = i218::phy_id(&mut io).map_err(Error::label)?;
    crate::diagnostics::set(8, format_args!("I218 PHY ID {:08X} REV {:02X}", id, rev));
    crate::debug(b"MUSHA: I218_PHY_ID=");
    crate::debug(&crate::cpu::hex(id as u64));
    crate::debug(b" REV=");
    crate::debug(&crate::cpu::hex(rev as u64));
    crate::debug(b"\n");
    if id != 0x015400a0 {
        return Err(Error::InvalidPhyId.label());
    }
    if unsafe { pci::read(c.bus, c.device, c.function, 4) } & 4 != 0 {
        return Err("I218 DMA UNEXPECTED");
    }
    crate::diagnostics::net_stage("I218 PROBE DONE DMA DISABLED");
    crate::debug(b"MUSHA: I218_PHY_PROBE_OK NO_NETWORK DMA_DISABLED\n");
    Ok(())
}

#[cfg(feature = "i218-network")]
struct NetworkRegisters<'a> {
    c: pci::Controller,
    flash: pci::Controller,
    region: Option<i218::flash::Region>,
    budget: i218::budget::Budget,
    time: &'a mut acpi::Time,
}
#[cfg(feature = "i218-network")]
impl NetworkRegisters<'_> {
    fn check(&mut self, us: u32) -> Result<(), Error> {
        let now = self.time.now().map_err(|_| Error::Clock)?;
        self.budget.check(now, us)
    }
    fn pointer(
        &mut self,
        c: pci::Controller,
        offset: usize,
        width: usize,
    ) -> Result<*mut u8, Error> {
        self.check(0)?;
        if c.base == 0
            || c.base % 4096 != 0
            || offset % width != 0
            || offset.checked_add(width).is_none_or(|n| n > c.bytes)
        {
            return Err(Error::Resource);
        }
        c.base
            .checked_add(offset)
            .map(|p| p as *mut u8)
            .ok_or(Error::Resource)
    }
}
#[cfg(feature = "i218-network")]
impl Io for NetworkRegisters<'_> {
    fn read(&mut self, r: usize) -> Result<u32, Error> {
        Ok(unsafe { self.pointer(self.c, r, 4)?.cast::<u32>().read_volatile() })
    }
    fn write(&mut self, r: usize, v: u32) -> Result<(), Error> {
        if r == 0 && v & (1 << 26) != 0 {
            return Err(Error::Resource);
        } // Only raw reset path.
        unsafe {
            self.pointer(self.c, r, 4)?.cast::<u32>().write_volatile(v);
        }
        self.read(8)?;
        Ok(())
    }
    fn delay_us(&mut self, us: u32) -> Result<(), Error> {
        self.check(us)?;
        self.time.delay_us(us).map_err(|_| Error::Clock)?;
        self.check(0)
    }
}
#[cfg(feature = "i218-network")]
impl i218::flash::Io for NetworkRegisters<'_> {
    fn read16(&mut self, r: usize) -> Result<u16, Error> {
        if !matches!(r, 4 | 6) {
            return Err(Error::Resource);
        }
        Ok(unsafe {
            self.pointer(self.flash, r, 2)?
                .cast::<u16>()
                .read_volatile()
        })
    }
    fn write16(&mut self, r: usize, v: u16) -> Result<(), Error> {
        if !matches!(r, 4 | 6) || r == 6 && v & 6 != 0 {
            return Err(Error::Resource);
        } // No write/erase cycle.
        unsafe {
            self.pointer(self.flash, r, 2)?
                .cast::<u16>()
                .write_volatile(v);
        }
        Ok(())
    }
    fn read32(&mut self, r: usize) -> Result<u32, Error> {
        if !matches!(r, 0 | 0x10) {
            return Err(Error::Resource);
        }
        Ok(unsafe {
            self.pointer(self.flash, r, 4)?
                .cast::<u32>()
                .read_volatile()
        })
    }
    fn write32(&mut self, r: usize, v: u32) -> Result<(), Error> {
        if r != 8 || v > 0xffffff {
            return Err(Error::Resource);
        }
        unsafe {
            self.pointer(self.flash, r, 4)?
                .cast::<u32>()
                .write_volatile(v);
        }
        Ok(())
    }
    fn delay_us(&mut self, us: u32) -> Result<(), Error> {
        Io::delay_us(self, us)
    }
}
#[cfg(feature = "i218-network")]
impl i218::reset::Io for NetworkRegisters<'_> {
    fn pci_id(&mut self) -> Result<u32, Error> {
        self.check(0)?;
        Ok(unsafe { pci::read(self.c.bus, self.c.device, self.c.function, 0) })
    }
    fn bus_master(&mut self) -> Result<bool, Error> {
        self.check(0)?;
        Ok(unsafe { pci::read(self.c.bus, self.c.device, self.c.function, 4) } & 4 != 0)
    }
    fn raw_write(&mut self, r: usize, v: u32) -> Result<(), Error> {
        if !matches!(r, 0 | 0xc) {
            return Err(Error::Resource);
        }
        unsafe {
            self.pointer(self.c, r, 4)?.cast::<u32>().write_volatile(v);
        }
        Ok(())
    }
    fn nvm_word(&mut self, n: u16) -> Result<u16, Error> {
        let region = match self.region {
            Some(r) => r,
            None => {
                let r = i218::flash::Region::open(self)?;
                self.region = Some(r);
                r
            }
        };
        region.word(self, n)
    }
}
#[cfg(feature = "i218-network")]
pub(super) fn initialize(
    c: pci::Controller,
    flash: pci::Controller,
    time: &mut acpi::Time,
) -> Result<[u8; 6], &'static str> {
    if c.bytes < 0x6000 || flash.bytes < 0x1000 || flash.bdf() != c.bdf() {
        return Err("I218 FLASH RESOURCE");
    }
    let mut io = NetworkRegisters {
        c,
        flash,
        region: None,
        budget: i218::budget::Budget::new(time.now()?),
        time,
    };
    let reset = i218::reset::initialize(&mut io);
    if io.budget.expired() {
        return Err("I218 INIT BUDGET 5S");
    }
    match reset {
        Ok(()) => {}
        Err(i218::reset::Error::ResetUnconfirmed) => {
            crate::debug(b"MUSHA: I218_RESET_UNCONFIRMED DMA_DISABLED QUARANTINED\n");
            crate::stop();
        }
        Err(e) => return Err(e.label()),
    }
    crate::debug(b"MUSHA: I218_RESET_OK DMA_DISABLED\n");
    let result = i218::link::initialize(&mut io);
    if io.budget.expired() {
        return Err("I218 INIT BUDGET 5S");
    }
    result.map_err(Error::label)
}
