// Copyright 2026 Hirofumi Iwasaki
// SPDX-License-Identifier: Apache-2.0
//! Single CPU-state lease. Dropping the borrower retires it; confirmed owner
//! controller halt/BME-off is separately required before another acquisition.
#[derive(Clone, Copy, PartialEq, Eq)]
enum State {
    Free,
    Live,
    Retired,
}
pub struct Lease {
    state: State,
    owner: u16,
}
impl Lease {
    pub const fn new() -> Self {
        Self {
            state: State::Free,
            owner: 0,
        }
    }
    pub fn acquire(&mut self, owner: u16) -> bool {
        if self.state != State::Free {
            return false;
        }
        self.owner = owner;
        self.state = State::Live;
        true
    }
    pub fn retire(&mut self) {
        if self.state == State::Live {
            self.state = State::Retired;
        }
    }
    pub fn stopped(&mut self, owner: u16) -> bool {
        if self.state == State::Free {
            return true;
        }
        if self.owner != owner || self.state != State::Retired {
            return false;
        }
        self.state = State::Free;
        true
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exclusive_quarantine_and_restart() {
        let mut l = Lease::new();
        assert!(l.acquire(1));
        assert!(!l.acquire(1));
        assert!(!l.stopped(1));
        l.retire();
        assert!(!l.acquire(2));
        assert!(!l.stopped(2));
        assert!(l.stopped(1));
        assert!(l.acquire(2));
        l.retire();
        assert!(l.stopped(2));
    }
}
