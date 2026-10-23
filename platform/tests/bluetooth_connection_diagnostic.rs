// Copyright 2026 Hirofumi Iwasaki
// SPDX-License-Identifier: Apache-2.0
use musha_platform::bluetooth::{
    Event,
    connection_diagnostic::{Mode, Wait, event_metadata},
    stream::Stream,
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

#[test]
fn event_trace_follows_fragmented_hci_frames_without_retaining_payload() {
    let mut stream = Stream::<257>::new(9, false);
    let mut wait = Wait::new();
    wait.begin(Mode::Stored, 0x48, 100, 10100, [0; 3]);
    wait.accepted();
    let mut first = &[0x0f, 4, 0, 1][..];
    assert!(stream.next(9, 101, &mut first).unwrap().is_none());
    assert_eq!(stream.progress(), (4, 6));
    let mut rest = &[0x19, 0x20, 8, 4, 0, 0x48, 0, 1][..];
    let frame = stream.next(9, 102, &mut rest).unwrap().unwrap();
    wait.received(102, Event::parse(frame).unwrap());
    stream.release();
    assert_eq!(stream.progress(), (0, 0));
    let frame = stream.next(9, 102, &mut rest).unwrap().unwrap();
    let event = Event::parse(frame).unwrap();
    assert_eq!(event_metadata(event).unwrap().handle, 0x48);
    wait.received(102, event);
    stream.release();
    let snapshot = wait.finish(102, [1, 0, 0]).unwrap();
    assert_eq!(snapshot.mode, Mode::Stored);
    assert_eq!(snapshot.received_events, 2);
    assert_eq!(snapshot.last_event, Some((8, 4, 102)));
    assert_eq!(snapshot.completions, [1, 0, 0]);
}

#[test]
fn fresh_and_missing_event_snapshots_are_distinct_and_bounded() {
    let mut wait = Wait::new();
    wait.begin(Mode::Fresh, 0x49, 0, 30000, [0; 3]);
    assert_eq!(wait.finish(30000, [0; 3]).unwrap().last_event, None);
    wait.begin(Mode::Stored, 0x48, 0, 10000, [0; 3]);
    let event = Event::parse(&[0x0f, 4, 0, 1, 0x19, 0x20]).unwrap();
    for _ in 0..10 {
        wait.received(1, event);
    }
    let snapshot = wait.finish(10000, [10, 0, 0]).unwrap();
    assert_eq!(snapshot.received_events, 10);
    assert_eq!(snapshot.last_event, Some((0x0f, 4, 1)));
    wait.begin(Mode::Fresh, 0x49, 10001, 40001, [10, 0, 0]);
    let fresh = wait.finish(10002, [10, 0, 0]).unwrap();
    assert_eq!(fresh.received_events, 0);
    assert_eq!(fresh.last_event, None);
    assert!(Event::parse(&[8, 4, 0, 0x48, 0]).is_err());
}
