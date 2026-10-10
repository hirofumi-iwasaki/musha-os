// Copyright 2026 Hirofumi Iwasaki
// SPDX-License-Identifier: Apache-2.0
use musha_input::{
    hid::{Decoder, Error, Layout},
    sources::Sources,
};
fn boot(id: Option<u8>) -> Vec<u8> {
    let mut d = vec![0x05, 1, 0x09, 6, 0xa1, 1];
    if let Some(id) = id {
        d.extend([0x85, id]);
    }
    d.extend([
        0x05, 7, 0x19, 0xe0, 0x29, 0xe7, 0x15, 0, 0x25, 1, 0x75, 1, 0x95, 8, 0x81, 2,
    ]);
    d.extend([0x75, 8, 0x95, 1, 0x81, 1]);
    // Output fields share globals, but must not consume input bits.
    d.extend([0x05, 8, 0x19, 1, 0x29, 5, 0x75, 1, 0x95, 5, 0x91, 2]);
    d.extend([
        0x05, 7, 0x19, 0, 0x29, 101, 0x15, 0, 0x25, 101, 0x75, 8, 0x95, 6, 0x81, 0, 0xc0,
    ]);
    d
}
fn bitmap(id: u8, first: u8, last: u8, padding: u8) -> Vec<u8> {
    let mut d = vec![
        0x05, 1, 0x09, 6, 0xa1, 1, 0x85, id, 0x05, 7, 0x15, 0, 0x25, 1, 0x75, 1,
    ];
    if padding != 0 {
        d.extend([0x95, padding, 0x81, 1]);
    }
    d.extend([
        0x19,
        first,
        0x29,
        last,
        0x95,
        last - first + 1,
        0x81,
        2,
        0xc0,
    ]);
    d
}
#[test]
fn array_modifiers_ids_and_output_offsets() {
    for id in [None, Some(7)] {
        let layout = Layout::parse(&boot(id)).unwrap();
        assert_eq!(
            layout.report_bytes(id.unwrap_or(0)),
            Some(8 + usize::from(id.is_some()))
        );
        let mut decoder = Decoder::new(layout);
        let mut events = Vec::new();
        let mut report = vec![2, 0, 4, 4, 5, 0, 0, 0];
        if let Some(id) = id {
            report.insert(0, id);
        }
        decoder.update(&report, |k, d| events.push((k, d))).unwrap();
        assert_eq!(events, [(0xe1, true), (4, true), (5, true)]);
        decoder
            .update(&report, |_, _| panic!("duplicate report"))
            .unwrap();
        events.clear();
        decoder.release_all(|k, d| events.push((k, d)));
        assert_eq!(events, [(0xe1, false), (4, false), (5, false)]);
        decoder.release_all(|_, _| panic!("duplicate release"));
    }
}
#[test]
fn unaligned_bitmap_and_independent_report_ids_share_held_keys() {
    let mut descriptor = bitmap(1, 4, 11, 3);
    descriptor.extend(bitmap(2, 4, 11, 0));
    let mut decoder = Decoder::new(Layout::parse(&descriptor).unwrap());
    let mut events = Vec::new();
    decoder
        .update(&[1, 0x08, 0x04], |k, d| events.push((k, d)))
        .unwrap();
    assert_eq!(events, [(4, true), (11, true)]);
    decoder
        .update(&[2, 1], |_, _| panic!("key already held in ID 1"))
        .unwrap();
    events.clear();
    decoder
        .update(&[1, 0, 0], |k, d| events.push((k, d)))
        .unwrap();
    assert_eq!(events, [(11, false)]);
    events.clear();
    decoder.update(&[2, 0], |k, d| events.push((k, d))).unwrap();
    assert_eq!(events, [(4, false)]);
}
#[test]
fn invalid_report_is_transactional() {
    let mut decoder = Decoder::new(Layout::parse(&boot(Some(7))).unwrap());
    decoder
        .update(&[7, 2, 0, 4, 0, 0, 0, 0, 0], |_, _| {})
        .unwrap();
    for (report, error) in [
        (vec![], Error::Length),
        (vec![7; 8], Error::Length),
        (vec![7; 10], Error::Length),
        (vec![8; 9], Error::ReportId),
        (vec![7, 0, 0, 1, 0, 0, 0, 0, 0], Error::Rollover),
        (vec![7, 0, 0, 200, 0, 0, 0, 0, 0], Error::Value),
    ] {
        assert_eq!(
            decoder.update(&report, |_, _| panic!("invalid report changed state")),
            Err(error)
        );
    }
    let mut events = Vec::new();
    decoder
        .update(&[7, 0, 0, 0, 0, 0, 0, 0, 0], |k, d| events.push((k, d)))
        .unwrap();
    assert_eq!(events, [(0xe1, false), (4, false)]);
}
#[test]
fn push_pop_and_explicit_usage_list_mapping() {
    let d = [
        0x05, 1, 0x09, 6, 0xa1, 1, 0x05, 7, 0x15, 1, 0x25, 2, 0x75, 2, 0x95, 1, 0xa4, 0x75, 8,
        0x81, 1, 0xb4, 0x09, 4, 0x09, 5, 0x81, 0, 0xc0,
    ];
    let layout = Layout::parse(&d).unwrap();
    assert_eq!(layout.report_bytes(0), Some(2));
    let mut decoder = Decoder::new(layout);
    let mut events = Vec::new();
    decoder
        .update(&[0xff, 2], |k, d| events.push((k, d)))
        .unwrap();
    assert_eq!(events, [(5, true)]);
}
#[test]
fn composite_consumer_report_does_not_release_keyboard() {
    let mut d = boot(Some(1));
    d.extend([
        0x05, 0x0c, 0x09, 1, 0xa1, 1, 0x85, 2, 0x09, 0xe9, 0x75, 1, 0x95, 1, 0x81, 2, 0xc0,
    ]);
    let mut decoder = Decoder::new(Layout::parse(&d).unwrap());
    decoder
        .update(&[1, 0, 0, 4, 0, 0, 0, 0, 0], |_, _| {})
        .unwrap();
    decoder
        .update(&[2, 1], |_, _| panic!("consumer page emitted"))
        .unwrap();
    let mut e = Vec::new();
    decoder.release_all(|k, d| e.push((k, d)));
    assert_eq!(e, [(4, false)]);
}
#[test]
fn malformed_descriptors_and_capacity_limits() {
    let d = boot(None);
    for end in 0..d.len() {
        assert!(Layout::parse(&d[..end]).is_err(), "truncation {end}");
    }
    for d in [
        vec![0xfe, 0, 0],
        vec![0xb4],
        vec![0xc0],
        vec![0x85, 0],
        vec![0x05],
        vec![0xa4; 5],
        vec![0; 4097],
    ] {
        assert!(Layout::parse(&d).is_err());
    }
    let mut mixed = boot(None);
    mixed.extend(bitmap(1, 4, 5, 0));
    assert!(Layout::parse(&mixed).is_err());
    let mut too_many = Vec::new();
    for id in 1..=9 {
        too_many.extend(bitmap(id, 4, 5, 0));
    }
    assert!(matches!(Layout::parse(&too_many), Err(Error::Limit)));
    let mut relative = bitmap(1, 4, 5, 0);
    let n = relative.len();
    relative[n - 2] = 6;
    assert!(matches!(Layout::parse(&relative), Err(Error::Unsupported)));
    let mut signed = boot(None);
    let p = signed.windows(2).position(|w| w == [0x15, 0]).unwrap();
    signed[p + 1] = 0xff;
    assert!(Layout::parse(&signed).is_err());
}
#[test]
fn deterministic_descriptor_mutations_are_bounded_and_never_panic() {
    let seed = boot(Some(1));
    for i in 0..seed.len() {
        for byte in [0, 1, 0x7f, 0x80, 0xff] {
            let mut d = seed.clone();
            d[i] = byte;
            if let Ok(layout) = Layout::parse(&d) {
                let len = layout.report_bytes(1).unwrap_or(1);
                let mut decoder = Decoder::new(layout);
                let mut report = vec![0; len];
                report[0] = 1;
                let _ = decoder.update(&report, |_, _| {});
            }
        }
    }
}
#[test]
fn descriptor_decoder_feeds_existing_source_selector() {
    let mut decoder = Decoder::new(Layout::parse(&boot(Some(3))).unwrap());
    let mut sources = Sources::<2>::default();
    let internal = sources.connect(1, |_, _| {}).unwrap();
    let mut e = Vec::new();
    decoder
        .update(&[3, 2, 0, 4, 0, 0, 0, 0, 0], |k, d| {
            sources.key(internal, k, d, |k, d| e.push((k, d))).unwrap()
        })
        .unwrap();
    assert_eq!(e, [(0xe1, true), (4, true)]);
    e.clear();
    sources.disconnect(internal, |k, d| e.push((k, d))).unwrap();
    assert_eq!(e, [(0xe1, false), (4, false)]);
}
