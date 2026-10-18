// Copyright 2026 Hirofumi Iwasaki
// SPDX-License-Identifier: Apache-2.0
//! Bounded SC-only central SMP. No OOB, Numeric Comparison, CTKD or CSRK. The
//! responder's identity (IRK and identity address) is requested for reconnects
//! from resolvable private addresses; this central distributes no keys.
//! Caller selects a deliberate association model, supplies one-use entropy and
//! correlates the controller's Encryption Change before obtaining VerifiedBond.
use super::{
    Error,
    entropy::Random,
    legacy::VerifiedBond,
    sc_crypto::{self, Key},
};
use crate::pairing::Bond;
use subtle::ConstantTimeEq;
use zeroize::Zeroize;
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Method {
    JustWorks,
    DisplayPasskey,
    EnterPasskey,
}
pub enum Action {
    Send([u8; 65], usize),
    Display(u32),
    Enter,
    Encrypt([u8; 16]),
    Bond(VerifiedBond),
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum State {
    Response,
    Public,
    User,
    Confirm,
    Random,
    DhCheck,
    Encryption,
    Keys,
    Complete,
    Failed,
}
/// Responder key distribution requested: IdKey only (EncKey is unused in SC).
const ID_KEY: u8 = 0x02;
pub struct Central {
    key: Option<Key>,
    state: State,
    method: Method,
    passkey: u32,
    round: u8,
    generation: u64,
    handle: u16,
    deadline: u64,
    last: u64,
    request: [u8; 7],
    response: [u8; 7],
    peer: [u8; 64],
    dh: [u8; 32],
    local_nonce: [u8; 16],
    peer_nonce: [u8; 16],
    confirm: [u8; 16],
    mac: [u8; 16],
    ltk: [u8; 16],
    a1: [u8; 7],
    a2: [u8; 7],
    irk: Option<[u8; 16]>,
    identity: Option<(u8, [u8; 6])>,
}
impl Drop for Central {
    fn drop(&mut self) {
        self.wipe();
    }
}
impl Central {
    pub fn new(
        generation: u64,
        handle: u16,
        now: u64,
        method: Method,
        iat: u8,
        ia: [u8; 6],
        rat: u8,
        ra: [u8; 6],
        rng: &mut impl Random,
    ) -> Result<Self, Error> {
        if handle > 0xeff || iat > 1 || rat > 1 || ia == [0; 6] || ra == [0; 6] {
            return Err(Error::Invalid);
        }
        let key = Key::generate(rng)?;
        let mut a1 = [0; 7];
        a1[0] = iat;
        a1[1..].copy_from_slice(&ia);
        a1[1..].reverse();
        let mut a2 = [0; 7];
        a2[0] = rat;
        a2[1..].copy_from_slice(&ra);
        a2[1..].reverse();
        let io = match method {
            Method::JustWorks => 3,
            Method::DisplayPasskey => 0,
            Method::EnterPasskey => 2,
        };
        Ok(Self {
            key: Some(key),
            state: State::Response,
            method,
            passkey: 0,
            round: 0,
            generation,
            handle,
            deadline: now.checked_add(30000).ok_or(Error::Invalid)?,
            last: now,
            request: [
                1,
                io,
                0,
                if method == Method::JustWorks { 9 } else { 13 },
                16,
                0,
                ID_KEY,
            ],
            response: [0; 7],
            peer: [0; 64],
            dh: [0; 32],
            local_nonce: [0; 16],
            peer_nonce: [0; 16],
            confirm: [0; 16],
            mac: [0; 16],
            ltk: [0; 16],
            a1,
            a2,
            irk: None,
            identity: None,
        })
    }
    fn wipe(&mut self) {
        self.key.take();
        self.passkey.zeroize();
        self.dh.zeroize();
        self.local_nonce.zeroize();
        self.peer_nonce.zeroize();
        self.confirm.zeroize();
        self.mac.zeroize();
        self.ltk.zeroize();
        if let Some(irk) = self.irk.as_mut() {
            irk.zeroize();
        }
    }
    pub fn request(&self) -> [u8; 7] {
        self.request
    }
    pub fn generation(&self) -> u64 {
        self.generation
    }
    pub fn check(&mut self, generation: u64, now: u64) -> Result<(), Error> {
        if generation != self.generation {
            return Err(Error::Stale);
        }
        if matches!(self.state, State::Failed | State::Complete) {
            return Err(Error::Invalid);
        }
        if now < self.last || now >= self.deadline {
            self.fail();
            return Err(Error::Timeout);
        }
        self.last = now;
        Ok(())
    }
    pub fn cancel(&mut self) {
        self.fail();
    }
    fn fail(&mut self) {
        self.state = State::Failed;
        self.wipe();
    }
    fn send(code: u8, value: &[u8]) -> Action {
        let mut p = [0; 65];
        p[0] = code;
        p[1..1 + value.len()].copy_from_slice(value);
        Action::Send(p, 1 + value.len())
    }
    fn x(peer: &[u8; 64]) -> [u8; 32] {
        let mut x: [u8; 32] = peer[..32].try_into().unwrap();
        x.reverse();
        x
    }
    fn z(&self) -> u8 {
        if self.method == Method::JustWorks {
            0
        } else {
            0x80 | ((self.passkey >> self.round) & 1) as u8
        }
    }
    fn begin_round(&mut self, rng: &mut impl Random) -> Result<Action, Error> {
        rng.fill(&mut self.local_nonce)?;
        if self.method == Method::JustWorks {
            self.state = State::Confirm;
            return Ok(Self::send(4, &reversed(self.local_nonce)));
        }
        self.state = State::Confirm;
        let c = sc_crypto::f4(
            &Self::x(&self.key.as_ref().ok_or(Error::Invalid)?.public_wire()),
            &Self::x(&self.peer),
            &self.local_nonce,
            self.z(),
        );
        Ok(Self::send(3, &reversed(c)))
    }
    /// Called only after EnterPasskey request; caller must transfer the secret once.
    pub fn entered(
        &mut self,
        generation: u64,
        now: u64,
        value: u32,
        rng: &mut impl Random,
    ) -> Result<Action, Error> {
        self.check(generation, now)?;
        if self.state != State::User || self.method != Method::EnterPasskey || value > 999999 {
            self.fail();
            return Err(Error::Invalid);
        }
        self.passkey = value;
        let result = self.begin_round(rng);
        if result.is_err() {
            self.fail();
        }
        result
    }
    /// After a Display action, let the caller render the value before continuing.
    pub fn displayed(
        &mut self,
        generation: u64,
        now: u64,
        rng: &mut impl Random,
    ) -> Result<Action, Error> {
        self.check(generation, now)?;
        if self.state != State::User || self.method != Method::DisplayPasskey {
            self.fail();
            return Err(Error::Invalid);
        }
        let result = self.begin_round(rng);
        if result.is_err() {
            self.fail();
        }
        result
    }
    pub fn receive(
        &mut self,
        generation: u64,
        now: u64,
        p: &[u8],
        rng: &mut impl Random,
    ) -> Result<Action, Error> {
        self.check(generation, now)?;
        let result = self.receive_inner(p, rng);
        if result.is_err() {
            self.fail();
        }
        result
    }
    fn receive_inner(&mut self, p: &[u8], rng: &mut impl Random) -> Result<Action, Error> {
        if p.first() == Some(&5) {
            return Err(Error::Controller(*p.get(1).ok_or(Error::Length)?));
        }
        match self.state {
            State::Response => {
                if p.len() != 7 || p[0] != 2 {
                    return Err(Error::Length);
                }
                // Reject legacy/downgrade, OOB, short keys, and any distribution
                // beyond what was requested (none from us, IdKey from the peer).
                if p[1] > 4
                    || p[2] != 0
                    || p[3] & 0x08 == 0
                    || p[3] & !0x0d != 0
                    || p[3] & 3 > 1
                    || p[4] != 16
                    || p[5] != 0
                    || p[6] & !ID_KEY != 0
                {
                    return Err(Error::Unsupported);
                }
                match self.method {
                    Method::JustWorks if p[3] & 4 != 0 => return Err(Error::Unsupported),
                    Method::DisplayPasskey if !matches!(p[1], 2 | 4) => {
                        return Err(Error::Unsupported);
                    }
                    Method::EnterPasskey if !matches!(p[1], 0 | 1 | 2 | 4) => {
                        return Err(Error::Unsupported);
                    }
                    _ => {}
                }
                self.response.copy_from_slice(p);
                self.state = State::Public;
                Ok(Self::send(
                    12,
                    &self.key.as_ref().ok_or(Error::Invalid)?.public_wire(),
                ))
            }
            State::Public => {
                if p.len() != 65 || p[0] != 12 {
                    return Err(Error::Length);
                }
                self.peer.copy_from_slice(&p[1..]);
                self.dh = self.key.as_ref().ok_or(Error::Invalid)?.dh(&self.peer)?;
                match self.method {
                    Method::JustWorks => {
                        rng.fill(&mut self.local_nonce)?;
                        self.state = State::Confirm; // responder sends confirm first
                        Ok(Action::Send([0; 65], 0))
                    }
                    Method::DisplayPasskey => {
                        self.passkey = super::entropy::passkey(rng)?;
                        self.state = State::User;
                        Ok(Action::Display(self.passkey))
                    }
                    Method::EnterPasskey => {
                        self.state = State::User;
                        Ok(Action::Enter)
                    }
                }
            }
            State::Confirm => {
                if p.len() != 17 || p[0] != 3 {
                    return Err(Error::Length);
                }
                self.confirm.copy_from_slice(&p[1..]);
                self.confirm.reverse();
                self.state = State::Random;
                Ok(Self::send(4, &reversed(self.local_nonce)))
            }
            State::Random => {
                if p.len() != 17 || p[0] != 4 {
                    return Err(Error::Length);
                }
                self.peer_nonce.copy_from_slice(&p[1..]);
                self.peer_nonce.reverse();
                let expected = sc_crypto::f4(
                    &Self::x(&self.peer),
                    &Self::x(&self.key.as_ref().ok_or(Error::Invalid)?.public_wire()),
                    &self.peer_nonce,
                    self.z(),
                );
                if !bool::from(expected.ct_eq(&self.confirm)) {
                    return Err(Error::Invalid);
                }
                if self.method != Method::JustWorks && self.round < 19 {
                    self.round += 1;
                    return self.begin_round(rng);
                }
                (self.mac, self.ltk) = sc_crypto::f5(
                    &self.dh,
                    &self.local_nonce,
                    &self.peer_nonce,
                    &self.a1,
                    &self.a2,
                );
                let mut r = [0; 16];
                if self.method != Method::JustWorks {
                    r[12..].copy_from_slice(&self.passkey.to_be_bytes());
                }
                let io = [self.request[3], self.request[2], self.request[1]];
                let check = sc_crypto::f6(
                    &self.mac,
                    &self.local_nonce,
                    &self.peer_nonce,
                    &r,
                    &io,
                    &self.a1,
                    &self.a2,
                );
                r.zeroize();
                self.state = State::DhCheck;
                Ok(Self::send(13, &reversed(check)))
            }
            State::DhCheck => {
                if p.len() != 17 || p[0] != 13 {
                    return Err(Error::Length);
                }
                let mut r = [0; 16];
                if self.method != Method::JustWorks {
                    r[12..].copy_from_slice(&self.passkey.to_be_bytes());
                }
                let io = [self.response[3], self.response[2], self.response[1]];
                let mut expected = sc_crypto::f6(
                    &self.mac,
                    &self.peer_nonce,
                    &self.local_nonce,
                    &r,
                    &io,
                    &self.a2,
                    &self.a1,
                );
                expected.reverse();
                r.zeroize();
                let valid = bool::from(expected.ct_eq(&p[1..]));
                expected.zeroize();
                if !valid {
                    return Err(Error::Invalid);
                }
                self.state = State::Encryption;
                let mut key = self.ltk;
                key.reverse();
                Ok(Action::Encrypt(key))
            }
            // The peer distributes keys once the link is encrypted; they may be
            // read before the controller's Encryption Change event.
            State::Encryption | State::Keys if self.response[6] & ID_KEY != 0 => {
                match (p.first(), p.len()) {
                    (Some(8), 17) if self.irk.is_none() => {
                        self.irk = Some(p[1..].try_into().unwrap());
                    }
                    (Some(9), 8) if self.irk.is_some() && self.identity.is_none() => {
                        self.identity = Some((p[1], p[2..8].try_into().unwrap()));
                    }
                    _ => return Err(Error::Invalid),
                }
                if self.state == State::Keys && self.identity.is_some() {
                    return Ok(self.bond());
                }
                Ok(Action::Send([0; 65], 0))
            }
            _ => Err(Error::Invalid),
        }
    }
    /// Identity address: a public or static random identity from Identity
    /// Address Information, otherwise the address used for pairing.
    fn bond(&mut self) -> Action {
        let mut address: [u8; 6] = self.a2[1..].try_into().unwrap();
        address.reverse();
        let mut address_type = self.a2[0];
        if let Some((kind, identity)) = self.identity {
            if identity != [0; 6] && (kind == 0 || (kind == 1 && identity[5] & 0xc0 == 0xc0)) {
                address_type = kind;
                address = identity;
            }
        }
        let mut ltk = self.ltk;
        ltk.reverse();
        let bond = Bond {
            address_type,
            address,
            key_size: 16,
            flags: if self.method == Method::JustWorks {
                2
            } else {
                3
            },
            ediv: 0,
            random: [0; 8],
            ltk,
            irk: self.irk.unwrap_or([0; 16]),
        };
        self.state = State::Complete;
        self.wipe();
        Action::Bond(VerifiedBond {
            bond,
            generation: self.generation,
            handle: self.handle,
        })
    }
    /// No public boolean can authorize notifications: only a matching raw HCI
    /// Encryption Change with status=0 and enabled=1 produces VerifiedBond.
    pub fn encryption(
        &mut self,
        generation: u64,
        now: u64,
        event: super::Event<'_>,
    ) -> Result<Action, Error> {
        self.check(generation, now)?;
        let p = event.parameters;
        if self.state != State::Encryption
            || event.code != 8
            || p.len() != 4
            || super::word(&p[1..3]) != self.handle
            || p[0] != 0
            || p[3] != 1
        {
            self.fail();
            return Err(Error::Invalid);
        }
        // Requested identity keys still to come: wait in Keys (SMP timeout).
        if self.response[6] & ID_KEY != 0 && self.identity.is_none() {
            self.state = State::Keys;
            return Ok(Action::Send([0; 65], 0));
        }
        Ok(self.bond())
    }
    /// Whether the peer agreed to distribute its identity keys.
    pub fn identity_requested(&self) -> bool {
        self.response[6] & ID_KEY != 0
    }
}
fn reversed(mut value: [u8; 16]) -> [u8; 16] {
    value.reverse();
    value
}
