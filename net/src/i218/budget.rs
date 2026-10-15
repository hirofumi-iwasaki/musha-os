// Copyright 2026 Hirofumi Iwasaki
// SPDX-License-Identifier: Apache-2.0
//! One monotonic deadline shared by every nested initialization access/wait.
use super::Error;
pub struct Budget {
    start: u64,
    last: u64,
    expired: bool,
}
impl Budget {
    pub fn new(now: u64) -> Self {
        Self {
            start: now,
            last: now,
            expired: false,
        }
    }
    pub fn check(&mut self, now: u64, delay_us: u32) -> Result<(), Error> {
        if now < self.last {
            self.expired = true;
            return Err(Error::Clock);
        }
        self.last = now;
        let elapsed = now.checked_sub(self.start).ok_or(Error::Clock)?;
        let remaining = 5000u64.saturating_sub(elapsed);
        if self.expired || elapsed >= 5000 || u64::from(delay_us) > remaining * 1000 {
            self.expired = true;
            return Err(Error::InitBudget);
        }
        Ok(())
    }
    pub fn expired(&self) -> bool {
        self.expired
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn aggregate_not_per_wait() {
        let mut b = Budget::new(100);
        for n in 0..50 {
            assert!(b.check(100 + n * 100, 1000).is_ok());
        }
        assert_eq!(b.check(5100, 0), Err(Error::InitBudget));
        assert_eq!(b.check(5100, 0), Err(Error::InitBudget));
    }
    #[test]
    fn delay_boundary_and_clock() {
        assert!(Budget::new(0).check(4999, 1000).is_ok());
        assert_eq!(Budget::new(0).check(4999, 1001), Err(Error::InitBudget));
        let mut b = Budget::new(100);
        assert_eq!(b.check(99, 0), Err(Error::Clock));
        assert!(b.expired());
        assert!(Budget::new(u64::MAX - 1).check(u64::MAX, 0).is_ok());
    }
}
