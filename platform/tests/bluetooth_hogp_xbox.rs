// Copyright 2026 Hirofumi Iwasaki
// SPDX-License-Identifier: Apache-2.0
#![cfg(feature = "bluetooth-security")]
//! Independently authored Xbox-shaped HOGP server: one gamepad application
//! collection with the input report (ID 1, 16 bytes), AC Home (ID 2), rumble
//! output (ID 3) and battery (ID 4). Not a copied or measured descriptor.
use musha_input::gamepad::Change as Pad;
use musha_platform::{
    bluetooth::{
        Event,
        hogp::{Change, Discovery, DiscoveryAction},
        reconnect::Store,
    },
    pairing::{self, Bond},
};
fn verified() -> musha_platform::bluetooth::legacy::VerifiedBond {
    let bond = Bond {
        address_type: 0,
        address: [2; 6],
        key_size: 16,
        flags: 2,
        ediv: 0,
        random: [0; 8],
        ltk: [7; 16],
        irk: [0; 16],
    };
    let mut r = [0; 4096];
    pairing::encode_state(&[bond], None, &mut r).unwrap();
    let mut store = Store::new();
    store.load(&r).unwrap();
    let mut p = store.begin(9, 5, 0, 0, [2; 6]).unwrap();
    p.command_key(9, 1).unwrap();
    p.encryption(9, 2, Event::parse(&[8, 4, 0, 5, 0, 1]).unwrap())
        .unwrap()
}
fn map() -> Vec<u8> {
    let mut m = vec![5, 1, 9, 5, 0xa1, 1, 0x85, 1];
    for (a, b) in [(0x30, 0x31), (0x32, 0x35)] {
        m.extend([
            9, 1, 0xa1, 0, 9, a, 9, b, 0x15, 0, 0x27, 255, 255, 0, 0, 0x95, 2, 0x75, 16, 0x81, 2,
            0xc0,
        ]);
    }
    for u in [0xc5, 0xc4] {
        m.extend([
            5, 2, 9, u, 0x15, 0, 0x26, 255, 3, 0x95, 1, 0x75, 10, 0x81, 2, 0x15, 0, 0x25, 0, 0x75,
            6, 0x95, 1, 0x81, 3,
        ]);
    }
    m.extend([
        5, 1, 9, 0x39, 0x15, 1, 0x25, 8, 0x35, 0, 0x46, 0x3b, 1, 0x66, 0x14, 0, 0x75, 4, 0x95, 1,
        0x81, 0x42, 0x75, 4, 0x95, 1, 0x15, 0, 0x25, 0, 0x35, 0, 0x45, 0, 0x65, 0, 0x81, 3,
    ]);
    m.extend([
        5, 9, 0x19, 1, 0x29, 15, 0x15, 0, 0x25, 1, 0x75, 1, 0x95, 15, 0x81, 2, 0x15, 0, 0x25, 0,
        0x75, 1, 0x95, 1, 0x81, 3,
    ]);
    m.extend([
        5, 0x0c, 0x0a, 0xb2, 0, 0x15, 0, 0x25, 1, 0x95, 1, 0x75, 1, 0x81, 2, 0x15, 0, 0x25, 0,
        0x75, 7, 0x95, 1, 0x81, 3,
    ]);
    // ID 2: AC Home, one bit.
    m.extend([
        0x85, 2, 5, 0x0c, 0x0a, 0x23, 2, 0x15, 0, 0x25, 1, 0x95, 1, 0x75, 1, 0x81, 2, 0x15, 0,
        0x25, 0, 0x75, 7, 0x95, 1, 0x81, 3,
    ]);
    // ID 3: rumble output in a logical collection.
    m.extend([
        5, 0x0f, 9, 0x21, 0x85, 3, 0xa1, 2, 9, 0x97, 0x15, 0, 0x25, 1, 0x75, 4, 0x95, 1, 0x91, 2,
        0x15, 0, 0x25, 0, 0x75, 4, 0x95, 1, 0x91, 3, 9, 0x70, 0x15, 0, 0x25, 0x64, 0x75, 8, 0x95,
        4, 0x91, 2, 9, 0x50, 0x66, 1, 0x10, 0x55, 0x0e, 0x15, 0, 0x26, 255, 0, 0x75, 8, 0x95, 1,
        0x91, 2, 9, 0xa7, 0x15, 0, 0x26, 255, 0, 0x75, 8, 0x95, 1, 0x91, 2, 0x65, 0, 0x55, 0, 9,
        0x7c, 0x15, 0, 0x26, 255, 0, 0x75, 8, 0x95, 1, 0x91, 2, 0xc0,
    ]);
    // ID 4: battery strength.
    m.extend([
        0x85, 4, 5, 6, 9, 0x20, 0x15, 0, 0x26, 255, 0, 0x75, 8, 0x95, 1, 0x81, 2, 0xc0,
    ]);
    m
}
/// (declaration, properties, value, uuid): Report Map, three notifying input
/// reports and one write-only output report. Service 1..20.
const CHARS: [(u16, u8, u16, u16); 5] = [
    (2, 0x02, 3, 0x2a4b),
    (4, 0x12, 5, 0x2a4d),
    (8, 0x12, 9, 0x2a4d),
    (12, 0x0e, 13, 0x2a4d),
    (14, 0x12, 15, 0x2a4d),
];
fn respond(p: &[u8], map: &[u8], requests: &mut Vec<u8>) -> Vec<u8> {
    requests.push(p[0]);
    let h = u16::from_le_bytes([p[1], p[2]]);
    match p[0] {
        0x10 if h == 1 => vec![0x11, 6, 1, 0, 20, 0, 0x12, 0x18],
        0x10 => vec![1, 0x10, p[1], p[2], 0x0a],
        8 => {
            // At most three 7-byte entries fit an MTU-23 response.
            let mut r = vec![9, 7];
            for &(d, props, v, uuid) in CHARS.iter().filter(|c| c.0 >= h).take(3) {
                r.extend(d.to_le_bytes());
                r.push(props);
                r.extend(v.to_le_bytes());
                r.extend(uuid.to_le_bytes());
            }
            if r.len() == 2 {
                vec![1, 8, p[1], p[2], 0x0a]
            } else {
                r
            }
        }
        0x0a if h == 3 => [&[0x0b][..], &map[..22]].concat(),
        0x0c => {
            assert_eq!(u16::from_le_bytes([p[1], p[2]]), 3);
            let at = u16::from_le_bytes([p[3], p[4]]) as usize;
            if at >= map.len() {
                return vec![1, 0x0c, 3, 0, 7]; // Invalid Offset
            }
            [&[0x0d][..], &map[at..map.len().min(at + 22)]].concat()
        }
        // Each notifying report: CCCD at value+1, reference at value+2.
        4 => {
            let end = u16::from_le_bytes([p[3], p[4]]);
            if h + 1 > end + 1 || ![6, 10, 16].contains(&h) {
                return vec![1, 4, p[1], p[2], 0x0a];
            }
            let mut r = vec![5, 1];
            r.extend(h.to_le_bytes());
            r.extend([2, 0x29]);
            r.extend((h + 1).to_le_bytes());
            r.extend([8, 0x29]);
            r
        }
        0x0a => match h {
            7 => vec![0x0b, 1, 1],
            11 => vec![0x0b, 2, 1],
            17 => vec![0x0b, 4, 1],
            _ => panic!("reference {h}"),
        },
        _ => panic!("unexpected request {:02x}", p[0]),
    }
}
#[test]
fn xbox_shaped_map_is_read_by_blob_and_binds_only_the_gamepad_report() {
    let map = map();
    assert!((280..=340).contains(&map.len()), "{}", map.len());
    let bond = verified();
    let mut d = Discovery::new();
    let mut slot: Option<musha_platform::bluetooth::hogp::Reports> = None;
    let mut action = d.request();
    let mut requests = vec![];
    let mut reports = None;
    for _ in 0..80 {
        match action {
            DiscoveryAction::Send(p, n) => {
                // A notification arrives before every response.
                let mut early = vec![0x1b, 5, 0];
                early.extend([0; 16]);
                assert!(matches!(
                    d.receive_into(&early, &bond, &mut slot),
                    Ok(DiscoveryAction::Ignore)
                ));
                action = d
                    .receive_into(&respond(&p[..n], &map, &mut requests), &bond, &mut slot)
                    .unwrap();
            }
            DiscoveryAction::Ready => {
                reports = slot.take();
                break;
            }
            _ => panic!("skip action escaped"),
        }
    }
    let mut reports = reports.expect("discovery did not finish");
    // Blob reads cover the whole map: one Read plus ceil((len-22)/22) Read Blob.
    let blobs = requests.iter().filter(|&&r| r == 0x0c).count();
    assert!(blobs >= map.len().div_ceil(22) - 1);
    // Only report 1 is bound and subscribed; AC Home and battery are not.
    assert_eq!(reports.count(), 1);
    assert_eq!(reports.unmapped(), 2);
    assert_eq!(reports.cccd(0), Some(6));
    assert_eq!(reports.cccd(1), None);
    assert!(reports.single_kind().unwrap()); // gamepad
    // A 19-byte notification on report 1 decodes; battery notifications do not error.
    let mut p = vec![0x1b, 5, 0];
    p.extend([0; 16]);
    p[3 + 13] = 1; // first button: sticks 8, triggers 4, hat 1 byte precede it
    let mut seen = vec![];
    reports.notification(9, 5, &p, |c| seen.push(c)).unwrap();
    assert!(seen.contains(&Change::Gamepad(Pad::Button(0, true))));
    seen.clear();
    reports
        .notification(9, 5, &[0x1b, 15, 0, 80], |c| seen.push(c))
        .unwrap();
    assert!(seen.is_empty());
    assert!(
        reports
            .notification(9, 5, &[0x1b, 21, 0, 1], |_| {})
            .is_err()
    );
}
#[test]
fn unknown_fields_inside_the_gamepad_report_are_still_refused() {
    let mut m = map();
    // Turn AC Home (0x0223) into an unknown consumer usage.
    let at = m.windows(3).position(|w| w == [0x0a, 0x23, 2]).unwrap();
    m[at + 1] = 0x24;
    assert!(musha_input::gamepad::Layout::parse(&m).is_err());
}

/// The IMG_7740 request sequence: HID service from 0x0012, characteristic
/// discovery at 0x0016/0x001d then Attribute Not Found at 0x0023, Report Map at
/// 0x001c (283 bytes), descriptors at 0x001f (one 10-byte response) and the
/// Report Reference read at 0x0020. Handles mirror the log; bytes are synthetic.
fn observed_map() -> Vec<u8> {
    let mut m = map();
    // Drop AC Home, one rumble field and the hat's Physical Maximum to 283 bytes.
    let home = m.windows(2).position(|w| w == [0x85, 2]).unwrap();
    m.drain(home..home + 27);
    let rumble = m.windows(2).position(|w| w == [9, 0x7c]).unwrap();
    m.drain(rumble..rumble + 13);
    let physical = m.windows(3).position(|w| w == [0x46, 0x3b, 1]).unwrap();
    m.drain(physical..physical + 3);
    // Same unit, one-byte form.
    let unit = m.windows(3).position(|w| w == [0x66, 0x14, 0]).unwrap();
    m.splice(unit..unit + 3, [0x65, 0x14]);
    m
}
fn respond_observed(p: &[u8], map: &[u8], log: &mut Vec<(u8, u16)>) -> Vec<u8> {
    let h = u16::from_le_bytes([p[1], p[2]]);
    log.push((p[0], h));
    let chars = |entries: &[(u16, u8, u16, u16)]| {
        let mut r = vec![9, 7];
        for &(d, props, v, uuid) in entries {
            r.extend(d.to_le_bytes());
            r.push(props);
            r.extend(v.to_le_bytes());
            r.extend(uuid.to_le_bytes());
        }
        r
    };
    match (p[0], h) {
        (0x10, 1) => vec![
            0x11, 6, 1, 0, 0x11, 0, 0x01, 0x18, 0x12, 0, 0x26, 0, 0x12, 0x18,
        ],
        (0x08, 0x12) => chars(&[(0x14, 0x02, 0x15, 0x2a4a)]),
        (0x08, 0x16) => chars(&[
            (0x17, 0x04, 0x18, 0x2a4c),
            (0x19, 0x06, 0x1a, 0x2a4e),
            (0x1b, 0x02, 0x1c, 0x2a4b),
        ]),
        (0x08, 0x1d) => chars(&[(0x1d, 0x12, 0x1e, 0x2a4d), (0x21, 0x0e, 0x22, 0x2a4d)]),
        (0x08, 0x23) => vec![1, 0x08, 0x23, 0, 0x0a],
        (0x0a, 0x1c) => [&[0x0b][..], &map[..22]].concat(),
        (0x0c, 0x1c) => {
            let at = u16::from_le_bytes([p[3], p[4]]) as usize;
            [&[0x0d][..], &map[at..map.len().min(at + 22)]].concat()
        }
        (0x04, 0x1f) => vec![5, 1, 0x1f, 0, 2, 0x29, 0x20, 0, 8, 0x29],
        (0x0a, 0x20) => vec![0x0b, 1, 1],
        _ => panic!("unexpected request {:02x} handle {h:04x}", p[0]),
    }
}
#[test]
fn observed_xbox_sequence_reaches_the_report_reference_and_binds() {
    let map = observed_map();
    assert_eq!(map.len(), 283);
    let bond = verified();
    let mut d = Discovery::new();
    let mut slot: Option<musha_platform::bluetooth::hogp::Reports> = None;
    let mut action = d.request();
    let mut log = vec![];
    for _ in 0..60 {
        match action {
            DiscoveryAction::Send(p, n) => {
                action = d
                    .receive_into(&respond_observed(&p[..n], &map, &mut log), &bond, &mut slot)
                    .unwrap();
            }
            DiscoveryAction::Ready => break,
            _ => panic!("skip action escaped"),
        }
    }
    assert!(matches!(action, DiscoveryAction::Ready));
    let chars: Vec<u16> = log.iter().filter(|r| r.0 == 0x08).map(|r| r.1).collect();
    assert_eq!(chars, [0x12, 0x16, 0x1d, 0x23]);
    assert_eq!(log.iter().filter(|r| r.0 == 0x0c).count(), 12);
    assert_eq!(&log[log.len() - 2..], &[(0x04, 0x1f), (0x0a, 0x20)]);
    let reports = slot.expect("reports bound in place");
    assert_eq!((reports.count(), reports.cccd(0)), (1, Some(0x1f)));
}

/// Desk check of a reboot reconnect: the saved record supplies LTK, IRK and
/// identity; a resolvable private advertiser is resolved on the host; the
/// connection is encrypted with the stored LTK (LE Start Encryption, no
/// Pairing Request); HOGP discovery then runs again and binds.
#[test]
fn saved_bond_reconnects_from_a_private_address_and_rebinds() {
    let mut irk: [u8; 16] = core::array::from_fn(|i| {
        u8::from_str_radix(&"ec0234a357c8ad05341010a60a397d9b"[i * 2..i * 2 + 2], 16).unwrap()
    });
    irk.reverse();
    let rpa = [0xaa, 0xfb, 0x0d, 0x94, 0x81, 0x70];
    let identity = [0x31, 0x32, 0x33, 0x34, 0x35, 0x36];
    let bond = Bond {
        address_type: 0,
        address: identity,
        key_size: 16,
        flags: 2,
        ediv: 0,
        random: [0; 8],
        ltk: [0x5c; 16],
        irk,
    };
    let mut record = [0; 4096];
    pairing::encode_state(&[bond], Some([4; 32]), &mut record).unwrap();
    let mut store = Store::new();
    store.load(&record).unwrap();
    for (kind, address, via_irk) in [(1, rpa, true), (0, identity, false)] {
        assert!(store.connectable(0, kind, &address));
        assert_eq!(store.resolved(kind, &address), via_irk);
        let mut pending = store.begin(11, 0x48, 0, kind, address).unwrap();
        let key = pending.command_key(11, 1).unwrap();
        key.use_with(|k| assert_eq!(*k, [0x5c; 16]));
        let verified = pending
            .encryption(11, 2, Event::parse(&[8, 4, 0, 0x48, 0, 1]).unwrap())
            .unwrap();
        assert_eq!(verified.bond().address, identity);
        let map = observed_map();
        let mut d = Discovery::new();
        let mut slot: Option<musha_platform::bluetooth::hogp::Reports> = None;
        let mut action = d.request();
        let mut log = vec![];
        for _ in 0..60 {
            match action {
                DiscoveryAction::Send(p, n) => {
                    action = d
                        .receive_into(
                            &respond_observed(&p[..n], &map, &mut log),
                            &verified,
                            &mut slot,
                        )
                        .unwrap();
                }
                DiscoveryAction::Ready => break,
                _ => panic!("skip action escaped"),
            }
        }
        assert!(matches!(action, DiscoveryAction::Ready));
        assert_eq!(slot.unwrap().count(), 1);
    }
}
