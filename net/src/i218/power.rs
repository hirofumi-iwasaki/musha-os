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
//! Bounded adaptation of the managed-ME branch of
//! e1000_disable_ulp_lpt_lp(force=true), pinned FreeBSD revision
//! 833d39bd2e38421a14ea489a956261a0fa993fbc. Not the complete PHY init flow.
//! Caller validates exact hardware/BAR and keeps PCI bus mastering disabled.
//! Software ULP, reset, link setup and DMA are deliberately not implemented.
use super::{Device, Io};
pub const H2ME: usize = 0x5b50;
pub const FWSM: usize = 0x5b54;
const FW_VALID: u32 = 0x8000;
const CFG_DONE: u32 = 0x400;
const ULP: u32 = 0x800;
const ENFORCE: u32 = 0x1000;
const POLLS: usize = 30;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Io(super::Error),
    UnsupportedDevice,
    FirmwareUnavailable,
    RequestBusy,
    FirmwareLost,
    Timeout,
    Cleanup,
}
impl Error {
    pub fn label(self) -> &'static str {
        match self {
            Self::Io(e) => e.label(),
            Self::UnsupportedDevice => "I218 POWER DEVICE UNSUPPORTED",
            Self::FirmwareUnavailable => "I218 SOFTWARE ULP REQUIRED",
            Self::RequestBusy => "I218 ME REQUEST BUSY",
            Self::FirmwareLost => "I218 ME LOST",
            Self::Timeout => "I218 ME ULP TIMEOUT",
            Self::Cleanup => "I218 ME CLEANUP UNCONFIRMED",
        }
    }
}
fn read(io: &mut impl Io, offset: usize) -> Result<u32, Error> {
    let v = io.read(offset).map_err(Error::Io)?;
    if v == u32::MAX {
        return Err(Error::Io(super::Error::Resource));
    }
    Ok(v)
}
fn cleanup(io: &mut impl Io) -> Result<(), Error> {
    let value = read(io, H2ME).map_err(|_| Error::Cleanup)?;
    io.write(H2ME, value & !ENFORCE)
        .map_err(|_| Error::Cleanup)?;
    if read(io, H2ME).map_err(|_| Error::Cleanup)? & ENFORCE != 0 {
        return Err(Error::Cleanup);
    }
    Ok(())
}
/// Exit ULP through ME only. At most300ms of requested delay; Io itself must
/// be finite. No writes before exact V3/firmware/request checks. Once our write
/// is attempted every ordinary error attempts cleanup; cleanup failure wins.
/// This clears the owned ENFORCE request even on failure, preserving other bits.
/// It cannot undo a firmware power transition or guarantee cleanup after panic.
pub fn managed_exit(io: &mut impl Io, device: Device) -> Result<(), Error> {
    if device != Device::I218V3 {
        return Err(Error::UnsupportedDevice);
    }
    if read(io, FWSM)? & FW_VALID == 0 {
        return Err(Error::FirmwareUnavailable);
    }
    let request = read(io, H2ME)?;
    if request & ENFORCE != 0 {
        return Err(Error::RequestBusy);
    }
    let result = (|| {
        io.write(H2ME, (request & !ULP) | ENFORCE)
            .map_err(Error::Io)?;
        for poll in 0..=POLLS {
            let status = read(io, FWSM)?;
            if status & FW_VALID == 0 {
                return Err(Error::FirmwareLost);
            }
            if status & CFG_DONE == 0 {
                return Ok(());
            }
            if poll == POLLS {
                return Err(Error::Timeout);
            }
            io.delay_us(10_000).map_err(Error::Io)?;
        }
        unreachable!()
    })();
    cleanup(io)?;
    result
}
