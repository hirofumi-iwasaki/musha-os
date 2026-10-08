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
//! Rust adaptation of FreeBSD e1000 PHY/ICH semaphore routines.
//! Upstream: 833d39bd2e38421a14ea489a956261a0fa993fbc, sys/dev/e1000.
//! Adapted: e1000_{read,write}_phy_reg_mdic, __e1000_read_phy_reg_hv,
//! e1000_set_page_igp, e1000_{acquire,release}_swflag_ich8lan.
//! Wakeup/debug pages, PHY writes other than page selection, reset and DMA
//! are deliberately not exposed by this bounded probe foundation.
//! Rust adaptation copyright 2026 Hirofumi Iwasaki; BSD-3-Clause.

pub const MDIC: usize = 0x20;
pub const EXTCNF_CTRL: usize = 0xf00;
const SWFLAG: u32 = 0x20;
const READY: u32 = 1 << 28;
const MDIC_ERROR: u32 = 1 << 30;
const READ: u32 = 1 << 27;
const WRITE: u32 = 1 << 26;
const POLLS: usize = 640 * 3;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Device {
    I218V,
    I218V2,
    I218V3,
}
pub fn classify(id: u32) -> Option<Device> {
    if id as u16 != 0x8086 {
        return None;
    }
    match id >> 16 {
        0x1559 => Some(Device::I218V),
        0x15a1 => Some(Device::I218V2),
        0x15a3 => Some(Device::I218V3),
        _ => None,
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Resource,
    Clock,
    SemaphoreBusy,
    SemaphoreDenied,
    OwnershipLost,
    SemaphoreRelease,
    MdioTimeout,
    MdioError,
    MdioMismatch,
    UnsupportedPage,
    InvalidRegister,
    InvalidPhyId,
}
impl Error {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Resource => "I218 RESOURCE",
            Self::Clock => "I218 CLOCK",
            Self::SemaphoreBusy => "I218 PHY BUSY",
            Self::SemaphoreDenied => "I218 PHY DENIED",
            Self::OwnershipLost => "I218 OWNERSHIP LOST",
            Self::SemaphoreRelease => "I218 RELEASE FAILED",
            Self::MdioTimeout => "I218 MDIO TIMEOUT",
            Self::MdioError => "I218 MDIO ERROR",
            Self::MdioMismatch => "I218 MDIO MISMATCH",
            Self::UnsupportedPage => "I218 PAGE UNSUPPORTED",
            Self::InvalidRegister => "I218 REGISTER INVALID",
            Self::InvalidPhyId => "I218 PHY ID INVALID",
        }
    }
}
/// Implementations must provide serialized register accesses and finite delays.
/// A delay error aborts the transaction; callers still attempt lock cleanup.
pub trait Io {
    fn read(&mut self, offset: usize) -> Result<u32, Error>;
    fn write(&mut self, offset: usize, value: u32) -> Result<(), Error>;
    fn delay_us(&mut self, us: u32) -> Result<(), Error>;
}
#[derive(Clone, Copy)]
pub struct Register {
    page: u16,
    register: u8,
}
impl Register {
    pub fn new(page: u16, register: u8) -> Result<Self, Error> {
        if register > 31 {
            return Err(Error::InvalidRegister);
        }
        // 0..15 are common registers; debug pages 1..767 and wakeup page
        // 800 require different protocols. No truncating page<<5 conversion.
        if page > 2047 || page == 800 || (page > 0 && page < 768) {
            return Err(Error::UnsupportedPage);
        }
        Ok(Self { page, register })
    }
}
fn release(io: &mut impl Io) -> Result<(), Error> {
    let value = io.read(EXTCNF_CTRL).map_err(|_| Error::SemaphoreRelease)?;
    if value & SWFLAG == 0 {
        return Err(Error::OwnershipLost);
    }
    io.write(EXTCNF_CTRL, value & !SWFLAG)
        .map_err(|_| Error::SemaphoreRelease)?;
    if io.read(EXTCNF_CTRL).map_err(|_| Error::SemaphoreRelease)? & SWFLAG != 0 {
        return Err(Error::SemaphoreRelease);
    }
    Ok(())
}
fn cancel_request(io: &mut impl Io) -> Result<(), Error> {
    let value = io.read(EXTCNF_CTRL).map_err(|_| Error::SemaphoreRelease)?;
    io.write(EXTCNF_CTRL, value & !SWFLAG)
        .map_err(|_| Error::SemaphoreRelease)?;
    if io.read(EXTCNF_CTRL).map_err(|_| Error::SemaphoreRelease)? & SWFLAG != 0 {
        return Err(Error::SemaphoreRelease);
    }
    Ok(())
}
fn acquire(io: &mut impl Io) -> Result<(), Error> {
    let mut clear = None;
    for _ in 0..100 {
        let v = io.read(EXTCNF_CTRL)?;
        if v & SWFLAG == 0 {
            clear = Some(v);
            break;
        }
        io.delay_us(1000)?;
    }
    let v = clear.ok_or(Error::SemaphoreBusy)?;
    // Once a request is attempted, every error path cancels our request.
    let result = (|| {
        io.write(EXTCNF_CTRL, v | SWFLAG)?;
        for _ in 0..1000 {
            if io.read(EXTCNF_CTRL)? & SWFLAG != 0 {
                return Ok(());
            }
            io.delay_us(1000)?;
        }
        Err(Error::SemaphoreDenied)
    })();
    if result.is_err() {
        cancel_request(io)?;
    }
    result
}
fn mdic(io: &mut impl Io, addr: u8, reg: u8, data: Option<u16>) -> Result<u16, Error> {
    if addr > 31 || reg > 31 {
        return Err(Error::InvalidRegister);
    }
    let command = ((addr as u32) << 21)
        | ((reg as u32) << 16)
        | match data {
            Some(v) => WRITE | v as u32,
            None => READ,
        };
    io.write(MDIC, command)?;
    for _ in 0..POLLS {
        io.delay_us(50)?;
        let v = io.read(MDIC)?;
        if v & READY == 0 {
            continue;
        }
        if v & MDIC_ERROR != 0 {
            return Err(Error::MdioError);
        }
        // Match both address and register: never accept another transaction's data.
        if v & (0x03e00000 | 0x001f0000) != command & (0x03e00000 | 0x001f0000) {
            return Err(Error::MdioMismatch);
        }
        return Ok(v as u16);
    }
    Err(Error::MdioTimeout)
}
fn read_locked(io: &mut impl Io, reg: Register) -> Result<u16, Error> {
    let addr = if reg.page >= 768 { 1 } else { 2 };
    if reg.register > 15 {
        let page = if reg.page == 768 { 0 } else { reg.page };
        mdic(io, 1, 31, Some(page << 5))?;
    }
    mdic(io, addr, reg.register, None)
}
/// A single transaction acquires/releases software ownership, including errors.
pub fn read(io: &mut impl Io, register: Register) -> Result<u16, Error> {
    acquire(io)?;
    let result = read_locked(io, register);
    release(io)?;
    result
}
/// Read both common ID registers under one ownership interval (PHY address 2).
/// Return the revision separately; reject floating/absent PHY responses.
pub fn phy_id(io: &mut impl Io) -> Result<(u32, u8), Error> {
    acquire(io)?;
    let result = (|| {
        let hi = read_locked(io, Register::new(0, 2)?)?;
        let lo = read_locked(io, Register::new(0, 3)?)?;
        if hi == 0 || hi == 0xffff || lo == 0xffff {
            return Err(Error::InvalidPhyId);
        }
        let raw = ((hi as u32) << 16) | lo as u32;
        Ok((raw & 0xfffffff0, (raw & 15) as u8))
    })();
    release(io)?;
    result
}

#[cfg(test)]
mod tests {
    extern crate std;
    use super::*;
    use std::vec::Vec;
    #[derive(Clone, Copy, PartialEq)]
    enum Mode {
        Normal,
        Denied,
        Timeout,
        MdioError,
        ReadFailure,
        Mismatch,
        ReleaseFailure,
        Lost,
    }
    struct Fake {
        ctrl: u32,
        command: u32,
        writes: Vec<(usize, u32)>,
        mode: Mode,
        delays: usize,
        delay_failure: Option<usize>,
        mdio_reads: usize,
        invalid_id: bool,
    }
    impl Fake {
        fn new(mode: Mode) -> Self {
            Self {
                ctrl: 0x80000080,
                command: 0,
                writes: Vec::new(),
                mode,
                delays: 0,
                delay_failure: None,
                mdio_reads: 0,
                invalid_id: false,
            }
        }
        fn released(&self) {
            assert_eq!(self.ctrl, 0x80000080);
        }
    }
    impl Io for Fake {
        fn read(&mut self, offset: usize) -> Result<u32, Error> {
            match offset {
                EXTCNF_CTRL => Ok(self.ctrl),
                MDIC => {
                    if self.mode == Mode::ReadFailure {
                        return Err(Error::Resource);
                    }
                    self.mdio_reads += 1;
                    if self.mode == Mode::Timeout || self.mdio_reads <= 2 {
                        return Ok(self.command);
                    }
                    if self.mode == Mode::Lost {
                        self.ctrl &= !SWFLAG;
                    }
                    let reg = (self.command >> 16) & 31;
                    let data = if self.invalid_id {
                        0xffff
                    } else {
                        match reg {
                            2 => 0x0154,
                            3 => 0x00a1,
                            _ => 0x5a5a,
                        }
                    };
                    Ok((self.command & !0xffff)
                        | READY
                        | data
                        | if self.mode == Mode::MdioError {
                            MDIC_ERROR
                        } else {
                            0
                        } ^ if self.mode == Mode::Mismatch {
                            1 << 21
                        } else {
                            0
                        })
                }
                _ => Err(Error::Resource),
            }
        }
        fn write(&mut self, offset: usize, value: u32) -> Result<(), Error> {
            self.writes.push((offset, value));
            match offset {
                EXTCNF_CTRL => {
                    if self.mode != Mode::Denied
                        && !(self.mode == Mode::ReleaseFailure && value & SWFLAG == 0)
                    {
                        self.ctrl = value;
                    }
                    Ok(())
                }
                MDIC => {
                    assert!(self.ctrl & SWFLAG != 0, "MDIO without ownership");
                    self.command = value;
                    self.mdio_reads = 0;
                    Ok(())
                }
                _ => Err(Error::Resource),
            }
        }
        fn delay_us(&mut self, us: u32) -> Result<(), Error> {
            assert!(us == 50 || us == 1000);
            self.delays += 1;
            if self.delay_failure == Some(self.delays) {
                Err(Error::Clock)
            } else {
                Ok(())
            }
        }
    }
    #[test]
    fn exact_device_classification() {
        assert_eq!(classify(0x15a38086), Some(Device::I218V3));
        assert_eq!(classify(0x15598086), Some(Device::I218V));
        assert_eq!(classify(0x15a18086), Some(Device::I218V2));
        for id in [0x15a31234, 0x15a28086, 0x15708086, 0x10d38086, 0xffff_ffff] {
            assert_eq!(classify(id), None);
        }
    }
    #[test]
    fn phy_pair_is_one_owned_transaction() {
        let mut io = Fake::new(Mode::Normal);
        assert_eq!(phy_id(&mut io), Ok((0x015400a0, 1)));
        io.released();
        assert_eq!(io.writes.len(), 4);
        assert_eq!(io.writes[1], (MDIC, READ | (2 << 21) | (2 << 16)));
        assert_eq!(io.writes[2], (MDIC, READ | (2 << 21) | (3 << 16)));
    }
    #[test]
    fn paged_access_and_alias_use_correct_phy_address() {
        for (page, encoded) in [(768, 0), (769, 769 << 5), (2047, 2047 << 5)] {
            let mut io = Fake::new(Mode::Normal);
            assert_eq!(read(&mut io, Register::new(page, 20).unwrap()), Ok(0x5a5a));
            assert_eq!(
                io.writes[1],
                (MDIC, WRITE | (1 << 21) | (31 << 16) | encoded)
            );
            assert_eq!(io.writes[2], (MDIC, READ | (1 << 21) | (20 << 16)));
            io.released();
        }
        let mut io = Fake::new(Mode::Normal);
        read(&mut io, Register::new(0, 20).unwrap()).unwrap();
        assert_eq!(io.writes[1], (MDIC, WRITE | (1 << 21) | (31 << 16)));
        assert_eq!(io.writes[2], (MDIC, READ | (2 << 21) | (20 << 16)));
    }
    #[test]
    fn unsupported_addresses_rejected_before_io() {
        for page in [1, 767, 800, 2048, u16::MAX] {
            assert!(matches!(
                Register::new(page, 20),
                Err(Error::UnsupportedPage)
            ));
        }
        assert!(matches!(Register::new(0, 32), Err(Error::InvalidRegister)));
        let mut io = Fake::new(Mode::Normal);
        assert_eq!(mdic(&mut io, 32, 2, None), Err(Error::InvalidRegister));
        assert!(io.writes.is_empty());
    }
    #[test]
    fn busy_owner_is_never_cleared() {
        let mut io = Fake::new(Mode::Normal);
        io.ctrl |= SWFLAG;
        assert_eq!(phy_id(&mut io), Err(Error::SemaphoreBusy));
        assert_eq!(io.delays, 100);
        assert!(io.writes.is_empty());
        assert!(io.ctrl & SWFLAG != 0);
    }
    #[test]
    fn denied_request_is_cancelled_and_bounded() {
        let mut io = Fake::new(Mode::Denied);
        assert_eq!(phy_id(&mut io), Err(Error::SemaphoreDenied));
        assert_eq!(io.delays, 1000);
        assert_eq!(io.writes.len(), 2);
        io.released();
    }
    #[test]
    fn mdio_errors_release_ownership() {
        for (mode, error) in [
            (Mode::Timeout, Error::MdioTimeout),
            (Mode::MdioError, Error::MdioError),
            (Mode::ReadFailure, Error::Resource),
            (Mode::Mismatch, Error::MdioMismatch),
        ] {
            let mut io = Fake::new(mode);
            assert_eq!(phy_id(&mut io), Err(error));
            io.released();
            if mode == Mode::Timeout {
                assert_eq!(io.delays, POLLS);
            }
            assert_eq!(io.writes.last(), Some(&(EXTCNF_CTRL, 0x80000080)));
        }
    }
    #[test]
    fn clock_error_after_request_cleans_up() {
        let mut denied = Fake::new(Mode::Denied);
        denied.delay_failure = Some(1);
        assert_eq!(phy_id(&mut denied), Err(Error::Clock));
        denied.released();
        let mut mdio = Fake::new(Mode::Normal);
        mdio.delay_failure = Some(1);
        assert_eq!(phy_id(&mut mdio), Err(Error::Clock));
        mdio.released();
    }
    #[test]
    fn invalid_phy_response_is_not_success() {
        let mut io = Fake::new(Mode::Normal);
        io.invalid_id = true;
        assert_eq!(phy_id(&mut io), Err(Error::InvalidPhyId));
        io.released();
    }
    #[test]
    fn release_failure_and_lost_ownership_are_reported() {
        let mut io = Fake::new(Mode::ReleaseFailure);
        assert_eq!(phy_id(&mut io), Err(Error::SemaphoreRelease));
        assert_ne!(io.ctrl & SWFLAG, 0);
        let mut lost = Fake::new(Mode::Lost);
        assert_eq!(
            read(&mut lost, Register::new(0, 2).unwrap()),
            Err(Error::OwnershipLost)
        );
        assert_eq!(lost.writes.len(), 2);
        lost.released();
    }
    #[test]
    fn failed_page_selection_does_not_issue_data_read() {
        let mut io = Fake::new(Mode::MdioError);
        assert_eq!(
            read(&mut io, Register::new(769, 20).unwrap()),
            Err(Error::MdioError)
        );
        assert_eq!(io.writes.iter().filter(|(r, _)| *r == MDIC).count(), 1);
        assert_eq!(
            io.writes[1],
            (MDIC, WRITE | (1 << 21) | (31 << 16) | (769 << 5))
        );
        io.released();
    }
}
