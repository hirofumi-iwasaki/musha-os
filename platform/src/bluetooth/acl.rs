// Copyright 2026 Hirofumi Iwasaki
// SPDX-License-Identifier: Apache-2.0
//! One bounded LE connection reassembly/credit domain. Caller never reuses a
//! generation until old transport DMA has been confirmed stopped.
use super::{Acl, Error, word};
pub const MAX_SDU: usize = 1024;
pub struct Reassembly {
    generation: u64,
    handle: u16,
    buffer: [u8; MAX_SDU],
    used: usize,
    expected: usize,
    channel: u16,
    deadline: Option<u64>,
    last: u64,
}
impl Reassembly {
    pub fn new(generation: u64, handle: u16) -> Result<Self, Error> {
        if handle > 0x0eff {
            return Err(Error::Invalid);
        }
        Ok(Self {
            generation,
            handle,
            buffer: [0; MAX_SDU],
            used: 0,
            expected: 0,
            channel: 0,
            deadline: None,
            last: 0,
        })
    }
    pub fn clear(&mut self) {
        self.buffer.fill(0);
        self.used = 0;
        self.expected = 0;
        self.deadline = None;
    }
    pub fn receive(
        &mut self,
        generation: u64,
        now: u64,
        packet: &[u8],
        mut emit: impl FnMut(u16, &[u8]),
    ) -> Result<bool, Error> {
        if generation != self.generation {
            return Err(Error::Stale);
        }
        if now < self.last || self.deadline.is_some_and(|d| now >= d) {
            self.clear();
            return Err(Error::Timeout);
        }
        self.last = now;
        let acl = Acl::parse(packet)?;
        if acl.handle != self.handle {
            return Err(Error::Stale);
        }
        let payload = if acl.boundary == 1 {
            if self.deadline.is_none() {
                return Err(Error::Invalid);
            }
            acl.payload
        } else {
            if self.deadline.is_some() {
                self.clear();
                return Err(Error::Busy);
            }
            if acl.payload.len() < 4 {
                return Err(Error::Length);
            }
            let expected = word(acl.payload) as usize;
            let channel = word(&acl.payload[2..4]);
            if expected == 0 || expected > MAX_SDU || !matches!(channel, 4 | 6) {
                return Err(Error::Unsupported);
            }
            self.expected = expected;
            self.channel = channel;
            self.used = 0;
            self.deadline = Some(now.checked_add(1000).ok_or(Error::Invalid)?);
            &acl.payload[4..]
        };
        if payload.is_empty() || payload.len() > self.expected - self.used {
            self.clear();
            return Err(Error::Length);
        }
        self.buffer[self.used..self.used + payload.len()].copy_from_slice(payload);
        self.used += payload.len();
        if self.used != self.expected {
            return Ok(false);
        }
        emit(self.channel, &self.buffer[..self.used]);
        self.clear();
        Ok(true)
    }
}
pub struct Credits {
    generation: u64,
    handle: u16,
    maximum: u16,
    outstanding: u16,
    packet_bytes: u16,
}
impl Credits {
    pub fn new(
        generation: u64,
        handle: u16,
        maximum: u16,
        packet_bytes: u16,
    ) -> Result<Self, Error> {
        if handle > 0x0eff || maximum == 0 || packet_bytes < 27 || packet_bytes > 1024 {
            return Err(Error::Invalid);
        }
        Ok(Self {
            generation,
            handle,
            maximum,
            outstanding: 0,
            packet_bytes,
        })
    }
    /// Reserve exactly once before actual USB submission. Ambiguous send failure
    /// tears down the link, rather than refunding credit and duplicating a packet.
    pub fn reserve(&mut self, generation: u64, bytes: usize) -> Result<(), Error> {
        if generation != self.generation {
            return Err(Error::Stale);
        }
        if bytes == 0 || bytes > self.packet_bytes as usize {
            return Err(Error::Length);
        }
        if self.outstanding == self.maximum {
            return Err(Error::Busy);
        }
        self.outstanding += 1;
        Ok(())
    }
    pub fn completed(&mut self, generation: u64, event: super::Event<'_>) -> Result<(), Error> {
        if generation != self.generation {
            return Err(Error::Stale);
        }
        if event.code != 0x13 {
            return Err(Error::Unsupported);
        }
        let p = event.parameters;
        let n = *p.first().ok_or(Error::Length)? as usize;
        if n == 0 || p.len() != 1 + n * 4 {
            return Err(Error::Length);
        }
        let mut count = None;
        for (i, b) in p[1..].chunks_exact(4).enumerate() {
            let handle = word(b);
            if handle > 0x0eff
                || p[1..1 + i * 4]
                    .chunks_exact(4)
                    .any(|old| word(old) == handle)
            {
                return Err(Error::Invalid);
            }
            if handle == self.handle {
                count = Some(word(&b[2..]));
            }
        }
        if let Some(n) = count {
            if n > self.outstanding {
                return Err(Error::Invalid);
            }
            self.outstanding -= n;
        }
        Ok(())
    }
}
/// Decode HCI return parameters, including status. LE zero packet count requests
/// the standard Read Buffer Size fallback for a dual-mode shared ACL pool.
/// BR/EDR links remain unsupported; the single LE connection owns these credits.
pub fn capacity(opcode:u16, result:&[u8])->Result<Option<(u16,u16)>,Error>{
    let expected=match opcode{0x2002=>4,0x1005=>8,_=>return Err(Error::Unsupported)};
    if result.len()!=expected{return Err(Error::Length);}
    if result[0]!=0{return Err(Error::Controller(result[0]));}
    let bytes=u16::from_le_bytes([result[1],result[2]]);
    let packets=if opcode==0x2002{result[3]as u16}else{u16::from_le_bytes([result[4],result[5]])};
    if opcode==0x2002 && packets==0{return Ok(None);}
    if !(27..=1024).contains(&bytes)||packets==0{return Err(Error::Invalid);}
    Ok(Some((bytes,packets)))
}
