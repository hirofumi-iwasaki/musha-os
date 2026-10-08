// Copyright 2026 Hirofumi Iwasaki
// SPDX-License-Identifier: Apache-2.0
use super::{Error, Reader, u32le};
const ESP: [u8; 16] = [
    0x28, 0x73, 0x2a, 0xc1, 0x1f, 0xf8, 0xd2, 0x11, 0xba, 0x4b, 0, 0xa0, 0xc9, 0x3e, 0xc9, 0x3b,
];
fn u64le(b: &[u8], i: usize) -> u64 {
    u64::from_le_bytes(b[i..i + 8].try_into().unwrap())
}
fn crc_update(mut crc: u32, bytes: &[u8]) -> u32 {
    for &byte in bytes {
        crc ^= byte as u32;
        for _ in 0..8 {
            crc = (crc >> 1) ^ (0xedb88320 & 0u32.wrapping_sub(crc & 1));
        }
    }
    crc
}
#[derive(Clone, Copy)]
struct Header {
    first: u64,
    last: u64,
    guid: [u8; 16],
    array: u64,
    crc: u32,
}
fn header(b: &mut [u8], blocks: u64, size: usize, backup: bool) -> Result<Header, Error> {
    if &b[..8] != b"EFI PART" {
        return Err(Error::Corrupt);
    }
    if u32le(b, 8) != 0x10000 {
        return Err(Error::Unsupported);
    }
    let length = u32le(b, 12) as usize;
    if !(92..=size).contains(&length) || u32le(b, 20) != 0 {
        return Err(Error::Corrupt);
    }
    let expected = u32le(b, 16);
    b[16..20].fill(0);
    if !crc_update(!0, &b[..length]) != expected {
        return Err(Error::Corrupt);
    }
    // A bounded initial implementation: the standard 16KiB array only.
    if u32le(b, 80) != 128 || u32le(b, 84) != 128 {
        return Err(Error::Unsupported);
    }
    let last_block = blocks - 1;
    if u64le(b, 24) != if backup { last_block } else { 1 }
        || u64le(b, 32) != if backup { 1 } else { last_block }
    {
        return Err(Error::Corrupt);
    }
    let first = u64le(b, 40);
    let last = u64le(b, 48);
    let array = u64le(b, 72);
    let sectors = 16384 / size as u64;
    let end = array.checked_add(sectors).ok_or(Error::Corrupt)?;
    if first < 2 + sectors
        || first > last
        || last >= last_block - sectors
        || (!backup && (array < 2 || end > first))
        || (backup && (array <= last || end > last_block))
    {
        return Err(Error::Corrupt);
    }
    let guid: [u8; 16] = b[56..72].try_into().unwrap();
    if guid == [0; 16] {
        return Err(Error::Corrupt);
    }
    Ok(Header {
        first,
        last,
        guid,
        array,
        crc: u32le(b, 88),
    })
}
#[derive(Clone, Copy)]
struct Partition {
    first: u64,
    last: u64,
    guid: [u8; 16],
}
/// Strictly validates both GPT copies; does not repair or use a damaged copy.
pub(super) fn partition<F: FnMut(u64, &mut [u8]) -> Result<(), Error>>(
    r: &mut Reader<'_, F>,
    mbr: &[u8],
) -> Result<(u64, u64), Error> {
    if r.blocks < 2 + 2 * (16384 / r.size as u64) + 2 {
        return Err(Error::Corrupt);
    }
    let mut protective = false;
    for entry in mbr[446..510].chunks_exact(16) {
        if entry == [0; 16] {
            continue;
        }
        if protective
            || entry[0] != 0
            || entry[4] != 0xee
            || u32le(entry, 8) != 1
            || u32le(entry, 12) as u64 != (r.blocks - 1).min(u32::MAX as u64)
        {
            return Err(Error::Corrupt);
        }
        protective = true;
    }
    if !protective {
        return Err(Error::Corrupt);
    }
    let mut a = [0u8; 4096];
    let mut b = [0u8; 4096];
    r.sector(1, &mut a)?;
    let primary = header(&mut a, r.blocks, r.size, false)?;
    r.sector(r.blocks - 1, &mut b)?;
    let backup = header(&mut b, r.blocks, r.size, true)?;
    if primary.first != backup.first
        || primary.last != backup.last
        || primary.guid != backup.guid
        || primary.crc != backup.crc
    {
        return Err(Error::Corrupt);
    }
    let empty = Partition {
        first: 0,
        last: 0,
        guid: [0; 16],
    };
    let mut used = [empty; 128];
    let mut count = 0;
    let mut esp = None;
    let mut crc = !0;
    for sector in 0..16384 / r.size as u64 {
        r.sector(primary.array + sector, &mut a)?;
        r.sector(backup.array + sector, &mut b)?;
        if a[..r.size] != b[..r.size] {
            return Err(Error::Corrupt);
        }
        crc = crc_update(crc, &a[..r.size]);
        for entry in a[..r.size].chunks_exact(128) {
            if entry[..16] == [0; 16] {
                continue;
            }
            let part = Partition {
                first: u64le(entry, 32),
                last: u64le(entry, 40),
                guid: entry[16..32].try_into().unwrap(),
            };
            if part.first < primary.first
                || part.last > primary.last
                || part.first > part.last
                || part.guid == [0; 16]
                || used[..count]
                    .iter()
                    .any(|p| p.guid == part.guid || (p.first <= part.last && part.first <= p.last))
            {
                return Err(Error::Corrupt);
            }
            used[count] = part;
            count += 1;
            if entry[..16] == ESP {
                if esp.is_some() {
                    return Err(Error::Unsupported);
                }
                esp = Some((part.first, part.last - part.first + 1));
            }
        }
    }
    if !crc != primary.crc {
        return Err(Error::Corrupt);
    }
    esp.ok_or(Error::Unsupported)
}
