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
