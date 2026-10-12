// Copyright 2026 Hirofumi Iwasaki
// SPDX-License-Identifier: Apache-2.0
//! Transport-independent HID boot-report transitions.
//! No USB host, PCI, DMA or application dependencies.
//! T2 reports may use this only after its descriptor/protocol is verified.
#![no_std]
#![forbid(unsafe_code)]

pub mod hid;
pub mod sources;

#[derive(Default)]
pub struct BootKeyboardState {
    previous: [u8; 8],
}
impl BootKeyboardState {
    /// Generate releases when a source is detached. Repeated detach is silent.
    pub fn release_all(&mut self, emit: impl FnMut(u8, bool)) {
        self.update([0; 8], emit);
    }
    /// Emits modifier transitions and distinct usages; ignores reserved byte.
    /// Rollover preserves previous state rather than generating false releases.
    pub fn update(&mut self, report: [u8; 8], mut emit: impl FnMut(u8, bool)) -> bool {
        if report[2..].iter().any(|v| (1..=3).contains(v)) {
            return false;
        }
        for bit in 0..8 {
            if (self.previous[0] ^ report[0]) & (1 << bit) != 0 {
                emit(0xe0 + bit, report[0] & (1 << bit) != 0);
            }
        }
        for (before, after, down) in [
            (&self.previous[2..], &report[2..], false),
            (&report[2..], &self.previous[2..], true),
        ] {
            for (i, key) in before.iter().enumerate() {
                if *key != 0 && !before[..i].contains(key) && !after.contains(key) {
                    emit(*key, down);
                }
            }
        }
        self.previous = report;
        true
    }
}
#[cfg(test)]
mod tests {
    extern crate std;
    use super::*;
    use std::vec::Vec;
    #[test]
    fn independent_sources_keep_independent_state() {
        let mut external = BootKeyboardState::default();
        let mut internal = BootKeyboardState::default();
        let mut e = Vec::new();
        external.update([2, 0, 4, 0, 0, 0, 0, 0], |k, d| e.push((k, d)));
        assert_eq!(e, [(0xe1, true), (4, true)]);
        e.clear();
        internal.update([0, 0, 5, 0, 0, 0, 0, 0], |k, d| e.push((k, d)));
        assert_eq!(e, [(5, true)]);
        e.clear();
        external.release_all(|k, d| e.push((k, d)));
        assert_eq!(e, [(0xe1, false), (4, false)]);
        e.clear();
        internal.update([0; 8], |k, d| e.push((k, d)));
        assert_eq!(e, [(5, false)]);
    }
    #[test]
    fn disconnect_after_rollover_releases_real_keys_once() {
        let mut s = BootKeyboardState::default();
        let mut e = Vec::new();
        s.update([0x22, 0, 4, 4, 5, 0, 0, 0], |_, _| {});
        assert!(!s.update([0, 0, 1, 1, 1, 1, 1, 1], |_, _| panic!("rollover emitted")));
        s.release_all(|k, d| e.push((k, d)));
        assert_eq!(e, [(0xe1, false), (0xe5, false), (4, false), (5, false)]);
        s.release_all(|_, _| panic!("second detach emitted"));
    }
    #[test]
    fn unchanged_report_emits_nothing_and_sources_can_restart() {
        let mut s = BootKeyboardState::default();
        let r = [0, 99, 4, 0, 0, 0, 0, 0];
        s.update(r, |_, _| {});
        s.update(r, |_, _| panic!("repeat"));
        s.release_all(|_, _| {});
        let mut e = Vec::new();
        s.update(r, |k, d| e.push((k, d)));
        assert_eq!(e, [(4, true)]);
    }
}
