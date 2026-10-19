// Copyright 2026 Hirofumi Iwasaki
// SPDX-License-Identifier: Apache-2.0
//! Explicit re-registration intent; no key deletion or hardware access.
use core::sync::atomic::{AtomicBool, AtomicU8, Ordering};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    StartFresh,
    RestartFresh,
    AlreadyPairing,
}
pub fn action(started: bool, pairing: bool, subscribed: bool) -> Action {
    if !started {
        Action::StartFresh
    } else if subscribed || !pairing {
        Action::RestartFresh
    } else {
        Action::AlreadyPairing
    }
}
pub struct Restart {
    pending: AtomicBool,
    safe: AtomicBool,
    force: AtomicBool,
    count: AtomicU8,
}
impl Restart {
    pub const fn new() -> Self {
        Self {
            pending: AtomicBool::new(false),
            safe: AtomicBool::new(false),
            force: AtomicBool::new(false),
            count: AtomicU8::new(0),
        }
    }
    pub fn request(&self, force: bool) -> bool {
        if self.pending.load(Ordering::Relaxed) {
            if force {
                self.force.store(true, Ordering::Relaxed);
            }
            return true;
        }
        if self
            .count
            .try_update(Ordering::Relaxed, Ordering::Relaxed, |n| {
                (n < 8).then_some(n + 1)
            })
            .is_err()
        {
            return false;
        }
        self.force.store(force, Ordering::Relaxed);
        self.safe.store(false, Ordering::Relaxed);
        self.pending.store(true, Ordering::Relaxed);
        true
    }
    pub fn isolated(&self) {
        self.pending.store(false, Ordering::Relaxed);
        self.safe.store(false, Ordering::Relaxed);
        self.force.store(false, Ordering::Relaxed);
    }
    pub fn wants(&self) -> bool {
        self.pending.load(Ordering::Relaxed)
    }
    pub fn confirm(&self, stopped: bool) {
        if self.wants() {
            self.safe.store(stopped, Ordering::Relaxed);
        }
    }
    pub fn take(&self) -> bool {
        self.safe.load(Ordering::Relaxed) && self.pending.swap(false, Ordering::Relaxed)
    }
    pub fn take_force(&self) -> bool {
        self.safe.load(Ordering::Relaxed)
            && !self.wants()
            && self.force.swap(false, Ordering::Relaxed)
    }
}
