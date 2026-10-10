// Copyright 2026 Hirofumi Iwasaki
// SPDX-License-Identifier: Apache-2.0
//! Deterministic mutation/property campaign, not coverage-guided fuzzing.
use musha_input::hid::{Decoder, Layout};
use std::{cell::RefCell, collections::BTreeSet, time::Instant};
thread_local! {
    static REPRO: RefCell<(u64, usize, u64, Vec<u8>)> = const { RefCell::new((0, 0, 0, Vec::new())) };
}
fn random(s: &mut u64) -> u64 {
    *s ^= *s << 13;
    *s ^= *s >> 7;
    *s ^= *s << 17;
    *s
}
fn fixture(id: u8) -> Vec<u8> {
    let mut d = vec![5, 1, 9, 6, 0xa1, 1];
    if id != 0 {
        d.extend([0x85, id]);
    }
    d.extend([
        5, 7, 0x19, 0xe0, 0x29, 0xe7, 0x15, 0, 0x25, 1, 0x75, 1, 0x95, 8, 0x81, 2, 0x75, 8, 0x95,
        6, 0x19, 0, 0x29, 101, 0x25, 101, 0x81, 0, 0xc0,
    ]);
    d
}
fn apply(held: &mut BTreeSet<u8>, key: u8, down: bool) {
    assert!(key > 3, "reserved usage emitted");
    if down {
        assert!(held.insert(key), "duplicate press");
    } else {
        assert!(held.remove(&key), "unmatched release");
    }
}
fn check(d: &[u8], state: &mut u64) -> (usize, usize) {
    let Ok(layout) = Layout::parse(d) else {
        return (0, 0);
    };
    let formats: Vec<_> = (0..=255)
        .filter_map(|id| layout.report_bytes(id).map(|n| (id, n)))
        .collect();
    assert!(formats.iter().all(|(_, n)| *n <= 513));
    let mut decoder = Decoder::new(layout);
    let mut replay = Decoder::new(Layout::parse(d).unwrap());
    let mut held = BTreeSet::new();
    let mut accepted = 0;
    for i in 0..24 {
        let (id, len) = formats[random(state) as usize % formats.len()];
        let mut r = vec![0; len];
        for b in &mut r {
            *b = (random(state) % 102) as u8;
        }
        if id != 0 {
            r[0] = id;
        }
        match i % 6 {
            0 => {
                r.fill(0);
                if id != 0 {
                    r[0] = id;
                }
            }
            1 => {
                r.pop();
            }
            2 => r.push(0),
            3 => {
                if !r.is_empty() {
                    r[0] = random(state) as u8;
                }
            }
            _ => {}
        }
        let mut events = Vec::new();
        let result = decoder.update(&r, |k, p| events.push((k, p)));
        if result.is_err() {
            assert!(events.is_empty(), "error emitted events");
        } else {
            accepted += 1;
            let mut expected = Vec::new();
            replay.update(&r, |k, p| expected.push((k, p))).unwrap();
            assert_eq!(events, expected, "rejected report corrupted state");
            for (k, p) in events {
                apply(&mut held, k, p);
            }
            decoder
                .update(&r, |_, _| panic!("identical report emitted events"))
                .unwrap();
        }
    }
    let mut releases = Vec::new();
    decoder.release_all(|k, p| {
        releases.push((k, p));
        apply(&mut held, k, p);
    });
    let mut expected = Vec::new();
    replay.release_all(|k, p| expected.push((k, p)));
    assert_eq!(releases, expected, "rejected report changed final state");
    assert!(held.is_empty());
    decoder.release_all(|_, _| panic!("second release_all emitted events"));
    (1, accepted)
}
fn main() {
    let args: Vec<_> = std::env::args().collect();
    let cases: usize = args.get(1).map(|s| s.parse().unwrap()).unwrap_or(10_000);
    let seed: u64 = args.get(2).map(|s| s.parse().unwrap()).unwrap_or(0x719bce);
    assert!(seed != 0);
    // Workspace release builds abort on panic: a hook preserves reproduction
    // data before abort, without relying on catch_unwind.
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        REPRO.with(|repro| {
            let r = repro.borrow();
            eprintln!(
                "FAIL seed={} case={} report_seed={} descriptor={:02x?}",
                r.0, r.1, r.2, r.3
            );
        });
        original_hook(info);
    }));
    let mut state = seed;
    let start = Instant::now();
    let (mut layouts, mut reports) = (0, 0);
    for case in 0..cases {
        let mut d = fixture((case % 8) as u8);
        match case % 6 {
            0 => {}
            1 => {
                for _ in 0..1 + random(&mut state) % 8 {
                    let pos = random(&mut state) as usize % d.len();
                    d[pos] = random(&mut state) as u8;
                }
            }
            2 => {
                d.truncate(random(&mut state) as usize % (d.len() + 1));
            }
            3 => {
                let n = random(&mut state) as usize % 4098;
                d = (0..n).map(|_| random(&mut state) as u8).collect();
            }
            4 => {
                d = fixture(1);
                d.extend(fixture(2));
            }
            _ => {
                let padding = (random(&mut state) % 8) as u8;
                d = vec![
                    5, 1, 9, 6, 0xa1, 1, 0x85, 3, 5, 7, 0x15, 0, 0x25, 1, 0x75, 1, 0x95, padding,
                    0x81, 1, 0x19, 4, 0x29, 67, 0x95, 64, 0x81, 2, 0xc0,
                ];
            }
        }
        let report_seed = state;
        REPRO.with(|repro| {
            *repro.borrow_mut() = (seed, case, report_seed, d);
            let input = repro.borrow();
            let (l, r) = check(&input.3, &mut state);
            layouts += l;
            reports += r;
        });
    }
    println!(
        "PASS seed={seed} cases={cases} accepted_layouts={layouts} accepted_reports={reports} elapsed={:?}",
        start.elapsed()
    );
}
