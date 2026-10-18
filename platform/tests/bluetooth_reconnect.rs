// Copyright 2026 Hirofumi Iwasaki
// SPDX-License-Identifier: Apache-2.0
#![cfg(feature = "bluetooth-security")]
use musha_platform::{
    bluetooth::{
        self as bt, Error, Event,
        hogp::{Change, Discovery, DiscoveryAction, ReportSpec, Reports},
        reconnect::Store,
    },
    pairing::{self, Bond},
};
fn bond() -> Bond {
    Bond {
        address_type: 0,
        address: [2; 6],
        key_size: 16,
        flags: 3,
        ediv: 0,
        random: [0; 8],
        ltk: [7; 16],
        irk: [0; 16],
    }
}
fn record(b: Bond) -> [u8; 4096] {
    let mut r = [0; 4096];
    pairing::encode_state(&[b], Some([9; 32]), &mut r).unwrap();
    r
}
fn verified() -> bt::legacy::VerifiedBond {
    let mut store = Store::new();
    store.load(&record(bond())).unwrap();
    let mut p = store.begin(9, 5, 0, 0, [2; 6]).unwrap();
    p.command_key(9, 1).unwrap();
    p.encryption(9, 2, Event::parse(&[8, 4, 0, 5, 0, 1]).unwrap())
        .unwrap()
}
const KB: &[u8] = &[
    5, 1, 9, 6, 0xa1, 1, 0x85, 2, 5, 7, 0x19, 0, 0x29, 101, 0x15, 0, 0x25, 101, 0x75, 8, 0x95, 1,
    0x81, 0, 0xc0,
];
const GP: &[u8] = &[
    5, 1, 9, 5, 0xa1, 1, 0x85, 4, 5, 9, 0x19, 1, 0x29, 4, 0x15, 0, 0x25, 1, 0x75, 1, 0x95, 4, 0x81,
    2, 0x75, 4, 0x95, 1, 0x81, 3, 0x85, 5, 5, 1, 9, 0x30, 9, 0x31, 0x15, 0x81, 0x25, 0x7f, 0x75, 8,
    0x95, 2, 0x81, 2, 9, 0x39, 0x15, 0, 0x25, 7, 0x75, 4, 0x95, 1, 0x81, 0x42, 0x75, 4, 0x95, 1,
    0x81, 3, 0xc0,
];
fn map() -> Vec<u8> {
    [KB, GP].concat()
}
fn spec(value: u16, id: u8) -> ReportSpec {
    ReportSpec {
        value,
        end: value + 4,
        properties: 0x10,
        reference: value + 2,
        cccd: value + 1,
        id,
        kind: 1,
    }
}
#[test]
fn stored_key_identity_type_generation_status_and_one_use() {
    let mut s = Store::new();
    assert_eq!(s.load(&record(bond())), Ok(Some([9; 32])));
    assert!(s.any());
    assert!(s.matches(0, &[2; 6]));
    assert!(!s.matches(1, &[2; 6]));
    assert!(!s.matches(0, &[3; 6]));
    let mut p = s.begin(9, 5, 100, 0, [2; 6]).unwrap();
    assert!(p.command_key(8, 101).is_err());
    p.command_key(9, 101)
        .unwrap()
        .use_with(|k| assert_eq!(k, &[7; 16]));
    assert!(p.command_key(9, 102).is_err());
    let b = p
        .encryption(9, 103, Event::parse(&[8, 4, 0, 5, 0, 1]).unwrap())
        .unwrap();
    assert_eq!(b.bond().flags, 3);
    assert!(
        p.encryption(9, 104, Event::parse(&[8, 4, 0, 5, 0, 1]).unwrap())
            .is_err()
    );
}
#[test]
fn stored_key_failure_cancel_timeout_no_pairing_or_hash_mutation() {
    for event in [
        [8, 4, 6, 5, 0, 0],
        [8, 4, 0, 6, 0, 1],
        [8, 4, 0, 5, 0, 0],
        [8, 4, 0, 5, 0, 2],
    ] {
        let mut s = Store::new();
        s.load(&record(bond())).unwrap();
        let mut p = s.begin(1, 5, 0, 0, [2; 6]).unwrap();
        p.command_key(1, 1).unwrap();
        assert!(p.encryption(1, 2, Event::parse(&event).unwrap()).is_err());
        assert!(p.command_key(1, 3).is_err());
    }
    let mut s = Store::new();
    s.load(&record(bond())).unwrap();
    let mut p = s.begin(1, 5, 0, 0, [2; 6]).unwrap();
    assert_eq!(p.check(1, 10000), Err(Error::Timeout));
    let mut p = s.begin(1, 5, 0, 0, [2; 6]).unwrap();
    p.cancel();
    assert!(p.command_key(1, 1).is_err());
    assert_eq!(
        pairing::consumed_config_hash(&record(bond())),
        Ok(Some([9; 32]))
    );
}
#[test]
fn stored_bad_legacy_short_rpa_and_corrupt_records_refuse() {
    for (kind, address, flags, size, ediv) in [
        (0, [2; 6], 0, 16, 0),
        (0, [2; 6], 2, 15, 0),
        (1, [2; 6], 2, 16, 0),
        (0, [2; 6], 2, 16, 1),
    ] {
        let mut b = bond();
        b.address_type = kind;
        b.address = address;
        b.flags = flags;
        b.key_size = size;
        b.ediv = ediv;
        let mut s = Store::new();
        s.load(&record(b)).unwrap();
        assert!(!s.any());
        assert!(s.begin(1, 5, 0, kind, address).is_err());
    }
    let mut b = bond();
    b.address_type = 1;
    b.address[5] = 0xc2;
    let mut s = Store::new();
    s.load(&record(b)).unwrap();
    assert!(s.matches(1, &b.address));
    let mut bad = record(bond());
    bad[80] ^= 1;
    assert!(s.load(&bad).is_err());
    assert!(!s.any());
    assert!(!s.matches(0, &[2; 6]));
}
#[test]
fn reload_is_new_generation_not_old_handle() {
    let mut s = Store::new();
    s.load(&record(bond())).unwrap();
    let mut a = s.begin(1, 5, 0, 0, [2; 6]).unwrap();
    a.cancel();
    s.clear();
    assert!(!s.any());
    s.load(&record(bond())).unwrap();
    let mut b = s.begin(2, 6, 0, 0, [2; 6]).unwrap();
    b.command_key(2, 1).unwrap();
    assert!(
        b.encryption(1, 2, Event::parse(&[8, 4, 0, 6, 0, 1]).unwrap())
            .is_err()
    );
    assert!(
        b.encryption(2, 3, Event::parse(&[8, 4, 0, 6, 0, 1]).unwrap())
            .is_ok()
    );
}
#[test]
fn mixed_keyboard_and_gamepad_ids_decode_atomically() {
    let b = verified();
    let mut r = Reports::bind(&b, &map(), &[spec(10, 2), spec(20, 4), spec(30, 5)]).unwrap();
    assert_eq!(r.count(), 3);
    assert!(r.supports(false) && r.supports(true));
    assert_eq!(r.cccd(2), Some(31));
    let mut events = vec![];
    r.notification(9, 5, &[0x1b, 10, 0, 4], |e| events.push(e))
        .unwrap();
    assert_eq!(events, [Change::Key(4, true)]);
    r.notification(9, 5, &[0x1b, 20, 0, 3], |e| events.push(e))
        .unwrap();
    assert!(
        events.contains(&Change::Gamepad(musha_input::gamepad::Change::Button(
            0, true
        )))
    );
    r.notification(9, 5, &[0x1b, 30, 0, 0x81, 127, 8], |e| events.push(e))
        .unwrap();
    assert!(
        events.contains(&Change::Gamepad(musha_input::gamepad::Change::Axis(
            0, -127
        )))
    );
    assert!(events.contains(&Change::Gamepad(musha_input::gamepad::Change::Axis(9, -1))));
    let n = events.len();
    assert!(
        r.notification(9, 5, &[0x1b, 30, 0, 0x80, 0, 0], |_| panic!())
            .is_err()
    );
    assert_eq!(events.len(), n);
    r.disconnect(|e| events.push(e));
    assert!(events.contains(&Change::Key(4, false)));
    assert!(
        events.contains(&Change::Gamepad(musha_input::gamepad::Change::Button(
            0, false
        )))
    );
    assert!(
        r.notification(9, 5, &[0x1b, 10, 0, 0], |_| panic!())
            .is_err()
    );
}
#[test]
fn report_id_handle_reference_and_descriptor_negatives() {
    let b = verified();
    assert!(Reports::bind(&b, &map(), &[spec(10, 2), spec(20, 2)]).is_err());
    assert!(Reports::bind(&b, &map(), &[spec(10, 99)]).is_err());
    assert!(Reports::bind(&b, &map(), &[spec(10, 2), spec(10, 4)]).is_err());
    let mut s = spec(10, 2);
    s.cccd = 40;
    assert!(Reports::bind(&b, &map(), &[s]).is_err());
    let mut r = Reports::bind(&b, &map(), &[spec(10, 2), spec(20, 4), spec(30, 5)]).unwrap();
    for (g, h, p) in [
        (8, 5, vec![0x1b, 10, 0, 4]),
        (9, 6, vec![0x1b, 10, 0, 4]),
        (9, 5, vec![0x1b, 11, 0, 4]),
        (9, 5, vec![0x1b, 10, 0, 2, 4]),
        (9, 5, vec![0x1b, 30, 0, 1]),
    ] {
        assert!(r.notification(g, h, &p, |_| panic!()).is_err());
    }
    let bad = [
        5, 1, 9, 5, 0xa1, 1, 0x85, 1, 5, 0xff, 9, 1, 0x15, 0, 0x25, 1, 0x75, 1, 0x95, 1, 0x81, 2,
        0xc0,
    ];
    assert!(Reports::bind(&b, &bad, &[spec(10, 1)]).is_err());
}
#[test]
fn same_discovery_executor_multiple_report_descriptors_and_blob() {
    let bond = verified();
    let mut d = Discovery::new();
    let mut slot: Option<musha_platform::bluetooth::hogp::Reports> = None;
    let mut action = d.request();
    let map = map();
    let mut steps = 0;
    loop {
        let DiscoveryAction::Send(p, n) = action else {
            let DiscoveryAction::Ready = action else {
                unreachable!()
            };
            let mut reports = slot.take().unwrap();
            assert_eq!(reports.count(), 3);
            let mut e = vec![];
            reports
                .notification(9, 5, &[0x1b, 12, 0, 1], |c| e.push(c))
                .unwrap();
            assert_eq!(
                e,
                [Change::Gamepad(musha_input::gamepad::Change::Button(
                    0, true
                ))]
            );
            break;
        };
        steps += 1;
        assert!(steps < 40);
        let h = u16::from_le_bytes([p[1], p[2]]);
        let response = match p[0] {
            0x10 => vec![0x11, 6, 1, 0, 28, 0, 0x12, 0x18],
            8 if h == 1 => vec![
                9, 7, 2, 0, 2, 3, 0, 0x4b, 0x2a, 4, 0, 0x10, 5, 0, 0x4d, 0x2a,
            ],
            8 if h == 6 => vec![
                9, 7, 11, 0, 0x10, 12, 0, 0x4d, 0x2a, 19, 0, 0x10, 20, 0, 0x4d, 0x2a,
            ],
            8 => vec![1, 8, p[1], p[2], 0x0a],
            0x0a if h == 3 => {
                let mut b = vec![0x0b];
                b.extend_from_slice(&map[..22]);
                b
            }
            0x0c => {
                assert_eq!(n, 5);
                let offset = u16::from_le_bytes([p[3], p[4]]) as usize;
                let mut b = vec![0x0d];
                b.extend_from_slice(&map[offset..map.len().min(offset + 22)]);
                b
            }
            4 if matches!(h, 6 | 13 | 21) => vec![5, 1, p[1], p[2], 2, 0x29, p[1] + 1, 0, 8, 0x29],
            4 => vec![1, 4, p[1], p[2], 0x0a],
            0x0a => vec![
                0x0b,
                match h {
                    7 => 2,
                    14 => 4,
                    22 => 5,
                    _ => panic!("reference"),
                },
                1,
            ],
            _ => panic!("unexpected request"),
        };
        let diagnostic = d.diagnostic(&response);
        assert_eq!(diagnostic.request, p[0]);
        assert_eq!(diagnostic.request_handle, h);
        assert_eq!(diagnostic.response, response.first().copied());
        let text = format!("{}", diagnostic);
        assert!(text.contains(&format!("request={:02x}", p[0])));
        if response[0] == 1 {
            assert_eq!(diagnostic.att_error, Some((p[0], h, 0x0a)));
            assert!(text.contains("code=0a"));
        }
        // In particular, AttributeNotFound ends characteristics/descriptors normally.
        action = d.receive_into(&response, &bond, &mut slot).unwrap();
    }
}
#[test]
fn bounded_arbitrary_maps_never_panic() {
    let mut seed = 19u64;
    for n in 0..600 {
        let mut bytes = vec![0u8; n % 180];
        for b in &mut bytes {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
            *b = (seed >> 32) as u8;
        }
        let _ = musha_input::gamepad::Layout::parse(&bytes);
    }
}
#[test]
fn keyboard_ids_share_hold_ownership() {
    let mut other = KB.to_vec();
    other[7] = 3;
    let map = [KB, &other].concat();
    let b = verified();
    let mut r = Reports::bind(&b, &map, &[spec(10, 2), spec(20, 3)]).unwrap();
    let mut events = vec![];
    for (h, k) in [(10, 4), (20, 4), (10, 0), (20, 0)] {
        r.notification(9, 5, &[0x1b, h, 0, k], |c| events.push(c))
            .unwrap();
    }
    assert_eq!(events, [Change::Key(4, true), Change::Key(4, false)]);
}
#[test]
fn sizes_ranges_stack_and_unknown_fields_fail_closed() {
    let mut bad = GP.to_vec();
    let pos = bad.windows(2).position(|b| b == [0x75, 8]).unwrap();
    bad[pos + 1] = 255;
    assert!(musha_input::gamepad::Layout::parse(&bad).is_err());
    let mut bad = GP.to_vec();
    let pos = bad.windows(2).position(|b| b == [0x95, 2]).unwrap();
    bad[pos + 1] = 255;
    assert!(musha_input::gamepad::Layout::parse(&bad).is_err());
    let mut bad = GP.to_vec();
    let pos = bad.windows(2).position(|b| b == [0x25, 1]).unwrap();
    bad[pos + 1] = 2;
    assert!(musha_input::gamepad::Layout::parse(&bad).is_err());
    for prefix in [
        vec![0xb4],
        vec![0xa4, 0xa4, 0xa4, 0xa4, 0xa4],
        vec![0xfe, 1, 2],
    ] {
        assert!(musha_input::gamepad::Layout::parse(&[prefix, GP.to_vec()].concat()).is_err());
    }
    let b = verified();
    let mut d = Discovery::new();
    let mut slot: Option<musha_platform::bluetooth::hogp::Reports> = None;
    assert!(
        d.receive_into(&[1, 0x10, 2, 0, 0x0a], &b, &mut slot)
            .is_err()
    );
    assert!(
        d.receive_into(&[0x11, 6, 1, 0, 32, 0, 0x12, 0x18], &b, &mut slot)
            .is_err()
    );
}
#[test]
fn directed_empty_ad_reconnect_uses_only_stored_identity_and_authenticated_map() {
    let mut store = Store::new();
    store.load(&record(bond())).unwrap();
    assert!(store.connectable(1, 0, &[2; 6])); // directed: no AD UUID/Appearance needed.
    assert!(store.connectable(0, 0, &[2; 6]));
    assert!(!store.connectable(2, 0, &[2; 6]));
    assert!(!store.connectable(1, 0, &[3; 6]));
    assert!(!store.connectable(1, 1, &[2; 6])); // random/RPA cannot alias public.
    let b = verified();
    // Independently authored map: gamepad axis plus the known Share bit.
    let share_map = [
        5, 1, 9, 5, 0xa1, 1, 0x85, 1, 9, 0x30, 0x15, 0, 0x26, 255, 0, 0x75, 8, 0x95, 1, 0x81, 2, 5,
        0x0c, 9, 0xb2, 0x25, 1, 0x75, 1, 0x81, 2, 0x75, 7, 0x81, 3, 0xc0,
    ];
    let mut reports = Reports::bind(&b, &share_map, &[spec(10, 1)]).unwrap();
    let mut seen = Vec::new();
    reports
        .notification(9, 5, &[0x1b, 10, 0, 42, 0], |c| seen.push(c))
        .unwrap();
    assert_eq!(
        seen,
        [bt::hogp::Change::Gamepad(
            musha_input::gamepad::Change::Axis(0, 42)
        )]
    );
    seen.clear();
    reports
        .notification(9, 5, &[0x1b, 10, 0, 42, 1], |c| seen.push(c))
        .unwrap();
    assert!(seen.is_empty());
    assert!(
        !Reports::bind(&b, KB, &[spec(10, 2)])
            .unwrap()
            .single_kind()
            .unwrap()
    );
    assert!(
        Reports::bind(&b, GP, &[spec(20, 4), spec(30, 5)])
            .unwrap()
            .single_kind()
            .unwrap()
    );
    assert!(
        Reports::bind(&b, &map(), &[spec(10, 2), spec(20, 4), spec(30, 5)])
            .unwrap()
            .single_kind()
            .is_err()
    );
}

#[test]
fn discovery_diagnostics_missing_service_and_malformed_do_not_claim_success() {
    let bond = verified();
    for p in [vec![1, 0x10, 1, 0, 0x0a], vec![], vec![1, 0x10]] {
        let mut d = Discovery::new();
        let mut slot: Option<musha_platform::bluetooth::hogp::Reports> = None;
        let before = d.diagnostic(&p);
        assert_eq!(before.stage, "services");
        assert_eq!(before.request, 0x10);
        assert!(d.receive_into(&p, &bond, &mut slot).is_err());
        assert_eq!(d.diagnostic(&[]).stage, "failed");
        // Saved metadata remains the rejecting stage, even after fail-closed state change.
        assert_eq!(before.stage, "services");
        assert!(!format!("{}", before).contains("status=00"));
    }
}

#[test]
fn discovery_map_limit_diagnostic_uses_actual_rejecting_executor() {
    let bond = verified();
    let mut d = Discovery::new();
    let mut slot: Option<musha_platform::bluetooth::hogp::Reports> = None;
    for p in [
        vec![0x11, 6, 1, 0, 32, 0, 0x12, 0x18],
        vec![
            9, 7, 2, 0, 2, 3, 0, 0x4b, 0x2a, 4, 0, 0x10, 5, 0, 0x4d, 0x2a,
        ],
        vec![1, 8, 6, 0, 0x0a],
    ] {
        d.receive_into(&p, &bond, &mut slot).unwrap();
    }
    let mut p = vec![0x0b; 23];
    d.receive_into(&p, &bond, &mut slot).unwrap();
    p[0] = 0x0d;
    for _ in 1..93 {
        d.receive_into(&p, &bond, &mut slot).unwrap();
    }
    let diagnostic = d.diagnostic(&p);
    assert_eq!(diagnostic.stage, "report-map");
    assert_eq!(diagnostic.map_bytes, 2046);
    assert_eq!(diagnostic.request_handle, 3);
    assert_eq!(
        diagnostic.reason(Error::Length),
        "map-length-or-response-format"
    );
    assert!(matches!(
        d.receive_into(&p, &bond, &mut slot),
        Err(Error::Length)
    ));
    assert_eq!(d.diagnostic(&[]).stage, "failed");
}
