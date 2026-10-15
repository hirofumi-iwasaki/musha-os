// Copyright 2026 Hirofumi Iwasaki
// SPDX-License-Identifier: Apache-2.0
//! Finite boot-owned entropy, supplied by a trusted SP800-90 UEFI RNG before EBS.
//! No PRNG, clock/seed fallback, firmware pointer or reuse after consumption.
use super::Error;
use zeroize::Zeroize;
pub trait Random {
    fn fill(&mut self, out: &mut [u8]) -> Result<(), Error>;
}
pub struct Pool<const N: usize> {
    bytes: [u8; N],
    position: usize,
    ready: bool,
}
impl<const N: usize> Pool<N> {
    pub const fn new() -> Self {
        Self {
            bytes: [0; N],
            position: N,
            ready: false,
        }
    }
    /// Provider failure or obvious stuck output invalidates the entire pool.
    /// These checks are not a certification of firmware entropy quality.
    pub fn initialize(
        &mut self,
        mut provider: impl FnMut(&mut [u8]) -> Result<(), Error>,
    ) -> Result<(), Error> {
        if self.ready || N == 0 || N % 32 != 0 {
            return Err(Error::Invalid);
        }
        self.bytes.zeroize();
        self.position = N;
        let mut previous = [0; 32];
        for chunk in self.bytes.chunks_mut(32) {
            if provider(chunk).is_err()
                || chunk.iter().all(|b| *b == 0)
                || chunk.iter().all(|b| *b == 255)
                || chunk == previous
            {
                self.bytes.zeroize();
                previous.zeroize();
                return Err(Error::Entropy);
            }
            previous.copy_from_slice(chunk);
        }
        previous.zeroize();
        self.position = 0;
        self.ready = true;
        Ok(())
    }
    pub fn remaining(&self) -> usize {
        N - self.position
    }
}
impl<const N: usize> Random for Pool<N> {
    fn fill(&mut self, out: &mut [u8]) -> Result<(), Error> {
        if !self.ready || out.is_empty() || out.len() > N - self.position {
            out.zeroize();
            return Err(Error::Entropy);
        }
        let end = self.position + out.len();
        out.copy_from_slice(&self.bytes[self.position..end]);
        self.bytes[self.position..end].zeroize();
        self.position = end;
        Ok(())
    }
}
impl<const N: usize> Drop for Pool<N> {
    fn drop(&mut self) {
        self.bytes.zeroize();
    }
}
/// Uniform six-digit sampling without modulo bias or unbounded rejection.
pub fn passkey(rng: &mut impl Random) -> Result<u32, Error> {
    for _ in 0..16 {
        let mut b = [0; 4];
        rng.fill(&mut b)?;
        let n = u32::from_le_bytes(b);
        b.zeroize();
        if n < 4_294_000_000 {
            return Ok(n % 1_000_000);
        }
    }
    Err(Error::Entropy)
}
