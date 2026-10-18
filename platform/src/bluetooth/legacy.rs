// Copyright 2026 Hirofumi Iwasaki
// SPDX-License-Identifier: Apache-2.0
//! LE Legacy initiator fixture engine, not a runtime security manager. All arrays
//! are SMP/HCI little-endian wire order. Crypto primitive is RustCrypto AES.
//! Caller must supply a fresh CSPRNG nonce and deliberate pairing authorization;
//! no production entropy/UI exists here. No Debug implementation exposes secrets.
use super::Error;
use crate::pairing::Bond;
use aes::{
    Aes128,
    cipher::{BlockEncrypt, KeyInit, generic_array::GenericArray},
};
use subtle::ConstantTimeEq;
use zeroize::Zeroize;
fn e(key: &[u8; 16], input: &[u8; 16]) -> [u8; 16] {
    let mut k = *key;
    k.reverse();
    let mut b = *input;
    b.reverse();
    let aes = Aes128::new(GenericArray::from_slice(&k));
    aes.encrypt_block(GenericArray::from_mut_slice(&mut b));
    k.zeroize();
    b.reverse();
    b
}
/// Random address hash ah (Core Vol 3 Part H 2.2.2), little-endian wire order:
/// `irk` as distributed in Identity Information, `prand` the upper three octets
/// of an HCI-order resolvable private address. Returns its lower three octets.
pub fn ah(irk: &[u8; 16], prand: &[u8; 3]) -> [u8; 3] {
    let mut input = [0; 16];
    input[..3].copy_from_slice(prand);
    let out = e(irk, &input);
    [out[0], out[1], out[2]]
}
/// True for a resolvable private address (HCI order) generated from `irk`.
pub fn resolves(irk: &[u8; 16], address: &[u8; 6]) -> bool {
    *irk != [0; 16]
        && address[5] & 0xc0 == 0x40
        && bool::from(ah(irk, address[3..6].try_into().unwrap())[..].ct_eq(&address[..3]))
}
pub fn c1(
    tk: &[u8; 16],
    r: &[u8; 16],
    request: &[u8; 7],
    response: &[u8; 7],
    iat: u8,
    rat: u8,
    ia: &[u8; 6],
    ra: &[u8; 6],
) -> [u8; 16] {
    let mut p1 = [0; 16];
    p1[0] = iat;
    p1[1] = rat;
    p1[2..9].copy_from_slice(request);
    p1[9..].copy_from_slice(response);
    for i in 0..16 {
        p1[i] ^= r[i];
    }
    let mut x = e(tk, &p1);
    p1.zeroize();
    for i in 0..6 {
        x[i] ^= ra[i];
        x[i + 6] ^= ia[i];
    }
    let out = e(tk, &x);
    x.zeroize();
    out
}
pub fn s1(tk: &[u8; 16], responder: &[u8; 16], initiator: &[u8; 16]) -> [u8; 16] {
    let mut input = [0; 16];
    input[..8].copy_from_slice(&initiator[..8]);
    input[8..].copy_from_slice(&responder[..8]);
    let out = e(tk, &input);
    input.zeroize();
    out
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Association {
    JustWorks,
    DisplayPasskey(u32),
}
pub enum Action {
    None,
    Send([u8; 17], usize),
    Encrypt([u8; 16]),
    Bond(VerifiedBond),
}
/// Only this verified protocol state creates a bond for persistence. A fixture
/// can drive it, but production may trust it only after correlated HCI encryption.
pub struct VerifiedBond {
    pub(crate) bond: Bond,
    pub(crate) generation: u64,
    pub(crate) handle: u16,
}
impl VerifiedBond {
    pub fn bond(&self) -> &Bond {
        &self.bond
    }
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum State {
    Response,
    Confirm,
    Random,
    Encryption,
    Key,
    Identity,
    Complete,
    Failed,
}
pub struct Legacy {
    generation: u64,
    handle: u16,
    state: State,
    request: [u8; 7],
    response: [u8; 7],
    tk: [u8; 16],
    nonce: [u8; 16],
    peer_confirm: [u8; 16],
    ltk: [u8; 16],
    iat: u8,
    rat: u8,
    ia: [u8; 6],
    ra: [u8; 6],
    method: Association,
    deadline: u64,
    last: u64,
}
impl Drop for Legacy {
    fn drop(&mut self) {
        if let Association::DisplayPasskey(n) = &mut self.method {
            n.zeroize();
        }
        self.tk.zeroize();
        self.nonce.zeroize();
        self.peer_confirm.zeroize();
        self.ltk.zeroize();
    }
}
impl Legacy {
    pub fn new(
        generation: u64,
        handle: u16,
        now: u64,
        method: Association,
        nonce: [u8; 16],
        iat: u8,
        ia: [u8; 6],
        rat: u8,
        ra: [u8; 6],
    ) -> Result<Self, Error> {
        if handle > 0x0eff || iat > 1 || rat > 1 || nonce == [0; 16] {
            return Err(Error::Invalid);
        }
        let mut tk = [0; 16];
        if let Association::DisplayPasskey(n) = method {
            if n > 999999 {
                return Err(Error::Invalid);
            }
            tk[..4].copy_from_slice(&n.to_le_bytes());
        }
        let request = [
            1,
            if method == Association::JustWorks {
                3
            } else {
                0
            },
            0,
            if method == Association::JustWorks {
                1
            } else {
                5
            },
            16,
            0,
            1,
        ];
        Ok(Self {
            generation,
            handle,
            state: State::Response,
            request,
            response: [0; 7],
            tk,
            nonce,
            peer_confirm: [0; 16],
            ltk: [0; 16],
            iat,
            rat,
            ia,
            ra,
            method,
            deadline: now.checked_add(30000).ok_or(Error::Invalid)?,
            last: now,
        })
    }
    pub fn request(&self) -> [u8; 7] {
        self.request
    }
    fn check(&mut self, generation: u64, now: u64) -> Result<(), Error> {
        if generation != self.generation {
            return Err(Error::Stale);
        }
        if self.state == State::Failed || self.state == State::Complete {
            return Err(Error::Invalid);
        }
        if now < self.last || now >= self.deadline {
            self.state = State::Failed;
            return Err(Error::Timeout);
        }
        self.last = now;
        Ok(())
    }
    pub fn receive(&mut self, generation: u64, now: u64, p: &[u8]) -> Result<Action, Error> {
        self.check(generation, now)?;
        let result = self.receive_checked(p);
        if result.is_err() {
            self.state = State::Failed;
            self.tk.zeroize();
            self.nonce.zeroize();
            self.ltk.zeroize();
        }
        result
    }
    fn receive_checked(&mut self, p: &[u8]) -> Result<Action, Error> {
        if p.first() == Some(&5) {
            return Err(Error::Controller(*p.get(1).ok_or(Error::Length)?));
        }
        match self.state {
            State::Response => {
                if p.len() != 7 || p[0] != 2 {
                    return Err(Error::Length);
                }
                // LTK-only legacy subset, no OOB/IRK/signing/CTKD. Do not imply SC.
                if p[1] > 4
                    || p[2] != 0
                    || p[3] & !0x0d != 0
                    || p[3] & 3 != 1
                    || p[4] != 16
                    || p[5] != 0
                    || p[6] != 1
                {
                    return Err(Error::Unsupported);
                }
                match self.method {
                    Association::JustWorks if p[3] & 4 != 0 => return Err(Error::Unsupported),
                    Association::DisplayPasskey(_) if !matches!(p[1], 2 | 4) || p[3] & 4 == 0 => {
                        return Err(Error::Unsupported);
                    }
                    _ => {}
                }
                self.response.copy_from_slice(p);
                let value = c1(
                    &self.tk,
                    &self.nonce,
                    &self.request,
                    &self.response,
                    self.iat,
                    self.rat,
                    &self.ia,
                    &self.ra,
                );
                let mut out = [0; 17];
                out[0] = 3;
                out[1..].copy_from_slice(&value);
                self.state = State::Confirm;
                Ok(Action::Send(out, 17))
            }
            State::Confirm => {
                if p.len() != 17 || p[0] != 3 {
                    return Err(Error::Length);
                }
                self.peer_confirm.copy_from_slice(&p[1..]);
                let mut out = [0; 17];
                out[0] = 4;
                out[1..].copy_from_slice(&self.nonce);
                self.state = State::Random;
                Ok(Action::Send(out, 17))
            }
            State::Random => {
                if p.len() != 17 || p[0] != 4 {
                    return Err(Error::Length);
                }
                let peer: [u8; 16] = p[1..].try_into().unwrap();
                let mut check = c1(
                    &self.tk,
                    &peer,
                    &self.request,
                    &self.response,
                    self.iat,
                    self.rat,
                    &self.ia,
                    &self.ra,
                );
                let good = bool::from(check.ct_eq(&self.peer_confirm));
                check.zeroize();
                if !good {
                    return Err(Error::Invalid);
                }
                let stk = s1(&self.tk, &peer, &self.nonce);
                self.tk.zeroize();
                self.nonce.zeroize();
                self.state = State::Encryption;
                Ok(Action::Encrypt(stk))
            }
            State::Key => {
                if p.len() != 17 || p[0] != 6 {
                    return Err(Error::Length);
                }
                self.ltk.copy_from_slice(&p[1..]);
                if self.ltk == [0; 16] {
                    return Err(Error::Invalid);
                }
                self.state = State::Identity;
                Ok(Action::None) // No SMP response required.
            }
            State::Identity => {
                if p.len() != 11 || p[0] != 7 {
                    return Err(Error::Length);
                }
                let bond = Bond {
                    address_type: self.rat,
                    address: self.ra,
                    key_size: 16,
                    flags: if self.method == Association::JustWorks {
                        0
                    } else {
                        1
                    },
                    ediv: u16::from_le_bytes([p[1], p[2]]),
                    random: p[3..].try_into().unwrap(),
                    ltk: self.ltk,
                    irk: [0; 16],
                    own_address_type: 0,
                    own_address: [0; 6],
                };
                self.ltk.zeroize();
                self.state = State::Complete;
                Ok(Action::Bond(VerifiedBond {
                    bond,
                    generation: self.generation,
                    handle: self.handle,
                }))
            }
            _ => Err(Error::Invalid),
        }
    }
    /// Parse a correlated HCI Encryption Change, not a caller-provided success bit.
    pub fn encryption(
        &mut self,
        generation: u64,
        now: u64,
        event: super::Event<'_>,
    ) -> Result<(), Error> {
        self.check(generation, now)?;
        let p = event.parameters;
        if event.code != 8 || p.len() != 4 || super::word(&p[1..3]) != self.handle {
            return Err(Error::Stale);
        }
        if self.state != State::Encryption || p[0] != 0 || p[3] != 1 {
            self.state = State::Failed;
            return Err(Error::Invalid);
        }
        self.state = State::Key;
        Ok(())
    }
}
