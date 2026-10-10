// SPDX-License-Identifier: Apache-2.0
#![no_std]
pub mod acpi_read;
pub mod controllers;
pub mod diagnostics;
pub mod pci_resources;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(C)]
pub struct Timer {
    pub port: u16,
    pub bits: u8,
}
fn word(b: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(b.get(at..at + 4)?.try_into().ok()?))
}
pub fn checksum(b: &[u8]) -> bool {
    b.iter().fold(0u8, |a, v| a.wrapping_add(*v)) == 0
}
pub fn fadt(b: &[u8]) -> Option<Timer> {
    if b.len() < 116 || b.get(..4)? != b"FACP" || word(b, 4)? as usize != b.len() || !checksum(b) {
        return None;
    }
    if b.len() >= 116 && word(b, 112)? & (1 << 20) != 0 {
        return None;
    } // HW_REDUCED_ACPI
    let bits = if word(b, 112)? & 256 != 0 { 32 } else { 24 };
    let mut address = word(b, 76)? as u64;
    if b.len() >= 220 {
        let extended = u64::from_le_bytes(b[212..220].try_into().ok()?);
        if extended != 0 {
            // This implementation supports only System I/O. Use extended GAS
            // when usable; ACPI permits legacy fallback when it is not usable.
            if b[208] == 1 {
                address = extended;
            }
        }
    }
    if b[91] != 4 || address == 0 || address > 65532 {
        return None;
    }
    Some(Timer {
        port: address as u16,
        bits,
    })
}
pub struct Clock {
    last: u32,
    mask: u32,
    ticks: u64,
}
impl Clock {
    pub fn new(timer: Timer, sample: u32) -> Option<Self> {
        let mask = match timer.bits {
            24 => 0xffffff,
            32 => u32::MAX,
            _ => return None,
        };
        Some(Self {
            last: sample & mask,
            mask,
            ticks: 0,
        })
    }
    // Caller must sample more often than a full counter period (4.68 s at 24 bits).
    pub fn sample(&mut self, raw: u32) -> Option<u64> {
        let value = raw & self.mask;
        self.ticks = self
            .ticks
            .checked_add(value.wrapping_sub(self.last) as u64 & self.mask as u64)?;
        self.last = value;
        Some(self.ticks / 3_579_545 * 1000 + self.ticks % 3_579_545 * 1000 / 3_579_545)
    }
}
/// Bounded short PM-timer delay used by MDIO, including counter wrap.
pub struct ShortDelay {
    start: u32,
    mask: u32,
    ticks: u32,
}
impl ShortDelay {
    pub fn new(timer: Timer, sample: u32, us: u32) -> Option<Self> {
        if !(1..=1000).contains(&us) {
            return None;
        }
        let mask = match timer.bits {
            24 => 0xffffff,
            32 => u32::MAX,
            _ => return None,
        };
        let ticks = ((us as u64 * 3_579_545).div_ceil(1_000_000)) as u32;
        Some(Self {
            start: sample & mask,
            mask,
            ticks,
        })
    }
    pub fn complete(&self, sample: u32) -> bool {
        ((sample & self.mask).wrapping_sub(self.start) & self.mask) >= self.ticks
    }
}
#[cfg(test)]
mod short_delay_tests {
    use super::*;
    #[test]
    fn calibrated_delay_and_wrap() {
        for bits in [24, 32] {
            let mask = if bits == 24 { 0xffffff } else { u32::MAX };
            let d = ShortDelay::new(Timer { port: 0x408, bits }, mask - 100, 50).unwrap();
            assert!(!d.complete(77));
            assert!(d.complete(78));
        }
        let d = ShortDelay::new(
            Timer {
                port: 0x408,
                bits: 24,
            },
            0,
            1000,
        )
        .unwrap();
        assert!(!d.complete(3579));
        assert!(d.complete(3580));
    }
    #[test]
    fn rejects_invalid_delays() {
        for us in [0, 1001, u32::MAX] {
            assert!(
                ShortDelay::new(
                    Timer {
                        port: 0x408,
                        bits: 24
                    },
                    0,
                    us
                )
                .is_none()
            );
        }
        assert!(
            ShortDelay::new(
                Timer {
                    port: 0x408,
                    bits: 16
                },
                0,
                50
            )
            .is_none()
        );
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn table() -> [u8; 220] {
        let mut b = [0; 220];
        b[..4].copy_from_slice(b"FACP");
        b[4..8].copy_from_slice(&220u32.to_le_bytes());
        b[76..80].copy_from_slice(&0x408u32.to_le_bytes());
        b[91] = 4;
        fix(&mut b);
        b
    }
    fn fix(b: &mut [u8]) {
        b[9] = 0;
        b[9] = 0u8.wrapping_sub(b.iter().fold(0u8, |a, v| a.wrapping_add(*v)));
    }
    #[test]
    fn validates_table() {
        let mut b = table();
        assert_eq!(
            fadt(&b),
            Some(Timer {
                port: 0x408,
                bits: 24
            })
        );
        b[50] ^= 1;
        assert_eq!(fadt(&b), None);
        assert_eq!(fadt(&b[..100]), None);
    }
    #[test]
    fn extended_and_bounds() {
        let mut b = table();
        b[208] = 1;
        b[212..220].copy_from_slice(&0x508u64.to_le_bytes());
        b[113] = 1;
        fix(&mut b);
        assert_eq!(
            fadt(&b),
            Some(Timer {
                port: 0x508,
                bits: 32
            })
        );
        b[212..220].copy_from_slice(&65535u64.to_le_bytes());
        fix(&mut b);
        assert_eq!(fadt(&b), None);
    }
    #[test]
    fn wrap_and_frequency() {
        for bits in [24, 32] {
            let t = Timer { port: 1, bits };
            let mask = if bits == 24 { 0xffffff } else { u32::MAX };
            let mut c = Clock::new(t, mask - 9).unwrap();
            assert_eq!(c.sample(10), Some(0));
            assert_eq!(c.sample(3_579_555), Some(1000));
        }
    }
}

/// Advance configuration DWORD slots separately from UEFI logical BAR indices.
/// Zero/unimplemented BARs retain an index; 64-bit upper halves do not.
pub fn next_bar_indices(slot: usize, resource: u8, raw: u32) -> (usize, u8) {
    (slot + if raw & 7 == 4 { 2 } else { 1 }, resource + 1)
}

/// Classify physical BAR slots without treating a 64-bit upper DWORD as a BAR.
/// Unsupported encodings remain explicit; this is diagnostic, not permission.
pub fn bar_layout(header: u8, bars: [u32; 6]) -> [&'static str; 6] {
    let count = match header & 0x7f {
        0 => 6,
        1 => 2,
        _ => 0,
    };
    let mut result = ["NOT BAR"; 6];
    let mut i = 0;
    while i < count {
        let raw = bars[i];
        result[i] = if raw == 0 {
            "ZERO"
        } else if raw & 1 != 0 {
            "IO"
        } else {
            match raw & 6 {
                0 => "MEM32",
                4 => "MEM64",
                _ => "RESERVED",
            }
        };
        if raw != 0 && raw & 7 == 4 {
            if i + 1 == count {
                result[i] = "INVALID64";
            } else {
                result[i + 1] = "UPPER64";
                i += 1;
            }
        }
        i += 1;
    }
    result
}

/// Structural fields only: callers must independently apply memory policy.
pub fn bar_descriptor_fields(b: &[u8]) -> Result<[u64; 8], &'static str> {
    if b.len() < 48 {
        return Err("DESCRIPTOR TRUNCATED");
    }
    if b[0] != 0x8a {
        return Err("DESCRIPTOR TAG");
    }
    if b[1..3] != [43, 0] {
        return Err("DESCRIPTOR LENGTH");
    }
    if b[46] != 0x79 {
        return Err("DESCRIPTOR END TAG");
    }
    if b[47] != 0 && b[..48].iter().fold(0u8, |a, v| a.wrapping_add(*v)) != 0 {
        return Err("DESCRIPTOR CHECKSUM");
    }
    let word = |offset| u64::from_le_bytes(b[offset..offset + 8].try_into().unwrap());
    Ok([
        b[3] as u64,
        b[4] as u64,
        b[5] as u64,
        word(6),
        word(14),
        word(22),
        word(30),
        word(38),
    ])
}

#[cfg(test)]
mod bar_inspection_tests {
    use super::*;
    #[test]
    fn logical_indices_cover_mixed_and_zero_bars() {
        let raws = [0xb000000c, 0, 0xc000000c, 0, 0x3001, 0x81500000];
        let (mut slot, mut resource) = (0, 0);
        let mut seen = [(0, 0); 4];
        let mut count = 0;
        while slot < 6 {
            seen[count] = (slot, resource);
            count += 1;
            (slot, resource) = next_bar_indices(slot, resource, raws[slot]);
        }
        assert_eq!(seen, [(0, 0), (2, 1), (4, 2), (5, 3)]);
        assert_eq!(next_bar_indices(0, 0, 0), (1, 1));
        assert_eq!(next_bar_indices(1, 1, 0x2000), (2, 2));
        assert_eq!(next_bar_indices(0, 0, 4), (2, 1));
        assert_eq!(next_bar_indices(0, 0, 0x3001), (1, 1));
    }
    #[test]
    fn layouts_preserve_upper_halves_and_bounds() {
        assert_eq!(
            bar_layout(0, [0x1004, 0x3000, 0x2001, 0x8008, 2, 0]),
            ["MEM64", "UPPER64", "IO", "MEM32", "RESERVED", "ZERO"]
        );
        for upper in [0, 1, 4, u32::MAX] {
            assert_eq!(
                bar_layout(0, [4, upper, 0, 0, 0, 4]),
                ["MEM64", "UPPER64", "ZERO", "ZERO", "ZERO", "INVALID64"]
            );
        }
        assert_eq!(
            bar_layout(0x81, [0, 4, 0, 0, 0, 0]),
            [
                "ZERO",
                "INVALID64",
                "NOT BAR",
                "NOT BAR",
                "NOT BAR",
                "NOT BAR"
            ]
        );
        assert_eq!(bar_layout(2, [0; 6]), ["NOT BAR"; 6]);
    }
    #[test]
    fn structural_success_never_grants_memory_permission() {
        let mut b = [0u8; 48];
        b[0] = 0x8a;
        b[1] = 43;
        b[46] = 0x79;
        for kind in [0, 1, 2, 255] {
            b[3] = kind;
            assert_eq!(bar_descriptor_fields(&b).unwrap()[0], kind as u64);
            assert!(memory_bar_descriptor(&b).is_err());
        }
        assert!(bar_descriptor_fields(&b[..47]).is_err());
        for (offset, value) in [(0, 0), (1, 44), (46, 0), (47, 1)] {
            let mut bad = b;
            bad[offset] = value;
            assert!(bar_descriptor_fields(&bad).is_err());
        }
    }
}

/// Decode the fixed UEFI GetBarAttributes QWORD memory descriptor and end tag.
/// Keep host addresses as u64; direct hardware access has stricter policy below.
pub fn memory_bar_descriptor(b: &[u8]) -> Result<(u64, u64), &'static str> {
    if b.len() < 48 {
        return Err("DESCRIPTOR TRUNCATED");
    }
    if b[0] != 0x8a {
        return Err("DESCRIPTOR TAG");
    }
    if b[1..3] != [43, 0] {
        return Err("DESCRIPTOR LENGTH");
    }
    if b[3] != 0 {
        return Err("DESCRIPTOR TYPE");
    }
    if b[46] != 0x79 {
        return Err("DESCRIPTOR END TAG");
    }
    if b[47] != 0 && b[..48].iter().fold(0u8, |a, v| a.wrapping_add(*v)) != 0 {
        return Err("DESCRIPTOR CHECKSUM");
    }
    // Translation is defined by UEFI, but recovery currently assumes identical
    // host and PCI addresses. Reject explicitly rather than call it malformed.
    if b[30..38] != [0; 8] {
        return Err("RESOURCE TRANSLATION");
    }
    let base = u64::from_le_bytes(b[14..22].try_into().unwrap());
    let bytes = u64::from_le_bytes(b[38..46].try_into().unwrap());
    if bytes == 0 || base.checked_add(bytes).is_none() {
        return Err("RESOURCE RANGE");
    }
    Ok((base, bytes))
}

/// Validated memory BAR for direct MMIO access; policy is unchanged.
pub fn bar_resource(b: &[u8]) -> Option<(usize, usize)> {
    let (base, bytes) = memory_bar_descriptor(b).ok()?;
    let (base, bytes) = (usize::try_from(base).ok()?, usize::try_from(bytes).ok()?);
    if base < 0x100000
        || base % 4096 != 0
        || !(4096..=1024 * 1024).contains(&bytes)
        || bytes % 4096 != 0
        || base.checked_add(bytes)? >= 1usize << 47
    {
        return None;
    }
    Some((base, bytes))
}
#[cfg(test)]
mod bar_tests {
    use super::*;
    #[test]
    fn descriptor_errors_and_large_peer_ranges() {
        let mut b = [0u8; 48];
        b[0] = 0x8a;
        b[1] = 43;
        b[46] = 0x79;
        b[14..22].copy_from_slice(&0x100000000u64.to_le_bytes());
        b[38..46].copy_from_slice(&0x20000000u64.to_le_bytes());
        assert_eq!(memory_bar_descriptor(&b), Ok((0x100000000, 0x20000000)));
        assert_eq!(bar_resource(&b), None); // controller access size policy
        assert_eq!(memory_bar_descriptor(&b[..47]), Err("DESCRIPTOR TRUNCATED"));
        for (offset, value, reason) in [
            (0, 0, "DESCRIPTOR TAG"),
            (1, 44, "DESCRIPTOR LENGTH"),
            (3, 1, "DESCRIPTOR TYPE"),
            (46, 0, "DESCRIPTOR END TAG"),
            (30, 1, "RESOURCE TRANSLATION"),
            (47, 1, "DESCRIPTOR CHECKSUM"),
        ] {
            let mut bad = b;
            bad[offset] = value;
            assert_eq!(memory_bar_descriptor(&bad), Err(reason));
        }
        b[38..46].fill(0);
        assert_eq!(memory_bar_descriptor(&b), Err("RESOURCE RANGE"));
    }
    #[test]
    fn refuses_io_translation_and_overflow() {
        let mut b = [0u8; 48];
        b[0] = 0x8a;
        b[1] = 43;
        b[46] = 0x79;
        b[14..22].copy_from_slice(&0xf0000000u64.to_le_bytes());
        b[38..46].copy_from_slice(&0x4000u64.to_le_bytes());
        assert_eq!(bar_resource(&b), Some((0xf0000000, 0x4000)));
        b[3] = 1;
        assert_eq!(bar_resource(&b), None);
        b[3] = 0;
        b[30] = 1;
        assert_eq!(bar_resource(&b), None);
        b[30] = 0;
        b[14..22].copy_from_slice(&u64::MAX.to_le_bytes());
        assert_eq!(bar_resource(&b), None);
    }
}

pub mod pci_recovery;
