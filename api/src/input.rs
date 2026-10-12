// Copyright 2026 Hirofumi Iwasaki
// SPDX-License-Identifier: Apache-2.0
//! FIFO with latest-state reconciliation after overflow. No allocation.
use crate::KeyEvent;
const CAPACITY: usize = 64;
pub(crate) struct Input {
    fifo: [Option<KeyEvent>; CAPACITY],
    head: usize,
    count: usize,
    actual: [bool; 256],
    delivered: [bool; 256],
    reconcile: bool,
    lost: u64,
}
impl Input {
    pub fn new() -> Self {
        Self {
            fifo: [None; CAPACITY],
            head: 0,
            count: 0,
            actual: [false; 256],
            delivered: [false; 256],
            reconcile: false,
            lost: 0,
        }
    }
    pub fn push(&mut self, usage: u8, pressed: bool, timestamp_ms: u64) {
        self.actual[usage as usize] = pressed;
        if self.count == CAPACITY || self.reconcile {
            self.reconcile = true;
            self.lost = self.lost.saturating_add(1);
            return;
        }
        self.fifo[(self.head + self.count) % CAPACITY] = Some(KeyEvent {
            usage,
            pressed,
            timestamp_ms,
        });
        self.count += 1;
    }
    pub fn next(&mut self, now_ms: u64) -> Option<KeyEvent> {
        if self.count != 0 {
            let event = self.fifo[self.head].take().unwrap();
            self.head = (self.head + 1) % CAPACITY;
            self.count -= 1;
            self.delivered[event.usage as usize] = event.pressed;
            return Some(event);
        }
        if self.reconcile {
            // Release before press, modifiers first in each phase. A pending
            // reconciliation coalesces new producer input until fully drained.
            for pressed in [false, true] {
                for key in (0xe0..=0xe7).chain(0..0xe0).chain(0xe8..256) {
                    if self.actual[key] != self.delivered[key] && self.actual[key] == pressed {
                        self.delivered[key] = pressed;
                        return Some(KeyEvent {
                            usage: key as u8,
                            pressed,
                            timestamp_ms: now_ms,
                        });
                    }
                }
            }
            self.reconcile = false;
        }
        None
    }
    pub fn lost(&self) -> u64 {
        self.lost
    }
}
#[cfg(test)]
mod tests {
    extern crate std;
    use super::*;
    use std::{collections::BTreeSet, vec::Vec};
    fn fill(input: &mut Input) {
        for _ in 0..32 {
            input.push(5, true, 10);
            input.push(5, false, 10);
        }
    }
    #[test]
    fn lost_shift_and_key_releases_reach_consumer_once() {
        let mut q = Input::new();
        q.push(0xe1, true, 1);
        q.push(4, true, 1);
        q.next(1).unwrap();
        q.next(1).unwrap();
        fill(&mut q);
        q.push(0xe1, false, 11);
        q.push(4, false, 11);
        assert_eq!(q.lost(), 2);
        for _ in 0..64 {
            assert_eq!(q.next(20).unwrap().timestamp_ms, 10);
        }
        assert_eq!(
            q.next(20),
            Some(KeyEvent {
                usage: 0xe1,
                pressed: false,
                timestamp_ms: 20
            })
        );
        assert_eq!(
            q.next(20),
            Some(KeyEvent {
                usage: 4,
                pressed: false,
                timestamp_ms: 20
            })
        );
        assert_eq!(q.next(20), None);
        assert_eq!(q.next(20), None);
    }
    #[test]
    fn new_input_during_recovery_coalesces_without_reordering_old_fifo() {
        let mut q = Input::new();
        fill(&mut q);
        q.push(4, true, 11);
        q.push(4, false, 12);
        q.push(0xe1, true, 13);
        for _ in 0..32 {
            q.next(14).unwrap();
        }
        q.push(6, true, 15); // even though FIFO space is now available
        for _ in 0..32 {
            assert_eq!(q.next(16).unwrap().usage, 5);
        }
        let e = q.next(16).unwrap();
        assert_eq!((e.usage, e.pressed), (0xe1, true));
        q.push(0xe1, false, 17);
        let e = q.next(18).unwrap();
        assert_eq!((e.usage, e.pressed), (0xe1, false));
        let e = q.next(18).unwrap();
        assert_eq!((e.usage, e.pressed), (6, true));
        assert_eq!(q.next(18), None);
        assert_eq!(q.lost(), 5);
        q.push(6, false, 19);
        assert_eq!(q.next(20).unwrap().timestamp_ms, 19);
    }
    #[test]
    fn all_usages_recover_and_overflow_counter_saturates() {
        let mut q = Input::new();
        for key in 0..=255 {
            q.push(key, true, 1);
        }
        let mut held = BTreeSet::new();
        while let Some(e) = q.next(2) {
            assert!(e.pressed);
            assert!(held.insert(e.usage));
        }
        assert_eq!(held.len(), 256);
        assert_eq!(q.lost(), 192);
        for key in 0..=255 {
            q.push(key, false, 3);
        }
        let mut count = 0;
        while let Some(e) = q.next(4) {
            assert!(!e.pressed);
            assert!(held.remove(&e.usage));
            count += 1;
        }
        assert_eq!(count, 256);
        assert!(held.is_empty());
        q.lost = u64::MAX;
        fill(&mut q);
        q.push(4, true, 20);
        assert_eq!(q.lost(), u64::MAX);
    }
    #[test]
    fn deterministic_producer_consumer_interleavings_converge() {
        let mut q = Input::new();
        let mut actual = BTreeSet::new();
        let mut app = BTreeSet::new();
        let mut random = 0x71e2b39au32;
        for now in 0..50_000u64 {
            random ^= random << 13;
            random ^= random >> 17;
            random ^= random << 5;
            let key = (random >> 8) as u8;
            if random & 7 != 0 {
                let pressed = !actual.contains(&key);
                if pressed {
                    actual.insert(key);
                } else {
                    actual.remove(&key);
                }
                q.push(key, pressed, now);
            } else if let Some(e) = q.next(now) {
                if e.pressed {
                    app.insert(e.usage);
                } else {
                    app.remove(&e.usage);
                }
            }
        }
        let mut drained = Vec::new();
        while let Some(e) = q.next(50_000) {
            if e.pressed {
                app.insert(e.usage);
            } else {
                app.remove(&e.usage);
            }
            drained.push(e);
            assert!(drained.len() <= 64 + 256);
        }
        assert_eq!(app, actual);
        assert!(q.lost() > 0);
        for key in actual {
            q.push(key, false, 50_001);
        }
        while let Some(e) = q.next(50_002) {
            assert!(!e.pressed);
            app.remove(&e.usage);
        }
        assert!(app.is_empty());
    }
}
