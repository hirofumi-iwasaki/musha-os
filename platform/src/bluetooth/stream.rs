// Copyright 2026 Hirofumi Iwasaki
// SPDX-License-Identifier: Apache-2.0
//! Bounded HCI-over-USB framing, independent from ACL/L2CAP fragmentation.
//! A completion may end mid-header/payload or contain multiple HCI packets.
use super::Error;
pub struct Stream<const N: usize> {
    bytes: [u8; N],
    generation: u64,
    acl: bool,
    used: usize,
    expected: usize,
    complete: bool,
    deadline: Option<u64>,
    last: u64,
    failed: bool,
}
impl<const N: usize> Stream<N> {
    pub const fn new(generation: u64, acl: bool) -> Self {
        Self {
            bytes: [0; N],
            generation,
            acl,
            used: 0,
            expected: 0,
            complete: false,
            deadline: None,
            last: 0,
            failed: false,
        }
    }
    fn erase(&mut self) {
        #[cfg(feature = "bluetooth-legacy")]
        {
            use zeroize::Zeroize;
            self.bytes.zeroize();
        }
        #[cfg(not(feature = "bluetooth-legacy"))]
        self.bytes.fill(0);
        self.used = 0;
        self.expected = 0;
        self.complete = false;
        self.deadline = None;
    }
    pub fn release(&mut self) {
        if self.complete {
            self.erase();
        }
    }
    /// Header/payload progress only; never expose buffered event bytes.
    pub fn progress(&self) -> (usize, usize) {
        (self.used, self.expected)
    }
    fn reject(&mut self, error: Error) -> Error {
        self.erase();
        self.failed = true;
        error
    }
    pub fn check(&mut self, generation: u64, now: u64) -> Result<(), Error> {
        if self.failed || generation != self.generation {
            return Err(self.reject(Error::Stale));
        }
        if now < self.last || self.deadline.is_some_and(|d| now >= d) {
            return Err(self.reject(Error::Timeout));
        }
        self.last = now;
        Ok(())
    }
    /// The returned packet borrows CPU storage, never the DMA receive buffer.
    /// Consume it before the next call; no caller-owned pointer is retained.
    pub fn next<'a>(
        &'a mut self,
        generation: u64,
        now: u64,
        input: &mut &[u8],
    ) -> Result<Option<&'a [u8]>, Error> {
        if self.complete {
            self.erase();
        }
        self.check(generation, now)?;
        let header = if self.acl { 4 } else { 2 };
        if N < header {
            return Err(self.reject(Error::Length));
        }
        while !input.is_empty() {
            if self.used == 0 {
                self.deadline = Some(
                    now.checked_add(1000)
                        .ok_or_else(|| self.reject(Error::Timeout))?,
                );
            }
            let target = if self.expected == 0 {
                header
            } else {
                self.expected
            };
            let take = (target - self.used).min(input.len());
            self.bytes[self.used..self.used + take].copy_from_slice(&input[..take]);
            self.used += take;
            *input = &input[take..];
            if self.used == header && self.expected == 0 {
                let len = if self.acl {
                    u16::from_le_bytes([self.bytes[2], self.bytes[3]]) as usize
                } else {
                    self.bytes[1] as usize
                };
                self.expected = header + len;
                if self.expected > N {
                    return Err(self.reject(Error::Length));
                }
            }
            if self.expected != 0 && self.used == self.expected {
                self.complete = true;
                self.deadline = None;
                return Ok(Some(&self.bytes[..self.used]));
            }
        }
        Ok(None)
    }
}
impl<const N: usize> Drop for Stream<N> {
    fn drop(&mut self) {
        self.erase();
    }
}
