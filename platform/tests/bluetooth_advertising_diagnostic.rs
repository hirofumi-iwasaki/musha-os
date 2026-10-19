// Copyright 2026 Hirofumi Iwasaki
// SPDX-License-Identifier: Apache-2.0
use musha_platform::bluetooth::advertising_diagnostic::{Budget, Candidate, classify};
const HID: &[u8] = &[3, 3, 0x12, 0x18, 3, 0x19, 0xc4, 3];
#[test]
fn actual_classification_preserves_current_per_report_selection() {
    assert_eq!(classify(0, HID).candidate, Some(Candidate::Gamepad));
    assert_eq!(
        classify(1, &[3, 2, 0x12, 0x18, 3, 0x19, 0xc1, 3]).candidate,
        Some(Candidate::Keyboard)
    );
    assert_eq!(
        classify(0, &[3, 3, 0x12, 0x18]).reason,
        "missing-appearance"
    );
    assert_eq!(classify(0, &[3, 0x19, 0xc4, 3]).reason, "no-hid-uuid");
    assert_eq!(classify(4, HID).reason, "scan-response");
    assert_eq!(classify(4, HID).candidate, None);
    for ty in [2, 3] {
        assert_eq!(classify(ty, HID).reason, "nonconnectable");
    }
    // Split ADV and SCAN_RSP fields stay separate, matching production behavior.
    assert_eq!(classify(0, &[3, 3, 0x12, 0x18]).candidate, None);
    assert_eq!(classify(4, &[3, 0x19, 0xc4, 3]).candidate, None);
    assert_eq!(
        classify(0, &[3, 3, 0x12, 0x18, 3, 0x19, 0xc2, 3]).reason,
        "unsupported-appearance"
    );
}
#[test]
fn malformed_fields_are_visible_without_changing_legacy_ignored_field_rules() {
    let bad = classify(0, &[9, 3, 0x12]);
    assert!(!bad.ad_valid);
    assert_eq!(bad.reason, "malformed-ad");
    assert_eq!(classify(0, &[2, 3, 0x12]).reason, "uuid-width");
    assert_eq!(
        classify(0, &[3, 3, 0x12, 0x18, 2, 0x19, 0xc4]).reason,
        "appearance-width"
    );
    assert_eq!(
        classify(0, &[3, 3, 0x12, 0x18, 3, 0x19, 0xc4, 3, 3, 2, 0x0f, 0x18]).candidate,
        None
    );
    // Odd UUID field was ignored by old selector; valid earlier HID still wins.
    assert_eq!(
        classify(0, &[3, 3, 0x12, 0x18, 3, 0x19, 0xc4, 3, 2, 2, 0x12]).candidate,
        Some(Candidate::Gamepad)
    );
}
#[test]
fn report_budget_is_bounded_with_reserved_hid_records_and_suppression_counts() {
    let mut b = Budget::new();
    for _ in 0..24 {
        assert!(b.admits(classify(0, &[])));
    }
    assert!(!b.admits(classify(0, &[])));
    assert_eq!(b.suppressed(), 1);
    for _ in 0..16 {
        assert!(b.admits(classify(0, HID)));
    }
    for _ in 0..100 {
        assert!(!b.admits(classify(0, HID)));
    }
    assert_eq!(b.suppressed(), 101);
    // New device/session starts fresh; automatic rescans retain the same budget.
    assert!(Budget::new().admits(classify(0, HID)));
}

#[test]
fn flags_metadata_is_strict_and_never_changes_candidate_selection() {
    use musha_platform::bluetooth::advertising_diagnostic::Flags;
    assert_eq!(classify(0, HID).flags, Flags::Missing);
    for v in 0..4 {
        let mut data = vec![2, 1, v];
        data.extend(HID);
        let c = classify(0, &data);
        assert_eq!(c.candidate, Some(Candidate::Gamepad));
        assert_eq!(c.flags.limited(), Some(v & 1 != 0));
        assert_eq!(c.flags.general(), Some(v & 2 != 0));
    }
    for prefix in [&[1, 1][..], &[3, 1, 0, 0][..], &[2, 1, 1, 2, 1, 2][..]] {
        let mut data = prefix.to_vec();
        data.extend(HID);
        let c = classify(0, &data);
        assert_eq!(c.flags, Flags::Invalid);
        assert_eq!(c.candidate, Some(Candidate::Gamepad));
        assert_eq!(c.flags.general(), None);
    }
    assert_eq!(classify(0, &[9, 1, 2]).flags, Flags::Invalid);
}
