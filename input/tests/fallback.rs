// Copyright 2026 Hirofumi Iwasaki
// SPDX-License-Identifier: Apache-2.0
use musha_input::{
    BootKeyboardState,
    sources::{Error, Sources},
};
use std::collections::BTreeSet;

#[test]
fn repeated_disconnect_reconnect_preserves_application_key_state() {
    let mut sources = Sources::<2>::default();
    let internal = sources.connect(1, |_, _| {}).unwrap();
    let mut internal_decoder = BootKeyboardState::default();
    let mut held = BTreeSet::new();
    for cycle in 0..1000 {
        let mut emit = |key, down| {
            if down {
                assert!(held.insert(key), "duplicate press at {cycle}");
            } else {
                assert!(held.remove(&key), "unmatched release at {cycle}");
            }
        };
        internal_decoder.update([2, 0, 4, 0, 0, 0, 0, 0], |k, d| {
            sources.key(internal, k, d, &mut emit).unwrap()
        });
        let external = sources.connect(0, &mut emit).unwrap();
        let mut external_decoder = BootKeyboardState::default();
        external_decoder.update([0x20, 0, 5, 0, 0, 0, 0, 0], |k, d| {
            sources.key(external, k, d, &mut emit).unwrap()
        });
        // Internal Shift+A is released while hidden: fallback must not replay it.
        internal_decoder.update([0, 0, 6, 0, 0, 0, 0, 0], |k, d| {
            sources.key(internal, k, d, &mut emit).unwrap()
        });
        assert!(!external_decoder.update([0, 0, 1, 1, 1, 1, 1, 1], |_, _| panic!("rollover")));
        sources.disconnect(external, &mut emit).unwrap();
        assert_eq!(
            sources.key(external, 5, false, |_, _| panic!("stale event")),
            Err(Error::Stale)
        );
        assert_eq!(held, BTreeSet::from([6]));
        internal_decoder.update([0; 8], |k, d| {
            sources
                .key(internal, k, d, |k, d| {
                    assert!(!d);
                    assert!(held.remove(&k));
                })
                .unwrap()
        });
        assert!(held.is_empty());
    }
    sources
        .disconnect(internal, |_, _| panic!("already released"))
        .unwrap();
}

#[test]
fn inactive_disconnect_and_modifier_fallback_do_not_disturb_active_keys() {
    let mut s = Sources::<3>::default();
    let a = s.connect(0, |_, _| {}).unwrap();
    let b = s.connect(1, |_, _| {}).unwrap();
    let c = s.connect(2, |_, _| {}).unwrap();
    for source in [a, b, c] {
        s.key(source, 0xe1, true, |_, _| {}).unwrap();
        s.key(source, 4, true, |_, _| {}).unwrap();
    }
    s.disconnect(b, |_, _| panic!("inactive disconnect emitted"))
        .unwrap();
    s.disconnect(a, |_, _| panic!("same held keys emitted"))
        .unwrap();
    let mut events = Vec::new();
    s.disconnect(c, |k, d| events.push((k, d))).unwrap();
    assert_eq!(events, [(0xe1, false), (4, false)]);
    assert_eq!(s.active(), None);
}
