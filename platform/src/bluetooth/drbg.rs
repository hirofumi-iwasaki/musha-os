// Copyright 2026 Hirofumi Iwasaki
// SPDX-License-Identifier: Apache-2.0
//! Original HMAC_DRBG/SHA-256 implementation, NIST SP800-90A Rev1 §10.1.2.
//! https://nvlpubs.nist.gov/nistpubs/SpecialPublications/NIST.SP.800-90Ar1.pdf
//! Provider bytes are trusted for entropy quantity; the stuck-output checks do
//! not establish entropy or NIST validation. No clock input or weak-only seed.
use crate::config_hash::sha256;
use zeroize::Zeroize;

pub const MAX_SOURCE_BYTES: usize = 64;
pub const MIN_STRONG_BYTES: usize = 48;
pub const MAX_REQUEST_BYTES: usize = 8192; // Local cap 2^16 bits, below SP800-90A maximum 2^19.
pub const RESEED_INTERVAL: u64 = 1 << 48;
const MAX_SEED: usize = 320;
const DOMAIN: &[u8] = b"Musha-OS HMAC_DRBG v1";
#[derive(Clone, Copy, Default)]
pub struct Sources<'a> {
    pub firmware: Option<&'a [u8]>,
    pub rdseed: Option<&'a [u8]>,
    pub rdrand: Option<&'a [u8]>,
    pub hci: Option<&'a [u8]>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    NoStrongSource,
    SourceLength,
    Health,
    NotReady,
    AlreadyReady,
    RequestLength,
    ReseedRequired,
    Counter,
}

/// Per-source 8-byte stuck checks, including adjacent equal blocks. This is a
/// rejection heuristic, not SP800-90B source qualification or entropy credit.
pub fn healthy(bytes: &[u8]) -> bool {
    if bytes.is_empty() || bytes.len() > MAX_SOURCE_BYTES || bytes.len() % 8 != 0 {
        return false;
    }
    let mut previous: Option<&[u8]> = None;
    for block in bytes.chunks_exact(8) {
        if block.iter().all(|b| *b == 0)
            || block.iter().all(|b| *b == 255)
            || previous == Some(block)
        {
            return false;
        }
        previous = Some(block);
    }
    true
}
// K is always 32 bytes; all messages here have a compile-time bounded maximum.
fn hmac(key: &[u8; 32], parts: &[&[u8]]) -> [u8; 32] {
    let mut inner = [0x36u8; 64 + MAX_SEED + 33];
    let mut outer = [0x5cu8; 96];
    for i in 0..32 {
        inner[i] ^= key[i];
        outer[i] ^= key[i];
    }
    let mut len = 64;
    for part in parts {
        inner[len..len + part.len()].copy_from_slice(part);
        len += part.len();
    }
    let mut digest = sha256(&inner[..len]);
    outer[64..].copy_from_slice(&digest);
    let result = sha256(&outer);
    digest.zeroize();
    inner.zeroize();
    outer.zeroize();
    result
}
pub struct Drbg {
    key: [u8; 32],
    value: [u8; 32],
    counter: u64,
    reseeds: u64,
    previous: [[u8; 8]; 4],
    seen: u8,
}
impl Drbg {
    pub const fn new() -> Self {
        Self {
            key: [0; 32],
            value: [0; 32],
            counter: 0,
            reseeds: 0,
            previous: [[0; 8]; 4],
            seen: 0,
        }
    }
    pub fn is_ready(&self) -> bool {
        self.counter != 0
    }
    /// SP800-90A request counter: starts at1 and increments once per successful
    /// Generate call, not per block. This is NOT a count of entropy reseeds.
    pub fn reseed_counter(&self) -> u64 {
        self.counter
    }
    pub fn actual_reseeds(&self) -> u64 {
        self.reseeds
    }
    fn update(&mut self, data: &[u8]) {
        self.key = hmac(&self.key, &[&self.value, &[0], data]);
        self.value = hmac(&self.key, &[&self.value]);
        if !data.is_empty() {
            self.key = hmac(&self.key, &[&self.value, &[1], data]);
            self.value = hmac(&self.key, &[&self.value]);
        }
    }
    fn instantiate_material(&mut self, material: &[u8]) {
        self.key = [0; 32];
        self.value = [1; 32];
        self.update(material);
        self.counter = 1;
    }
    /// Each credited source must provide48..64 bytes: at least384 trusted
    /// entropy bits for256-bit strength with combined nonce (§8.6.7). Length
    /// alone does not prove those bits. Auxiliary sources receive no credit.
    pub fn instantiate(&mut self, sources: Sources<'_>) -> Result<(), Error> {
        if self.is_ready() {
            return Err(Error::AlreadyReady);
        }
        let mut material = [0; MAX_SEED];
        let (len, seen, mut previous) = self.material(sources, &mut material)?;
        self.instantiate_material(&material[..len]);
        self.seen = seen;
        self.previous = previous;
        previous.zeroize();
        material.zeroize();
        Ok(())
    }
    fn material(
        &self,
        sources: Sources<'_>,
        out: &mut [u8; MAX_SEED],
    ) -> Result<(usize, u8, [[u8; 8]; 4]), Error> {
        // Validate everything before modifying this instance or emitting bytes.
        let inputs = [
            sources.firmware,
            sources.rdseed,
            sources.rdrand,
            sources.hci,
        ];
        let mut seen = self.seen;
        let mut previous = self.previous;
        let mut len = DOMAIN.len();
        out[..len].copy_from_slice(DOMAIN);
        let mut strong = false;
        for (index, input) in inputs.into_iter().enumerate() {
            if let Some(bytes) = input {
                let min = if index < 2 { MIN_STRONG_BYTES } else { 8 };
                if bytes.len() < min || bytes.len() > MAX_SOURCE_BYTES || bytes.len() % 8 != 0 {
                    out.zeroize();
                    previous.zeroize();
                    return Err(Error::SourceLength);
                }
                if !healthy(bytes) || (seen & (1 << index) != 0 && previous[index] == bytes[..8]) {
                    out.zeroize();
                    previous.zeroize();
                    return Err(Error::Health);
                }
                if index < 2 {
                    strong = true;
                }
                out[len] = index as u8 + 1;
                out[len + 1..len + 3].copy_from_slice(&(bytes.len() as u16).to_be_bytes());
                len += 3;
                out[len..len + bytes.len()].copy_from_slice(bytes);
                len += bytes.len();
                previous[index].copy_from_slice(&bytes[bytes.len() - 8..]);
                seen |= 1 << index;
            }
        }
        if !strong {
            out.zeroize();
            previous.zeroize();
            return Err(Error::NoStrongSource);
        }
        Ok((len, seen, previous))
    }
    /// Only fresh strong source material can reset the request counter.
    /// Validation failure preserves the established DRBG unchanged.
    pub fn reseed(&mut self, sources: Sources<'_>) -> Result<(), Error> {
        if !self.is_ready() {
            return Err(Error::NotReady);
        }
        let next = self.reseeds.checked_add(1).ok_or(Error::Counter)?;
        let mut material = [0; MAX_SEED];
        let (len, seen, mut previous) = self.material(sources, &mut material)?;
        self.update(&material[..len]);
        self.counter = 1;
        self.reseeds = next;
        self.seen = seen;
        self.previous = previous;
        previous.zeroize();
        material.zeroize();
        Ok(())
    }
    /// Historical API name: mixes HCI auxiliary input only. It neither resets
    /// reseed_counter nor counts as a fresh-entropy reseed or enables a DRBG.
    pub fn reseed_auxiliary(&mut self, bytes: &[u8]) -> Result<(), Error> {
        if !self.is_ready() {
            return Err(Error::NotReady);
        }
        if !healthy(bytes) || (self.seen & 8 != 0 && self.previous[3] == bytes[..8]) {
            return Err(Error::Health);
        }
        let mut material = [0; MAX_SEED];
        let len = DOMAIN.len();
        material[..len].copy_from_slice(DOMAIN);
        material[len] = 4;
        material[len + 1..len + 3].copy_from_slice(&(bytes.len() as u16).to_be_bytes());
        material[len + 3..len + 3 + bytes.len()].copy_from_slice(bytes);
        self.update(&material[..len + 3 + bytes.len()]);
        material.zeroize();
        self.previous[3].copy_from_slice(&bytes[bytes.len() - 8..]);
        self.seen |= 8;
        Ok(())
    }
    pub fn generate(&mut self, out: &mut [u8]) -> Result<(), Error> {
        let error = if !self.is_ready() {
            Some(Error::NotReady)
        } else if out.is_empty() || out.len() > MAX_REQUEST_BYTES {
            Some(Error::RequestLength)
        } else if self.counter > RESEED_INTERVAL {
            Some(Error::ReseedRequired)
        } else {
            None
        };
        if let Some(error) = error {
            out.zeroize();
            return Err(error);
        }
        for chunk in out.chunks_mut(32) {
            self.value = hmac(&self.key, &[&self.value]);
            chunk.copy_from_slice(&self.value[..chunk.len()]);
        }
        self.update(&[]);
        self.counter += 1;
        Ok(())
    }
}
impl Drop for Drbg {
    fn drop(&mut self) {
        self.key.zeroize();
        self.value.zeroize();
        self.previous.zeroize();
        self.counter.zeroize();
        self.reseeds.zeroize();
        self.seen.zeroize();
    }
}

#[cfg(test)]
mod tests {
    extern crate std;
    use super::*;
    fn sample(offset: u8) -> [u8; 64] {
        core::array::from_fn(|i| (i as u8).wrapping_add(offset))
    }
    fn seeded() -> Drbg {
        let bytes = sample(1);
        let mut d = Drbg::new();
        d.instantiate(Sources {
            firmware: Some(&bytes),
            ..Sources::default()
        })
        .unwrap();
        d
    }
    fn hex(text: &str) -> std::vec::Vec<u8> {
        text.as_bytes()
            .chunks_exact(2)
            .map(|b| {
                let nibble = |c: u8| match c {
                    b'0'..=b'9' => c - b'0',
                    b'a'..=b'f' => c - b'a' + 10,
                    _ => panic!("hex"),
                };
                nibble(b[0]) * 16 + nibble(b[1])
            })
            .collect()
    }
    #[test]
    fn nist_cavp_sha256_no_reseed_count_zero_second_generate() {
        let fixture = include_str!("../../tests/fixtures/hmac-drbg-sha256.rsp");
        let field = |name: &str| {
            fixture
                .lines()
                .find_map(|line| line.strip_prefix(name))
                .unwrap()
        };
        let mut material = hex(field("EntropyInput = "));
        material.extend(hex(field("Nonce = ")));
        let expected = hex(field("ReturnedBits = "));
        let mut d = Drbg::new();
        d.instantiate_material(&material);
        let mut output = [0; 128];
        d.generate(&mut output).unwrap();
        d.generate(&mut output).unwrap();
        assert_eq!(output.as_slice(), expected);
        assert_eq!(d.reseed_counter(), 3);
    }
    #[test]
    fn hmac_sha256_rfc4231_case_one() {
        // https://www.rfc-editor.org/rfc/rfc4231.html#section-4.2
        let mut key = [0; 32];
        key[..20].fill(0x0b);
        assert_eq!(
            hmac(&key, &[b"Hi There"]).as_slice(),
            hex("b0344c61d8db38535ca8afceaf0bf12b881dc200c9833da726e9376c2e32cff7")
        );
    }
    #[test]
    fn strong_source_matrix_and_tag_separation() {
        let fw = sample(1);
        let rd = sample(91);
        let aux = sample(31);
        let mut outputs = std::vec::Vec::new();
        for sources in [
            Sources {
                firmware: Some(&fw),
                ..Sources::default()
            },
            Sources {
                rdseed: Some(&fw),
                ..Sources::default()
            },
            Sources {
                firmware: Some(&fw),
                rdseed: Some(&rd),
                ..Sources::default()
            },
        ] {
            let mut d = Drbg::new();
            d.instantiate(sources).unwrap();
            let mut out = [0; 32];
            d.generate(&mut out).unwrap();
            assert!(!outputs.contains(&out));
            outputs.push(out);
        }
        for sources in [
            Sources::default(),
            Sources {
                rdrand: Some(&aux),
                ..Sources::default()
            },
            Sources {
                hci: Some(&aux),
                ..Sources::default()
            },
            Sources {
                rdrand: Some(&aux),
                hci: Some(&rd),
                ..Sources::default()
            },
        ] {
            let mut d = Drbg::new();
            assert_eq!(d.instantiate(sources), Err(Error::NoStrongSource));
            assert!(!d.is_ready());
        }
    }
    #[test]
    fn health_rejects_zero_ff_equal_blocks_and_lengths() {
        assert!(!healthy(&[0; 64]));
        assert!(!healthy(&[255; 64]));
        assert!(!healthy(&[5; 64]));
        assert!(!healthy(&[]));
        assert!(!healthy(&[1; 7]));
        assert!(!healthy(&[1; 72]));
        let mut stuck = sample(1);
        stuck[16..24].fill(0);
        assert!(!healthy(&stuck));
        let mut repeated = sample(1);
        let block: [u8; 8] = repeated[8..16].try_into().unwrap();
        repeated[16..24].copy_from_slice(&block);
        assert!(!healthy(&repeated));
        assert!(healthy(&sample(1)));
        let short = sample(1);
        let mut d = Drbg::new();
        assert_eq!(
            d.instantiate(Sources {
                firmware: Some(&short[..32]),
                ..Sources::default()
            }),
            Err(Error::SourceLength)
        );
    }
    #[test]
    fn requests_count_once_and_invalid_outputs_are_zero() {
        let mut d = seeded();
        assert_eq!(d.reseed_counter(), 1);
        let mut out = [0; 65];
        d.generate(&mut out).unwrap();
        assert_eq!(d.reseed_counter(), 2);
        assert_eq!(d.actual_reseeds(), 0);
        let mut empty = [];
        assert_eq!(d.generate(&mut empty), Err(Error::RequestLength));
        assert_eq!(d.reseed_counter(), 2);
        let mut large = std::vec![9;MAX_REQUEST_BYTES+1];
        assert_eq!(d.generate(&mut large), Err(Error::RequestLength));
        assert!(large.iter().all(|b| *b == 0));
        d.counter = RESEED_INTERVAL;
        d.generate(&mut out).unwrap();
        assert_eq!(d.counter, RESEED_INTERVAL + 1);
        out.fill(9);
        assert_eq!(d.generate(&mut out), Err(Error::ReseedRequired));
        assert_eq!(out, [0; 65]);
    }
    #[test]
    fn only_real_strong_reseed_resets_counter() {
        let mut d = seeded();
        let mut out = [0; 32];
        d.generate(&mut out).unwrap();
        let fresh = sample(91);
        d.reseed_auxiliary(&fresh[..32]).unwrap();
        assert_eq!((d.reseed_counter(), d.actual_reseeds()), (2, 0));
        assert_eq!(
            d.reseed(Sources {
                rdrand: Some(&fresh),
                ..Sources::default()
            }),
            Err(Error::NoStrongSource)
        );
        d.reseed(Sources {
            rdseed: Some(&fresh),
            ..Sources::default()
        })
        .unwrap();
        assert_eq!((d.reseed_counter(), d.actual_reseeds()), (1, 1));
    }
    #[test]
    fn auxiliary_failure_preserves_state_and_never_initializes() {
        let mut unready = Drbg::new();
        assert_eq!(unready.reseed_auxiliary(&sample(1)), Err(Error::NotReady));
        let mut out = [9; 32];
        assert_eq!(unready.generate(&mut out), Err(Error::NotReady));
        assert_eq!(out, [0; 32]);
        let mut a = seeded();
        let mut b = seeded();
        assert_eq!(a.reseed_auxiliary(&[0; 32]), Err(Error::Health));
        let mut x = [0; 32];
        let mut y = [0; 32];
        a.generate(&mut x).unwrap();
        b.generate(&mut y).unwrap();
        assert_eq!(x, y);
    }
    #[test]
    fn health_continuity_across_calls_and_atomic_failure() {
        let mut d = seeded();
        let mut next = sample(91);
        let previous = d.previous[0];
        next[..8].copy_from_slice(&previous);
        assert_eq!(
            d.reseed(Sources {
                firmware: Some(&next),
                ..Sources::default()
            }),
            Err(Error::Health)
        );
        assert_eq!(d.actual_reseeds(), 0);
        let aux = sample(11);
        d.reseed_auxiliary(&aux[..32]).unwrap();
        let mut next_aux = sample(91);
        next_aux[..8].copy_from_slice(&aux[24..32]);
        assert_eq!(d.reseed_auxiliary(&next_aux[..32]), Err(Error::Health));
    }
    #[test]
    fn successful_auxiliary_changes_output_without_counter_credit() {
        let mut a = seeded();
        let mut b = seeded();
        a.reseed_auxiliary(&sample(91)).unwrap();
        let mut x = [0; 32];
        let mut y = [0; 32];
        a.generate(&mut x).unwrap();
        b.generate(&mut y).unwrap();
        assert_ne!(x, y);
        assert_eq!(a.counter, b.counter);
        assert_eq!(a.reseeds, 0);
    }
    #[test]
    fn source_length_tag_distinguishes_prefix_and_reinit_refused() {
        let fw = sample(1);
        let mut a = Drbg::new();
        let mut b = Drbg::new();
        a.instantiate(Sources {
            firmware: Some(&fw[..48]),
            ..Sources::default()
        })
        .unwrap();
        b.instantiate(Sources {
            firmware: Some(&fw),
            ..Sources::default()
        })
        .unwrap();
        let mut x = [0; 32];
        let mut y = [0; 32];
        a.generate(&mut x).unwrap();
        b.generate(&mut y).unwrap();
        assert_ne!(x, y);
        assert_eq!(
            a.instantiate(Sources {
                firmware: Some(&fw),
                ..Sources::default()
            }),
            Err(Error::AlreadyReady)
        );
    }
}
