// Copyright 2026 Hirofumi Iwasaki
// SPDX-License-Identifier: Apache-2.0
//! Bluetooth SC cryptographic compositions, not custom primitives. Numeric
//! inputs/outputs use canonical big-endian; public_wire uses SMP little-endian.
use super::{Error, entropy::Random};
use aes::Aes128;
use cmac::{Cmac, Mac};
use p256::{PublicKey, SecretKey, elliptic_curve::sec1::ToEncodedPoint};
use zeroize::Zeroize;
pub fn cmac(key: &[u8; 16], parts: &[&[u8]]) -> [u8; 16] {
    let mut m = <Cmac<Aes128> as Mac>::new_from_slice(key).unwrap();
    for part in parts {
        m.update(part);
    }
    m.finalize().into_bytes().into()
}
pub fn f4(u: &[u8; 32], v: &[u8; 32], x: &[u8; 16], z: u8) -> [u8; 16] {
    cmac(x, &[u, v, &[z]])
}
pub fn f5(
    w: &[u8; 32],
    n1: &[u8; 16],
    n2: &[u8; 16],
    a1: &[u8; 7],
    a2: &[u8; 7],
) -> ([u8; 16], [u8; 16]) {
    const SALT: [u8; 16] = [
        0x6c, 0x88, 0x83, 0x91, 0xaa, 0xf5, 0xa5, 0x38, 0x60, 0x37, 0x0b, 0xdb, 0x5a, 0x60, 0x83,
        0xbe,
    ];
    let mut t = cmac(&SALT, &[w]);
    let mac = cmac(&t, &[&[0], b"btle", n1, n2, a1, a2, &[1, 0]]);
    let ltk = cmac(&t, &[&[1], b"btle", n1, n2, a1, a2, &[1, 0]]);
    t.zeroize();
    (mac, ltk)
}
pub fn f6(
    w: &[u8; 16],
    n1: &[u8; 16],
    n2: &[u8; 16],
    r: &[u8; 16],
    io: &[u8; 3],
    a1: &[u8; 7],
    a2: &[u8; 7],
) -> [u8; 16] {
    cmac(w, &[n1, n2, r, io, a1, a2])
}
pub fn g2(u: &[u8; 32], v: &[u8; 32], x: &[u8; 16], y: &[u8; 16]) -> u32 {
    let mut value = cmac(x, &[u, v, y]);
    let n = u32::from_be_bytes(value[12..].try_into().unwrap());
    value.zeroize();
    n
}
/// SecretKey implements zeroize-on-drop. Not Clone/Debug; no scalar export.
pub struct Key {
    secret: SecretKey,
}
impl Key {
    pub fn generate(rng: &mut impl Random) -> Result<Self, Error> {
        for _ in 0..16 {
            let mut b = [0; 32];
            if let Err(error) = rng.fill(&mut b) {
                b.zeroize();
                return Err(error);
            }
            let key = SecretKey::from_slice(&b);
            b.zeroize();
            if let Ok(secret) = key {
                return Ok(Self { secret });
            }
        }
        Err(Error::Entropy)
    }
    pub fn public_wire(&self) -> [u8; 64] {
        let point = self.secret.public_key().to_encoded_point(false);
        let mut wire = [0; 64];
        wire[..32].copy_from_slice(point.x().unwrap());
        wire[32..].copy_from_slice(point.y().unwrap());
        wire[..32].reverse();
        wire[32..].reverse();
        wire
    }
    pub fn dh(&self, peer_wire: &[u8; 64]) -> Result<[u8; 32], Error> {
        if peer_wire == &self.public_wire() {
            return Err(Error::Invalid);
        }
        let mut point = [0; 65];
        point[0] = 4;
        point[1..33].copy_from_slice(&peer_wire[..32]);
        point[33..].copy_from_slice(&peer_wire[32..]);
        point[1..33].reverse();
        point[33..].reverse();
        // The SIG debug private key is public knowledge; never accept its point.
        const DEBUG_X: [u8; 32] = [
            0x20, 0xb0, 0x03, 0xd2, 0xf2, 0x97, 0xbe, 0x2c, 0x5e, 0x2c, 0x83, 0xa7, 0xe9, 0xf9,
            0xa5, 0xb9, 0xef, 0xf4, 0x91, 0x11, 0xac, 0xf4, 0xfd, 0xdb, 0xcc, 0x03, 0x01, 0x48,
            0x0e, 0x35, 0x9d, 0xe6,
        ];
        if point[1..33] == DEBUG_X {
            return Err(Error::Invalid);
        }
        let peer = PublicKey::from_sec1_bytes(&point).map_err(|_| Error::Invalid)?;
        let mut scalar = self.secret.to_nonzero_scalar();
        let shared = p256::ecdh::diffie_hellman(&scalar, peer.as_affine());
        scalar.zeroize();
        Ok((*shared.raw_secret_bytes()).into())
    }
}
