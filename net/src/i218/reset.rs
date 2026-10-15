/******************************************************************************
  SPDX-License-Identifier: BSD-3-Clause

  Copyright (c) 2001-2020, Intel Corporation
  All rights reserved.

  Redistribution and use in source and binary forms, with or without
  modification, are permitted provided that the following conditions are met:

   1. Redistributions of source code must retain the above copyright notice,
      this list of conditions and the following disclaimer.

   2. Redistributions in binary form must reproduce the above copyright
      notice, this list of conditions and the following disclaimer in the
      documentation and/or other materials provided with the distribution.

   3. Neither the name of the Intel Corporation nor the names of its
      contributors may be used to endorse or promote products derived from
      this software without specific prior written permission.

  THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS"
  AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
  IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE
  ARE DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT OWNER OR CONTRIBUTORS BE
  LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR
  CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF
  SUBSTITUTE GOODS OR SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS
  INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN
  CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE)
  ARISING IN ANY WAY OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE
  POSSIBILITY OF SUCH DAMAGE.

******************************************************************************/
//! I218-V3 hardware-autoload reset slice, adapted from pinned FreeBSD
//! e1000_reset_hw_ich8lan/get_cfg_done/post_phy_reset. Software LCD/OEM NVM
//! configurations are rejected before reset; they are never silently skipped.
use super::{Error as IoError, Io as Registers, Register, power};
pub trait Io: Registers {
    fn pci_id(&mut self) -> Result<u32, IoError>;
    fn bus_master(&mut self) -> Result<bool, IoError>;
    /// No STATUS flush! CTRL global reset must not trigger any following MMIO
    /// read until the delay + PCI config/STRAP synchronization completes.
    fn raw_write(&mut self, register: usize, value: u32) -> Result<(), IoError>;
    fn nvm_word(&mut self, index: u16) -> Result<u16, IoError>;
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Io(IoError),
    Power(power::Error),
    Device,
    DmaActive,
    SoftwareNvmRequired,
    NvmChecksum,
    ResetBlocked,
    Timeout,
    ResetUnconfirmed,
}
impl Error {
    pub fn label(self) -> &'static str {
        match self {
            Self::Io(e) => e.label(),
            Self::Power(e) => e.label(),
            Self::Device => "I218 RESET DEVICE",
            Self::DmaActive => "I218 DMA ACTIVE",
            Self::SoftwareNvmRequired => "I218 SOFTWARE NVM CONFIG REQUIRED",
            Self::NvmChecksum => "I218 NVM CHECKSUM",
            Self::ResetBlocked => "I218 PHY RESET BLOCKED",
            Self::Timeout => "I218 RESET TIMEOUT",
            Self::ResetUnconfirmed => "I218 RESET UNCONFIRMED",
        }
    }
}
fn read(io: &mut impl Io, r: usize) -> Result<u32, Error> {
    let v = io.read(r).map_err(Error::Io)?;
    if v == u32::MAX {
        return Err(Error::Io(IoError::Resource));
    }
    Ok(v)
}
fn write(io: &mut impl Io, r: usize, v: u32) -> Result<(), Error> {
    io.write(r, v).map_err(Error::Io)
}
fn delay(io: &mut impl Io, us: u32) -> Result<(), Error> {
    io.delay_us(us).map_err(Error::Io)
}
fn wait(io: &mut impl Io, r: usize, mask: u32, value: u32, n: usize, us: u32) -> Result<(), Error> {
    for _ in 0..n {
        if read(io, r)? & mask == value {
            return Ok(());
        }
        delay(io, us)?;
    }
    Err(Error::Timeout)
}
pub(super) fn phy_write(io: &mut impl Registers, reg: Register, data: u16) -> Result<(), IoError> {
    let addr = if reg.page >= 768 { 1 } else { 2 };
    if reg.register > 15 {
        let page = if reg.page == 768 { 0 } else { reg.page };
        super::mdic(io, 1, 31, Some(page << 5))?;
    }
    super::mdic(io, addr, reg.register, Some(data))?;
    Ok(())
}
fn update_phy(
    io: &mut impl Registers,
    page: u16,
    reg: u8,
    clear: u16,
    set: u16,
) -> Result<(), IoError> {
    let r = Register::new(page, reg)?;
    let v = super::read_locked(io, r)?;
    phy_write(io, r, (v & !clear) | set)
}
fn software_ulp(io: &mut impl Io) -> Result<(), Error> {
    wait(io, power::FWSM, 0x40, 0x40, 31, 10_000).map_err(|e| {
        if e == Error::Timeout {
            Error::ResetBlocked
        } else {
            e
        }
    })?;
    super::acquire(io).map_err(Error::Io)?;
    let result = (|| {
        let counter = read(io, 0x3c)?;
        write(io, 0x3c, (counter & !0x0c000000) | 0x08000000)?;
        let ctrl = read(io, 0)?;
        write(io, 0, (ctrl | 0x10000) & !0x20000)?;
        delay(io, 1000)?;
        write(io, 0, ctrl & !0x10000 & !0x20000)?;
        wait(io, 0x18, 4, 4, 21, 5000)?;
        delay(io, 30_000)?;
        let reg = Register::new(769, 23).map_err(Error::Io)?;
        let value = match super::read_locked(io, reg) {
            Ok(v) => v,
            Err(_) => {
                let v = read(io, 0x18)?;
                write(io, 0x18, v | 0x800)?;
                delay(io, 50_000)?;
                super::read_locked(io, reg).map_err(Error::Io)?
            }
        };
        phy_write(io, reg, value & !1).map_err(Error::Io)?;
        let v = read(io, 0x18)?;
        write(io, 0x18, v & !0x800)?;
        update_phy(io, 770, 17, 0, 0x4000).map_err(Error::Io)?;
        let reg = Register::new(779, 16).map_err(Error::Io)?;
        let value = super::read_locked(io, reg).map_err(Error::Io)? & !0x1d74;
        phy_write(io, reg, value).map_err(Error::Io)?;
        phy_write(io, reg, value | 1).map_err(Error::Io)?;
        let v = read(io, 0xe4)?;
        write(io, 0xe4, v & !0x20)?;
        Ok(())
    })();
    super::release(io).map_err(Error::Io)?;
    result?;
    // Force=true upstream resets the PHY after releasing the ULP transaction.
    super::acquire(io).map_err(Error::Io)?;
    let result = (|| {
        let ctrl = read(io, 0)?;
        write(io, 0, ctrl | (1 << 31))?;
        delay(io, 100)?;
        write(io, 0, ctrl)?;
        delay(io, 150)?;
        Ok(())
    })();
    super::release(io).map_err(Error::Io)?;
    result?;
    delay(io, 50_000)?;
    Ok(())
}
/// Full chip reset remains restricted to the validated hardware-autoload path.
/// Return no success token for software-required LCD/OEM configurations.
pub fn initialize(io: &mut impl Io) -> Result<(), Error> {
    if io.pci_id().map_err(Error::Io)? != 0x15a38086 {
        return Err(Error::Device);
    }
    if io.bus_master().map_err(Error::Io)? {
        return Err(Error::DmaActive);
    }
    if read(io, 0x28)? & (1 << 27) != 0 {
        return Err(Error::SoftwareNvmRequired);
    }
    let mut sum = 0u16;
    for n in 0..64 {
        sum = sum.wrapping_add(io.nvm_word(n).map_err(Error::Io)?);
    }
    if sum != 0xbaba {
        return Err(Error::NvmChecksum);
    }
    // init_phy_workarounds_pchlan gates automatic PHY configuration before
    // ULP recovery, preserving all other EXTCNF bits.
    let config = read(io, super::EXTCNF_CTRL)?;
    write(io, super::EXTCNF_CTRL, config | 0x80)?;
    if read(io, power::FWSM)? & 0x8000 != 0 {
        power::managed_exit(io, super::Device::I218V3).map_err(Error::Power)?;
    } else {
        software_ulp(io)?;
    }
    let (phy, _) = super::phy_id(io).map_err(Error::Io)?;
    if phy != 0x015400a0 {
        return Err(Error::Io(IoError::InvalidPhyId));
    }
    // Disable PCIe master requests before resetting the MAC (PCI BME is off).
    let ctrl = read(io, 0)?;
    write(io, 0, ctrl | 4)?;
    wait(io, 8, 1 << 19, 0, 800, 100)?;
    write(io, 0xd8, u32::MAX)?;
    write(io, 0x100, 0)?;
    write(io, 0x400, 8)?;
    delay(io, 10_000)?;
    wait(io, power::FWSM, 0x40, 0x40, 31, 10_000).map_err(|e| {
        if e == Error::Timeout {
            Error::ResetBlocked
        } else {
            e
        }
    })?;
    let ctrl = read(io, 0)? | (1 << 31);
    super::acquire(io).map_err(Error::Io)?;
    let id = match io.pci_id() {
        Ok(v) if v == 0x15a38086 => v,
        _ => {
            super::release(io).map_err(Error::Io)?;
            return Err(Error::Device);
        }
    };
    if io.raw_write(0xc, id & 0xffff).is_err() {
        super::release(io).map_err(Error::Io)?;
        return Err(Error::Io(IoError::Resource));
    }
    // An attempted reset is ambiguous on IO failure. No subsequent MMIO is safe
    // until delay and PCI synchronization; caller must quarantine this error.
    if io.raw_write(0, ctrl | (1 << 26)).is_err() {
        return Err(Error::ResetUnconfirmed);
    }
    if io.delay_us(20_000).is_err() {
        return Err(Error::ResetUnconfirmed);
    }
    let id = io.pci_id().map_err(|_| Error::ResetUnconfirmed)?;
    if id != 0x15a38086 || io.raw_write(0xc, id & 0xffff).is_err() {
        return Err(Error::ResetUnconfirmed);
    }
    wait(io, 0, 1 << 26, 0, 1000, 100).map_err(|_| Error::ResetUnconfirmed)?;
    // SWFLAG is cleared by global reset; do not steal a newly held flag.
    if read(io, super::EXTCNF_CTRL)? & 0x20 != 0 {
        return Err(Error::ResetUnconfirmed);
    }
    delay(io, 10_000)?;
    wait(io, 8, 0x200, 0x200, 1500, 100)?;
    let status = read(io, 8)?;
    write(io, 8, status & !0x200 & !(1 << 10))?;
    // Post-PHY-reset host wakeup clear. LCD/OEM software branch was rejected.
    delay(io, 10_000)?;
    super::acquire(io).map_err(Error::Io)?;
    let result = update_phy(io, 769, 17, 0x10, 0).map_err(Error::Io);
    super::release(io).map_err(Error::Io)?;
    result?;
    write(io, 0xd8, u32::MAX)?;
    let _ = read(io, 0xc0)?;
    let bias = read(io, 0x3004)?;
    write(io, 0x3004, bias | 0x50000)?;
    if io.bus_master().map_err(Error::Io)? {
        return Err(Error::DmaActive);
    }
    Ok(())
}
