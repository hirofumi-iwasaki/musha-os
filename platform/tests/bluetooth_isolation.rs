// Copyright 2026 Hirofumi Iwasaki
// SPDX-License-Identifier: Apache-2.0
#![cfg(feature = "bluetooth-security")]
//! The same orchestration functions used by live prepare/poll, with injected I/O.
#[path = "../../boot/src/xhci/usb/bluetooth/isolation.rs"]
mod isolation;
use isolation::{SlotStop, disable, local_failure, remaining};
struct Devices {
    calls: Vec<&'static str>,
    fail: Option<&'static str>,
    keyboard: bool,
    storage: bool,
    dma_reserved: bool,
    detached: bool,
}
impl Devices {
    fn new(fail: Option<&'static str>) -> Self {
        Self {
            calls: vec![],
            fail,
            keyboard: true,
            storage: true,
            dma_reserved: true,
            detached: false,
        }
    }
    fn call(&mut self, name: &'static str) -> Result<(), &'static str> {
        self.calls.push(name);
        if self.fail == Some(name) {
            Err("injected")
        } else {
            Ok(())
        }
    }
    fn service(&self) -> (bool, bool) {
        (self.keyboard, self.storage)
    }
}
impl SlotStop for Devices {
    fn retire(&mut self, slot: u8) -> Result<(), &'static str> {
        assert_eq!(slot, 2);
        self.call("retire")
    }
    fn disable(&mut self, slot: u8) -> Result<(), &'static str> {
        assert_eq!(slot, 2);
        self.call("disable-correlated")
    }
    fn detach(&mut self, slot: u8) {
        assert_eq!(slot, 2);
        self.calls.push("detach");
        self.detached = true;
    }
}
#[test]
fn each_bluetooth_failure_keeps_other_services_only_after_correlated_disable() {
    for error in [
        "BT ENTROPY RESET",
        "BT ENTROPY CREDIT",
        "BT ENTROPY COMMAND TIMEOUT",
        "BT INITIALIZATION TIMEOUT",
        "BT SCAN STATE",
        "BT COMMAND REJECTED",
        "BT COMMAND TIMEOUT",
        "BT DISCONNECTED",
        "BT SMP TIMEOUT",
        "BT GATT STATE",
        "TRANSFER TIMEOUT",
        "SHORT CONTROL WRITE",
    ] {
        assert!(local_failure(error));
        let mut d = Devices::new(None);
        disable(&mut d, 2).unwrap();
        assert_eq!(d.calls, ["retire", "disable-correlated", "detach"]);
        assert!(d.detached && d.dma_reserved);
        assert_eq!(d.service(), (true, true));
    }
}
#[test]
fn ambiguous_retirement_and_disable_failure_never_allow_detach_or_continuation() {
    for stage in ["retire", "disable-correlated"] {
        let mut d = Devices::new(Some(stage));
        assert!(disable(&mut d, 2).is_err());
        assert!(!d.detached);
        assert!(d.dma_reserved);
    }
}
#[test]
fn shared_controller_ring_and_clock_errors_keep_whole_host_failure() {
    for e in [
        "COMMAND TIMEOUT",
        "COMMAND REJECTED",
        "BAD TRANSFER",
        "BAD COMPLETION",
        "MMIO RANGE",
        "HOST SYSTEM ERROR",
        "CLOCK OVERFLOW",
        "NO TIMER",
        "BT CONTEXT COMMAND",
        "BT REPAIR REQUEST",
    ] {
        assert!(!local_failure(e), "{e}");
    }
}
#[test]
fn common_operation_budget_clamps_each_blocking_wait_without_underflow() {
    for (now, expected) in [(0, 1000), (999, 1000), (1000, 1000), (1500, 500), (1999, 1)] {
        assert_eq!(remaining(2000, now), Ok(expected));
    }
    assert!(remaining(2000, 2000).is_err());
    assert!(remaining(2000, u64::MAX).is_err());
}

#[test]
fn command_observation_never_invents_success_for_missing_status() {
    use isolation::command_event as parse;
    assert_eq!(
        parse(&[0x0e, 4, 1, 3, 0x0c, 0]),
        Ok(Some(("CC", 1, 0x0c03, Some(0))))
    );
    assert_eq!(
        parse(&[0x0f, 4, 0x0c, 2, 0x0d, 0x20]),
        Ok(Some(("CS", 2, 0x200d, Some(0x0c))))
    );
    assert!(parse(&[0x0e, 3, 1, 3, 0x0c]).is_err());
    assert_eq!(parse(&[0x0e, 3, 1, 0, 0]), Ok(Some(("CC", 1, 0, None))));
    assert!(parse(&[0x0f, 3, 0, 1, 3]).is_err());
}
#[test]
fn clearing_bluetooth_held_keys_does_not_release_usb_held_keys() {
    let mut keys = musha_input::merge::Keys::new();
    let mut events = vec![];
    keys.key(0, 4, true, |_, _| {});
    keys.key(1, 4, true, |_, _| {});
    keys.key(1, 5, true, |_, _| {});
    keys.clear(1, |k, d| events.push((k, d)));
    assert_eq!(events, [(5, false)]);
    keys.key(0, 4, false, |k, d| events.push((k, d)));
    assert_eq!(events, [(5, false), (4, false)]);
}

#[test]
fn warmup_waits_for_correlated_complete_without_renewing_deadline() {
    use isolation::{WarmupEvent, warmup_event};
    let mut n = 0;
    for p in [
        vec![0x13, 5, 1, 5, 0, 1, 0],
        vec![0x0f, 4, 0, 1, 3, 0x0c],
        vec![0x0e, 12, 1, 1, 0x10, 0, 6, 0xbb, 0x22, 6, 10, 0, 0xbb, 0x22],
    ] {
        assert!(matches!(
            warmup_event(&p, 0x0c03, 1000, 100, &mut n),
            Ok(WarmupEvent::Pending(_))
        ));
    }
    assert!(matches!(
        warmup_event(&[0x0e, 4, 1, 3, 0x0c, 0], 0x0c03, 1000, 999, &mut n),
        Ok(WarmupEvent::Complete {
            credits: 1,
            result: [0]
        })
    ));
    assert!(matches!(
        warmup_event(&[0x0e, 4, 1, 3, 0x0c, 0], 0x0c03, 1000, 1000, &mut n),
        Err("BT ENTROPY COMMAND TIMEOUT")
    ));
}
#[test]
fn warmup_malformed_errors_and_zero_credits_are_rejected() {
    for p in [
        vec![],
        vec![0x13, 1, 0],
        vec![0x13, 9, 2, 5, 0, 1, 0, 5, 0, 1, 0],
        vec![0x13, 5, 1],
        vec![0x13, 5, 1, 0xff, 0xff, 1, 0],
        vec![0x0f, 3, 0, 1, 3],
        vec![0x0f, 4, 0, 0, 3, 0x0c],
        vec![0x0f, 4, 1, 1, 3, 0x0c],
        vec![0x0f, 4, 0, 1, 1, 0x10],
        vec![0x0e, 4, 0, 3, 0x0c, 0],
        vec![0x0e, 3, 1, 3, 0x0c],
        vec![0x10, 1, 1],
    ] {
        assert!(isolation::warmup_event(&p, 0x0c03, 1000, 1, &mut 0).is_err());
    }
}
#[test]
fn warmup_async_events_have_a_fixed_count_cap() {
    let mut n = 0;
    for _ in 0..64 {
        assert!(
            isolation::warmup_event(&[0x13, 5, 1, 5, 0, 1, 0], 0x0c03, 1000, 1, &mut n).is_ok()
        );
    }
    assert!(matches!(
        isolation::warmup_event(&[0x13, 5, 1, 5, 0, 1, 0], 0x0c03, 1000, 1, &mut n),
        Err("BT ENTROPY EVENT LIMIT")
    ));
}
#[test]
fn warmup_rand_payload_is_only_consumed_after_matching_complete_and_health_checked() {
    use isolation::{WarmupEvent, warmup_event};
    use musha_platform::bluetooth::sources::{HciFailure, HciRand};
    let mut samples = HciRand::new();
    let mut n = 0;
    for index in 0..4u8 {
        let mut p = vec![0x0e, 12, 1, 0x18, 0x20, 0];
        p.extend_from_slice(&(u64::from(index) + 17).to_le_bytes());
        let WarmupEvent::Complete { result, .. } =
            warmup_event(&p, 0x2018, 1000, 10, &mut n).unwrap()
        else {
            panic!()
        };
        let ready = samples.complete(result).unwrap();
        assert_eq!(ready.is_some(), index == 3);
    }
    let mut bad = HciRand::new();
    let p = [0x0e, 12, 1, 0x18, 0x20, 0, 0, 0, 0, 0, 0, 0, 0, 0];
    let WarmupEvent::Complete { result, .. } = warmup_event(&p, 0x2018, 1000, 10, &mut n).unwrap()
    else {
        panic!()
    };
    // Existing HciRand health policy, not a diagnostic-only duplicate.
    let mut outcome = bad.complete(result);
    for _ in 1..4 {
        if outcome.is_ok() {
            outcome = bad.complete(result);
        }
    }
    assert!(matches!(outcome, Err(HciFailure::Health)));
}

#[test]
fn drain_quiet_and_hard_deadlines_do_not_clear_pending_in() {
    let mut d = isolation::Drain::new(100);
    assert!(!d.finished(149).unwrap());
    assert!(d.finished(150).unwrap());
    d.event(140).unwrap();
    assert!(!d.finished(199).unwrap());
    assert!(d.finished(200).unwrap());
    d.event(299).unwrap();
    assert!(d.finished(300).unwrap());
    assert!(d.finished(99).is_err());
}
#[test]
fn late_reset_retry_is_once_and_keeps_original_deadline() {
    use isolation::{WarmupEvent, retry_after_reset, warmup_event};
    let mut count = 0;
    let mut used = false;
    let p = [0x0e, 4, 1, 3, 0x0c, 0];
    assert!(matches!(
        warmup_event(&p, 0x1001, 1000, 100, &mut count),
        Ok(WarmupEvent::LateReset { credits: 1 })
    ));
    retry_after_reset(0x1001, 1000, 100, &mut used).unwrap();
    assert!(retry_after_reset(0x1001, 1000, 101, &mut used).is_err());
    assert!(retry_after_reset(0x2018, 1000, 1000, &mut false).is_err());
    assert!(warmup_event(&[0x0e, 4, 1, 3, 0x0c, 1], 0x1001, 1000, 100, &mut count).is_err());
    assert!(matches!(
        warmup_event(&[0xff, 0], 0x1001, 1000, 101, &mut count),
        Ok(WarmupEvent::Pending("unknown-wellformed-discard"))
    ));
    assert!(matches!(
        warmup_event(&p, 0x0c03, 1000, 100, &mut count),
        Ok(WarmupEvent::Complete { .. })
    ));
}

#[test]
fn drained_old_reset_then_version_barrier_before_rand() {
    use isolation::{WarmupEvent, warmup_event};
    let mut n = 0;
    let reset = [0x0e, 4, 1, 3, 0x0c, 0];
    // During drain the correlated parser result is deliberately not applied.
    assert!(matches!(
        warmup_event(&reset, 0x0c03, 2000, 20, &mut n),
        Ok(WarmupEvent::Complete { .. })
    ));
    assert!(matches!(
        warmup_event(&[5, 4, 0, 5, 0, 0x13], 0x0c03, 2000, 21, &mut n),
        Ok(WarmupEvent::Pending(_))
    ));
    assert!(matches!(
        warmup_event(&reset, 0x0c03, 1000, 100, &mut n),
        Ok(WarmupEvent::Complete { .. })
    ));
    assert!(matches!(
        warmup_event(&reset, 0x1001, 1100, 110, &mut n),
        Ok(WarmupEvent::LateReset { .. })
    ));
    let version = [0x0e, 12, 1, 1, 0x10, 0, 6, 0xbb, 0x22, 6, 10, 0, 0xbb, 0x22];
    let WarmupEvent::Complete { result, .. } =
        warmup_event(&version, 0x1001, 1100, 120, &mut n).unwrap()
    else {
        panic!()
    };
    assert!(isolation::valid_version(result));
    assert!(!isolation::valid_version(&result[..8]));
    assert!(!isolation::valid_version(&[1; 9]));
}

#[test]
fn drain_rejects_malformed_known_return_lengths_but_records_valid_error_status() {
    use isolation::{WarmupEvent, warmup_event};
    assert!(warmup_event(&[0x0e, 5, 1, 3, 0x0c, 0, 0], 0x0c03, 1000, 1, &mut 0).is_err());
    assert!(warmup_event(&[0x0e, 4, 1, 1, 0x10, 0], 0x0c03, 1000, 1, &mut 0).is_err());
    assert!(matches!(
        warmup_event(&[0x0e, 4, 1, 3, 0x0c, 1], 0x0c03, 1000, 1, &mut 0),
        Ok(WarmupEvent::Complete { .. })
    ));
}
