// Copyright 2026 Hirofumi Iwasaki
// SPDX-License-Identifier: Apache-2.0
#![cfg(feature = "bluetooth-security")]
//! Responder identity distribution (IRK + identity address) and host-side
//! resolvable private address resolution for reconnects. Synthetic keys.
use musha_platform::{
    bluetooth::{
        Error, Event,
        entropy::Random,
        legacy::{ah, resolves},
        reconnect::Store,
        sc::{Action, Central, Method},
        sc_crypto::{Key, f4, f5, f6},
    },
    pairing::{self, Bond},
};
struct Fixed([u8; 32]);
impl Random for Fixed {
    fn fill(&mut self, b: &mut [u8]) -> Result<(), Error> {
        b.copy_from_slice(&self.0[..b.len()]);
        Ok(())
    }
}
fn hex16(s: &str) -> [u8; 16] {
    core::array::from_fn(|i| u8::from_str_radix(&s[i * 2..i * 2 + 2], 16).unwrap())
}
/// Core Vol 3 Part H appendix ah vector, cross-checked with OpenSSL AES-128:
/// IRK ec0234a3..397d9b, prand 708194 -> hash 0dfbaa. Wire order is reversed.
fn irk() -> [u8; 16] {
    let mut k = hex16("ec0234a357c8ad05341010a60a397d9b");
    k.reverse();
    k
}
const RPA: [u8; 6] = [0xaa, 0xfb, 0x0d, 0x94, 0x81, 0x70];
fn sent(action: Action) -> Vec<u8> {
    match action {
        Action::Send(p, n) => p[..n].to_vec(),
        _ => panic!("expected packet"),
    }
}
fn wire16(code: u8, mut b: [u8; 16]) -> Vec<u8> {
    b.reverse();
    let mut p = vec![code];
    p.extend_from_slice(&b);
    p
}
fn x(w: &[u8; 64]) -> [u8; 32] {
    let mut b: [u8; 32] = w[..32].try_into().unwrap();
    b.reverse();
    b
}
const RA: [u8; 6] = [7, 8, 9, 10, 11, 12];
/// Just Works up to the controller's LTK request; returns the central.
fn paired_until_encrypt(distribution: u8) -> Central {
    let mut rng = Fixed([1; 32]);
    let mut c = Central::new(
        3,
        5,
        0,
        Method::JustWorks,
        0,
        [1, 2, 3, 4, 5, 6],
        1,
        RA,
        &mut rng,
    )
    .unwrap();
    let resp = [2, 0, 0, 9, 16, 0, distribution];
    let public = sent(c.receive(3, 1, &resp, &mut rng).unwrap());
    let local: [u8; 64] = public[1..].try_into().unwrap();
    let peer = Key::generate(&mut Fixed([2; 32])).unwrap();
    let wire = peer.public_wire();
    let mut packet = vec![12];
    packet.extend_from_slice(&wire);
    assert!(sent(c.receive(3, 2, &packet, &mut rng).unwrap()).is_empty());
    let (na, nb) = ([1; 16], [4; 16]);
    let confirm = f4(&x(&wire), &x(&local), &nb, 0);
    assert_eq!(
        sent(c.receive(3, 3, &wire16(3, confirm), &mut rng).unwrap()),
        wire16(4, na)
    );
    assert_eq!(
        sent(c.receive(3, 4, &wire16(4, nb), &mut rng).unwrap())[0],
        13
    );
    let a1 = [0, 6, 5, 4, 3, 2, 1];
    let a2 = [1, 12, 11, 10, 9, 8, 7];
    let (mac, _) = f5(&peer.dh(&local).unwrap(), &na, &nb, &a1, &a2);
    let eb = f6(&mac, &nb, &na, &[0; 16], &[9, 0, 0], &a2, &a1);
    assert!(matches!(
        c.receive(3, 5, &wire16(13, eb), &mut rng).unwrap(),
        Action::Encrypt(_)
    ));
    c
}
fn encrypted(c: &mut Central, now: u64) -> Result<Action, Error> {
    c.encryption(3, now, Event::parse(&[8, 4, 0, 5, 0, 1]).unwrap())
}
fn identity_info() -> Vec<u8> {
    [&[8][..], &irk()].concat()
}
fn identity_address(kind: u8, address: [u8; 6]) -> Vec<u8> {
    [&[9, kind][..], &address].concat()
}
const PUBLIC: [u8; 6] = [0x31, 0x32, 0x33, 0x34, 0x35, 0x36];

#[test]
fn ah_matches_the_core_vector_and_resolution_is_strict() {
    let mut prand = [0x94, 0x81, 0x70];
    assert_eq!(ah(&irk(), &prand), [0xaa, 0xfb, 0x0d]);
    assert!(resolves(&irk(), &RPA));
    let mut wrong = RPA;
    wrong[0] ^= 1;
    assert!(!resolves(&irk(), &wrong));
    // Static (11) and non-resolvable (00) random addresses never resolve.
    let mut kind = RPA;
    kind[5] |= 0xc0;
    assert!(!resolves(&irk(), &kind));
    kind[5] &= 0x3f;
    assert!(!resolves(&irk(), &kind));
    assert!(!resolves(&[0; 16], &RPA));
    prand[0] ^= 1;
    assert_ne!(ah(&irk(), &prand), [0xaa, 0xfb, 0x0d]);
}

#[test]
fn pairing_requests_only_the_peer_identity_key() {
    let mut rng = Fixed([1; 32]);
    let c = Central::new(3, 5, 0, Method::JustWorks, 0, [1; 6], 0, RA, &mut rng).unwrap();
    let req = c.request();
    assert_eq!((req[5], req[6]), (0, 0x02));
    for (initiator, responder) in [(1, 2), (0, 3), (0, 4), (2, 0)] {
        let mut c = Central::new(3, 5, 0, Method::JustWorks, 0, [1; 6], 0, RA, &mut rng).unwrap();
        let resp = [2, 0, 0, 9, 16, initiator, responder];
        assert!(
            c.receive(3, 1, &resp, &mut rng).is_err(),
            "{initiator} {responder}"
        );
    }
}

#[test]
fn identity_after_encryption_completes_the_bond_with_irk_and_identity() {
    let mut c = paired_until_encrypt(2);
    assert!(c.identity_requested());
    // Encryption first: the bond waits for the identity keys.
    assert!(sent(encrypted(&mut c, 6).unwrap()).is_empty());
    let mut rng = Fixed([1; 32]);
    assert!(sent(c.receive(3, 7, &identity_info(), &mut rng).unwrap()).is_empty());
    let Action::Bond(bond) = c
        .receive(3, 8, &identity_address(0, PUBLIC), &mut rng)
        .unwrap()
    else {
        panic!("no bond")
    };
    let b = bond.bond();
    assert_eq!((b.address_type, b.address, b.irk), (0, PUBLIC, irk()));
}

#[test]
fn identity_read_before_the_encryption_event_is_kept() {
    let mut c = paired_until_encrypt(2);
    let mut rng = Fixed([1; 32]);
    assert!(sent(c.receive(3, 6, &identity_info(), &mut rng).unwrap()).is_empty());
    assert!(
        sent(
            c.receive(3, 7, &identity_address(0, PUBLIC), &mut rng)
                .unwrap()
        )
        .is_empty()
    );
    let Action::Bond(bond) = encrypted(&mut c, 8).unwrap() else {
        panic!("no bond")
    };
    assert_eq!(bond.bond().address, PUBLIC);
}

#[test]
fn invalid_identity_falls_back_to_the_pairing_address_and_order_is_enforced() {
    let mut c = paired_until_encrypt(2);
    let mut rng = Fixed([1; 32]);
    assert!(sent(encrypted(&mut c, 6).unwrap()).is_empty());
    c.receive(3, 7, &identity_info(), &mut rng).unwrap();
    // A random identity must be static (top bits 11); keep the pairing address.
    let Action::Bond(bond) = c
        .receive(3, 8, &identity_address(1, RPA), &mut rng)
        .unwrap()
    else {
        panic!("no bond")
    };
    assert_eq!((bond.bond().address_type, bond.bond().address), (1, RA));
    assert_eq!(bond.bond().irk, irk());
    // Identity Address before Identity Information is refused.
    let mut c = paired_until_encrypt(2);
    assert!(
        c.receive(3, 6, &identity_address(0, PUBLIC), &mut rng)
            .is_err()
    );
    // Without agreed distribution, encryption completes the bond at once.
    let mut c = paired_until_encrypt(0);
    assert!(!c.identity_requested());
    assert!(matches!(encrypted(&mut c, 6).unwrap(), Action::Bond(_)));
}

#[test]
fn stored_irk_resolves_a_reconnect_from_a_private_address() {
    let bond = Bond {
        address_type: 0,
        address: PUBLIC,
        key_size: 16,
        flags: 2,
        ediv: 0,
        random: [0; 8],
        ltk: [7; 16],
        irk: irk(),
    };
    let mut record = [0; 4096];
    pairing::encode_state(&[bond], None, &mut record).unwrap();
    let mut store = Store::new();
    store.load(&record).unwrap();
    assert!(store.connectable(0, 1, &RPA));
    assert!(store.resolved(1, &RPA));
    assert!(store.connectable(0, 0, &PUBLIC));
    assert!(!store.resolved(0, &PUBLIC));
    let mut other = RPA;
    other[1] ^= 0x10;
    assert!(!store.connectable(0, 1, &other));
    assert!(store.begin(9, 5, 0, 1, RPA).is_ok());
    assert!(store.begin(9, 5, 0, 1, other).is_err());
}

#[test]
fn without_a_stored_bond_directed_or_known_addresses_never_reconnect() {
    // The host failed to save (or never had) a bond while the controller kept
    // one: its directed advertising must not start a reconnect, so only a new
    // pairing (pairing mode, HID advertising) can proceed.
    let empty = Store::new();
    assert!(!empty.any());
    for (event, kind, address) in [(1, 0, PUBLIC), (0, 0, PUBLIC), (1, 1, RPA)] {
        assert!(!empty.connectable(event, kind, &address));
        assert!(!empty.resolved(kind, &address));
        assert!(empty.begin(9, 5, 0, kind, address).is_err());
    }
    // A bond for another controller does not adopt this one either.
    let other = Bond {
        address_type: 0,
        address: [9; 6],
        key_size: 16,
        flags: 2,
        ediv: 0,
        random: [0; 8],
        ltk: [7; 16],
        irk: [0; 16],
    };
    let mut record = [0; 4096];
    pairing::encode_state(&[other], None, &mut record).unwrap();
    let mut store = Store::new();
    store.load(&record).unwrap();
    assert!(!store.connectable(1, 0, &PUBLIC));
    assert!(!store.connectable(1, 1, &RPA));
}

#[test]
fn awaiting_response_ends_when_the_peer_answers() {
    let mut rng = Fixed([1; 32]);
    let mut c = Central::new(3, 5, 0, Method::JustWorks, 0, [1; 6], 0, RA, &mut rng).unwrap();
    assert!(c.awaiting_response());
    c.receive(3, 1, &[2, 0, 0, 9, 16, 0, 2], &mut rng).unwrap();
    assert!(!c.awaiting_response());
}
