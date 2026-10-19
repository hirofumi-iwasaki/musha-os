// Copyright 2026 Hirofumi Iwasaki
// SPDX-License-Identifier: Apache-2.0
//! Metadata-only, once-per-connection observation at the pairing reply boundary.
#[derive(Default)]
pub struct Once {
    reported: bool,
    terminal_reported: bool,
}
impl Once {
    pub const fn new() -> Self {
        Self {
            reported: false,
            terminal_reported: false,
        }
    }
    /// Independent final sample, at most one in addition to the 5s sample.
    pub fn terminal(&mut self) -> bool {
        if self.terminal_reported {
            return false;
        }
        self.terminal_reported = true;
        true
    }
    pub fn due(&mut self, awaiting: bool, elapsed: u64) -> bool {
        if self.reported || !awaiting || elapsed < super::retry::REPLY_MS {
            return false;
        }
        self.reported = true;
        true
    }
}
#[derive(Debug, PartialEq, Eq)]
pub struct Endpoint {
    pub state: u8,
    pub kind: u8,
    pub max_packet: u16,
}
impl Endpoint {
    pub fn decode(words: [u32; 2]) -> Self {
        Self {
            state: (words[0] & 7) as u8,
            kind: ((words[1] >> 3) & 7) as u8,
            max_packet: (words[1] >> 16) as u16,
        }
    }
}
pub fn counts(current: [u32; 3], baseline: [u32; 3]) -> [u32; 3] {
    core::array::from_fn(|i| current[i].saturating_sub(baseline[i]))
}

/// Observed output-context position, not proof that hardware consumed a TD.
#[derive(Debug, PartialEq, Eq)]
pub struct Position {
    pub dequeue: u64,
    pub dcs: u8,
    pub stable: bool,
    pub ring_index: Option<usize>,
    pub expected: u64,
    pub cycle: u8,
    pub control: u32,
}
impl Position {
    pub fn decode(
        first: [u32; 2],
        second: [u32; 2],
        ring: u64,
        entries: usize,
        expected: u64,
        cycle: u8,
        control: u32,
    ) -> Self {
        let dequeue = (u64::from(second[1]) << 32 | u64::from(second[0])) & !15;
        let offset = dequeue.checked_sub(ring);
        let ring_index = offset.and_then(|n| {
            if n % 16 == 0 && n / 16 < entries as u64 {
                Some((n / 16) as usize)
            } else {
                None
            }
        });
        Self {
            dequeue,
            dcs: (second[0] & 1) as u8,
            stable: first == second,
            ring_index,
            expected,
            cycle,
            control,
        }
    }
    pub fn relation(&self) -> &'static str {
        if !self.stable {
            "unstable"
        } else if self.dequeue == self.expected {
            "same-td"
        } else if self.ring_index.is_some() {
            "in-ring"
        } else {
            "outside-ring"
        }
    }
}
