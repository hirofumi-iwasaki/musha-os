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
