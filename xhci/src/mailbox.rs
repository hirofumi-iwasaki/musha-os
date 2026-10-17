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
    retired: [Option<(usize, u8, u8)>; 6],
}
impl Mailbox {
    pub const fn new() -> Self {
        Self {
            pending: [None; 3],
            saved: [None; 3],
            retired: [None; 6],
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
    /// Retain exact TD identities while Disable Slot is pending and afterwards.
    /// This consumes only stale completion metadata, never buffers. No allocation
    /// or CPU lease is released; the outer controller stop still owns reclamation.
    pub fn retire_slot(&mut self, slot: u8, control: &[usize]) -> Result<(), &'static str> {
        if !(1..=8).contains(&slot) || control.len() > 3 || self.retired.iter().any(Option::is_some)
        {
            return Err("BT RETIRE OWNER");
        }
        for (i, pending) in self.pending.iter().enumerate() {
            if let Some(p) = pending {
                if p.slot != slot {
                    return Err("BT RETIRE OWNER");
                }
                self.retired[i] = Some((p.trb, slot, p.dci));
            }
        }
        for (i, &trb) in control.iter().enumerate() {
            if trb == 0 || trb % 16 != 0 {
                return Err("BT RETIRE TD");
            }
            self.retired[3 + i] = Some((trb, slot, 1));
        }
        Ok(())
    }
    pub fn save(&mut self, words: [u32; 4]) -> bool {
        let address = words[0] as u64 | ((words[1] as u64) << 32);
        if (words[3] >> 10) & 63 == 32
            && words[3] & (4 | 0xe00000) == 0
            && words[2] >> 24 != 0
            && self.retired.iter().flatten().any(|&(trb, slot, dci)| {
                address == trb as u64
                    && (words[3] >> 24) as u8 == slot
                    && (words[3] >> 16) & 31 == dci as u32
            })
        {
            return true;
        }

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

#[cfg(test)]
mod retirement_tests {
    use super::*;
    fn event(trb: u32, slot: u8, dci: u8, cc: u32) -> [u32; 4] {
        [
            trb,
            0,
            cc << 24,
            (32 << 10) | ((dci as u32) << 16) | ((slot as u32) << 24) | 1,
        ]
    }
    #[test]
    fn quiet_pending_in_cannot_be_rearmed_and_retirement_keeps_exact_loan() {
        let mut m = Mailbox::new();
        m.arm(0, 0x1000, 2, 3, 257, true).unwrap();
        // Quiet poll: no completion exists. Neither take nor elapsed time owns
        // a cancellation; DMA loan remains pending until completion or retire.
        assert_eq!(m.take(0), None);
        assert!(m.arm(0, 0x1010, 2, 3, 257, true).is_err());
        m.retire_slot(2, &[0x2000, 0x2010, 0x2020]).unwrap();
        assert!(m.pending[0].is_some());
        assert!(m.save(event(0x1000, 2, 3, 26)));
        assert!(!m.save(event(0x1010, 2, 3, 26)));
        assert!(m.arm(0, 0x1010, 2, 3, 257, true).is_err());
    }
    #[test]
    fn retirement_consumes_only_exact_old_bt_td_and_keeps_other_saved_work() {
        let mut m = Mailbox::new();
        m.arm(0, 0x1000, 2, 3, 257, true).unwrap();
        m.retire_slot(2, &[0x2000, 0x2010, 0x2020]).unwrap();
        for cc in [1, 6, 13, 26, 27, 28] {
            assert!(m.save(event(0x1000, 2, 3, cc)));
            assert!(m.save(event(0x2010, 2, 1, cc)));
        }
        for w in [
            event(0x1010, 2, 3, 26),
            event(0x1000, 3, 3, 26),
            event(0x1000, 2, 5, 26),
            event(0x2030, 2, 1, 26),
            event(0x1000, 2, 3, 0),
        ] {
            assert!(!m.save(w));
        }
        let mut ed = event(0x1000, 2, 3, 26);
        ed[3] |= 4;
        assert!(!m.save(ed));
        assert!(m.retire_slot(3, &[]).is_err()); // no widening to another slot
        assert!(m.pending[0].is_some()); // no reclamation even after retire
    }
}
