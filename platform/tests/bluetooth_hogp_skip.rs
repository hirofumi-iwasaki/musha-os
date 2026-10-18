// Copyright 2026 Hirofumi Iwasaki
// SPDX-License-Identifier: Apache-2.0
#![cfg(feature = "bluetooth-security")]
//! Discovery while the peripheral already notifies (IMG_7694/IMG_7739 shape:
//! a 19-byte Handle Value Notification answering nothing). Synthetic bytes.
use musha_platform::{
    bluetooth::{
        Error, Event,
        att::peer_request_reply,
        hogp::{Discovery, DiscoveryAction, MAX_SKIPPED},
        reconnect::Store,
    },
    pairing::{self, Bond},
};
fn verified() -> musha_platform::bluetooth::legacy::VerifiedBond {
    let bond = Bond {
        address_type: 0,
        address: [2; 6],
        key_size: 16,
        flags: 3,
        ediv: 0,
        random: [0; 8],
        ltk: [7; 16],
        irk: [0; 16],
        own_address_type: 0,
        own_address: [0; 6],
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
fn notification() -> Vec<u8> {
    let mut p = vec![0x1b, 0x1e, 0];
    p.extend([0x80; 16]);
    p
}
const GP: &[u8] = &[
    5, 1, 9, 5, 0xa1, 1, 0x85, 4, 5, 9, 0x19, 1, 0x29, 4, 0x15, 0, 0x25, 1, 0x75, 1, 0x95, 4, 0x81,
    2, 0x75, 4, 0x95, 1, 0x81, 3, 0xc0,
];
/// One synthetic GATT server: HID service 1..16, Report Map at 3, one input
/// report at 5 (CCCD 6, reference 7).
fn respond(p: &[u8]) -> Vec<u8> {
    let h = u16::from_le_bytes([p[1], p[2]]);
    match p[0] {
        0x10 => vec![0x11, 6, 1, 0, 16, 0, 0x12, 0x18],
        8 if h == 1 => vec![
            9, 7, 2, 0, 2, 3, 0, 0x4b, 0x2a, 4, 0, 0x10, 5, 0, 0x4d, 0x2a,
        ],
        8 => vec![1, 8, p[1], p[2], 0x0a],
        0x0a if h == 3 => [&[0x0b][..], &GP[..22]].concat(),
        0x0c => {
            let at = u16::from_le_bytes([p[3], p[4]]) as usize;
            [&[0x0d][..], &GP[at..GP.len().min(at + 22)]].concat()
        }
        0x0a => vec![0x0b, 4, 1],
        4 if h == 6 => vec![5, 1, 6, 0, 2, 0x29, 7, 0, 8, 0x29],
        4 => vec![1, 4, p[1], p[2], 0x0a],
        _ => panic!("unexpected request {:02x}", p[0]),
    }
}
#[test]
fn notifications_before_the_service_response_are_skipped() {
    let bond = verified();
    for extra in 1..=3 {
        let mut d = Discovery::new();
        let mut slot: Option<musha_platform::bluetooth::hogp::Reports> = None;
        let DiscoveryAction::Send(request, _) = d.request() else {
            unreachable!()
        };
        assert_eq!(request[0], 0x10);
        for _ in 0..extra {
            assert!(matches!(
                d.receive_into(&notification(), &bond, &mut slot),
                Ok(DiscoveryAction::Ignore)
            ));
        }
        assert_eq!(d.skipped(), extra);
        // Skipped PDUs are not discovery pages and do not change the stage.
        assert_eq!(d.diagnostic(&[]).pages, 0);
        assert_eq!(d.stage_name(), "services");
        let Ok(DiscoveryAction::Send(next, _)) =
            d.receive_into(&[0x11, 6, 1, 0, 16, 0, 0x12, 0x18], &bond, &mut slot)
        else {
            panic!("service response not accepted")
        };
        assert_eq!(next[0], 0x08);
        assert_eq!(d.stage_name(), "characteristics");
    }
}
#[test]
fn indication_is_confirmed_and_error_response_is_still_handled() {
    let bond = verified();
    let mut d = Discovery::new();
    let mut slot: Option<musha_platform::bluetooth::hogp::Reports> = None;
    assert!(matches!(
        d.receive_into(&[0x1d, 0x1e, 0, 1], &bond, &mut slot),
        Ok(DiscoveryAction::Confirm)
    ));
    // A real Error Response for the awaited request keeps its meaning.
    assert_eq!(
        d.receive_into(&[1, 0x10, 1, 0, 0x0a], &bond, &mut slot)
            .err(),
        Some(Error::Unsupported)
    );
}
#[test]
fn skipping_is_bounded() {
    let bond = verified();
    let mut d = Discovery::new();
    let mut slot: Option<musha_platform::bluetooth::hogp::Reports> = None;
    for _ in 0..MAX_SKIPPED {
        assert!(d.receive_into(&notification(), &bond, &mut slot).is_ok());
    }
    assert_eq!(
        d.receive_into(&notification(), &bond, &mut slot).err(),
        Some(Error::Length)
    );
    assert_eq!(d.stage_name(), "failed");
}
#[test]
fn full_discovery_completes_with_notifications_between_every_response() {
    let bond = verified();
    let mut d = Discovery::new();
    let mut slot: Option<musha_platform::bluetooth::hogp::Reports> = None;
    let mut action = d.request();
    for _ in 0..40 {
        match action {
            DiscoveryAction::Send(p, n) => {
                assert!(matches!(
                    d.receive_into(&notification(), &bond, &mut slot),
                    Ok(DiscoveryAction::Ignore)
                ));
                action = d.receive_into(&respond(&p[..n]), &bond, &mut slot).unwrap();
            }
            DiscoveryAction::Ready => {
                let reports = slot.take().unwrap();
                assert_eq!(reports.count(), 1);
                assert_eq!(reports.cccd(0), Some(6));
                assert!(d.skipped() >= 5);
                return;
            }
            _ => panic!("skip action escaped the loop"),
        }
    }
    panic!("discovery did not finish");
}
#[test]
fn peer_requests_are_answered_and_other_pdus_are_not() {
    assert_eq!(
        peer_request_reply(&[0x02, 0xf7, 0]),
        Some(([0x03, 23, 0, 0, 0], 3))
    );
    assert_eq!(
        peer_request_reply(&[0x08, 1, 0, 0xff, 0xff, 0x00, 0x2a]),
        Some(([0x01, 0x08, 1, 0, 0x0a], 5))
    );
    assert_eq!(
        peer_request_reply(&[0x10, 1, 0, 0xff, 0xff, 0x00, 0x28]),
        Some(([0x01, 0x10, 1, 0, 0x0a], 5))
    );
    assert_eq!(
        peer_request_reply(&[0x12, 0x2a, 0, 1, 0]),
        Some(([0x01, 0x12, 0x2a, 0, 0x06], 5))
    );
    assert_eq!(
        peer_request_reply(&[0x0a, 3, 0]),
        Some(([0x01, 0x0a, 3, 0, 0x06], 5))
    );
    for p in [
        &notification()[..],
        &[0x1d, 1, 0][..],
        &[0x52, 1, 0, 1][..],
        &[0x13][..],
        &[][..],
    ] {
        assert_eq!(peer_request_reply(p), None);
    }
}
