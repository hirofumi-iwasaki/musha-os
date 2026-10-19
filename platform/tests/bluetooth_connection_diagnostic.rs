// Copyright 2026 Hirofumi Iwasaki
// SPDX-License-Identifier: Apache-2.0
use musha_platform::bluetooth::{
    Event,
    connection_diagnostic::{Mode, Wait, event_metadata},
};
#[test]
fn wait_observes_existing_deadline_once_without_enforcing_it() {
    let mut w = Wait::new();
    assert!(w.due(99999, [0; 3]).is_none());
    w.begin(Mode::Stored, 0x48, 100, 10100, [5, 6, 7]);
    w.accepted();
    assert!(w.due(5099, [5, 6, 8]).is_none());
    let s = w.due(5100, [6, 6, 8]).unwrap();
    assert_eq!(
        (
            s.mode,
            s.handle,
            s.elapsed,
            s.deadline,
            s.accepted,
            s.completions
        ),
        (Mode::Stored, 0x48, 5000, 10100, true, [1, 0, 1])
    );
    assert!(w.due(10101, [8, 6, 9]).is_none());
    let s = w.finish(10102, [8, 6, 9]).unwrap();
    assert_eq!(s.elapsed, 10002);
    assert_eq!(s.deadline, 10100);
    assert!(w.finish(10103, [0; 3]).is_none());
    w.begin(Mode::Fresh, 7, 200, 30200, [10; 3]);
    let s = w.due(5200, [0; 3]).unwrap();
    assert!(!s.accepted);
    assert_eq!(s.completions, [0; 3]);
    w.clear();
    assert!(w.due(99999, [0; 3]).is_none());
    assert!(!Mode::from_pairing(true).stored_key());
    assert!(Mode::from_pairing(false).stored_key());
}
#[test]
fn hci_failure_metadata_is_keyless_and_length_checked() {
    let m = event_metadata(Event::parse(&[8, 4, 6, 0x48, 0, 0]).unwrap()).unwrap();
    assert_eq!((m.status, m.handle, m.detail), (6, 0x48, 0));
    let m = event_metadata(Event::parse(&[5, 4, 0, 0x47, 0, 0x13]).unwrap()).unwrap();
    assert_eq!((m.status, m.handle, m.detail), (0, 0x47, 0x13));
    assert!(event_metadata(Event::parse(&[8, 3, 0, 0, 0]).unwrap()).is_none());
    assert!(event_metadata(Event::parse(&[0x13, 4, 0, 0, 0, 0]).unwrap()).is_none());
}

#[test]
fn error_snapshot_reuses_host_clock_even_when_boot_epoch_is_different() {
    let host_begin = 1519;
    let boot_epoch_at_begin = 2814;
    let mut w = Wait::new();
    w.begin(Mode::Stored, 0x48, host_begin, host_begin + 10000, [0; 3]);
    assert!(w.due(host_begin + 5000, [1, 0, 0]).is_some());
    let host_last = 8353;
    let boot_epoch_at_error = 9648;
    assert_eq!(
        boot_epoch_at_error - host_last,
        boot_epoch_at_begin - host_begin
    );
    assert!(w.due(host_last, [1, 0, 0]).is_none());
    // There is deliberately no epoch/time argument at the error boundary.
    let s = w.finish_observed([1, 0, 0]).unwrap();
    assert_eq!(s.elapsed, 6834);
    assert_eq!(s.sampled_at, host_last);
    assert_eq!(s.deadline, 11519);
    assert_eq!(s.completions, [1, 0, 0]);
    assert_ne!(s.elapsed, boot_epoch_at_error - host_begin);
    assert!(w.finish_observed([0; 3]).is_none());
}
