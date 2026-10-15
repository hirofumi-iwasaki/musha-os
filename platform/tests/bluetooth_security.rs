// Copyright 2026 Hirofumi Iwasaki
// SPDX-License-Identifier: Apache-2.0
#![cfg(feature = "bluetooth-security")]
use musha_platform::bluetooth::{
    Error,
    entropy::{Pool, Random},
    sc_crypto::*,
};
fn hex<const N: usize>(s: &str) -> [u8; N] {
    let mut b = [0; N];
    for i in 0..N {
        b[i] = u8::from_str_radix(&s[i * 2..i * 2 + 2], 16).unwrap();
    }
    b
}
#[test]
fn official_cmac_f4_f5_f6_g2_vectors() {
    let k = hex("2b7e151628aed2a6abf7158809cf4f3c");
    assert_eq!(cmac(&k, &[&[]]), hex("bb1d6929e95937287fa37d129b756746"));
    assert_eq!(
        cmac(&k, &[&hex::<16>("6bc1bee22e409f96e93d7e117393172a")]),
        hex("070a16b46b4d4144f79bdd9dd04a287c")
    );
    let u = hex("20b003d2f297be2c5e2c83a7e9f9a5b9eff49111acf4fddbcc0301480e359de6");
    let v = hex("55188b3d32f6bb9a900afcfbeed4e72a59cb9ac2f19d7cfb6b4fdd49f47fc5fd");
    let n1 = hex("d5cb8454d177733effffb2ec712baeab");
    let n2 = hex("a6e8e7cc25a75f6e216583f7ff3dc4cf");
    assert_eq!(f4(&u, &v, &n1, 0), hex("f2c916f107a9bd1cf1eda1bea974872d"));
    let w = hex("ec0234a357c8ad05341010a60a397d9b99796b13b4f866f1868d34f373bfa698");
    let a1 = hex("0056123737bfce");
    let a2 = hex("00a713702dcfc1");
    let (mac, ltk) = f5(&w, &n1, &n2, &a1, &a2);
    assert_eq!(mac, hex("2965f176a1084a02fd3f6a20ce636e20"));
    assert_eq!(ltk, hex("6986791169d7cd23980522b594750a38"));
    assert_eq!(
        f6(
            &mac,
            &n1,
            &n2,
            &hex("12a3343bb453bb5408da42d20c2d0fc8"),
            &hex("010102"),
            &a1,
            &a2
        ),
        hex("e3c473989cd0e8c5d26c0b09da958f61")
    );
    assert_eq!(g2(&u, &v, &n1, &n2), 0x2f9ed5ba);
}
struct Fixed([u8; 32]);
impl Random for Fixed {
    fn fill(&mut self, b: &mut [u8]) -> Result<(), Error> {
        b.copy_from_slice(&self.0[..b.len()]);
        Ok(())
    }
}
#[test]
fn ecdh_rejects_invalid_curve_zero_scalar_reflection_and_debug_peer() {
    assert!(Key::generate(&mut Fixed([0; 32])).is_err());
    let key = Key::generate(&mut Fixed([1; 32])).unwrap();
    assert!(key.dh(&[0; 64]).is_err());
    assert!(key.dh(&[255; 64]).is_err());
    assert!(key.dh(&key.public_wire()).is_err());
    let debug = Key::generate(&mut Fixed(hex(
        "3f49f6d4a3c55f3874c9b3e3d2103f504aff607beb40b7995899b8a6cd3c1abd",
    )))
    .unwrap();
    assert!(key.dh(&debug.public_wire()).is_err());
    let other = Key::generate(&mut Fixed([2; 32])).unwrap();
    assert_eq!(
        key.dh(&other.public_wire()).unwrap(),
        other.dh(&key.public_wire()).unwrap()
    );
}
#[test]
fn entropy_is_one_use_finite_and_provider_failure_clears_all() {
    let mut p = Pool::<64>::new();
    let mut n = 1;
    p.initialize(|b| {
        b.fill(n);
        n += 1;
        Ok(())
    })
    .unwrap();
    let mut a = [0; 32];
    p.fill(&mut a).unwrap();
    assert_eq!(a, [1; 32]);
    p.fill(&mut a).unwrap();
    assert_eq!(a, [2; 32]);
    assert_eq!(p.remaining(), 0);
    assert_eq!(p.fill(&mut a), Err(Error::Entropy));
    assert_eq!(a, [0; 32]);
    assert!(p.initialize(|_| Ok(())).is_err());
    let mut p = Pool::<64>::new();
    assert!(
        p.initialize(|b| {
            b.fill(1);
            Ok(())
        })
        .is_err()
    );
    assert_eq!(p.fill(&mut a), Err(Error::Entropy));
    let mut p = Pool::<64>::new();
    let mut n = 0;
    assert!(
        p.initialize(|b| {
            n += 1;
            b.fill(3);
            if n == 2 { Err(Error::Entropy) } else { Ok(()) }
        })
        .is_err()
    );
    assert_eq!(p.remaining(), 0);
}
#[test]
fn uniform_passkey_sampling_is_bounded_not_a_weak_modulo_fallback() {
    assert_eq!(
        musha_platform::bluetooth::entropy::passkey(&mut Fixed([255; 32])),
        Err(Error::Entropy)
    );
    let mut b = [0; 32];
    b[..4].copy_from_slice(&999999u32.to_le_bytes());
    assert_eq!(
        musha_platform::bluetooth::entropy::passkey(&mut Fixed(b)),
        Ok(999999)
    );
}
#[test]
fn official_p256_ecdh_sample_two() {
    // Core Vol2 PartG 7.1.2.2, independently published shared secret.
    let key = Key::generate(&mut Fixed(hex(
        "06a516693c9aa31a6084545d0c5db641b48572b97203ddffb7ac73f7d0457663",
    )))
    .unwrap();
    let mut wire = [0; 64];
    wire[..32].copy_from_slice(&hex::<32>(
        "f465e43ff23d3f1b9dc7dfc04da8758184dbc966204796eccf0d6cf5e16500cc",
    ));
    wire[32..].copy_from_slice(&hex::<32>(
        "0201d048bcbbd899eeefc424164e33c201c2b010ca6b4d43a8a155cad8ecb279",
    ));
    wire[..32].reverse();
    wire[32..].reverse();
    assert_eq!(
        key.dh(&wire).unwrap(),
        hex("ab85843a2f6d883f62e5684b38e307335fe6e1945ecd19604105c6f23221eb69")
    );
}
fn sent(action: musha_platform::bluetooth::sc::Action) -> Vec<u8> {
    match action {
        musha_platform::bluetooth::sc::Action::Send(p, n) => p[..n].to_vec(),
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
#[test]
fn sc_jw_passkey_and_encryption_authority_transcripts() {
    use musha_platform::bluetooth::{
        Event,
        sc::{Action, Central, Method},
    };
    for method in [
        Method::JustWorks,
        Method::DisplayPasskey,
        Method::EnterPasskey,
    ] {
        let mut rng = Fixed([1; 32]);
        let ia = [1, 2, 3, 4, 5, 6];
        let ra = [7, 8, 9, 10, 11, 12];
        let mut c = Central::new(3, 5, 0, method, 0, ia, 0, ra, &mut rng).unwrap();
        let req = c.request();
        let resp = [
            2,
            if method == Method::DisplayPasskey {
                2
            } else {
                0
            },
            0,
            if method == Method::JustWorks { 9 } else { 13 },
            16,
            0,
            0,
        ];
        let public = sent(c.receive(3, 1, &resp, &mut rng).unwrap());
        let local: [u8; 64] = public[1..].try_into().unwrap();
        let peer = Key::generate(&mut Fixed([2; 32])).unwrap();
        let peerwire = peer.public_wire();
        let mut packet = vec![12];
        packet.extend_from_slice(&peerwire);
        let action = c.receive(3, 2, &packet, &mut rng).unwrap();
        let pass = match method {
            Method::JustWorks => {
                assert!(sent(action).is_empty());
                0
            }
            Method::DisplayPasskey => {
                let Action::Display(n) = action else { panic!() };
                let first = sent(c.displayed(3, 3, &mut rng).unwrap());
                assert_eq!(first[0], 3);
                n
            }
            Method::EnterPasskey => {
                assert!(matches!(action, Action::Enter));
                assert_eq!(sent(c.entered(3, 3, 123456, &mut rng).unwrap())[0], 3);
                123456
            }
        };
        let na = [1; 16];
        let nb = [4; 16];
        let rounds = if method == Method::JustWorks { 1 } else { 20 };
        let mut last = vec![];
        for i in 0..rounds {
            let z = if method == Method::JustWorks {
                0
            } else {
                0x80 | ((pass >> i) & 1) as u8
            };
            let confirm = f4(&x(&peerwire), &x(&local), &nb, z);
            assert_eq!(
                sent(
                    c.receive(3, 4 + i as u64 * 2, &wire16(3, confirm), &mut rng)
                        .unwrap()
                ),
                wire16(4, na)
            );
            last = sent(
                c.receive(3, 5 + i as u64 * 2, &wire16(4, nb), &mut rng)
                    .unwrap(),
            );
            assert_eq!(last[0], if i + 1 == rounds { 13 } else { 3 });
        }
        let dh = peer.dh(&local).unwrap();
        let a1 = [0, 6, 5, 4, 3, 2, 1];
        let a2 = [0, 12, 11, 10, 9, 8, 7];
        let (mac, ltk) = f5(&dh, &na, &nb, &a1, &a2);
        let mut r = [0; 16];
        if method != Method::JustWorks {
            r[12..].copy_from_slice(&pass.to_be_bytes());
        }
        assert_eq!(
            last,
            wire16(13, f6(&mac, &na, &nb, &r, &[req[3], 0, req[1]], &a1, &a2))
        );
        let eb = f6(&mac, &nb, &na, &r, &[resp[3], 0, resp[1]], &a2, &a1);
        let Action::Encrypt(mut actual) = c.receive(3, 50, &wire16(13, eb), &mut rng).unwrap()
        else {
            panic!()
        };
        actual.reverse();
        assert_eq!(actual, ltk);
        let Action::Bond(bond) = c
            .encryption(3, 51, Event::parse(&[8, 4, 0, 5, 0, 1]).unwrap())
            .unwrap()
        else {
            panic!()
        };
        assert_eq!(
            bond.bond().flags,
            if method == Method::JustWorks { 2 } else { 3 }
        );
        assert_eq!(bond.bond().address, ra);
        // Exercise the exact discovery/binding code linked into normal runtime.
        use musha_platform::bluetooth::hogp::{Discovery, DiscoveryAction};
        let mut discovery = Discovery::new();
        assert!(matches!(discovery.request(), DiscoveryAction::Send(_, 7)));
        for response in [
            vec![0x11, 6, 1, 0, 32, 0, 0x12, 0x18],
            vec![
                9, 7, 2, 0, 2, 3, 0, 0x4b, 0x2a, 4, 0, 0x10, 5, 0, 0x4d, 0x2a,
            ],
            vec![1, 8, 6, 0, 0x0a],
        ] {
            assert!(matches!(
                discovery.receive(&response, &bond).unwrap(),
                DiscoveryAction::Send(_, _)
            ));
        }
        let map = [
            0x05, 1, 0x09, 6, 0xa1, 1, 0x85, 2, 0x05, 7, 0x19, 0, 0x29, 101, 0x15, 0, 0x25, 101,
            0x75, 8, 0x95, 1, 0x81, 0, 0xc0,
        ];
        let mut first = vec![0x0b];
        first.extend_from_slice(&map[..22]);
        assert!(matches!(
            discovery.receive(&first, &bond).unwrap(),
            DiscoveryAction::Send(_, 5)
        ));
        let mut rest = vec![0x0d];
        rest.extend_from_slice(&map[22..]);
        discovery.receive(&rest, &bond).unwrap();
        discovery
            .receive(&[5, 1, 6, 0, 2, 0x29, 7, 0, 8, 0x29], &bond)
            .unwrap();
        discovery.receive(&[1, 4, 8, 0, 0x0a], &bond).unwrap();
        let DiscoveryAction::Ready(mut keyboard) = discovery.receive(&[0x0b, 2, 1], &bond).unwrap()
        else {
            panic!()
        };
        assert_eq!(keyboard.cccd(0), Some(6));
        let mut keys = vec![];
        keyboard
            .notification(3, 5, &[0x1b, 5, 0, 4], |c| {
                if let musha_platform::bluetooth::hogp::Change::Key(k, d) = c {
                    keys.push((k, d))
                }
            })
            .unwrap();
        assert_eq!(keys, [(4, true)]);
        assert!(
            keyboard
                .notification(4, 5, &[0x1b, 5, 0, 4], |_| panic!())
                .is_err()
        );
        keyboard.disconnect(|c| {
            if let musha_platform::bluetooth::hogp::Change::Key(k, d) = c {
                keys.push((k, d))
            }
        });
        assert_eq!(keys.last(), Some(&(4, false)));
        let raw = Some(musha_platform::config_hash::sha256(b"pair=on\r\n"));
        let saved = bond.save(None, raw, |record| {
            assert!(musha_platform::pairing::recognized(record));
            assert_eq!(
                musha_platform::pairing::consumed_config_hash(record).unwrap(),
                raw
            );
            if method == Method::DisplayPasskey {
                Err(Error::Timeout)
            } else {
                Ok(())
            }
        });
        if method == Method::DisplayPasskey {
            assert_eq!(
                saved,
                Err(musha_platform::bluetooth::bonding::SaveError::PersistenceUnknown)
            );
        } else {
            assert_eq!(saved, Ok(raw));
        }

        assert!(
            c.encryption(3, 52, Event::parse(&[8, 4, 0, 5, 0, 1]).unwrap())
                .is_err()
        );
    }
}
#[test]
fn sc_rejects_downgrade_wrong_state_timeout_and_cancel() {
    use musha_platform::bluetooth::{
        Event,
        sc::{Central, Method},
    };
    for bad in [
        [2, 3, 0, 1, 16, 0, 0],
        [2, 3, 0, 9, 7, 0, 0],
        [2, 3, 1, 9, 16, 0, 0],
        [2, 3, 0, 9, 16, 1, 0],
        [2, 3, 0, 13, 16, 0, 0],
    ] {
        let mut c = Central::new(
            1,
            2,
            0,
            Method::JustWorks,
            0,
            [1; 6],
            0,
            [2; 6],
            &mut Fixed([1; 32]),
        )
        .unwrap();
        assert!(c.receive(1, 1, &bad, &mut Fixed([1; 32])).is_err());
        assert!(c.check(1, 2).is_err());
    }
    let mut c = Central::new(
        1,
        2,
        0,
        Method::EnterPasskey,
        0,
        [1; 6],
        0,
        [2; 6],
        &mut Fixed([1; 32]),
    )
    .unwrap();
    assert_eq!(c.check(2, 1), Err(Error::Stale));
    assert!(
        c.encryption(1, 1, Event::parse(&[8, 4, 0, 2, 0, 1]).unwrap())
            .is_err()
    );
    let mut c = Central::new(
        1,
        2,
        0,
        Method::EnterPasskey,
        0,
        [1; 6],
        0,
        [2; 6],
        &mut Fixed([1; 32]),
    )
    .unwrap();
    assert_eq!(c.check(1, 30000), Err(Error::Timeout));
    let mut c = Central::new(
        1,
        2,
        0,
        Method::EnterPasskey,
        0,
        [1; 6],
        0,
        [2; 6],
        &mut Fixed([1; 32]),
    )
    .unwrap();
    c.cancel();
    assert!(c.check(1, 1).is_err());
}
