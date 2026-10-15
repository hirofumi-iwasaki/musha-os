// Copyright 2026 Hirofumi Iwasaki
// SPDX-License-Identifier: Apache-2.0
//! Controller-scoped extra completion ownership. No buffer release on errors.
#[derive(Clone, Copy)]
struct Pending {
    trb: usize,
    slot: u8,
    dci: u8,
    bytes: u32,
    incoming: bool,
}
pub struct Mailbox {
    pending: [Option<Pending>; 3],
    saved: [Option<[u32; 4]>; 3],
}
impl Mailbox {
    pub const fn new() -> Self {
        Self {
            pending: [None; 3],
            saved: [None; 3],
        }
    }
    pub fn arm(
        &mut self,
        index: usize,
        trb: usize,
        slot: u8,
        dci: u8,
        bytes: u32,
        incoming: bool,
    ) -> Result<(), &'static str> {
        if index >= 3
            || self.pending[index].is_some()
            || self.saved[index].is_some()
            || trb % 16 != 0
            || trb == 0
            || !(1..=8).contains(&slot)
            || !(2..=31).contains(&dci)
            || bytes == 0
            || bytes > 2048
        {
            return Err("BT TRANSFER OWNER");
        }
        if self.pending.iter().flatten().any(|p| p.trb == trb) {
            return Err("BT DUPLICATE TD");
        }
        self.pending[index] = Some(Pending {
            trb,
            slot,
            dci,
            bytes,
            incoming,
        });
        Ok(())
    }
    fn matches(p: Pending, words: [u32; 4]) -> bool {
        crate::endpoint_completion(words, p.trb, p.slot, p.dci)
            || (p.incoming
                && crate::control::endpoint_residual(words, p.trb, p.slot, p.dci, p.bytes)
                    .is_some())
    }
    pub fn save(&mut self, words: [u32; 4]) -> bool {
        for i in 0..3 {
            if self.pending[i].is_some_and(|p| Self::matches(p, words)) {
                if self.saved[i].is_some() {
                    return false;
                }
                self.saved[i] = Some(words);
                return true;
            }
        }
        false
    }
    pub fn take(&mut self, index: usize) -> Option<[u32; 4]> {
        self.saved.get_mut(index)?.take()
    }
    /// Only after a correlated completion, never an ambiguous timeout.
    pub fn complete(&mut self, index: usize, words: [u32; 4]) -> Result<usize, &'static str> {
        let p = self.pending.get(index).and_then(|p| *p).ok_or("BT NO TD")?;
        if !Self::matches(p, words) {
            return Err("BT BAD COMPLETION");
        }
        self.pending[index] = None;
        self.saved[index] = None;
        Ok((p.bytes - (words[2] & 0xffffff)) as usize)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn extra_completions_are_exact_and_never_overwrite() {
        let mut m = Mailbox::new();
        m.arm(0, 0x1000, 2, 3, 257, true).unwrap();
        let good = [
            0x1000,
            0,
            (13 << 24) | 251,
            (32 << 10) | (3 << 16) | (2 << 24) | 1,
        ];
        let mut bad = good;
        bad[3] ^= 1 << 24;
        assert!(!m.save(bad));
        assert!(m.save(good));
        assert!(!m.save(good));
        assert!(m.arm(0, 0x2000, 2, 3, 257, true).is_err());
        let w = m.take(0).unwrap();
        assert_eq!(m.complete(0, w), Ok(6));
        assert!(m.arm(0, 0x2000, 2, 3, 257, true).is_ok());
        assert!(!m.save(good));
    }
}
