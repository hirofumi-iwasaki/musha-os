// Copyright 2026 Hirofumi Iwasaki
// SPDX-License-Identifier: Apache-2.0
//! Fixed nonces/keys are synthetic host fixtures only. No production RNG.
use musha_platform::bluetooth::{
    self as bt, Error, Event,
    acl::{Credits, Reassembly},
    att,
};
#[test]
fn fragmented_att_reassembles_once_and_rejects_stale_or_oversize() {
    let mut r = Reassembly::new(2, 0x40).unwrap();
    let mut seen = 0;
    assert_eq!(
        r.receive(
            2,
            0,
            &[0x40, 0x20, 6, 0, 4, 0, 4, 0, 0x1b, 1],
            |_, _| panic!()
        ),
        Ok(false)
    );
    assert_eq!(
        r.receive(1, 1, &[0x40, 0x10, 2, 0, 0, 7], |_, _| panic!()),
        Err(Error::Stale)
    );
    assert_eq!(
        r.receive(2, 1, &[0x40, 0x10, 2, 0, 0, 7], |cid, p| {
            assert_eq!(cid, 4);
            assert_eq!(p, [0x1b, 1, 0, 7]);
            seen += 1;
        }),
        Ok(true)
    );
    assert_eq!(seen, 1);
    assert_eq!(
        r.receive(2, 2, &[0x40, 0x10, 1, 0, 1], |_, _| panic!()),
        Err(Error::Invalid)
    );
    assert_eq!(
        r.receive(2, 3, &[0x40, 0x20, 4, 0, 1, 4, 4, 0], |_, _| panic!()),
        Err(Error::Unsupported)
    );
}
#[test]
fn fragment_timeout_empty_overrun_and_new_start_do_not_publish() {
    for next in [
        &[0x40, 0x10, 2, 0, 1, 2][..],
        &[0x40, 0x20, 5, 0, 1, 0, 4, 0, 1][..],
        &[0x40, 0x10, 0, 0][..],
    ] {
        let mut r = Reassembly::new(1, 0x40).unwrap();
        r.receive(1, 0, &[0x40, 0x20, 5, 0, 2, 0, 4, 0, 1], |_, _| panic!())
            .unwrap();
        assert!(r.receive(1, 1, next, |_, _| panic!()).is_err());
    }
    let mut r = Reassembly::new(1, 0x40).unwrap();
    r.receive(1, 0, &[0x40, 0x20, 5, 0, 2, 0, 4, 0, 1], |_, _| panic!())
        .unwrap();
    assert_eq!(
        r.receive(1, 1000, &[0x40, 0x10, 1, 0, 2], |_, _| panic!()),
        Err(Error::Timeout)
    );
}
#[test]
fn acl_credits_correlate_handles_and_reject_duplicate_or_excess_completion() {
    let mut c = Credits::new(7, 0x40, 1, 27).unwrap();
    c.reserve(7, 27).unwrap();
    assert_eq!(c.reserve(7, 1), Err(Error::Busy));
    assert_eq!(
        c.completed(6, Event::parse(&[0x13, 5, 1, 0x40, 0, 1, 0]).unwrap()),
        Err(Error::Stale)
    );
    c.completed(7, Event::parse(&[0x13, 5, 1, 0x40, 0, 1, 0]).unwrap())
        .unwrap();
    c.reserve(7, 1).unwrap();
    assert_eq!(
        c.completed(7, Event::parse(&[0x13, 5, 1, 0x40, 0, 2, 0]).unwrap()),
        Err(Error::Invalid)
    );
    assert_eq!(
        c.completed(
            7,
            Event::parse(&[0x13, 9, 2, 0x40, 0, 1, 0, 0x40, 0, 0, 0]).unwrap()
        ),
        Err(Error::Invalid)
    );
    assert_eq!(c.reserve(7, 28), Err(Error::Length));
}
#[test]
fn gatt_discovers_hid_service_and_report_without_hardcoded_handles() {
    let mut found = 0;
    att::services(
        &[0x11, 6, 1, 0, 5, 0, 0x0f, 0x18, 6, 0, 20, 0, 0x12, 0x18],
        |s| {
            if s.uuid16 == Some(0x1812) {
                assert_eq!((s.start, s.end), (6, 20));
                found += 1;
            }
        },
    )
    .unwrap();
    assert_eq!(found, 1);
    att::characteristics(&[9, 7, 7, 0, 0x12, 8, 0, 0x4d, 0x2a], 6, 20, |c| {
        assert_eq!((c.value, c.uuid16), (8, Some(0x2a4d)))
    })
    .unwrap();
    assert_eq!(
        att::characteristics(&[9, 7, 7, 0, 0x12, 21, 0, 0x4d, 0x2a], 6, 20, |_| panic!()),
        Err(Error::Invalid)
    );
    assert!(
        att::services(
            &[0x11, 6, 1, 0, 5, 0, 0x12, 0x18, 4, 0, 9, 0, 1, 0],
            |_| panic!()
        )
        .is_err()
    );
}
#[cfg(feature = "bluetooth-legacy")]
mod legacy {
    use super::*;
    use bt::bonding::SaveError;
    use bt::legacy::{self, Action, Association, Legacy, VerifiedBond};
    fn wire<const N: usize>(s: &str) -> [u8; N] {
        let mut b = [0; N];
        for i in 0..N {
            b[N - 1 - i] = u8::from_str_radix(&s[i * 2..i * 2 + 2], 16).unwrap();
        }
        b
    }
    fn engine(method: Association) -> Legacy {
        Legacy::new(7, 0x40, 0, method, [1; 16], 0, [1; 6], 1, [2; 6]).unwrap()
    }
    fn to_encryption(method: Association) -> Legacy {
        let mut s = engine(method);
        let response = [
            2,
            if method == Association::JustWorks {
                3
            } else {
                2
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
        let request = s.request();
        assert!(matches!(
            s.receive(7, 1, &response),
            Ok(Action::Send(_, 17))
        ));
        let mut tk = [0; 16];
        if let Association::DisplayPasskey(n) = method {
            tk[..4].copy_from_slice(&n.to_le_bytes());
        }
        let confirm = legacy::c1(&tk, &[2; 16], &request, &response, 0, 1, &[1; 6], &[2; 6]);
        let mut p = [3; 17];
        p[1..].copy_from_slice(&confirm);
        assert!(matches!(s.receive(7, 2, &p), Ok(Action::Send(_, 17))));
        p[0] = 4;
        p[1..].fill(2);
        match s.receive(7, 3, &p).unwrap() {
            Action::Encrypt(k) => assert_ne!(k, [0; 16]),
            _ => panic!(),
        };
        s
    }
    pub fn bond() -> VerifiedBond {
        let mut s = to_encryption(Association::JustWorks);
        s.encryption(7, 4, Event::parse(&[8, 4, 0, 0x40, 0, 1]).unwrap())
            .unwrap();
        assert!(matches!(s.receive(7, 5, &[6; 17]), Ok(Action::None)));
        let p = [7, 1, 0, 2, 2, 2, 2, 2, 2, 2, 2];
        match s.receive(7, 6, &p).unwrap() {
            Action::Bond(b) => b,
            _ => panic!(),
        }
    }
    #[test]
    fn official_c1_and_s1_vectors_use_explicit_wire_endianness() {
        assert_eq!(
            legacy::c1(
                &[0; 16],
                &wire("5783D52156AD6F0E6388274EC6702EE0"),
                &wire("07071000000101"),
                &wire("05000800000302"),
                1,
                0,
                &wire("A1A2A3A4A5A6"),
                &wire("B1B2B3B4B5B6")
            ),
            wire("1E1E3FEF878988EAD2A74DC5BEF13B86")
        );
        assert_eq!(
            legacy::s1(
                &[0; 16],
                &wire("000F0E0D0C0B0A091122334455667788"),
                &wire("010203040506070899AABBCCDDEEFF00")
            ),
            wire("9a1fe1f0e8b0f49b5b4216ae796da062")
        );
    }
    #[test]
    fn jw_passkey_and_encryption_before_distribution() {
        assert_eq!(bond().bond().flags, 0);
        let mut s = to_encryption(Association::DisplayPasskey(123456));
        assert!(s.receive(7, 4, &[6; 17]).is_err());
        let mut s = to_encryption(Association::DisplayPasskey(123456));
        s.encryption(7, 4, Event::parse(&[8, 4, 0, 0x40, 0, 1]).unwrap())
            .unwrap();
        assert!(matches!(s.receive(7, 5, &[6; 17]), Ok(Action::None)));
        match s.receive(7, 6, &[7; 11]).unwrap() {
            Action::Bond(b) => assert_eq!(b.bond().flags, 1),
            _ => panic!(),
        }
    }
    #[test]
    fn no_passkey_downgrade_or_short_key_and_invalid_confirm() {
        let mut s = engine(Association::DisplayPasskey(1));
        assert!(s.receive(7, 1, &[2, 3, 0, 1, 16, 0, 1]).is_err());
        let mut s = engine(Association::JustWorks);
        assert!(s.receive(7, 1, &[2, 3, 0, 1, 7, 0, 1]).is_err());
        let mut s = engine(Association::JustWorks);
        s.receive(7, 1, &[2, 3, 0, 1, 16, 0, 1]).unwrap();
        s.receive(7, 2, &[3; 17]).unwrap();
        assert!(s.receive(7, 3, &[4; 17]).is_err());
    }
    #[test]
    fn failed_pairing_timeout_generation_and_encryption_do_not_bond() {
        let mut s = engine(Association::JustWorks);
        assert!(matches!(s.receive(6, 1, &[5, 1]), Err(Error::Stale)));
        assert!(matches!(s.receive(7, 30000, &[5, 1]), Err(Error::Timeout)));
        let mut s = to_encryption(Association::JustWorks);
        assert!(
            s.encryption(7, 4, Event::parse(&[8, 4, 0, 0x40, 0, 0]).unwrap())
                .is_err()
        );
        assert!(s.receive(7, 5, &[6; 17]).is_err());
    }
    #[test]
    fn durable_save_consumes_only_after_writer_success_and_validates_old_record() {
        let hash = musha_platform::config_hash::sha256(b"pair=on\r\n");
        let mut record = [0; 4096];
        assert_eq!(
            bond().save(None, Some(hash), |b| {
                record.copy_from_slice(b);
                Ok(())
            }),
            Ok(Some(hash))
        );
        let mut decoded = [*bond().bond(); 8];
        let (n, consumed) = musha_platform::pairing::decode_state(&record, &mut decoded).unwrap();
        assert_eq!(n, 1);
        assert_eq!(musha_platform::pairing::bond_count(&record), Ok(1));
        assert_eq!(consumed, Some(hash));
        assert!(!musha_platform::pairing::requested(
            true,
            Some(hash),
            consumed
        ));
        assert!(musha_platform::pairing::requested(
            true,
            Some([9; 32]),
            consumed
        ));
        assert_eq!(
            bond().save(Some(&record), Some([9; 32]), |_| Err(Error::Invalid)),
            Err(SaveError::PersistenceUnknown)
        );
        record[100] ^= 1;
        assert!(matches!(
            bond().save(Some(&record), Some(hash), |_| panic!()),
            Err(SaveError::Record(_))
        ));
    }
    #[test]
    fn authenticated_record_cannot_be_replaced_by_jw_and_key_debug_is_redacted() {
        let b = *bond().bond();
        let mut old = [0; 4096];
        let mut authenticated = b;
        authenticated.flags = 1;
        musha_platform::pairing::encode(&[authenticated], &mut old).unwrap();
        assert_eq!(
            bond().save(Some(&old), None, |_| panic!()),
            Err(SaveError::Security)
        );
        let text = format!("{:?}", b);
        assert!(text.contains("REDACTED"));
        assert!(!text.contains("ltk"));
    }
    #[test]
    fn hogp_notification_bound_to_verified_connection_generation_handle_id_and_size() {
        let b = bond();
        let report = att::InputReport::bind(&b, 8, &[2, 1], 8, 0x10).unwrap();
        assert_eq!(
            report
                .notification(7, 0x40, &[0x1b, 8, 0, 0, 0, 4, 0, 0, 0, 0, 0])
                .unwrap(),
            (2, &[0, 0, 4, 0, 0, 0, 0, 0][..])
        );
        assert_eq!(
            report.notification(8, 0x40, &[0x1b, 8, 0]),
            Err(Error::Stale)
        );
        assert!(report.notification(7, 0x40, &[0x1b, 8, 0]).is_err());
        assert!(att::InputReport::bind(&b, 8, &[2, 2], 8, 0x10).is_err());
    }
    #[test]
    fn hogp_keyboard_map_notification_to_common_key_and_disconnect_releases() {
        let b = bond();
        let map = [
            0x05, 1, 0x09, 6, 0xa1, 1, 0x85, 2, 0x05, 7, 0x19, 0, 0x29, 101, 0x15, 0, 0x25, 101,
            0x75, 8, 0x95, 1, 0x81, 0, 0xc0,
        ];
        let mut k = bt::hogp::Keyboard::bind(&b, 8, &[2, 1], &map, 0x10).unwrap();
        let mut events = Vec::new();
        k.notification(7, 0x40, &[0x1b, 8, 0, 4], |u, d| events.push((u, d)))
            .unwrap();
        assert_eq!(events, [(4, true)]);
        assert!(
            k.notification(7, 0x40, &[0x1b, 8, 0, 0, 0], |_, _| panic!())
                .is_err()
        );
        k.disconnect(|u, d| events.push((u, d)));
        assert_eq!(events, [(4, true), (4, false)]);
        assert_eq!(
            k.notification(7, 0x40, &[0x1b, 8, 0, 4], |_, _| panic!()),
            Err(Error::Stale)
        );
        assert!(
            bt::hogp::Keyboard::bind(&b, 8, &[2, 1], &[0x05, 1, 0x09, 5, 0xa1, 1, 0xc0], 0x10)
                .is_err()
        );
    }
}
