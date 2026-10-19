// Copyright 2026 Hirofumi Iwasaki
// SPDX-License-Identifier: Apache-2.0
use musha_platform::bluetooth::repair::{Action, Restart, action};
#[test]
fn explicit_p_overrides_stored_reconnect_in_every_started_phase() {
    assert_eq!(action(false, false, false), Action::StartFresh);
    assert_eq!(action(true, true, true), Action::RestartFresh);
    assert_eq!(action(true, false, true), Action::RestartFresh);
    for _phase in 0..=8 {
        assert_eq!(action(true, false, false), Action::RestartFresh);
        assert_eq!(action(true, true, false), Action::AlreadyPairing);
    }
}
#[test]
fn repeated_requests_coalesce_and_force_survives_only_confirmed_stop() {
    let r = Restart::new();
    assert!(!r.take());
    assert!(r.request(false));
    for _ in 0..100 {
        assert!(r.request(true));
    }
    assert!(r.wants());
    assert!(!r.take_force());
    assert!(!r.take());
    r.confirm(false);
    assert!(!r.take());
    r.confirm(true);
    assert!(!r.take_force());
    assert!(r.take());
    assert!(!r.take());
    assert!(r.take_force());
    assert!(!r.take_force());
    // One pending held request consumed only one of the eight restart slots.
    for _ in 0..7 {
        assert!(r.request(false));
        r.confirm(true);
        assert!(r.take());
        assert!(!r.take_force());
    }
    assert!(!r.request(true));
}
#[test]
fn isolation_cancels_force_and_unconfirmed_reuse_stays_forbidden() {
    let r = Restart::new();
    assert!(r.request(true));
    r.confirm(false);
    assert!(!r.take());
    r.isolated();
    r.confirm(true);
    assert!(!r.wants());
    assert!(!r.take());
    assert!(!r.take_force());
}
