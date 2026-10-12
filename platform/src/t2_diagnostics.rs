// Copyright 2026 Hirofumi Iwasaki
// SPDX-License-Identifier: Apache-2.0
//! Read-only evidence parsers. No result authorizes MMIO or DMA access.
pub const BCE_ID: u32 = 0x1801_106b;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Capabilities {
    pub pm: Option<(u8, u16)>,
    pub msi: Option<(u8, u16)>,
    pub msix: Option<(u8, u16)>,
}
/// Parse only the conventional capability list of a type-0 PCI header.
/// All offsets are checked before indexing; cycles and duplicate IDs are errors.
pub fn capabilities(config: &[u32; 64]) -> Result<Capabilities, &'static str> {
    if config[0] == u32::MAX || (config[3] >> 16) & 0x7f != 0 {
        return Err("CONFIG HEADER");
    }
    let mut result = Capabilities::default();
    if config[1] & (1 << 20) == 0 {
        return Ok(result);
    }
    let mut at = (config[13] & 0xff) as usize;
    let mut seen = 0u64;
    while at != 0 {
        if !(0x40..=0xfc).contains(&at) || at & 3 != 0 {
            return Err("CAP POINTER");
        }
        let bit = 1u64 << (at / 4);
        if seen & bit != 0 {
            return Err("CAP CYCLE");
        }
        seen |= bit;
        let word = config[at / 4];
        let slot = match word & 0xff {
            1 => {
                if at > 0xf8 {
                    return Err("PM TRUNCATED");
                }
                Some((&mut result.pm, config[at / 4 + 1] as u16))
            }
            5 => {
                let control = (word >> 16) as u16;
                let bytes = 10
                    + if control & 0x80 != 0 { 4 } else { 0 }
                    + if control & 0x100 != 0 { 10 } else { 0 };
                if at + bytes > 256 {
                    return Err("MSI TRUNCATED");
                }
                Some((&mut result.msi, control))
            }
            0x11 => {
                if at + 12 > 256 {
                    return Err("MSIX TRUNCATED");
                }
                Some((&mut result.msix, (word >> 16) as u16))
            }
            _ => None,
        };
        if let Some((slot, value)) = slot {
            if slot.replace((at as u8, value)).is_some() {
                return Err("CAP DUPLICATE");
            }
        }
        at = ((word >> 8) & 0xff) as usize;
    }
    Ok(result)
}

/// SMBIOS entry point -> physical table address and bounded table size.
pub fn smbios_entry(b: &[u8], v3: bool) -> Option<(u64, usize)> {
    let minimum = if v3 { 24 } else { 31 };
    if b.len() < minimum {
        return None;
    }
    let len = b[if v3 { 6 } else { 5 }] as usize;
    if len < minimum || len > 32 || len > b.len() || !super::checksum(&b[..len]) {
        return None;
    }
    let (address, bytes) = if v3 {
        if b.get(..5)? != b"_SM3_" {
            return None;
        }
        (
            u64::from_le_bytes(b[16..24].try_into().ok()?),
            u32::from_le_bytes(b[12..16].try_into().ok()?) as usize,
        )
    } else {
        if b.get(..4)? != b"_SM_" || b.get(16..21)? != b"_DMI_" || !super::checksum(&b[16..31]) {
            return None;
        }
        (
            u32::from_le_bytes(b[24..28].try_into().ok()?) as u64,
            u16::from_le_bytes(b[22..24].try_into().ok()?) as usize,
        )
    };
    if address == 0 || !(6..=65536).contains(&bytes) || address.checked_add(bytes as u64)? > 1 << 47
    {
        return None;
    }
    Some((address, bytes))
}
/// Only the type-1 product name is retained. Never export serial/UUID strings.
pub fn smbios_product(table: &[u8]) -> Result<&str, &'static str> {
    if table.len() > 65536 {
        return Err("SMBIOS SIZE");
    }
    let mut at = 0;
    for _ in 0..1024 {
        let h = table.get(at..at + 4).ok_or("SMBIOS HEADER")?;
        let len = h[1] as usize;
        if len < 4 {
            return Err("SMBIOS LENGTH");
        }
        let strings = at.checked_add(len).ok_or("SMBIOS LENGTH")?;
        let rest = table.get(strings..).ok_or("SMBIOS LENGTH")?;
        let end = rest
            .windows(2)
            .position(|b| b == [0, 0])
            .ok_or("SMBIOS STRINGS")?;
        if h[0] == 1 {
            if len < 8 {
                return Err("SMBIOS TYPE1");
            }
            let index = table[at + 5] as usize;
            if index == 0 {
                return Err("SMBIOS NO PRODUCT");
            }
            let value = rest[..end]
                .split(|b| *b == 0)
                .nth(index - 1)
                .ok_or("SMBIOS INDEX")?;
            if value.is_empty() || value.len() > 48 || !value.iter().all(|b| (32..=126).contains(b))
            {
                return Err("SMBIOS PRODUCT");
            }
            return core::str::from_utf8(value).map_err(|_| "SMBIOS PRODUCT");
        }
        if h[0] == 127 {
            return Err("SMBIOS NO TYPE1");
        }
        at = strings + end + 2;
    }
    Err("SMBIOS LIMIT")
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn capabilities_valid_and_absent() {
        let mut c = [0; 64];
        c[0] = BCE_ID;
        assert_eq!(capabilities(&c), Ok(Capabilities::default()));
        c[1] = 1 << 20;
        c[13] = 0x40;
        c[16] = 0x0000_5001;
        c[17] = 8;
        c[20] = 0x0187_7015; // unknown capability is skipped
        c[28] = 0x0087_9005;
        c[36] = 0x8007_0011;
        assert_eq!(
            capabilities(&c),
            Ok(Capabilities {
                pm: Some((0x40, 8)),
                msi: Some((0x70, 0x87)),
                msix: Some((0x90, 0x8007))
            })
        );
    }
    #[test]
    fn rejects_cycles_bad_offsets_duplicate_and_short_caps() {
        let mut c = [0; 64];
        c[0] = BCE_ID;
        c[1] = 1 << 20;
        for at in [1, 0x3c, 0x41, 0xff] {
            c[13] = at;
            assert!(capabilities(&c).is_err());
        }
        c[13] = 0x40;
        c[16] = 0x4005;
        assert_eq!(capabilities(&c), Err("CAP CYCLE"));
        c[16] = 0x5005;
        c[20] = 5;
        assert_eq!(capabilities(&c), Err("CAP DUPLICATE"));
        c[13] = 0xfc;
        for id in [1, 5, 0x11] {
            c[63] = id;
            assert!(capabilities(&c).is_err());
        }
        c[13] = 0xf0;
        c[60] = 0x0180_0005;
        assert_eq!(capabilities(&c), Err("MSI TRUNCATED"));
        c[0] = u32::MAX;
        assert!(capabilities(&c).is_err());
    }
    #[test]
    fn smbios_product_is_bounded_and_ignores_serial() {
        let t = b"\x01\x08\x00\x00\x01\x02\x03\x04Apple\0MacBookPro15,1\0version\0SECRET\0\0";
        assert_eq!(smbios_product(t), Ok("MacBookPro15,1"));
        for n in 0..t.len() {
            assert!(smbios_product(&t[..n]).is_err());
        }
        let mut bad = *t;
        bad[1] = 3;
        assert!(smbios_product(&bad).is_err());
        let mut bad = *t;
        bad[5] = 9;
        assert!(smbios_product(&bad).is_err());
        assert!(smbios_product(b"\x7f\x04\0\0\0\0").is_err());
    }
    #[test]
    fn smbios_entry_checks_checksum_bounds_and_version() {
        let mut v3 = [0u8; 24];
        v3[..5].copy_from_slice(b"_SM3_");
        v3[6] = 24;
        v3[12..16].copy_from_slice(&4096u32.to_le_bytes());
        v3[16..24].copy_from_slice(&0x100000u64.to_le_bytes());
        v3[5] = 0u8.wrapping_sub(v3.iter().fold(0u8, |a, b| a.wrapping_add(*b)));
        assert_eq!(smbios_entry(&v3, true), Some((0x100000, 4096)));
        assert_eq!(smbios_entry(&v3, false), None);
        for n in 0..24 {
            assert_eq!(smbios_entry(&v3[..n], true), None);
        }
        v3[12] ^= 1;
        assert_eq!(smbios_entry(&v3, true), None);
        let mut v2 = [0u8; 31];
        v2[..4].copy_from_slice(b"_SM_");
        v2[5] = 31;
        v2[16..21].copy_from_slice(b"_DMI_");
        v2[22..24].copy_from_slice(&64u16.to_le_bytes());
        v2[24..28].copy_from_slice(&0x200000u32.to_le_bytes());
        v2[21] = 0u8.wrapping_sub(v2[16..].iter().fold(0u8, |a, b| a.wrapping_add(*b)));
        v2[4] = 0u8.wrapping_sub(v2.iter().fold(0u8, |a, b| a.wrapping_add(*b)));
        assert_eq!(smbios_entry(&v2, false), Some((0x200000, 64)));
        v2[22] = 0;
        v2[4] = 0;
        v2[4] = 0u8.wrapping_sub(v2.iter().fold(0u8, |a, b| a.wrapping_add(*b)));
        assert_eq!(smbios_entry(&v2, false), None);
    }
}
