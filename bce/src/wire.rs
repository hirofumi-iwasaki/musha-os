// Copyright (c) 2026 Abdelkader Boudih <freebsd@seuros.com>
// Copyright (c) 2026 Hirofumi Iwasaki (Rust adaptation)
// SPDX-License-Identifier: BSD-2-Clause
//! Explicit little-endian encoding; never cast packed data to Rust structs.
use crate::Error;
pub const PROTOCOL_VERSION: u64 = 0x20001;
pub const SET_PROTOCOL: u8 = 0x0c;
pub const PENDING: u16 = 0x8000;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Message(u64);
impl Message {
    pub fn new(kind: u8, value: u64) -> Result<Self, Error> {
        if kind > 63 || value >= 1 << 58 {
            return Err(Error::Invalid);
        }
        Ok(Self(((kind as u64) << 58) | value))
    }
    pub fn from_raw(raw: u64) -> Self {
        Self(raw)
    }
    pub fn raw(self) -> u64 {
        self.0
    }
    pub fn kind(self) -> u8 {
        (self.0 >> 58) as u8
    }
    pub fn value(self) -> u64 {
        self.0 & ((1 << 58) - 1)
    }
    /// Four DWORD mailbox writes: low, high, zero, zero.
    pub fn words(self) -> [u32; 4] {
        [self.0 as u32, (self.0 >> 32) as u32, 0, 0]
    }
}
/// Structural address check only. Memory-map/ownership checks belong to adapter.
fn range(address: u64, length: u64) -> Result<(), Error> {
    if address == 0 || address & 3 != 0 || length == 0 || address.checked_add(length).is_none() {
        return Err(Error::Invalid);
    }
    Ok(())
}
pub fn submission(address: u64, length: u64) -> Result<[u8; 32], Error> {
    range(address, length)?;
    let mut b = [0; 32];
    b[..8].copy_from_slice(&length.to_le_bytes());
    b[8..16].copy_from_slice(&address.to_le_bytes());
    // No scatter/gather in this subset.
    Ok(b)
}
pub fn queue_memory(
    qid: u16,
    count: u16,
    vector_or_cq: u16,
    address: u64,
    stride: u64,
) -> Result<[u8; 24], Error> {
    if qid >= 256 || !(2..=256).contains(&count) || ![24, 32, 64].contains(&stride) {
        return Err(Error::Invalid);
    }
    let length = (count as u64).checked_mul(stride).ok_or(Error::Invalid)?;
    range(address, length)?;
    let mut b = [0; 24];
    b[..2].copy_from_slice(&qid.to_le_bytes());
    b[2..4].copy_from_slice(&count.to_le_bytes());
    b[4..6].copy_from_slice(&vector_or_cq.to_le_bytes());
    b[8..16].copy_from_slice(&address.to_le_bytes());
    b[16..].copy_from_slice(&length.to_le_bytes());
    Ok(b)
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Completion {
    pub result: u64,
    pub data_size: u64,
    pub qid: u16,
    pub index: u16,
    pub status: u16,
    pub flags: u16,
}
impl Completion {
    pub fn decode(b: &[u8]) -> Result<Self, Error> {
        let b: &[u8; 24] = b.try_into().map_err(|_| Error::Length)?;
        Ok(Self {
            result: u64::from_le_bytes(b[..8].try_into().unwrap()),
            data_size: u64::from_le_bytes(b[8..16].try_into().unwrap()),
            qid: u16::from_le_bytes([b[16], b[17]]),
            index: u16::from_le_bytes([b[18], b[19]]),
            status: u16::from_le_bytes([b[20], b[21]]),
            flags: u16::from_le_bytes([b[22], b[23]]),
        })
    }
}

/// User queue registration payload, matching FreeBSD bce_cmdq_reg_cmd.
/// Structural validation only; allocation ownership and CQ dependencies are external.
#[derive(Clone, Copy)]
pub struct RegistrationConfig<'a> {
    pub qid: u16,
    pub count: u16,
    pub vector_or_cq: u16,
    pub address: u64,
    pub stride: u64,
    pub name: Option<&'a [u8]>,
    pub out: bool,
}
pub fn register_queue(c: RegistrationConfig<'_>) -> Result<[u8; 64], Error> {
    if !(2..256).contains(&c.qid) {
        return Err(Error::QueueId);
    }
    let cfg = queue_memory(c.qid, c.count, c.vector_or_cq, c.address, c.stride)?;
    let mut b = [0; 64];
    b[0] = 0x20;
    b[2] = u8::from(c.out);
    b[4..6].copy_from_slice(&cfg[0..2]);
    b[8..12].copy_from_slice(&cfg[2..6]);
    if let Some(name) = c.name {
        // Reject rather than silently truncate an ambiguous queue name.
        if name.is_empty() || name.len() > 32 || name.contains(&0) {
            return Err(Error::Length);
        }
        b[2] |= 2;
        b[14..16].copy_from_slice(&(name.len() as u16).to_le_bytes());
        b[16..16 + name.len()].copy_from_slice(name);
    }
    b[48..64].copy_from_slice(&cfg[8..24]);
    Ok(b)
}
/// Full 64-byte command slot, including zeroed padding. true=flush, false=unregister.
pub fn simple_queue_command(qid: u16, flush: bool) -> Result<[u8; 64], Error> {
    if !(2..256).contains(&qid) {
        return Err(Error::QueueId);
    }
    let mut b = [0; 64];
    b[0] = if flush { 0x40 } else { 0x30 };
    b[4..6].copy_from_slice(&qid.to_le_bytes());
    Ok(b)
}
