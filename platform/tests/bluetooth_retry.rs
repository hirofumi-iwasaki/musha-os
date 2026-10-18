// Copyright 2026 Hirofumi Iwasaki
// SPDX-License-Identifier: Apache-2.0
//! The IMG_7743/7744 situation: the controller keeps an old bond, connects at
//! once and ignores the Pairing Request. Policy decisions only; no radio.
use musha_platform::bluetooth::retry::{DEFER_MS, Decision, MAX_ATTEMPTS, REPLY_MS, Retry};

const XBOX: [u8; 6] = [1, 2, 3, 4, 5, 6];
const OTHER: [u8; 6] = [9, 9, 9, 9, 9, 9];

#[test]
fn silent_peer_is_disconnected_after_five_seconds_and_deferred() {
    let mut r = Retry::new();
    assert_eq!(r.decide(0, false), Decision::Wait);
    assert_eq!(r.decide(REPLY_MS - 1, false), Decision::Wait);
    assert_eq!(r.decide(REPLY_MS, false), Decision::Disconnect);
    r.disconnected(XBOX, 10_000);
    // The same controller keeps reconnecting; it is skipped for 10 s while the
    // user enters pairing mode. Another candidate is still accepted.
    assert!(!r.admits(&XBOX, 10_000));
    assert!(!r.admits(&XBOX, 10_000 + DEFER_MS - 1));
    assert!(r.admits(&OTHER, 12_000));
    assert!(r.admits(&XBOX, 10_000 + DEFER_MS));
    assert!(r.admits(&XBOX, 10_000 + DEFER_MS + 1));
}

#[test]
fn security_request_gives_up_at_once_and_attempts_are_bounded() {
    let mut r = Retry::new();
    assert_eq!(r.decide(0, true), Decision::Disconnect);
    for attempt in 0..MAX_ATTEMPTS {
        assert_eq!(r.decide(REPLY_MS, false), Decision::Disconnect, "{attempt}");
        r.disconnected(XBOX, attempt as u64 * 20_000);
    }
    assert_eq!(r.attempts(), MAX_ATTEMPTS);
    // After three silent connections the SMP timeout stops Bluetooth.
    assert_eq!(r.decide(REPLY_MS, false), Decision::Exhausted);
    assert_eq!(r.decide(0, true), Decision::Exhausted);
    assert_eq!(r.decide(0, false), Decision::Wait);
}
