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
//! Read-only I218/PCH-LPT flash sequencing adapted from pinned FreeBSD
//! e1000_ich8lan.c. No write/erase cycle or fallback to an invalid bank.
use super::Error;
pub trait Io {
    fn read16(&mut self, r: usize) -> Result<u16, Error>;
    fn write16(&mut self, r: usize, v: u16) -> Result<(), Error>;
    fn read32(&mut self, r: usize) -> Result<u32, Error>;
    fn write32(&mut self, r: usize, v: u32) -> Result<(), Error>;
    fn delay_us(&mut self, us: u32) -> Result<(), Error>;
}
#[derive(Clone, Copy)]
pub struct Region {
    base: u32,
    bank_bytes: u32,
    bank: u32,
}
fn status(io: &mut impl Io) -> Result<u16, Error> {
    let v = io.read16(4)?;
    if v == u16::MAX || v & 0x4000 == 0 {
        return Err(Error::Nvm);
    }
    Ok(v)
}
fn wait_idle(io: &mut impl Io) -> Result<(), Error> {
    for _ in 0..200_000 {
        if status(io)? & 0x20 == 0 {
            return Ok(());
        }
        io.delay_us(50)?;
    }
    Err(Error::NvmTimeout)
}
fn read_bytes(io: &mut impl Io, address: u32, size: u16) -> Result<u16, Error> {
    if address > 0xffffff
        || !(1..=2).contains(&size)
        || address
            .checked_add(size as u32)
            .is_none_or(|x| x > 0x1000000)
    {
        return Err(Error::Nvm);
    }
    wait_idle(io)?;
    let s = status(io)?;
    io.write16(4, s | 7)?; // W1C error/access/done, never a flash data write.
    let ctl = io.read16(6)?;
    io.write16(6, (ctl & !0x307) | ((size - 1) << 8))?; // cycle0 READ only.
    io.write32(8, address)?;
    io.write16(6, (ctl & !0x307) | ((size - 1) << 8) | 1)?;
    for _ in 0..200_000 {
        let s = status(io)?;
        if s & 1 != 0 {
            if s & 6 != 0 {
                return Err(Error::Nvm);
            }
            return Ok(io.read32(0x10)? as u16 & if size == 1 { 255 } else { 65535 });
        }
        io.delay_us(50)?;
    }
    Err(Error::NvmTimeout)
}
impl Region {
    pub fn open(io: &mut impl Io) -> Result<Self, Error> {
        let g = io.read32(0)?;
        if g == u32::MAX {
            return Err(Error::Nvm);
        }
        let base = (g & 0x1fff) << 12;
        let end = (((g >> 16) & 0x1fff) + 1) << 12;
        let bytes = end.checked_sub(base).ok_or(Error::Nvm)?;
        if bytes < 8192 || bytes % 8192 != 0 || end > 0x1000000 {
            return Err(Error::Nvm);
        }
        let mut result = Self {
            base,
            bank_bytes: bytes / 2,
            bank: 0,
        };
        for bank in 0..2 {
            let signature = read_bytes(io, base + bank * result.bank_bytes + 0x27, 1)?;
            if signature & 0xc0 == 0x80 {
                result.bank = bank;
                return Ok(result);
            }
        }
        Err(Error::Nvm)
    }
    pub fn word(&self, io: &mut impl Io, index: u16) -> Result<u16, Error> {
        let at = (index as u32) * 2;
        if index >= 2048 || at + 2 > self.bank_bytes {
            return Err(Error::Nvm);
        }
        read_bytes(io, self.base + self.bank * self.bank_bytes + at, 2)
    }
}
