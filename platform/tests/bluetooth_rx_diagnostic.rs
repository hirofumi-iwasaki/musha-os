// Copyright 2026 Hirofumi Iwasaki
// SPDX-License-Identifier: Apache-2.0
use musha_platform::bluetooth::rx_diagnostic::{Endpoint, Once, counts};
#[test]
fn no_reply_observation_is_once_per_connection_and_not_before_boundary() {
    let mut one = Once::new();
    assert!(!one.due(false, 5000));
    assert!(!one.due(true, 4999));
    assert!(one.due(true, 5000));
    assert!(!one.due(true, 5001));
    assert!(!one.due(false, 30000));
    one = Once::new();
    assert!(one.due(true, 5000));
}
#[test]
fn endpoint_metadata_and_connection_counts_are_extracted_without_payloads() {
    let e = Endpoint::decode([0xfffffff1, (64 << 16) | (6 << 3) | 7]);
    assert_eq!(
        e,
        Endpoint {
            state: 1,
            kind: 6,
            max_packet: 64
        }
    );
    assert_eq!(Endpoint::decode([2, 512 << 16]).state, 2);
    assert_eq!(counts([20, 0, 4], [10, 0, 3]), [10, 0, 1]);
    assert_eq!(counts([u32::MAX, 2, 0], [u32::MAX, 3, 0]), [0, 0, 0]);
}

#[test]
fn position_decodes_64bit_pointer_dcs_and_ring_location_without_progress_claim() {
    use musha_platform::bluetooth::rx_diagnostic::Position;
    let ring = 0x1234_0000_1000;
    let expected = ring + 32;
    let words = [(expected as u32) | 1, (expected >> 32) as u32];
    let p = Position::decode(words, words, ring, 256, expected, 1, 0x425);
    assert_eq!(p.dequeue, expected);
    assert_eq!(p.dcs, 1);
    assert_eq!(p.ring_index, Some(2));
    assert_eq!(p.relation(), "same-td");
    assert_eq!((p.cycle, p.control), (1, 0x425));
    // A different DCS remains just an observed position, not a completion assertion.
    let p = Position::decode(words, words, ring, 256, expected, 0, 0x424);
    assert_eq!(p.relation(), "same-td");
    assert_ne!(p.dcs, p.cycle);
    let link = ring + 255 * 16;
    let w = [link as u32, (link >> 32) as u32];
    let p = Position::decode(w, w, ring, 256, expected, 1, 0x425);
    assert_eq!(p.ring_index, Some(255));
    assert_eq!(p.relation(), "in-ring");
}
#[test]
fn changed_or_outside_dequeue_is_reported_without_fabricating_an_index() {
    use musha_platform::bluetooth::rx_diagnostic::Position;
    let p = Position::decode([0x1001, 0], [0x2001, 0], 0x1000, 256, 0x1000, 1, 0x425);
    assert!(!p.stable);
    assert_eq!(p.relation(), "unstable");
    assert_eq!(p.ring_index, None);
    for addr in [0xff0, 0x2000] {
        let w = [addr, 0];
        let p = Position::decode(w, w, 0x1000, 256, 0x1000, 1, 0);
        assert_eq!(p.relation(), "outside-ring");
        assert_eq!(p.ring_index, None);
    }
}
#[test]
fn wait_and_terminal_samples_have_independent_once_budgets_per_connection() {
    let mut b = Once::new();
    assert!(b.terminal());
    assert!(!b.terminal());
    assert!(b.due(true, 5000));
    assert!(!b.due(true, 6000));
    let mut b = Once::new();
    assert!(b.due(true, 5000));
    assert!(b.terminal());
    assert!(!b.terminal());
    assert!(!b.due(true, 9000));
}
