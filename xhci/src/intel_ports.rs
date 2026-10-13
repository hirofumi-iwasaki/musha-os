// Copyright 2026 Hirofumi Iwasaki
// SPDX-License-Identifier: Apache-2.0
//! Intel shared EHCI/xHCI port routing. Hardware I/O stays in the caller.
//! Independently implemented from register semantics; no Linux code copied.
pub const OFFSETS: [u8; 4] = [0xd0, 0xd4, 0xd8, 0xdc];
pub const ATTACH_POLL_MS: u64 = 20;
pub const ATTACH_POLLS: usize = 50; // One shared 1 s budget, not 1 s per port.
pub trait Config {
    fn read(&mut self, offset: u8) -> u32;
    fn write(&mut self, offset: u8, value: u32);
}
#[derive(Debug, PartialEq, Eq)]
pub struct Evidence {
    pub before: [u32; 4],
    pub after: [u32; 4],
    pub writes: u8,
    pub matched: bool,
}
pub fn eligible(id: u32, class: u32, subsystem: u32, intel_ehci: bool) -> bool {
    id as u16 == 0x8086
        && id >> 16 != 0xffff
        && class == 0x0c0330
        && subsystem != 0x90a8_104d
        && intel_ehci
}
pub fn route(config: &mut impl Config) -> Evidence {
    let before = OFFSETS.map(|o| config.read(o));
    let mut writes = 0;
    let mut matched = true;
    // SuperSpeed terminations first, so a USB3 device need not attach at USB2.
    for (reg, mask, bit) in [(2, 3, 2), (0, 1, 1)] {
        if before[reg] == u32::MAX || before[mask] == u32::MAX || before[mask] == 0 {
            continue;
        }
        config.write(OFFSETS[reg], before[mask]);
        matched &= config.read(OFFSETS[reg]) == before[mask];
        writes |= bit;
    }
    let after = OFFSETS.map(|o| config.read(o));
    for (reg, mask, bit) in [(0, 1, 1), (2, 3, 2)] {
        if writes & bit != 0 {
            matched &= after[reg] == before[mask];
        }
    }
    Evidence {
        before,
        after,
        writes,
        matched,
    }
}
/// A connected USB3 port still in RxDetect/Polling/Recovery is not settled.
/// Enumeration performs the existing reset and speed checks after this wait.
pub fn attach_pending(status: u32, major: u8) -> bool {
    major != 0 && (status & 1 == 0 || (major >= 3 && matches!((status >> 5) & 15, 5 | 7 | 8)))
}
#[cfg(test)]
mod tests {
    use super::*;
    struct Mock {
        values: [u32; 4],
        writes: [(u8, u32); 2],
        count: usize,
        ignore: bool,
    }
    impl Config for Mock {
        fn read(&mut self, o: u8) -> u32 {
            self.values[((o - 0xd0) / 4) as usize]
        }
        fn write(&mut self, o: u8, v: u32) {
            self.writes[self.count] = (o, v);
            self.count += 1;
            if !self.ignore {
                self.values[((o - 0xd0) / 4) as usize] = v;
            }
        }
    }
    fn mock(values: [u32; 4]) -> Mock {
        Mock {
            values,
            writes: [(0, 0); 2],
            count: 0,
            ignore: false,
        }
    }
    #[test]
    fn gates() {
        assert!(eligible(0x9cb18086, 0x0c0330, 0, true));
        for args in [
            (0x000d1b36, 0x0c0330, 0, true),
            (0x9cb18086, 0x0c0320, 0, true),
            (0x9cb18086, 0x0c0330, 0, false),
            (0x9cb18086, 0x0c0330, 0x90a8104d, true),
            (u32::MAX, 0x0c0330, 0, true),
        ] {
            assert!(!eligible(args.0, args.1, args.2, args.3));
        }
    }
    #[test]
    fn superspeed_first_and_readback() {
        let mut m = mock([1, 15, 0, 3]);
        let e = route(&mut m);
        assert_eq!(m.writes, [(0xd8, 3), (0xd0, 15)]);
        assert_eq!(e.before, [1, 15, 0, 3]);
        assert_eq!(e.after, [15, 15, 3, 3]);
        assert!(e.matched);
        assert_eq!(e.writes, 3);
    }
    #[test]
    fn zero_and_absent_never_written() {
        for v in [
            [0, 0, 0, 0],
            [u32::MAX; 4],
            [0, u32::MAX, 0, u32::MAX],
            [u32::MAX, 3, u32::MAX, 3],
        ] {
            let mut m = mock(v);
            assert_eq!(route(&mut m).writes, 0);
            assert_eq!(m.count, 0);
        }
    }
    #[test]
    fn independent_masks() {
        let mut m = mock([0, 0, 0, 7]);
        assert_eq!(route(&mut m).writes, 2);
        assert_eq!(m.writes[0], (0xd8, 7));
        assert_eq!(m.count, 1);
    }
    #[test]
    fn refused_write_visible() {
        let mut m = mock([0, 3, 0, 7]);
        m.ignore = true;
        assert!(!route(&mut m).matched);
    }
    #[test]
    fn bounded_attach_policy() {
        assert_eq!(ATTACH_POLL_MS * ATTACH_POLLS as u64, 1000);
        assert!(!attach_pending(0, 0));
        assert!(attach_pending(0, 2));
        assert!(!attach_pending(1 | (7 << 5), 2));
        for pls in [5, 7, 8] {
            assert!(attach_pending(1 | (pls << 5), 3));
        }
        assert!(!attach_pending(1, 3));
    }
}
