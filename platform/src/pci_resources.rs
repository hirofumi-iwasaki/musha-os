// SPDX-License-Identifier: Apache-2.0
//! Conservative peer-only overlap bounds; never a device mapping or BAR size.
//! PCI 3.0 section 6.2.5.1: memory BAR apertures are powers of two and naturally
//! aligned. For nonzero base B, actual size divides B, so size <= lowbit(B).

pub fn peer_memory32_bound(
    raw: u32,
    firmware_base: u64,
    firmware_bytes: u64,
) -> Result<(u64, u64), &'static str> {
    // Only ordinary 32-bit memory BARs. Prefetchability does not change alignment.
    if raw & 7 != 0 {
        return Err("PEER BAR TYPE");
    }
    let base = (raw & !15) as u64;
    if base == 0 {
        return Err("PEER BAR ZERO");
    }
    let bound = 1u64 << base.trailing_zeros();
    if firmware_base == 0
        || firmware_bytes < 16
        || !firmware_bytes.is_power_of_two()
        || firmware_bytes > bound
        || firmware_base % firmware_bytes != 0
        || firmware_base
            .checked_add(firmware_bytes)
            .is_none_or(|e| e > 1u64 << 32)
    {
        return Err("PEER RESOURCE RANGE");
    }
    Ok((base, bound))
}

pub fn disjoint(a: (u64, u64), b: (u64, u64)) -> bool {
    if a.1 == 0 || b.1 == 0 {
        return false;
    }
    match (a.0.checked_add(a.1), b.0.checked_add(b.1)) {
        (Some(a_end), Some(b_end)) => a_end <= b.0 || b_end <= a.0,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn h18_peer_is_bounded_without_trusting_firmware_size_at_live_base() {
        let live = peer_memory32_bound(0xfe010000, 0x81617000, 0x1000).unwrap();
        assert_eq!(live, (0xfe010000, 0x10000));
        assert!(disjoint(live, (0x8f900000, 0x10000)));
        assert!(disjoint((0x81617000, 0x1000), (0x8f900000, 0x10000)));
        // Beyond cached 4 KiB, but still possibly decoded: must block recovery.
        assert!(!disjoint(live, (0xfe018000, 0x1000)));
        assert!(!disjoint((0x81617000, 0x1000), (0x81617000, 0x1000)));
    }
    #[test]
    fn bound_contains_every_possible_naturally_aligned_32bit_aperture() {
        for bit in 4..32 {
            for prefix in [0u32, 0x55555555, 0xaaaaaaaa, u32::MAX] {
                let mask = !((1u32 << bit) - 1);
                let base = (prefix & mask) | (1u32 << bit);
                let base = base & !((1u32 << bit) - 1);
                let (_, bound) = peer_memory32_bound(base, 0x1000, 16).unwrap();
                for size_bit in 4..32 {
                    let size = 1u64 << size_bit;
                    if base as u64 % size == 0 {
                        assert!(size <= bound);
                    }
                }
                assert!((base as u64) + bound <= 1u64 << 32);
            }
        }
    }
    #[test]
    fn uncertain_or_unsupported_resources_fail_closed() {
        for raw in [0, 8, 0xfe010001, 0xfe010002, 0xfe010004, 0xfe010006] {
            assert!(peer_memory32_bound(raw, 0x81617000, 0x1000).is_err());
        }
        for (base, bytes) in [
            (0, 0x1000),
            (0x81617000, 0),
            (0x81617000, 0x1800),
            (0x81617001, 0x1000),
            (0x81600000, 0x20000),
            (0xfffffff0, 0x1000),
        ] {
            assert!(peer_memory32_bound(0xfe010000, base, bytes).is_err());
        }
        assert!(peer_memory32_bound(0xfe010008, 0x81617000, 0x1000).is_ok());
    }
    #[test]
    fn overlap_boundaries_and_overflow() {
        assert!(disjoint((0x1000, 0x1000), (0x2000, 0x1000)));
        assert!(!disjoint((0x1000, 0x1001), (0x2000, 0x1000)));
        assert!(!disjoint((u64::MAX, 1), (0, 1)));
        assert!(!disjoint((0, 0), (0x1000, 0x1000)));
        assert_eq!(
            peer_memory32_bound(0x80000000, 0x1000, 16),
            Ok((0x80000000, 0x80000000))
        );
    }
}
