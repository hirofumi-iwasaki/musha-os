// Copyright 2026 Hirofumi Iwasaki
// SPDX-License-Identifier: Apache-2.0
//! Read-only BOT/SCSI encoding and bounded descriptor validation.
use crate::Speed;
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
    Capacity,
    Read { lba: u32, sector_bytes: u32 },
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
        Command::Capacity => {
            b[14] = 10;
            b[15] = 0x25;
            8
        }
        Command::Read { lba, sector_bytes } => {
            if !matches!(sector_bytes, 512 | 4096) {
                return None;
            }
            b[14] = 10;
            b[15] = 0x28;
            b[17..21].copy_from_slice(&lba.to_be_bytes());
            b[23] = 1;
            sector_bytes as usize
        }
    };
    b[8..12].copy_from_slice(&(bytes as u32).to_le_bytes());
    b[12] = if bytes == 0 { 0 } else { 128 }; // LUN 0, no data-out commands
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
