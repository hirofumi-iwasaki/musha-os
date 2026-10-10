// SPDX-License-Identifier: Apache-2.0
//! Pre-ExitBootServices ACPI table checks; map attributes are capabilities.
pub fn readable_region(kind: u32, start: usize, end: usize, address: usize, size: usize) -> bool {
    matches!(kind, 2 | 4 | 6 | 9 | 10)
        && address != 0
        && size != 0
        && address
            .checked_add(size)
            .is_some_and(|limit| start <= address && limit <= end)
}
pub fn rsdp_error(bytes: &[u8]) -> Option<&'static str> {
    if bytes.len() != 20 || bytes.get(..8) != Some(&b"RSD PTR "[..]) {
        Some("RSDP SIGNATURE")
    } else if !super::checksum(bytes) {
        Some("RSDP CHECKSUM")
    } else {
        None
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn readable_acpi_and_configuration_types() {
        for kind in [2, 4, 6, 9, 10] {
            assert!(readable_region(kind, 0x1000, 0x2000, 0x1fec, 20));
        }
        // No Attribute input: RP/RO/XP capability bits cannot veto this policy.
        for kind in [0, 1, 3, 5, 7, 8, 11, 12, 13, 14, 15] {
            assert!(!readable_region(kind, 0x1000, 0x2000, 0x1000, 20));
        }
    }
    #[test]
    fn boundaries_remain_strict() {
        for (address, size) in [
            (0, 20),
            (0x1000, 0),
            (0xfff, 20),
            (0x1fed, 20),
            (usize::MAX, 20),
        ] {
            assert!(!readable_region(9, 0x1000, 0x2000, address, size));
        }
    }
    #[test]
    fn rsdp_reports_signature_separately_from_checksum() {
        let mut b = [0u8; 20];
        assert_eq!(rsdp_error(&b), Some("RSDP SIGNATURE"));
        b[..8].copy_from_slice(b"RSD PTR ");
        assert_eq!(rsdp_error(&b), Some("RSDP CHECKSUM"));
        b[8] = 0u8.wrapping_sub(b.iter().fold(0u8, |a, v| a.wrapping_add(*v)));
        assert_eq!(rsdp_error(&b), None);
        b[19] ^= 1;
        assert_eq!(rsdp_error(&b), Some("RSDP CHECKSUM"));
    }
}
