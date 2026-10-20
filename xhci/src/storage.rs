// Copyright 2026 Hirofumi Iwasaki
// SPDX-License-Identifier: Apache-2.0
//! Read-only BOT/SCSI encoding and bounded descriptor validation.
use crate::Speed;
pub mod transaction;
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Endpoint {
    pub number: u8,
    pub packet: u16,
    pub burst: u8,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Storage {
    pub configuration: u8,
    pub interface: u8,
    pub input: Endpoint,
    pub output: Endpoint,
}
pub fn configuration(b: &[u8], speed: Speed) -> Result<Option<Storage>, &'static str> {
    if b.len() < 9
        || b.len() > 1024
        || b[0] != 9
        || b[1] != 2
        || b[5] == 0
        || u16::from_le_bytes([b[2], b[3]]) as usize != b.len()
    {
        return Err("STORAGE CONFIG");
    }
    let mut pos = 9;
    let mut active = None;
    let mut input = None;
    let mut output = None;
    let mut count = 0;
    let mut expected = 0;
    let mut found = None;
    while pos < b.len() {
        if pos + 2 > b.len() {
            return Err("STORAGE TRUNCATED");
        }
        let len = b[pos] as usize;
        if len < 2 || pos + len > b.len() {
            return Err("STORAGE LENGTH");
        }
        let d = &b[pos..pos + len];
        if d[1] == 4 {
            if active.is_some() {
                if count != expected || input.is_none() || output.is_none() || found.is_some() {
                    return Err("STORAGE INTERFACE");
                }
                found = Some(Storage {
                    configuration: b[5],
                    interface: active.unwrap(),
                    input: input.unwrap(),
                    output: output.unwrap(),
                });
            }
            if len != 9 {
                return Err("STORAGE INTERFACE");
            }
            active = if d[3] == 0 && d[5..8] == [8, 6, 0x50] {
                Some(d[2])
            } else {
                None
            };
            input = None;
            output = None;
            count = 0;
            expected = d[4];
        } else if d[1] == 5 && active.is_some() {
            if len != 7 || d[2] & 0x70 != 0 || d[2] & 15 == 0 || d[3] != 2 {
                return Err("STORAGE ENDPOINT");
            }
            count += 1;
            let packet = u16::from_le_bytes([d[4], d[5]]);
            if !match speed {
                Speed::Full => matches!(packet, 8 | 16 | 32 | 64),
                Speed::High => packet == 512,
                Speed::Super => packet == 1024,
                _ => false,
            } {
                return Err("STORAGE PACKET");
            }
            let mut burst = 0;
            if matches!(speed, Speed::Super) {
                let next = pos + len;
                if next + 6 > b.len()
                    || b[next..next + 2] != [6, 48]
                    || b[next + 2] > 15
                    || b[next + 3] != 0
                    || b[next + 4] != 0
                    || b[next + 5] != 0
                {
                    return Err("STORAGE COMPANION");
                }
                burst = b[next + 2];
            }
            let endpoint = Endpoint {
                number: d[2] & 15,
                packet,
                burst,
            };
            let target = if d[2] & 128 != 0 {
                &mut input
            } else {
                &mut output
            };
            if target.is_some() {
                return Err("STORAGE DUPLICATE");
            }
            *target = Some(endpoint);
        }
        pos += len;
    }
    if active.is_some() {
        if count != expected || input.is_none() || output.is_none() || found.is_some() {
            return Err("STORAGE INTERFACE");
        }
        found = Some(Storage {
            configuration: b[5],
            interface: active.unwrap(),
            input: input.unwrap(),
            output: output.unwrap(),
        });
    }
    Ok(found)
}
#[derive(Clone, Copy)]
pub enum Command {
    Ready,
    Sense,
    ModeSense,
    CachingModeSense { allocation: u8 },
    Capacity,
    Read { lba: u32, sector_bytes: u32 },
    Write { lba: u32, sector_bytes: u32 },
    SynchronizeCache,
}
impl Command {
    /// Only WRITE(10) transfers payload from the host to the device.
    pub fn data_out(self) -> bool {
        matches!(self, Self::Write { .. })
    }
}
pub fn cbw(tag: u32, command: Command) -> Option<([u8; 31], usize)> {
    if tag == 0 {
        return None;
    }
    let mut b = [0u8; 31];
    b[..4].copy_from_slice(&0x43425355u32.to_le_bytes());
    b[4..8].copy_from_slice(&tag.to_le_bytes());
    let bytes = match command {
        Command::Ready => {
            b[14] = 6;
            0
        }
        Command::Sense => {
            b[14] = 6;
            b[15] = 3;
            b[19] = 18;
            18
        }
        Command::ModeSense => {
            b[14] = 6;
            b[15] = 0x1a;
            b[16] = 8;
            b[17] = 0x3f;
            b[19] = 4;
            4
        }
        Command::CachingModeSense { allocation } => {
            if allocation < 4 {
                return None;
            }
            b[14] = 6;
            b[15] = 0x1a;
            b[16] = 8;
            b[17] = 8;
            b[19] = allocation;
            allocation as usize
        }
        Command::Capacity => {
            b[14] = 10;
            b[15] = 0x25;
            8
        }
        Command::SynchronizeCache => {
            b[14] = 10;
            b[15] = 0x35;
            0
        }
        Command::Read { lba, sector_bytes } | Command::Write { lba, sector_bytes } => {
            if !matches!(sector_bytes, 512 | 4096) {
                return None;
            }
            b[14] = 10;
            b[15] = if matches!(command, Command::Write { .. }) {
                0x2a
            } else {
                0x28
            };
            b[17..21].copy_from_slice(&lba.to_be_bytes());
            b[23] = 1;
            sector_bytes as usize
        }
    };
    b[8..12].copy_from_slice(&(bytes as u32).to_le_bytes());
    b[12] = if bytes == 0 || command.data_out() {
        0
    } else {
        128
    };
    Some((b, bytes))
}
pub fn csw(b: &[u8], tag: u32) -> Option<bool> {
    if b.len() != 13
        || b[..4] != 0x53425355u32.to_le_bytes()
        || b[4..8] != tag.to_le_bytes()
        || b[8..12] != [0; 4]
        || b[12] > 1
    {
        return None;
    }
    Some(b[12] == 0)
}
/// Metadata of a failed settings WRITE or cache synchronization. No payload.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WriteFailure {
    pub opcode: u8,
    pub lba: Option<u32>,
    pub bytes: u32,
    pub tag: u32,
    pub csw_status: u8,
    pub residue: u32,
    pub sense: Option<Sense>,
}
/// Snapshot before REQUEST SENSE reuses the BOT command/status buffers. Uses
/// exactly the production CSW validator; does not relax residue/tag policy.
pub fn write_failure(command: Command, status: &[u8], tag: u32) -> Option<WriteFailure> {
    if csw(status, tag) != Some(false) {
        return None;
    }
    let (opcode, lba, bytes) = match command {
        Command::Write { lba, sector_bytes } => (0x2a, Some(lba), sector_bytes),
        Command::SynchronizeCache => (0x35, None, 0),
        _ => return None,
    };
    Some(WriteFailure {
        opcode,
        lba,
        bytes,
        tag,
        csw_status: status[12],
        residue: u32::from_le_bytes(status[8..12].try_into().ok()?),
        sense: None,
    })
}
/// Fixed-format REQUEST SENSE response. Additional length must cover ASC/ASCQ.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Sense {
    pub key: u8,
    pub asc: u8,
    pub ascq: u8,
}
impl Sense {
    pub fn write_protected(self) -> bool {
        self.key == 7 && self.asc == 0x27
    }
}
pub fn sense(bytes: &[u8]) -> Option<Sense> {
    if bytes.len() < 18
        || !matches!(bytes[0] & 0x7f, 0x70 | 0x71)
        || bytes[7] < 10
        || bytes[2] & 0xe0 != 0
    {
        return None;
    }
    Some(Sense {
        key: bytes[2] & 15,
        asc: bytes[12],
        ascq: bytes[13],
    })
}
/// MODE SENSE(6), four-byte header with DBD and all-pages request.
pub fn mode_write_protected(bytes: &[u8]) -> Option<bool> {
    if bytes.len() != 4 || bytes[0] < 3 {
        return None;
    }
    Some(bytes[2] & 0x80 != 0)
}
pub fn capacity(b: &[u8]) -> Option<(u64, u32)> {
    if b.len() != 8 {
        return None;
    }
    let last = u32::from_be_bytes(b[..4].try_into().ok()?);
    let size = u32::from_be_bytes(b[4..].try_into().ok()?);
    if last == u32::MAX || !matches!(size, 512 | 4096) {
        return None;
    }
    Some((last as u64 + 1, size))
}
pub fn read(tag: u32, lba: u64, blocks: u64, size: u32) -> Option<([u8; 31], usize)> {
    if lba >= blocks || lba > u32::MAX as u64 {
        return None;
    }
    cbw(
        tag,
        Command::Read {
            lba: lba as u32,
            sector_bytes: size,
        },
    )
}
pub fn hash(b: &[u8]) -> u64 {
    b.iter().fold(0xcbf29ce484222325u64, |h, v| {
        (h ^ *v as u64).wrapping_mul(0x100000001b3)
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn command_encoding_and_capacity_boundaries() {
        let (b, n) = read(7, 0x12345678, 0x12345679, 512).unwrap();
        assert_eq!(n, 512);
        assert_eq!(b[15], 0x28);
        assert_eq!(b[17..21], [0x12, 0x34, 0x56, 0x78]);
        assert_eq!(b[22..24], [0, 1]);
        assert_eq!(b[12], 128);
        assert!(read(1, 4, 4, 512).is_none());
        assert!(cbw(0, Command::Ready).is_none());
        assert_eq!(capacity(&[0, 0, 0, 0, 0, 0, 2, 0]), Some((1, 512)));
        assert!(capacity(&[255, 255, 255, 255, 0, 0, 2, 0]).is_none());
        assert!(capacity(&[0, 0, 0, 1, 0, 0, 0, 0]).is_none());
    }
    #[test]
    fn sense_requires_valid_fixed_response_and_classifies_protection() {
        let mut bytes = [0; 18];
        bytes[0] = 0x70;
        bytes[2] = 7;
        bytes[7] = 10;
        bytes[12] = 0x27;
        assert!(sense(&bytes).unwrap().write_protected());
        bytes[2] = 3;
        assert!(!sense(&bytes).unwrap().write_protected());
        bytes[7] = 9;
        assert_eq!(sense(&bytes), None);
        bytes[7] = 10;
        bytes[0] = 0x72;
        assert_eq!(sense(&bytes), None);
        assert_eq!(sense(&bytes[..17]), None);
    }
    #[test]
    fn mode_sense_detects_protection_without_writing() {
        let (b, n) = cbw(1, Command::ModeSense).unwrap();
        assert_eq!(
            (b[15], b[16], b[17], b[19], b[12], n),
            (0x1a, 8, 0x3f, 4, 128, 4)
        );
        assert_eq!(mode_write_protected(&[3, 0, 128, 0]), Some(true));
        assert_eq!(mode_write_protected(&[3, 0, 0, 0]), Some(false));
        assert_eq!(mode_write_protected(&[2, 0, 0, 0]), None);
    }
    #[test]
    fn write_and_flush_encoding() {
        for size in [512, 4096] {
            let (b, n) = cbw(
                2,
                Command::Write {
                    lba: 0x12345678,
                    sector_bytes: size,
                },
            )
            .unwrap();
            assert_eq!(n, size as usize);
            assert_eq!(b[12], 0);
            assert_eq!(b[15], 0x2a);
            assert_eq!(&b[17..21], &0x12345678u32.to_be_bytes());
            assert_eq!(&b[22..24], &[0, 1]);
        }
        assert!(
            cbw(
                2,
                Command::Write {
                    lba: 0,
                    sector_bytes: 1024
                }
            )
            .is_none()
        );
        let (b, n) = cbw(3, Command::SynchronizeCache).unwrap();
        assert_eq!((n, b[12], b[14], b[15]), (0, 0, 10, 0x35));
    }
    #[test]
    fn rejects_mismatched_status_and_residue() {
        let mut s = [0u8; 13];
        s[..4].copy_from_slice(&0x53425355u32.to_le_bytes());
        s[4] = 7;
        assert_eq!(csw(&s, 7), Some(true));
        assert_eq!(csw(&s, 8), None);
        s[12] = 1;
        assert_eq!(csw(&s, 7), Some(false));
        s[8] = 1;
        assert_eq!(csw(&s, 7), None);
        s[8] = 0;
        s[12] = 2;
        assert_eq!(csw(&s, 7), None);
        assert_eq!(csw(&s[..12], 7), None);
    }
    #[test]
    fn endpoint_companions_and_truncation() {
        let mut d = [
            9, 2, 44, 0, 1, 1, 0, 128, 50, 9, 4, 0, 0, 2, 8, 6, 80, 0, 7, 5, 129, 2, 0, 4, 0, 6,
            48, 15, 0, 0, 0, 7, 5, 2, 2, 0, 4, 0, 6, 48, 15, 0, 0, 0,
        ];
        assert_eq!(
            configuration(&d, Speed::Super)
                .unwrap()
                .unwrap()
                .input
                .burst,
            15
        );
        for n in 0..d.len() {
            assert!(configuration(&d[..n], Speed::Super).is_err());
        }
        d[26] = 47;
        assert!(configuration(&d, Speed::Super).is_err());
        d[26] = 48;
        d[28] = 1;
        assert!(configuration(&d, Speed::Super).is_err());
    }
}

#[cfg(test)]
mod write_diagnostic_tests {
    use super::*;
    fn failed(tag: u32) -> [u8; 13] {
        let mut s = [0; 13];
        s[..4].copy_from_slice(&0x53425355u32.to_le_bytes());
        s[4..8].copy_from_slice(&tag.to_le_bytes());
        s[12] = 1;
        s
    }
    #[test]
    fn production_write10_encodes_out_one_sector_and_big_endian_lba() {
        for lba in [0, 1, 0x12345678, u32::MAX] {
            for sector_bytes in [512, 4096] {
                let (b, n) = cbw(0x12345678, Command::Write { lba, sector_bytes }).unwrap();
                assert_eq!(n, sector_bytes as usize);
                assert_eq!(b[12], 0);
                assert_eq!(b[13], 0);
                assert_eq!(b[14], 10);
                assert_eq!(
                    &b[15..25],
                    &[
                        0x2a,
                        0,
                        lba.to_be_bytes()[0],
                        lba.to_be_bytes()[1],
                        lba.to_be_bytes()[2],
                        lba.to_be_bytes()[3],
                        0,
                        0,
                        1,
                        0
                    ]
                );
                assert_eq!(
                    u32::from_le_bytes(b[8..12].try_into().unwrap()),
                    sector_bytes
                );
                assert_eq!(u32::from_le_bytes(b[4..8].try_into().unwrap()), 0x12345678);
            }
        }
    }
    #[test]
    fn failure_snapshot_keeps_original_write_or_sync_identity_and_strict_csw_validation() {
        let s = failed(7);
        let write = Command::Write {
            lba: 0x12345678,
            sector_bytes: 512,
        };
        let w = write_failure(write, &s, 7).unwrap();
        assert_eq!(
            (
                w.opcode,
                w.lba,
                w.bytes,
                w.tag,
                w.csw_status,
                w.residue,
                w.sense
            ),
            (0x2a, Some(0x12345678), 512, 7, 1, 0, None)
        );
        let sync = write_failure(Command::SynchronizeCache, &s, 7).unwrap();
        assert_eq!((sync.opcode, sync.lba, sync.bytes), (0x35, None, 0));
        assert!(write_failure(write, &s, 8).is_none());
        assert!(write_failure(Command::Sense, &s, 7).is_none());
        let mut invalid = s;
        invalid[8] = 1;
        assert!(write_failure(write, &invalid, 7).is_none());
        invalid = s;
        invalid[12] = 0;
        assert!(write_failure(write, &invalid, 7).is_none());
        invalid = s;
        invalid[12] = 2;
        assert!(write_failure(write, &invalid, 7).is_none());
        assert!(write_failure(write, &s[..12], 7).is_none());
    }
    #[test]
    fn sense_metadata_preserves_non_protect_and_protect_causes() {
        for (key, asc, ascq) in [(6, 0x29, 0), (3, 0x0c, 2), (7, 0x27, 0)] {
            let mut b = [0; 18];
            b[0] = 0x70;
            b[2] = key;
            b[7] = 10;
            b[12] = asc;
            b[13] = ascq;
            let decoded = sense(&b).unwrap();
            let mut w = write_failure(Command::SynchronizeCache, &failed(1), 1).unwrap();
            w.sense = Some(decoded);
            assert_eq!(w.sense, Some(Sense { key, asc, ascq }));
            assert_eq!(decoded.write_protected(), key == 7 && asc == 0x27);
        }
    }
}

/// Exact current MODE SENSE(6) header: no padded/truncated reply is accepted.
pub fn caching_mode_length(header: &[u8]) -> Option<usize> {
    if header.len() != 4 {
        return None;
    }
    let length = header[0] as usize + 1;
    let descriptors = header[3] as usize;
    if !(6..=255).contains(&length) || descriptors % 8 != 0 || 4 + descriptors + 2 > length {
        return None;
    }
    Some(length)
}
/// WCE is known only from the complete requested current caching page (08/00).
/// Header equality also rejects a capability/protection change between queries.
pub fn caching_write_through(header: &[u8], reply: &[u8]) -> Option<bool> {
    let length = caching_mode_length(header)?;
    if reply.len() != length || reply.get(..4)? != header {
        return None;
    }
    let offset = 4 + header[3] as usize;
    let page = reply.get(offset..)?;
    if page.len() < 20 || page[0] & 0x7f != 8 || page[1] as usize + 2 != page.len() {
        return None;
    }
    Some(page[2] & 4 == 0)
}

#[cfg(test)]
mod caching_tests {
    extern crate std;
    use self::std::{vec, vec::Vec};
    use super::*;
    fn reply(descriptors: usize, wce: bool) -> Vec<u8> {
        let mut b = vec![0; 24 + descriptors];
        b[0] = (b.len() - 1) as u8;
        b[3] = descriptors as u8;
        b[4 + descriptors] = 8;
        b[5 + descriptors] = 18;
        b[6 + descriptors] = if wce { 4 } else { 0 };
        b
    }
    #[test]
    fn production_current_page_cdb_and_policy() {
        for allocation in [4, 24, 32, 255] {
            let (b, n) = cbw(7, Command::CachingModeSense { allocation }).unwrap();
            assert_eq!(n, allocation as usize);
            assert_eq!(&b[14..21], &[6, 0x1a, 8, 8, 0, allocation, 0]);
            assert_eq!(b[12], 128);
        }
        assert!(cbw(1, Command::CachingModeSense { allocation: 3 }).is_none());
        for descriptors in [0, 8, 16] {
            for wce in [false, true] {
                let b = reply(descriptors, wce);
                assert_eq!(caching_write_through(&b[..4], &b), Some(!wce));
            }
        }
    }
    #[test]
    fn missing_truncated_wrong_pages_and_changed_headers_are_unknown() {
        let b = reply(0, false);
        for n in 0..b.len() {
            assert_eq!(caching_write_through(&b[..4], &b[..n]), None);
        }
        for (at, value) in [
            (0, 22),
            (1, 1),
            (2, 128),
            (3, 8),
            (4, 9),
            (4, 0x48),
            (5, 17),
            (5, 19),
        ] {
            let mut bad = b.clone();
            bad[at] = value;
            assert_eq!(caching_write_through(&b[..4], &bad), None);
        }
        let mut b = reply(0, false);
        b.push(0);
        assert_eq!(caching_write_through(&b[..4], &b), None);
        for h in [[0, 0, 0, 0], [255, 0, 0, 0], [23, 0, 0, 1], [23, 0, 0, 24]] {
            assert_eq!(caching_mode_length(&h), None);
        }
        let mut b = reply(0, false);
        b[5] = 0;
        b.truncate(6);
        b[0] = 5;
        assert_eq!(caching_write_through(&b[..4], &b), None);
    }
}

/// One logical barrier completion, only after a confirmed write-through gate.
#[derive(Default)]
pub struct WriteThroughBarrier {
    pending: bool,
}
impl WriteThroughBarrier {
    pub const fn new() -> Self {
        Self { pending: false }
    }
    pub fn arm(&mut self, write_through: bool, bot_busy: bool) -> bool {
        self.pending = write_through && !bot_busy;
        self.pending
    }
    pub fn take(&mut self) -> bool {
        core::mem::take(&mut self.pending)
    }
    pub fn clear(&mut self) {
        self.pending = false;
    }
}
#[cfg(test)]
mod barrier_tests {
    use super::*;
    #[test]
    fn only_explicit_allowed_idle_barrier_completes_once() {
        let mut b = WriteThroughBarrier::new();
        assert!(!b.take());
        for allowed in [false, true] {
            for busy in [false, true] {
                assert_eq!(b.arm(allowed, busy), allowed && !busy);
                assert_eq!(b.take(), allowed && !busy);
                assert!(!b.take());
            }
        }
        assert!(b.arm(true, false));
        b.clear();
        assert!(!b.take());
        assert!(b.arm(true, false));
        assert!(!b.arm(true, true));
        assert!(!b.take());
    }
}
