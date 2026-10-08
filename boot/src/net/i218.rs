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
        if !matches!(offset, i218::MDIC | i218::EXTCNF_CTRL) {
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
