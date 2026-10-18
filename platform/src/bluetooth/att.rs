// Copyright 2026 Hirofumi Iwasaki
// SPDX-License-Identifier: Apache-2.0
//! Bounded ATT discovery/notification parsing and authenticated-session binding.
//! No characteristic is trusted from device names or hardcoded Xbox handles.
use super::{Error, word};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Service {
    pub start: u16,
    pub end: u16,
    pub uuid16: Option<u16>,
}
pub fn services(p: &[u8], mut emit: impl FnMut(Service)) -> Result<(), Error> {
    if p.len() < 2 || p[0] != 0x11 || !matches!(p[1], 6 | 20) {
        return Err(Error::Unsupported);
    }
    let n = p[1] as usize;
    if p.len() == 2 || (p.len() - 2) % n != 0 || p.len() > 517 {
        return Err(Error::Length);
    }
    let mut previous = 0;
    for b in p[2..].chunks_exact(n) {
        let start = word(b);
        let end = word(&b[2..]);
        if start == 0 || start > end || start <= previous {
            return Err(Error::Invalid);
        }
        previous = end;
    }
    for b in p[2..].chunks_exact(n) {
        emit(Service {
            start: word(b),
            end: word(&b[2..]),
            uuid16: if n == 6 { Some(word(&b[4..])) } else { None },
        });
    }
    Ok(())
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Characteristic {
    pub declaration: u16,
    pub properties: u8,
    pub value: u16,
    pub uuid16: Option<u16>,
}
pub fn characteristics(
    p: &[u8],
    start: u16,
    end: u16,
    mut emit: impl FnMut(Characteristic),
) -> Result<(), Error> {
    if p.len() < 2 || p[0] != 9 || !matches!(p[1], 7 | 21) {
        return Err(Error::Unsupported);
    }
    let n = p[1] as usize;
    if p.len() == 2 || (p.len() - 2) % n != 0 || p.len() > 517 {
        return Err(Error::Length);
    }
    let mut previous = 0;
    for b in p[2..].chunks_exact(n) {
        let d = word(b);
        let v = word(&b[3..]);
        if d < start || d <= previous || v <= d || v > end {
            return Err(Error::Invalid);
        }
        previous = v;
    }
    for b in p[2..].chunks_exact(n) {
        emit(Characteristic {
            declaration: word(b),
            properties: b[2],
            value: word(&b[3..]),
            uuid16: if n == 7 { Some(word(&b[5..])) } else { None },
        });
    }
    Ok(())
}
/// HOGP input report binding after encrypted pairing, report-reference/type and
/// report-map validation. Generic HID report decoding remains caller-owned.
pub struct InputReport {
    generation: u64,
    connection: u16,
    value: u16,
    id: u8,
    bytes: usize,
}
#[cfg(feature = "bluetooth-legacy")]
impl InputReport {
    pub fn bind(
        bond: &super::legacy::VerifiedBond,
        value: u16,
        reference: &[u8],
        report_bytes: usize,
        properties: u8,
    ) -> Result<Self, Error> {
        if value == 0
            || reference.len() != 2
            || reference[1] != 1
            || report_bytes == 0
            || report_bytes > 512
            || properties & 0x10 == 0
        {
            return Err(Error::Invalid);
        }
        Ok(Self {
            generation: bond.generation,
            connection: bond.handle,
            value,
            id: reference[0],
            bytes: report_bytes,
        })
    }
    pub fn notification<'a>(
        &self,
        generation: u64,
        connection: u16,
        p: &'a [u8],
    ) -> Result<(u8, &'a [u8]), Error> {
        if generation != self.generation || connection != self.connection {
            return Err(Error::Stale);
        }
        if p.len() < 3 || p[0] != 0x1b || word(&p[1..]) != self.value {
            return Err(Error::Invalid);
        }
        if p.len() != 3 + self.bytes {
            return Err(Error::Length);
        }
        Ok((self.id, &p[3..]))
    }
}

/// This central exposes no GATT database. A peer *request* must still be
/// answered, or the peer's ATT transaction times out and it disconnects.
/// Returns the reply for a request opcode; `None` for responses, notifications,
/// indications and commands (nothing is sent for those).
pub fn peer_request_reply(p: &[u8]) -> Option<([u8; 5], usize)> {
    let opcode = *p.first()?;
    let handle = if p.len() >= 3 { word(&p[1..]) } else { 0 };
    let error = |code: u8| {
        let h = handle.to_le_bytes();
        Some(([0x01, opcode, h[0], h[1], code], 5))
    };
    match opcode {
        // Exchange MTU: keep the default 23-byte MTU.
        0x02 => Some(([0x03, 23, 0, 0, 0], 3)),
        // Find Information, Find By Type Value, Read By Type, Read By Group Type.
        0x04 | 0x06 | 0x08 | 0x10 => error(0x0a),
        // Other requests (reads, writes, prepare/execute): Request Not Supported.
        0x0a | 0x0c | 0x0e | 0x12 | 0x16 | 0x18 | 0x20 => error(0x06),
        _ => None,
    }
}
