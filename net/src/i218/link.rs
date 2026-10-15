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
//! I218/i217 PHY link and hardware bits, pinned FreeBSD
//! e1000_initialize_hw_bits_ich8lan/setup_copper_link_pch_lpt and
//! e1000_copper_link_setup_82577/phy_setup_autoneg. Full-duplex Ethernet,
//! flow-control-none policy, software checksums and legacy descriptors only.
use super::{Error, Register, reset};
fn read(io: &mut impl reset::Io, r: usize) -> Result<u32, Error> {
    let v = io.read(r)?;
    if v == u32::MAX {
        return Err(Error::Resource);
    }
    Ok(v)
}
fn update(io: &mut impl reset::Io, r: usize, clear: u32, set: u32) -> Result<(), Error> {
    let v = read(io, r)?;
    io.write(r, (v & !clear) | set)
}
fn phy(io: &mut impl reset::Io, r: u8, clear: u16, set: u16) -> Result<(), Error> {
    let reg = Register::new(0, r)?;
    let v = super::read_locked(io, reg)?;
    reset::phy_write(io, reg, (v & !clear) | set)
}
pub fn initialize(io: &mut impl reset::Io) -> Result<[u8; 6], Error> {
    if io.pci_id()? != 0x15a38086 || io.bus_master()? {
        return Err(Error::Resource);
    }
    update(io, 0x18, 0, (1 << 22) | (1 << 20) | (1 << 17))?;
    for r in [0x3828, 0x3928] {
        update(io, r, 0x003f003f, (1 << 22) | 0x0101001f)?;
    }
    update(io, 0x3840, 0, (1 << 23) | (1 << 24) | (1 << 26) | (1 << 27))?;
    let multi = read(io, 0x400)? & (1 << 28) != 0;
    update(
        io,
        0x3940,
        1 << 28,
        (1 << 24) | (1 << 26) | (1 << 30) | if multi { 0 } else { 1 << 28 },
    )?;
    update(io, 0x5008, 0, 0xc0)?;
    update(io, 0x100c, 0, 1 << 16)?;
    update(
        io,
        0,
        4 | (1 << 11) | (1 << 12) | (1 << 27) | (1 << 28),
        (1 << 6) | (1 << 19),
    )?;
    // No pause frames advertised; preserve unrelated PHY control bits.
    super::acquire(io)?;
    let result = (|| {
        phy(io, 22, 0, 0x8c00)?;
        phy(io, 18, 0x600, 0x400)?;
        phy(io, 4, 0xde0, 0x140)?; // 10/100 full only; no pause/half.
        phy(io, 9, 0x1300, 0x200)?; // 1000 full, automatic master/slave.
        phy(io, 0, 0x0c00, 0x1200)?; // autoneg enable/restart, no powerdown/isolate.
        Ok(())
    })();
    super::release(io)?;
    result?;
    io.write(0x2160, 0)?;
    io.write(0x2168, 0)?;
    io.write(0x170, 0)?;
    let mut up = false;
    for _ in 0..600 {
        let status = read(io, 8)?;
        if status & 2 != 0 {
            if status & 1 == 0 {
                return Err(Error::LinkMode);
            }
            up = true;
            break;
        }
        io.delay_us(5000)?;
    }
    if !up {
        return Err(Error::LinkTimeout);
    }
    let lo = read(io, 0x5400)?;
    let hi = read(io, 0x5404)?;
    let mac = [
        lo as u8,
        (lo >> 8) as u8,
        (lo >> 16) as u8,
        (lo >> 24) as u8,
        hi as u8,
        (hi >> 8) as u8,
    ];
    if hi & (1 << 31) == 0 || mac[0] & 1 != 0 || mac == [0; 6] || mac == [255; 6] {
        return Err(Error::InvalidMac);
    }
    if io.bus_master()? {
        return Err(Error::Resource);
    }
    Ok(mac)
}

/// Caller first clears and reads back PCI BME. Confirm device master requests
/// drained before releasing buffers. A finite observation budget fails closed.
pub fn stop_master(io: &mut impl super::Io, bme_off: bool) -> Result<(), Error> {
    if !bme_off {
        return Err(Error::Resource);
    }
    io.write(0xd8, u32::MAX)?;
    io.write(0x100, 0)?;
    io.write(0x400, 0)?;
    let ctrl = io.read(0)?;
    if ctrl == u32::MAX {
        return Err(Error::Resource);
    }
    io.write(0, ctrl | 4)?;
    for _ in 0..1_000_000 {
        let status = io.read(8)?;
        if status == u32::MAX {
            return Err(Error::Resource);
        }
        if status & (1 << 19) == 0 {
            return Ok(());
        }
    }
    Err(Error::MasterPending)
}
