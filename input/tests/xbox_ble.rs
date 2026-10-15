// Copyright 2026 Hirofumi Iwasaki
// SPDX-License-Identifier: Apache-2.0
//! Independently constructed descriptor fixture for published Xbox BLE field
//! geometry. Neither xpadneo code nor its descriptor dump is copied.
use musha_input::gamepad::{Change, Decoder, Layout};
fn map(buttons: u8, consumer: u8) -> Vec<u8> {
    let mut m = vec![5, 1, 9, 5, 0xa1, 1, 0x85, 1];
    // Four unsigned 16-bit stick axes, semantic usage order X,Y,Z,Rz.
    m.extend([
        9, 0x30, 9, 0x31, 9, 0x32, 9, 0x35, 0x15, 0, 0x27, 255, 255, 0, 0, 0x75, 16, 0x95, 4, 0x81,
        2,
    ]);
    for usage in [0xc5, 0xc4] {
        m.extend([
            5, 2, 9, usage, 0x26, 255, 3, 0x75, 10, 0x95, 1, 0x81, 2, 0x75, 6, 0x81, 3,
        ]);
    }
    m.extend([
        5, 1, 9, 0x39, 0x25, 8, 0x15, 1, 0x75, 4, 0x81, 0x42, 0x81, 3,
    ]);
    m.extend([
        5, 9, 0x19, 1, 0x29, buttons, 0x15, 0, 0x25, 1, 0x75, 1, 0x95, buttons, 0x81, 2,
    ]);
    if buttons % 8 != 0 {
        m.extend([0x75, 8 - buttons % 8, 0x95, 1, 0x81, 3]);
    }
    m.extend([
        5, 0x0c, 9, consumer, 0x75, 1, 0x95, 1, 0x81, 2, 0x75, 7, 0x81, 3, 0xc0,
    ]);
    m
}
#[test]
fn xbox_field_geometry_and_share_is_not_a_gamepad_button() {
    let l = Layout::parse(&map(15, 0xb2)).unwrap();
    assert_eq!(l.report_bytes(1), Some(17));
    let mut d = Decoder::new(l);
    let mut p = [0; 17];
    p[0] = 1;
    p[13] = 1;
    let mut changes = Vec::new();
    d.update(&p, |c| changes.push(c)).unwrap();
    assert!(changes.contains(&Change::Axis(0, 0)));
    assert!(changes.contains(&Change::Axis(10, 0)));
    changes.clear();
    p[16] = 1;
    d.update(&p, |c| changes.push(c)).unwrap();
    assert!(changes.is_empty());
    p[1..3].copy_from_slice(&65535u16.to_le_bytes());
    p[14] = 1;
    d.update(&p, |c| changes.push(c)).unwrap();
    assert!(changes.contains(&Change::Axis(0, 65535)));
    assert!(changes.contains(&Change::Button(0, true)));
}
#[test]
fn unknown_consumer_refused_and_button32_keeps_its_meaning() {
    assert!(Layout::parse(&map(15, 0xb3)).is_err());
    let mut d = Decoder::new(Layout::parse(&map(32, 0xb2)).unwrap());
    let mut p = [0; 19];
    p[0] = 1;
    p[17] = 0x80;
    let mut changes = Vec::new();
    d.update(&p, |c| changes.push(c)).unwrap();
    assert!(changes.contains(&Change::Button(31, true)));
    changes.clear();
    p[18] = 1;
    d.update(&p, |c| changes.push(c)).unwrap();
    assert!(changes.is_empty());
}
