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

#[test]
fn lan_echo_is_default_off_and_only_explicit_on_enables_it() {
    assert!(!config::Config::default().lan_echo);
    for text in [
        b"".as_slice(),
        b"# defaults\n",
        b"log=trace\npair=on\ndisplay=1024x768\n",
        b"lan.echo=off\n",
    ] {
        assert!(!config::parse(text).unwrap().lan_echo);
    }
    assert!(config::parse(b"lan.echo=on\n").unwrap().lan_echo);
    assert!(
        config::parse(b" lan.echo = on # opt in\r\n")
            .unwrap()
            .lan_echo
    );
}
#[test]
fn lan_echo_duplicate_keys_follow_existing_last_key_wins_rule() {
    assert!(
        !config::parse(b"lan.echo=on\nlan.echo=off\n")
            .unwrap()
            .lan_echo
    );
    assert!(
        config::parse(b"lan.echo=off\nlan.echo=on\n")
            .unwrap()
            .lan_echo
    );
}
#[test]
fn invalid_lan_echo_rejects_the_entire_configuration_instead_of_partial_enablement() {
    for value in [
        "", "ON", "Off", "true", "false", "1", "0", "yes", "on=off", "on off",
    ] {
        let bytes = format!("log=quiet\nlan.echo=on\npair=on\nlan.echo={value}\n");
        assert_eq!(
            config::parse(bytes.as_bytes()),
            Err(ParseError::Value),
            "{value}"
        );
        // Mirrors the firmware caller's existing fallback, without file writes.
        assert!(!config::parse(bytes.as_bytes()).unwrap_or_default().lan_echo);
    }
    assert_eq!(
        config::parse(b"lan.echo=on\nbad\n"),
        Err(ParseError::Syntax)
    );
    assert_eq!(
        config::parse(b"lan.echo=on\n#\xff"),
        Err(ParseError::NonAscii)
    );
}
#[test]
fn echo_setting_preserves_display_trace_pair_unknown_keys_and_size_limits() {
    use musha_platform::config::{Config, Display, MAX_CONFIG_BYTES};
    assert_eq!(
        config::parse(
            b"future=value\r\nlan.echo=on\r\nlog=trace\r\npair=on\r\ndisplay=1600x1200\r\n"
        ),
        Ok(Config {
            log: LogLevel::Trace,
            display: Display::Exact(1600, 1200),
            pair: true,
            lan_echo: true
        })
    );
    assert_eq!(
        config::parse(b"lan.echo=on\npair=off"),
        Err(ParseError::Value)
    );
    let mut exact = b"lan.echo=on\n#".to_vec();
    exact.resize(MAX_CONFIG_BYTES, b'x');
    assert!(config::parse(&exact).unwrap().lan_echo);
    exact.push(b'x');
    assert_eq!(config::parse(&exact), Err(ParseError::TooLarge));
    assert!(!config::parse(&exact).unwrap_or_default().lan_echo);
}
