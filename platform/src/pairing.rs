// Copyright 2026 Hirofumi Iwasaki
// SPDX-License-Identifier: Apache-2.0
//! Bounded LE bonding record codec. This is persistence format, not an SMP stack.
//! CRC detects accidental corruption; it does not authenticate an untrusted drive.
//! Only recognized records are writable; this marker is not cryptographic ownership.
pub const CAPACITY: usize = 4096;
pub const MAX_BONDS: usize = 8;
const HEADER: usize = 64;
const RECORD: usize = 40;
const MAGIC: &[u8; 8] = b"MUSHALTK";
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Bounds,
    Version,
    Corrupt,
    Duplicate,
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Bond {
    /// LE public (0) or random (1) identity address, HCI byte order.
    pub address_type: u8,
    pub address: [u8; 6],
    pub key_size: u8,
    /// Security flags: bit0 authenticated; bit1 LE Secure Connections.
    pub flags: u8,
    pub ediv: u16,
    pub random: [u8; 8],
    pub ltk: [u8; 16],
}
impl core::fmt::Debug for Bond {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Bond")
            .field("address_type", &self.address_type)
            .field("key_size", &self.key_size)
            .field("flags", &self.flags)
            .field("key_material", &"REDACTED")
            .finish()
    }
}
fn crc(bytes: &[u8]) -> u32 {
    let mut c = !0u32;
    for &byte in bytes {
        c ^= byte as u32;
        for _ in 0..8 {
            c = (c >> 1) ^ (0xedb88320 & 0u32.wrapping_sub(c & 1));
        }
    }
    !c
}
fn valid(b: &Bond) -> bool {
    b.address_type <= 1
        && (7..=16).contains(&b.key_size)
        && b.flags & !3 == 0
        && b.address != [0; 6]
        && b.ltk != [0; 16]
}
/// Always writes a complete fixed-capacity file; unused bytes are zero and checked.
pub fn encode(bonds: &[Bond], out: &mut [u8]) -> Result<(), Error> {
    encode_state(bonds, None, out)
}
pub fn encode_state(
    bonds: &[Bond],
    consumed: Option<[u8; 32]>,
    out: &mut [u8],
) -> Result<(), Error> {
    if out.len() != CAPACITY || bonds.len() > MAX_BONDS {
        return Err(Error::Bounds);
    }
    for (i, bond) in bonds.iter().enumerate() {
        if !valid(bond) {
            return Err(Error::Corrupt);
        }
        if bonds[..i]
            .iter()
            .any(|b| b.address_type == bond.address_type && b.address == bond.address)
        {
            return Err(Error::Duplicate);
        }
    }
    out.fill(0);
    out[..8].copy_from_slice(MAGIC);
    out[8..10].copy_from_slice(&1u16.to_le_bytes());
    out[10..12].copy_from_slice(&(HEADER as u16).to_le_bytes());
    out[12..14].copy_from_slice(&(bonds.len() as u16).to_le_bytes());
    out[14..16].copy_from_slice(&(RECORD as u16).to_le_bytes());
    out[16..20].copy_from_slice(&((bonds.len() * RECORD) as u32).to_le_bytes());
    if let Some(hash) = consumed {
        out[24..28].copy_from_slice(&1u32.to_le_bytes());
        out[28..60].copy_from_slice(&hash);
    }
    for (bond, b) in bonds.iter().zip(out[HEADER..].chunks_exact_mut(RECORD)) {
        b[0] = bond.address_type;
        b[1] = bond.key_size;
        b[2] = bond.flags;
        b[4..10].copy_from_slice(&bond.address);
        b[10..12].copy_from_slice(&bond.ediv.to_le_bytes());
        b[12..20].copy_from_slice(&bond.random);
        b[20..36].copy_from_slice(&bond.ltk);
    }
    let checksum = crc(out);
    out[20..24].copy_from_slice(&checksum.to_le_bytes());
    Ok(())
}
pub fn decode(bytes: &[u8], out: &mut [Bond]) -> Result<usize, Error> {
    decode_state(bytes, out).map(|(n, _)| n)
}
pub fn decode_state(bytes: &[u8], out: &mut [Bond]) -> Result<(usize, Option<[u8; 32]>), Error> {
    if bytes.len() != CAPACITY {
        return Err(Error::Bounds);
    }
    if &bytes[..8] != MAGIC {
        return Err(Error::Corrupt);
    }
    if bytes[8..10] != 1u16.to_le_bytes() {
        return Err(Error::Version);
    }
    let consumed = match u32::from_le_bytes(bytes[24..28].try_into().unwrap()) {
        0 if bytes[28..60] == [0; 32] => None,
        1 => Some(bytes[28..60].try_into().unwrap()),
        _ => return Err(Error::Corrupt),
    };
    if bytes[60..64] != [0; 4] {
        return Err(Error::Corrupt);
    }
    let count = u16::from_le_bytes([bytes[12], bytes[13]]) as usize;
    if count > MAX_BONDS || out.len() < count {
        return Err(Error::Bounds);
    }
    let length = HEADER + count * RECORD;
    if bytes[10..12] != (HEADER as u16).to_le_bytes()
        || bytes[14..16] != (RECORD as u16).to_le_bytes()
        || bytes[16..20] != ((count * RECORD) as u32).to_le_bytes()
        || bytes[length..].iter().any(|b| *b != 0)
    {
        return Err(Error::Corrupt);
    }
    let mut copy = [0; CAPACITY];
    copy.copy_from_slice(bytes);
    copy[20..24].fill(0);
    if crc(&copy) != u32::from_le_bytes(bytes[20..24].try_into().unwrap()) {
        return Err(Error::Corrupt);
    }
    // Validate the complete input before exposing even the first key.
    for (i, b) in bytes[HEADER..length].chunks_exact(RECORD).enumerate() {
        let bond = Bond {
            address_type: b[0],
            key_size: b[1],
            flags: b[2],
            address: b[4..10].try_into().unwrap(),
            ediv: u16::from_le_bytes([b[10], b[11]]),
            random: b[12..20].try_into().unwrap(),
            ltk: b[20..36].try_into().unwrap(),
        };
        if !valid(&bond) || b[3] != 0 || b[36..].iter().any(|b| *b != 0) {
            return Err(Error::Corrupt);
        }
        for previous in bytes[HEADER..HEADER + i * RECORD].chunks_exact(RECORD) {
            if previous[0] == b[0] && previous[4..10] == b[4..10] {
                return Err(Error::Duplicate);
            }
        }
    }
    for (slot, b) in out
        .iter_mut()
        .zip(bytes[HEADER..length].chunks_exact(RECORD))
    {
        *slot = Bond {
            address_type: b[0],
            key_size: b[1],
            flags: b[2],
            address: b[4..10].try_into().unwrap(),
            ediv: u16::from_le_bytes([b[10], b[11]]),
            random: b[12..20].try_into().unwrap(),
            ltk: b[20..36].try_into().unwrap(),
        };
    }
    Ok((count, consumed))
}
/// Validation used to recognize the OS-created PAIRING.DAT format before overwrite.
pub fn recognized(bytes: &[u8]) -> bool {
    let mut bonds = [Bond {
        address_type: 0,
        address: [0; 6],
        key_size: 0,
        flags: 0,
        ediv: 0,
        random: [0; 8],
        ltk: [0; 16],
    }; MAX_BONDS];
    let valid = decode_state(bytes, &mut bonds).is_ok();
    clear_keys(&mut bonds);
    valid
}
fn clear_keys(bonds: &mut [Bond]) {
    for bond in bonds {
        for byte in &mut bond.ltk {
            unsafe {
                core::ptr::write_volatile(byte, 0);
            }
        }
    }
}
/// Read consumption while validating the complete record; temporary key copies
/// are wiped before return. This does not authenticate an untrusted medium.
pub fn consumed_config_hash(bytes: &[u8]) -> Result<Option<[u8; 32]>, Error> {
    let mut bonds = [Bond {
        address_type: 0,
        address: [0; 6],
        key_size: 0,
        flags: 0,
        ediv: 0,
        random: [0; 8],
        ltk: [0; 16],
    }; MAX_BONDS];
    let result = decode_state(bytes, &mut bonds).map(|(_, hash)| hash);
    clear_keys(&mut bonds);
    result
}
/// Number of bonds in a fully validated record; key copies are wiped.
pub fn bond_count(bytes: &[u8]) -> Result<usize, Error> {
    let mut bonds = [Bond {
        address_type: 0,
        address: [0; 6],
        key_size: 0,
        flags: 0,
        ediv: 0,
        random: [0; 8],
        ltk: [0; 16],
    }; MAX_BONDS];
    let result = decode_state(bytes, &mut bonds).map(|(n, _)| n);
    clear_keys(&mut bonds);
    result
}
/// Only exact raw CONFIG bytes are compared. Re-saving identical content does not
/// rearm pairing. Missing/malformed CONFIG never supplies a hash to this policy.
pub fn requested(pair: bool, raw: Option<[u8; 32]>, consumed: Option<[u8; 32]>) -> bool {
    pair && raw.is_some() && raw != consumed
}
/// Publish consumption only after pairing succeeds AND the record is durable.
/// A failed save leaves the old RAM state. A write/sync failure may already have
/// changed disk bytes; the next boot must inspect the actual validated record.
pub fn acknowledge(
    old: Option<[u8; 32]>,
    candidate: [u8; 32],
    paired: bool,
    durable: bool,
) -> Option<[u8; 32]> {
    if paired && durable {
        Some(candidate)
    } else {
        old
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    const B: Bond = Bond {
        address_type: 0,
        address: [1; 6],
        key_size: 16,
        flags: 2,
        ediv: 7,
        random: [2; 8],
        ltk: [3; 16],
    };
    #[test]
    fn roundtrip_empty_and_limit() {
        let mut bytes = [0; CAPACITY];
        let mut out = [B; MAX_BONDS];
        encode(&[], &mut bytes).unwrap();
        assert_eq!(decode(&bytes, &mut out), Ok(0));
        assert_eq!(&bytes[16..20], &0u32.to_le_bytes());
        let mut bonds = [B; MAX_BONDS];
        for (i, b) in bonds.iter_mut().enumerate() {
            b.address[0] = i as u8;
        }
        encode(&bonds, &mut bytes).unwrap();
        assert_eq!(decode(&bytes, &mut out), Ok(MAX_BONDS));
        assert_eq!(out, bonds);
        assert_eq!(&bytes[16..20], &320u32.to_le_bytes());
        assert_eq!(encode(&[B; MAX_BONDS + 1], &mut bytes), Err(Error::Bounds));
        assert_eq!(encode(&[B, B], &mut bytes), Err(Error::Duplicate));
    }
    #[test]
    fn consumption_requires_success_and_durability() {
        let a = crate::config_hash::sha256(b"pair=on\n");
        let b = crate::config_hash::sha256(b"pair=on\n#again\n");
        assert!(requested(true, Some(a), None));
        for (paired, durable) in [(false, false), (false, true), (true, false)] {
            assert_eq!(acknowledge(None, a, paired, durable), None);
            assert!(requested(
                true,
                Some(a),
                acknowledge(None, a, paired, durable)
            ));
        }
        let saved = acknowledge(None, a, true, true);
        assert!(!requested(true, Some(a), saved));
        assert!(requested(true, Some(b), saved));
        assert!(!requested(false, Some(b), saved));
        assert!(!requested(true, None, saved));
        let mut bytes = [0; CAPACITY];
        encode_state(&[B], saved, &mut bytes).unwrap();
        assert_eq!(decode_state(&bytes, &mut [B; MAX_BONDS]), Ok((1, saved)));
        assert!(recognized(&bytes));
    }
    #[test]
    fn corruption_is_rejected_before_output() {
        let mut good = [0; CAPACITY];
        encode(&[B], &mut good).unwrap();
        for i in 0..CAPACITY {
            let mut bad = good;
            bad[i] ^= 1;
            let mut out = [B; MAX_BONDS];
            assert!(decode(&bad, &mut out).is_err(), "byte {i}");
            assert_eq!(out, [B; MAX_BONDS]);
        }
        assert_eq!(
            decode(&good[..CAPACITY - 1], &mut [B; MAX_BONDS]),
            Err(Error::Bounds)
        );
        assert_eq!(decode(&good, &mut []), Err(Error::Bounds));
    }
}
