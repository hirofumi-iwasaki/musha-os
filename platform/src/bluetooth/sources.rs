// Copyright 2026 Hirofumi Iwasaki
// SPDX-License-Identifier: Apache-2.0
//! Bounded CPU entropy acquisition policy. No hardware access or clock seeds.
use zeroize::Zeroize;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Failure {
    Unsupported,
    RetryExhausted,
}
pub fn collect(
    supported: bool,
    retries: usize,
    mut instruction: impl FnMut() -> Option<u64>,
) -> Result<[u8; 64], Failure> {
    if !supported {
        return Err(Failure::Unsupported);
    }
    let mut output = [0; 64];
    for word in output.chunks_exact_mut(8) {
        let mut value = None;
        for _ in 0..retries {
            if let Some(n) = instruction() {
                value = Some(n);
                break;
            }
        }
        let Some(n) = value else {
            output.zeroize();
            return Err(Failure::RetryExhausted);
        };
        word.copy_from_slice(&n.to_le_bytes());
    }
    Ok(output)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unsupported_never_executes() {
        assert_eq!(collect(false, 10, || panic!()), Err(Failure::Unsupported));
    }
    #[test]
    fn carry_failure_retries_and_never_accepts_failed_value() {
        let mut n = 0;
        let a = collect(true, 10, || {
            n += 1;
            if n % 2 == 0 { Some(n) } else { None }
        })
        .unwrap();
        assert_eq!(n, 16);
        assert_eq!(&a[..8], &2u64.to_le_bytes());
    }
    #[test]
    fn retries_are_bounded_per_word() {
        let mut n = 0;
        assert_eq!(
            collect(true, 10, || {
                n += 1;
                None
            }),
            Err(Failure::RetryExhausted)
        );
        assert_eq!(n, 10);
    }
}

/// Four LE Rand responses, strictly bounded and never Debug-printable.
pub struct HciRand {
    bytes: [u8; 32],
    count: u8,
    done: bool,
}
impl HciRand {
    pub const fn new() -> Self {
        Self {
            bytes: [0; 32],
            count: 0,
            done: false,
        }
    }
    pub fn done(&self) -> bool {
        self.done
    }
    pub fn count(&self) -> u8 {
        self.count
    }
    pub fn complete(&mut self, result: &[u8]) -> Result<Option<[u8; 32]>, HciFailure> {
        if self.done || result.is_empty() {
            return Err(HciFailure::Format);
        }
        if result[0] != 0 {
            self.done = true;
            self.bytes.zeroize();
            return Ok(None);
        }
        if result.len() != 9 {
            self.done = true;
            self.bytes.zeroize();
            return Err(HciFailure::Format);
        }
        let at = self.count as usize * 8;
        self.bytes[at..at + 8].copy_from_slice(&result[1..]);
        self.count += 1;
        if self.count < 4 {
            return Ok(None);
        }
        self.done = true;
        if !super::drbg::healthy(&self.bytes) {
            self.bytes.zeroize();
            return Err(HciFailure::Health);
        }
        let output = self.bytes;
        self.bytes.zeroize();
        Ok(Some(output))
    }
}
impl Drop for HciRand {
    fn drop(&mut self) {
        self.bytes.zeroize();
    }
}
#[derive(Debug, PartialEq, Eq)]
pub enum HciFailure {
    Format,
    Health,
}
#[cfg(test)]
mod hci_tests {
    use super::*;
    #[test]
    fn four_complete_responses_only() {
        let mut s = HciRand::new();
        for n in 1..=4 {
            let mut p = [n; 9];
            p[0] = 0;
            let r = s.complete(&p).unwrap();
            assert_eq!(r.is_some(), n == 4);
        }
        assert!(s.done());
        assert_eq!(s.count(), 4);
        assert_eq!(s.complete(&[0; 9]), Err(HciFailure::Format));
    }
    #[test]
    fn unsupported_is_auxiliary_unavailable() {
        let mut s = HciRand::new();
        assert_eq!(s.complete(&[1]), Ok(None));
        assert!(s.done());
    }
    #[test]
    fn malformed_and_stuck_outputs_are_rejected() {
        let mut s = HciRand::new();
        assert_eq!(s.complete(&[0]), Err(HciFailure::Format));
        let mut s = HciRand::new();
        for _ in 0..3 {
            assert_eq!(s.complete(&[0; 9]), Ok(None));
        }
        assert_eq!(s.complete(&[0; 9]), Err(HciFailure::Health));
    }
}
