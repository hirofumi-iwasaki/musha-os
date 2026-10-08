// SPDX-License-Identifier: Apache-2.0
#![no_std]
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
