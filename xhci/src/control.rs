// SPDX-License-Identifier: Apache-2.0
//! One outstanding control TD; the last TRB is a toggle-cycle Link.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Cursor {
    index: usize,
    cycle: u32,
}
pub struct Plan {
    pub indices: [usize; 3],
    pub cycles: [u32; 3],
    pub count: usize,
    pub link_cycle: Option<u32>,
    pub next: Cursor,
}
impl Cursor {
    pub const NEW: Self = Self { index: 0, cycle: 1 };
    /// Planning does not advance ownership; commit `next` after Status completes.
    pub fn plan(self, data: bool) -> Plan {
        let mut p = Plan {
            indices: [0; 3],
            cycles: [0; 3],
            count: if data { 3 } else { 2 },
            link_cycle: None,
            next: self,
        };
        for i in 0..p.count {
            p.indices[i] = p.next.index;
            p.cycles[i] = p.next.cycle;
            p.next.index += 1;
            if p.next.index == 255 {
                p.link_cycle = Some(p.next.cycle);
                p.next.index = 0;
                p.next.cycle ^= 1;
            }
        }
        p
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn mixed_td_wraps_and_planning_is_atomic() {
        let mut c = Cursor::NEW;
        let mut n = 0;
        for i in 0..2000 {
            let p = c.plan(i % 2 == 0);
            assert_eq!(c.plan(i % 2 == 0).indices, p.indices);
            for j in 0..p.count {
                assert_eq!(p.indices[j], n % 255);
                assert_eq!(p.cycles[j], 1 ^ ((n / 255) & 1) as u32);
                n += 1;
            }
            c = p.next;
        }
    }
}

/// Validate a short Data Stage event independently from Status completion.
pub fn short_residual(words: [u32; 4], data: usize, slot: u8, requested: u32) -> Option<u32> {
    let remaining = words[2] & 0xffffff;
    if data % 16 != 0
        || requested == 0
        || remaining == 0
        || remaining > requested
        || words[0] as u64 | ((words[1] as u64) << 32) != data as u64
        || words[2] >> 24 != 13
        || (words[3] >> 10) & 63 != 32
        || (words[3] >> 24) as u8 != slot
        || (words[3] >> 16) & 31 != 1
        || words[3] & (4 | 0xe00000) != 0
    {
        return None;
    }
    Some(remaining)
}
#[cfg(test)]
mod short_tests {
    use super::*;
    #[test]
    fn short_stage_identity_and_length() {
        let mut w = [
            0x1000,
            0,
            (13 << 24) | 5,
            (32 << 10) | (1 << 16) | (2 << 24) | 1,
        ];
        assert_eq!(short_residual(w, 0x1000, 2, 18), Some(5));
        assert_eq!(short_residual(w, 0x1010, 2, 18), None);
        assert_eq!(short_residual(w, 0x1000, 3, 18), None);
        assert_eq!(short_residual(w, 0x1000, 2, 4), None);
        w[3] |= 4;
        assert_eq!(short_residual(w, 0x1000, 2, 18), None);
    }
}
