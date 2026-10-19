// Copyright 2026 Hirofumi Iwasaki
// SPDX-License-Identifier: Apache-2.0
use musha_platform::{
    bluetooth::adapter,
    config::{ParseError, parse},
};
#[test]
fn expectation_is_optional_strict_and_validates_without_selecting() {
    assert_eq!(parse(b"log=devices").unwrap().bt_expect, None);
    let e = parse(b"bt.expect=0A12:0001").unwrap().bt_expect;
    assert_eq!(e, Some((0x0a12, 1)));
    assert!(adapter::matches(e, (0x0a12, 1)));
    assert!(!adapter::matches(e, (0x8087, 0x0a2a)));
    assert!(adapter::matches(None, (0x8087, 0x0a2a)));
    for v in [
        "a12:0001",
        "0a12:1",
        "0a12-0001",
        "0g12:0001",
        "0a12:0001:0002",
        "",
    ] {
        assert_eq!(
            parse(format!("bt.expect={v}").as_bytes()),
            Err(ParseError::Value)
        );
    }
    assert_eq!(
        parse(
            b"bt.expect=0a12:0001
bt.expect=8087:0a2a"
        )
        .unwrap()
        .bt_expect,
        Some((0x8087, 0x0a2a))
    );
}

#[test]
fn validation_blocks_pairing_only_for_missing_or_mismatched_adapter_and_resets() {
    let mut v = adapter::Validation::new();
    assert!(!v.blocked());
    assert!(!v.selected());
    v.missing();
    assert!(v.blocked());
    v.observe(true);
    assert!(!v.blocked());
    assert!(v.selected());
    v.missing();
    assert!(!v.blocked()); // Unrelated host does not replace selected match.
    v.observe(false);
    assert!(v.blocked());
    v.missing();
    assert!(v.blocked());
    v = adapter::Validation::new();
    assert!(!v.blocked());
    assert!(!v.selected());
}
