// Copyright 2026 Hirofumi Iwasaki
// SPDX-License-Identifier: Apache-2.0
#![cfg(feature = "bluetooth-legacy")]
use musha_input::gamepad::Change;
use musha_platform::bluetooth::input_diagnostic::Diagnostic;
#[test]
fn changes_are_bounded_and_short_taps_and_trailing_release_are_kept() {
    let mut d = Diagnostic::new();
    assert!(d.poll(0).is_none());
    d.accepted(1, 16);
    let first = d.poll(0).unwrap();
    assert_eq!(first.reports, 1);
    assert_eq!(first.axes[9], -1);
    d.change(Change::Button(0, true), true);
    d.change(Change::Button(0, false), true);
    assert!(d.poll(249).is_none());
    let s = d.poll(250).unwrap();
    assert_eq!((s.buttons, s.pressed, s.released), (0, 1, 1));
    assert!(d.poll(500).is_none());
    d.change(Change::Button(31, true), true);
    assert_eq!(d.poll(500).unwrap().buttons, 0x80000000);
    d.change(Change::Button(31, false), true);
    let s = d.poll(750).unwrap();
    assert_eq!((s.buttons, s.released), (0, 0x80000000));
    for t in 751..1750 {
        d.change(Change::Axis(0, t as i32), true);
        assert_eq!(d.poll(t).is_some(), t % 250 == 0);
    }
}
#[test]
fn queue_full_metadata_labels_and_reset_are_explicit() {
    let mut d = Diagnostic::new();
    d.accepted(7, 32);
    d.change(Change::Axis(10, 1023), false);
    d.change(Change::Axis(11, 3), true);
    d.change(Change::Axis(9, -1), true);
    let s = d.poll(0).unwrap();
    assert_eq!((s.dropped, s.changes, s.id, s.bytes), (1, 3, 7, 32));
    assert_eq!(s.seen, (1 << 9) | (1 << 10) | (1 << 11));
    let mut lines = String::new();
    for n in 0..3 {
        s.line(n, &mut lines).unwrap();
        lines.push('\n');
    }
    assert!(lines.contains("queue-full=1"));
    assert!(lines.contains("Brake=1023 Accelerator=3"));
    assert!(!lines.contains("LT="));
    for _ in 0..1000 {
        d.accepted(7, 32);
    }
    assert!(d.poll(1000).is_none());
    d = Diagnostic::new();
    assert!(d.poll(2000).is_none());
}

#[test]
fn all_formatted_values_fit_the_viewer_retained_line_budget() {
    use musha_platform::bluetooth::input_diagnostic::Snapshot;
    let s = Snapshot {
        buttons: u32::MAX,
        pressed: u32::MAX,
        released: u32::MAX,
        axes: [i32::MIN; 12],
        seen: 0xfff,
        reports: u32::MAX,
        changes: u32::MAX,
        dropped: u32::MAX,
        id: u8::MAX,
        bytes: u16::MAX,
    };
    for n in 0..3 {
        let mut line = String::new();
        s.line(n, &mut line).unwrap();
        assert!(line.len() <= 136, "{line}");
    }
}
