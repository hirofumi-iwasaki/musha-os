// Copyright 2026 Hirofumi Iwasaki
// SPDX-License-Identifier: Apache-2.0
//! Owner-supplied layout, NOT a raw physical descriptor capture.
use musha_platform::bluetooth::{Error, usb};
fn interface(b: &mut Vec<u8>, number: u8, alt: u8, count: u8) {
    b.extend_from_slice(&[9, 4, number, alt, count, 0xe0, 1, 1, 0]);
}
fn endpoint(b: &mut Vec<u8>, address: u8, kind: u8, size: u16) {
    b.extend_from_slice(&[7, 5, address, kind, size as u8, (size >> 8) as u8, 1]);
}
fn header(b: &mut [u8]) {
    let n = b.len() as u16;
    b[2..4].copy_from_slice(&n.to_le_bytes());
}
fn primary() -> Vec<u8> {
    let mut b = vec![9, 2, 0, 0, 2, 1, 0, 0x80, 50];
    interface(&mut b, 0, 0, 3);
    endpoint(&mut b, 0x81, 3, 16);
    endpoint(&mut b, 0x02, 2, 64);
    endpoint(&mut b, 0x82, 2, 64);
    b
}
fn owner_intel() -> Vec<u8> {
    let mut b = primary();
    for (alt, size) in [0, 9, 17, 25, 33, 49].into_iter().enumerate() {
        interface(&mut b, 1, alt as u8, 2);
        endpoint(&mut b, 0x03, 1, size);
        endpoint(&mut b, 0x83, 1, size);
    }
    header(&mut b);
    assert_eq!(b.len(), 177);
    b
}
#[test]
fn owner_intel_177_ignores_later_sco_alternates() {
    let b = owner_intel();
    let t = usb::select(&b).unwrap();
    assert_eq!(t.interface, 0);
    assert_eq!((t.event.address, t.event.max_packet), (0x81, 16));
    assert_eq!((t.acl_out.address, t.acl_in.address), (2, 0x82));
}
#[test]
fn csr_zero_endpoint_sco_alt_zero_remains_supported() {
    let mut b = primary();
    interface(&mut b, 1, 0, 0);
    interface(&mut b, 1, 1, 2);
    endpoint(&mut b, 3, 1, 9);
    endpoint(&mut b, 0x83, 1, 9);
    header(&mut b);
    assert_eq!(usb::select(&b).unwrap().interface, 0);
}
#[test]
fn incomplete_primary_is_not_rescued_by_later_interface() {
    let mut b = owner_intel();
    b.drain(32..39);
    header(&mut b);
    assert_eq!(usb::select(&b), Err(Error::Invalid));
}
#[test]
fn isochronous_before_primary_completion_is_rejected() {
    let mut b = primary();
    b[21] = 1;
    header(&mut b);
    assert_eq!(usb::select(&b), Err(Error::Unsupported));
}
#[test]
fn malformed_later_descriptor_is_still_rejected() {
    let mut b = owner_intel();
    b[39] = 0;
    assert_eq!(usb::select(&b), Err(Error::Length));
}

#[test]
fn owner_177_builds_same_event_context_and_keeps_three_live_endpoints() {
    let t = usb::select(&owner_intel()).unwrap();
    let live = musha_platform::bluetooth::usb_context::plan(t, 3, [0x1000, 0x2000, 0x3000], false)
        .unwrap();
    let probe = musha_platform::bluetooth::usb_context::plan(t, 3, [0x1000, 0, 0], true).unwrap();
    assert_eq!(live.add_flags, 0x39);
    assert_eq!(live.context_entries, 5);
    assert_eq!(live.count, 3);
    assert_eq!(probe.add_flags, 9);
    assert_eq!(probe.context_entries, 3);
    assert_eq!(probe.count, 1);
    assert_eq!(live.endpoints[0], probe.endpoints[0]);
    for (i, dci, ty, packet) in [(0, 3, 7, 16), (1, 5, 6, 64), (2, 4, 2, 64)] {
        let e = live.endpoints[i];
        assert_eq!(e.dci, dci);
        assert_eq!((e.words[1] >> 3) & 7, ty);
        assert_eq!(e.words[1] >> 16, packet);
        assert_eq!(e.words[1] & 6, 6);
    }
    assert_eq!(
        live.endpoints[0].words,
        [
            3 << 16,
            (16 << 16) | (7 << 3) | 6,
            0x1001,
            0,
            257 | (257 << 16)
        ]
    );
    assert_eq!(
        t.command_setup(&[3, 12, 0], false).unwrap(),
        [0x20, 0, 0, 0, 0, 0, 3, 0]
    );
}
#[test]
fn csr_context_and_owner_sco_endpoints_never_publish_isochronous_contexts() {
    let mut b = primary();
    interface(&mut b, 1, 0, 0);
    interface(&mut b, 1, 1, 2);
    endpoint(&mut b, 3, 1, 9);
    endpoint(&mut b, 0x83, 1, 9);
    header(&mut b);
    let t = usb::select(&b).unwrap();
    let p = musha_platform::bluetooth::usb_context::plan(t, 3, [0x1000, 0x2000, 0x3000], false)
        .unwrap();
    assert_eq!(p.add_flags, 0x39);
    assert!(
        p.endpoints
            .iter()
            .all(|e| !matches!((e.words[1] >> 3) & 7, 1 | 5))
    );
    assert!(musha_platform::bluetooth::usb_context::plan(t, 3, [0x1001, 0, 0], false).is_err());
}
