// Copyright 2026 Hirofumi Iwasaki
// SPDX-License-Identifier: Apache-2.0
//! Validation of an already selected adapter; never selects or reorders devices.
pub fn matches(expected: Option<(u16, u16)>, actual: (u16, u16)) -> bool {
    expected.is_none_or(|e| e == actual)
}
/// Session-scoped validation state. Missing adapters on other live hosts must
/// never overwrite the already selected adapter's validation result.
pub struct Validation {
    selected: bool,
    blocked: bool,
}
impl Validation {
    pub const fn new() -> Self {
        Self {
            selected: false,
            blocked: false,
        }
    }
    pub fn observe(&mut self, matches: bool) {
        self.selected = true;
        self.blocked = !matches;
    }
    pub fn missing(&mut self) {
        if !self.selected {
            self.blocked = true;
        }
    }
    pub fn selected(&self) -> bool {
        self.selected
    }
    pub fn blocked(&self) -> bool {
        self.blocked
    }
}
