// SPDX-License-Identifier: Apache-2.0
//! Trace configuration and retained-record contracts, independent of the screen.
use musha_platform::{
    boot_log::{Level, Ring},
    config::{self, LogLevel, ParseError},
};
#[test]
fn all_five_levels_parse_and_invalid_trace_does_not_return_partial_configuration() {
    for (text, expected) in [
        (b"log=quiet".as_slice(), LogLevel::Quiet),
        (b"log=ready", LogLevel::Ready),
        (b"log=phases", LogLevel::Phases),
        (b"log=devices", LogLevel::Devices),
        (b"log=trace", LogLevel::Trace),
    ] {
        assert_eq!(config::parse(text).unwrap().log, expected);
    }
    assert_eq!(
        config::parse(b"log=quiet\nlog=trace\n").unwrap().log,
        LogLevel::Trace
    );
    for text in [
        b"log=Trace".as_slice(),
        b"log=trace-more",
        b"log=trace\ndisplay=0x1",
    ] {
        assert_eq!(config::parse(text), Err(ParseError::Value));
    }
}
#[test]
fn f8_and_digit8_are_the_only_override_keys() {
    assert!(config::force_devices(0x12, 0));
    assert!(config::force_devices(0, u16::from(b'8')));
    for (scan, character) in [
        (0, 0),
        (0x11, 0),
        (0x13, 0),
        (0, u16::from(b'7')),
        (0, u16::from(b'9')),
    ] {
        assert!(!config::force_devices(scan, character));
    }
}
#[test]
fn ring_roundtrips_trace_without_reclassifying_application_or_changing_names() {
    assert_eq!(Level::Application as u8, 4);
    assert_eq!(Level::Trace as u8, 5);
    let records: [(Level, &[u8]); 6] = [
        (Level::Error, b"ERROR: storage: README.TXT missing\n"),
        (Level::Ready, b"Musha-OS 0.4.0 ready\n"),
        (Level::Phases, b"Musha-OS 0.4.0 starting\n"),
        (Level::Devices, b"storage: SanDisk Ultra README.TXT\n"),
        (Level::Trace, b"usb: TRB 0x1234 AUTORUN.ELF\n"),
        (Level::Application, b"Hello Musha-OS!\n"),
    ];
    let mut ring = Ring::new();
    for (index, (level, bytes)) in records.iter().enumerate() {
        assert!(ring.push(*level, index as u64 * 17, index < 2, bytes));
    }
    assert_eq!(ring.records().count(), records.len());
    for (index, (record, (level, bytes))) in ring.records().zip(records).enumerate() {
        assert_eq!(record.level(), level);
        assert_eq!(record.milliseconds(), index as u64 * 17);
        assert_eq!(record.early(), index < 2);
        assert!(record.bytes().eq(bytes.iter().copied()));
    }
    assert_eq!(ring.lost(), 0);
}
