// Copyright 2026 Hirofumi Iwasaki
// SPDX-License-Identifier: Apache-2.0
//! Synthetic HID -> source selection -> real application FIFO, no hardware I/O.
use musha_api::Context;
use musha_framebuffer::Framebuffer;
use musha_input::{
    hid::{Decoder, Layout},
    sources::{Error as SourceError, Source, Sources},
};
use std::collections::BTreeSet;
fn decoder() -> Decoder {
    // Keyboard application: eight modifier bits, reserved byte, six array keys.
    let descriptor = [
        5, 1, 9, 6, 0xa1, 1, 5, 7, 0x19, 0xe0, 0x29, 0xe7, 0x15, 0, 0x25, 1, 0x75, 1, 0x95, 8,
        0x81, 2, 0x75, 8, 0x95, 1, 0x81, 1, 0x19, 0, 0x29, 101, 0x25, 101, 0x95, 6, 0x81, 0, 0xc0,
    ];
    Decoder::new(Layout::parse(&descriptor).unwrap())
}
fn context(test: impl FnOnce(&mut Context<'_>)) {
    let mut arena = [0u8; 1];
    let mut pixels = [0u32; 1];
    let fb = Framebuffer {
        base: pixels.as_mut_ptr() as usize,
        bytes: 4,
        width: 1,
        height: 1,
        stride: 1,
        format: 0,
    };
    // Both owned buffers outlive the context; no other access while it is live.
    let mut ctx = unsafe { Context::new(&mut arena, fb) }.unwrap();
    test(&mut ctx);
}
fn feed(
    d: &mut Decoder,
    s: &mut Sources<2>,
    source: Source,
    ctx: &mut Context<'_>,
    mods: u8,
    keys: &[u8],
) {
    let mut report = [0; 8];
    report[0] = mods;
    report[2..2 + keys.len()].copy_from_slice(keys);
    d.update(&report, |k, p| {
        s.key(source, k, p, |k, p| ctx.push_key(k, p)).unwrap()
    })
    .unwrap();
}
fn consume(ctx: &mut Context<'_>, held: &mut BTreeSet<u8>, limit: usize) -> usize {
    let mut n = 0;
    while n < limit {
        let Some(e) = ctx.next_key() else { break };
        if e.pressed {
            assert!(held.insert(e.usage), "duplicate press {}", e.usage);
        } else {
            assert!(held.remove(&e.usage), "unmatched release {}", e.usage);
        }
        n += 1;
    }
    n
}
fn drain(ctx: &mut Context<'_>, held: &mut BTreeSet<u8>, expected: &[u8]) {
    assert!(consume(ctx, held, 321) <= 320, "unbounded recovery");
    assert!(ctx.next_key().is_none());
    assert_eq!(*held, expected.iter().copied().collect());
}
fn burst(d: &mut Decoder, s: &mut Sources<2>, source: Source, ctx: &mut Context<'_>) {
    for _ in 0..40 {
        feed(d, s, source, ctx, 2, &[4, 5]);
        feed(d, s, source, ctx, 2, &[4]);
    }
}
#[test]
fn held_shift_disconnect_recovers_releases_after_overflow() {
    context(|ctx| {
        let mut s = Sources::default();
        let src = s.connect(1, |k, p| ctx.push_key(k, p)).unwrap();
        let mut d = decoder();
        let mut app = BTreeSet::new();
        feed(&mut d, &mut s, src, ctx, 2, &[4]);
        drain(ctx, &mut app, &[0xe1, 4]);
        burst(&mut d, &mut s, src, ctx);
        s.disconnect(src, |k, p| ctx.push_key(k, p)).unwrap();
        assert!(ctx.lost_key_events() > 0);
        drain(ctx, &mut app, &[]);
        assert_eq!(s.active(), None);
    });
}
#[test]
fn external_preemption_and_hidden_internal_changes_survive_overflow() {
    context(|ctx| {
        let mut s = Sources::default();
        let inner = s.connect(1, |k, p| ctx.push_key(k, p)).unwrap();
        let (mut a, mut b) = (decoder(), decoder());
        let mut app = BTreeSet::new();
        feed(&mut a, &mut s, inner, ctx, 2, &[4]);
        drain(ctx, &mut app, &[0xe1, 4]);
        burst(&mut a, &mut s, inner, ctx);
        let outer = s.connect(0, |k, p| ctx.push_key(k, p)).unwrap();
        feed(&mut b, &mut s, outer, ctx, 0x20, &[5]);
        feed(&mut a, &mut s, inner, ctx, 0, &[6]); // hidden source changes while external wins
        assert!(ctx.lost_key_events() > 0);
        drain(ctx, &mut app, &[0xe5, 5]);
        s.disconnect(outer, |k, p| ctx.push_key(k, p)).unwrap();
        drain(ctx, &mut app, &[6]);
        s.disconnect(inner, |k, p| ctx.push_key(k, p)).unwrap();
        drain(ctx, &mut app, &[]);
    });
}
#[test]
fn reconnect_rejects_late_old_reports_during_partial_fifo_drain() {
    context(|ctx| {
        let mut s = Sources::default();
        let old = s.connect(0, |k, p| ctx.push_key(k, p)).unwrap();
        let mut d = decoder();
        let mut app = BTreeSet::new();
        feed(&mut d, &mut s, old, ctx, 2, &[4]);
        drain(ctx, &mut app, &[0xe1, 4]);
        burst(&mut d, &mut s, old, ctx);
        assert_eq!(consume(ctx, &mut app, 17), 17);
        s.disconnect(old, |k, p| ctx.push_key(k, p)).unwrap();
        let fresh = s.connect(0, |k, p| ctx.push_key(k, p)).unwrap();
        let mut fresh_decoder = decoder();
        feed(&mut fresh_decoder, &mut s, fresh, ctx, 0, &[7]);
        // Decode a late release, then a late press from the retired connection.
        let mut rejected = 0;
        for report in [[0; 8], [2, 0, 4, 0, 0, 0, 0, 0]] {
            d.update(&report, |k, p| {
                assert_eq!(
                    s.key(old, k, p, |_, _| panic!("stale callback")),
                    Err(SourceError::Stale)
                );
                rejected += 1;
            })
            .unwrap();
        }
        assert_eq!(rejected, 4);
        assert_eq!(
            s.disconnect(old, |_, _| panic!("stale disconnect")),
            Err(SourceError::Stale)
        );
        drain(ctx, &mut app, &[7]);
        s.disconnect(fresh, |k, p| ctx.push_key(k, p)).unwrap();
        drain(ctx, &mut app, &[]);
    });
}
#[test]
fn malformed_reports_do_not_corrupt_pending_recovery() {
    context(|ctx| {
        let mut s = Sources::default();
        let src = s.connect(1, |k, p| ctx.push_key(k, p)).unwrap();
        let mut d = decoder();
        let mut app = BTreeSet::new();
        feed(&mut d, &mut s, src, ctx, 2, &[4]);
        drain(ctx, &mut app, &[0xe1, 4]);
        burst(&mut d, &mut s, src, ctx);
        let lost = ctx.lost_key_events();
        for report in [
            vec![],
            vec![0; 7],
            vec![0; 9],
            vec![0, 0, 1, 0, 0, 0, 0, 0],
            vec![0, 0, 200, 0, 0, 0, 0, 0],
        ] {
            assert!(
                d.update(&report, |_, _| panic!("invalid report emitted"))
                    .is_err()
            );
        }
        assert_eq!(ctx.lost_key_events(), lost);
        drain(ctx, &mut app, &[0xe1, 4]);
        feed(&mut d, &mut s, src, ctx, 0, &[]);
        drain(ctx, &mut app, &[]);
        s.disconnect(src, |_, _| panic!("duplicate release"))
            .unwrap();
    });
}
#[test]
fn repeated_switch_overflow_and_reconnect_converge_at_every_checkpoint() {
    context(|ctx| {
        let mut s = Sources::default();
        let inner = s.connect(1, |k, p| ctx.push_key(k, p)).unwrap();
        let mut a = decoder();
        let mut app = BTreeSet::new();
        for cycle in 0..1000 {
            ctx.advance(cycle).unwrap();
            feed(&mut a, &mut s, inner, ctx, 2, &[4]);
            burst(&mut a, &mut s, inner, ctx);
            consume(ctx, &mut app, (cycle % 31) as usize);
            let outer = s.connect(0, |k, p| ctx.push_key(k, p)).unwrap();
            let mut b = decoder();
            feed(&mut b, &mut s, outer, ctx, 0x20, &[5]);
            feed(&mut a, &mut s, inner, ctx, 0, &[6]);
            if cycle % 2 == 0 {
                drain(ctx, &mut app, &[0xe5, 5]);
            }
            s.disconnect(outer, |k, p| ctx.push_key(k, p)).unwrap();
            assert_eq!(
                s.key(outer, 5, true, |_, _| panic!("late key")),
                Err(SourceError::Stale)
            );
            drain(ctx, &mut app, &[6]);
            feed(&mut a, &mut s, inner, ctx, 0, &[]);
            drain(ctx, &mut app, &[]);
        }
        assert!(ctx.lost_key_events() > 1000);
        s.disconnect(inner, |_, _| panic!("all keys released"))
            .unwrap();
        assert!(app.is_empty());
        assert_eq!(s.active(), None);
    });
}
