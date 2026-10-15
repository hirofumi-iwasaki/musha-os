// Copyright 2026 Hirofumi Iwasaki
// SPDX-License-Identifier: Apache-2.0
//! Single-consumer six-digit entry; no text/key logging or held-key repeats.
use zeroize::Zeroize;
pub struct Secret(u32);
impl Secret {
    pub fn use_with<T>(&self, f: impl FnOnce(u32) -> T) -> T {
        f(self.0)
    }
}
impl Drop for Secret {
    fn drop(&mut self) {
        self.0.zeroize();
    }
}
pub enum Outcome {
    Submitted(Secret),
    Cancelled,
    TimedOut,
}
pub struct Entry {
    generation: u64,
    digits: [u8; 6],
    length: usize,
    deadline: u64,
    last: u64,
    finished: bool,
}
impl Entry {
    pub fn new(generation: u64, now: u64) -> Option<Self> {
        Some(Self {
            generation,
            digits: [0; 6],
            length: 0,
            deadline: now.checked_add(30000)?,
            last: now,
            finished: false,
        })
    }
    pub fn digits(&self) -> usize {
        self.length
    }
    pub fn tick(&mut self, generation: u64, now: u64) -> Option<Outcome> {
        if self.finished {
            return None;
        }
        if generation != self.generation || now < self.last || now >= self.deadline {
            self.finished = true;
            self.digits.zeroize();
            self.length = 0;
            return Some(Outcome::TimedOut);
        }
        self.last = now;
        None
    }
    pub fn key(&mut self, generation: u64, usage: u8, down: bool, now: u64) -> Option<Outcome> {
        if let Some(out) = self.tick(generation, now) {
            return Some(out);
        }
        if self.finished || !down {
            return None;
        }
        if usage == 0x29 {
            self.finished = true;
            self.digits.zeroize();
            self.length = 0;
            return Some(Outcome::Cancelled);
        }
        if usage == 0x2a {
            if self.length > 0 {
                self.length -= 1;
                self.digits[self.length].zeroize();
            }
            return None;
        }
        if usage == 0x28 && self.length == 6 {
            let mut value = 0;
            for digit in self.digits {
                value = value * 10 + digit as u32;
            }
            self.digits.zeroize();
            self.length = 0;
            self.finished = true;
            return Some(Outcome::Submitted(Secret(value)));
        }
        let digit = match usage {
            0x1e..=0x26 => usage - 0x1d,
            0x27 => 0,
            0x59..=0x61 => usage - 0x58,
            0x62 => 0,
            _ => return None,
        };
        if self.length < 6 {
            self.digits[self.length] = digit;
            self.length += 1;
        }
        None
    }
}
impl Drop for Entry {
    fn drop(&mut self) {
        self.digits.zeroize();
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn digits_leading_zero_range_backspace_enter_and_one_consumer() {
        let mut e = Entry::new(1, 0).unwrap();
        assert!(e.key(1, 0x28, true, 1).is_none());
        for u in [0x27, 0x1e, 0x1f, 0x20, 0x21, 0x22, 0x23] {
            assert!(e.key(1, u, true, 2).is_none());
        }
        assert_eq!(e.digits(), 6);
        e.key(1, 0x2a, true, 3);
        e.key(1, 0x26, false, 3);
        assert_eq!(e.digits(), 5);
        e.key(1, 0x26, true, 3);
        match e.key(1, 0x28, true, 4).unwrap() {
            Outcome::Submitted(s) => s.use_with(|v| assert_eq!(v, 12349)),
            _ => panic!(),
        };
        assert!(e.key(1, 0x28, true, 5).is_none());
    }
    #[test]
    fn cancel_deadline_backward_time_and_generation_never_submit() {
        let mut e = Entry::new(1, 0).unwrap();
        assert!(matches!(e.key(1, 0x29, true, 1), Some(Outcome::Cancelled)));
        for (generation, now) in [(2, 1), (1, 30000)] {
            let mut e = Entry::new(1, 0).unwrap();
            assert!(matches!(e.tick(generation, now), Some(Outcome::TimedOut)));
            assert_eq!(e.digits(), 0);
        }
        let mut e = Entry::new(1, 10).unwrap();
        assert!(matches!(e.tick(1, 9), Some(Outcome::TimedOut)));
        assert!(Entry::new(1, u64::MAX).is_none());
    }
}
