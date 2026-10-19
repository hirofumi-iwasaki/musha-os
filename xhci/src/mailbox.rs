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
    owner: Option<u8>,
    fault: Option<[u32; 4]>,
    faults: u32,
}
impl Mailbox {
    pub const fn new() -> Self {
        Self {
            pending: [None; 3],
            saved: [None; 3],
            retired: [None; 6],
            owner: None,
            fault: None,
            faults: 0,
        }
    }
    /// Attribute every otherwise-unclaimed Transfer Event of this slot (EP0,
    /// event, ACL or an unknown DCI) to Bluetooth instead of the shared ring.
    pub fn own(&mut self, slot: u8) -> Result<(), &'static str> {
        if !(1..=8).contains(&slot) || self.owner.is_some_and(|s| s != slot) {
            return Err("BT TRANSFER OWNER");
        }
        self.owner = Some(slot);
        Ok(())
    }
    /// Only after a correlated Disable Slot: a reused slot ID belongs to its new owner.
    pub fn disown(&mut self) {
        self.owner = None;
    }
    /// Consume an unmatched event of the owned slot. The first is kept for the
    /// Bluetooth poller; later ones are counted. No TD, buffer or DMA is released.
    pub fn capture_fault(&mut self, words: [u32; 4]) -> bool {
        if (words[3] >> 10) & 63 != 32 || self.owner != Some((words[3] >> 24) as u8) {
            return false;
        }
        if self.fault.is_none() {
            self.fault = Some(words);
        }
        self.faults = self.faults.saturating_add(1);
        true
    }
    /// Read-only ownership metadata; never consumes a completion or releases DMA.
    pub fn observation(&self, index: usize, trb: usize, slot: u8, dci: u8) -> (bool, bool) {
        let matches = self
            .pending
            .get(index)
            .and_then(|p| *p)
            .is_some_and(|p| p.trb == trb && p.slot == slot && p.dci == dci && p.incoming);
        (matches, self.saved.get(index).is_some_and(Option::is_some))
    }
    pub fn faulted(&self) -> bool {
        self.fault.is_some()
    }
    pub fn take_fault(&mut self) -> Option<([u32; 4], u32)> {
        let words = self.fault.take()?;
        let count = core::mem::take(&mut self.faults);
        Some((words, count))
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

#[cfg(test)]
mod fault_tests {
    use super::*;
    fn event(trb: u32, slot: u8, dci: u8, cc: u32, residual: u32) -> [u32; 4] {
        [
            trb,
            0,
            (cc << 24) | residual,
            (32 << 10) | ((dci as u32) << 16) | ((slot as u32) << 24) | 1,
        ]
    }
    #[test]
    fn owned_slot_error_events_become_bluetooth_faults_without_reclaiming_tds() {
        let mut m = Mailbox::new();
        m.own(2).unwrap();
        m.arm(0, 0x1000, 2, 3, 256, true).unwrap();
        // Babble (3) on the pending event TD, a STALL (6) on EP0 and an unknown
        // DCI are all Bluetooth faults, never matched completions.
        assert!(!m.save(event(0x1000, 2, 3, 3, 0)));
        assert!(m.capture_fault(event(0x1000, 2, 3, 3, 0)));
        assert!(m.capture_fault(event(0x3000, 2, 1, 6, 0)));
        assert!(m.capture_fault(event(0x4000, 2, 7, 1, 0)));
        assert!(m.faulted());
        let (first, count) = m.take_fault().unwrap();
        assert_eq!(first[2] >> 24, 3);
        assert_eq!(count, 3);
        assert!(!m.faulted());
        assert!(m.pending[0].is_some()); // the loan stays pending for retirement
        // Other slots and non-transfer events remain shared-ring failures.
        assert!(!m.capture_fault(event(0x1000, 3, 3, 3, 0)));
        let mut command = event(0x1000, 2, 0, 1, 0);
        command[3] = (33 << 10) | (2 << 24) | 1;
        assert!(!m.capture_fault(command));
    }
    #[test]
    fn matched_completions_take_precedence_and_disown_stops_attribution() {
        let mut m = Mailbox::new();
        m.own(2).unwrap();
        assert!(m.own(3).is_err());
        m.arm(0, 0x1000, 2, 3, 256, true).unwrap();
        assert!(m.save(event(0x1000, 2, 3, 13, 240)));
        assert!(!m.faulted());
        m.retire_slot(2, &[]).unwrap();
        m.disown();
        assert!(!m.capture_fault(event(0x5000, 2, 3, 3, 0)));
        assert!(m.own(3).is_ok());
        assert_eq!(m.own(0), Err("BT TRANSFER OWNER"));
    }
}

#[cfg(test)]
mod observation_tests {
    use super::Mailbox;
    #[test]
    fn observation_checks_exact_in_ownership_without_consuming_saved_event() {
        let mut m = Mailbox::new();
        assert_eq!(m.observation(1, 0x1000, 2, 5), (false, false));
        m.arm(1, 0x1000, 2, 5, 2048, true).unwrap();
        assert_eq!(m.observation(1, 0x1000, 2, 5), (true, false));
        assert_eq!(m.observation(1, 0x1010, 2, 5), (false, false));
        assert_eq!(m.observation(1, 0x1000, 3, 5), (false, false));
        assert_eq!(m.observation(1, 0x1000, 2, 7), (false, false));
        assert_eq!(m.observation(3, 0x1000, 2, 5), (false, false));
        let event = [
            0x1000,
            0,
            (13 << 24) | (2048 - 15),
            (2 << 24) | (5 << 16) | (32 << 10),
        ];
        assert!(m.save(event));
        assert_eq!(m.observation(1, 0x1000, 2, 5), (true, true));
        assert_eq!(m.observation(1, 0x1000, 2, 5), (true, true));
        assert_eq!(m.take(1), Some(event));
        assert_eq!(m.complete(1, event), Ok(15));
        assert_eq!(m.observation(1, 0x1000, 2, 5), (false, false));
    }
}
