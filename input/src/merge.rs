// Copyright 2026 Hirofumi Iwasaki
// SPDX-License-Identifier: Apache-2.0
//! OR held usages across two independent external keyboards. One source release
//! cannot release a key still held by the other; no repeat or synthetic press.
pub struct Keys {
    held: [[bool; 256]; 2],
}
impl Keys {
    pub const fn new() -> Self {
        Self {
            held: [[false; 256]; 2],
        }
    }
    pub fn key(&mut self, source: usize, key: u8, down: bool, mut emit: impl FnMut(u8, bool)) {
        if source >= 2 {
            return;
        }
        let key = key as usize;
        let before = self.held[0][key] || self.held[1][key];
        self.held[source][key] = down;
        let after = self.held[0][key] || self.held[1][key];
        if before != after {
            emit(key as u8, after);
        }
    }
    pub fn clear(&mut self, source: usize, mut emit: impl FnMut(u8, bool)) {
        if source >= 2 {
            return;
        }
        for key in 0..256 {
            self.key(source, key as u8, false, &mut emit);
        }
    }
}
#[cfg(test)]
mod tests {
    extern crate std;
    use super::*;
    #[test]
    fn overlapping_and_detached_sources() {
        let mut m = Keys::new();
        let mut out = std::vec::Vec::new();
        for (s, k, d) in [(0, 4, true), (1, 4, true), (0, 4, false), (1, 4, false)] {
            m.key(s, k, d, |k, d| out.push((k, d)));
        }
        assert_eq!(out, [(4, true), (4, false)]);
        m.key(1, 0xe1, true, |_, _| {});
        m.clear(1, |k, d| out.push((k, d)));
        assert_eq!(out.last(), Some(&(0xe1, false)));
        m.clear(1, |_, _| panic!());
    }
}
