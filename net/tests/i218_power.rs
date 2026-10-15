// Copyright 2026 Hirofumi Iwasaki
// SPDX-License-Identifier: Apache-2.0
use musha_net::i218::{
    Device, Error as IoError, Io,
    power::{self, Error},
};
#[derive(Default)]
struct Fake {
    h2me: u32,
    fwsm: u32,
    reads: usize,
    clear_after: Option<usize>,
    lose_at: Option<usize>,
    delays: Vec<u32>,
    writes: Vec<(usize, u32)>,
    delay_fail: bool,
    write_fail: bool,
    cleanup_fail: bool,
    read_fail: Option<usize>,
    extra_bit: bool,
}
impl Fake {
    fn new() -> Self {
        Self {
            h2me: 0xa0000801,
            fwsm: 0x8400,
            clear_after: Some(3),
            ..Self::default()
        }
    }
}
impl Io for Fake {
    fn read(&mut self, offset: usize) -> Result<u32, IoError> {
        match offset {
            power::FWSM => {
                self.reads += 1;
                if self.read_fail == Some(self.reads) {
                    return Err(IoError::Resource);
                }
                if self.lose_at == Some(self.reads) {
                    self.fwsm &= !0x8000;
                }
                if self.clear_after.is_some_and(|n| self.reads >= n) {
                    self.fwsm &= !0x400;
                }
                Ok(self.fwsm)
            }
            power::H2ME => Ok(self.h2me),
            _ => panic!("unexpected register read"),
        }
    }
    fn write(&mut self, offset: usize, value: u32) -> Result<(), IoError> {
        assert_eq!(offset, power::H2ME);
        self.writes.push((offset, value));
        if self.cleanup_fail && value & 0x1000 == 0 {
            return Err(IoError::Resource);
        }
        self.h2me = value;
        if self.extra_bit {
            self.h2me |= 0x20000000;
        }
        if self.write_fail && value & 0x1000 != 0 {
            return Err(IoError::Resource);
        }
        Ok(())
    }
    fn delay_us(&mut self, us: u32) -> Result<(), IoError> {
        self.delays.push(us);
        if self.delay_fail {
            Err(IoError::Clock)
        } else {
            Ok(())
        }
    }
}
#[test]
fn request_poll_cleanup_preserves_reserved_bits() {
    let mut f = Fake::new();
    power::managed_exit(&mut f, Device::I218V3).unwrap();
    assert_eq!(
        f.writes,
        [(power::H2ME, 0xa0001001), (power::H2ME, 0xa0000001)]
    );
    assert_eq!(f.delays, [10_000]);
    assert_eq!(f.h2me, 0xa0000001);
}
#[test]
fn no_firmware_means_no_guessed_software_fallback() {
    let mut f = Fake::new();
    f.fwsm = 0x400;
    assert_eq!(
        power::managed_exit(&mut f, Device::I218V3),
        Err(Error::FirmwareUnavailable)
    );
    assert!(f.writes.is_empty());
    assert!(f.delays.is_empty());
}
#[test]
fn other_variants_rejected_before_io() {
    for d in [Device::I218V, Device::I218V2] {
        let mut f = Fake::new();
        assert_eq!(
            power::managed_exit(&mut f, d),
            Err(Error::UnsupportedDevice)
        );
        assert_eq!(f.reads, 0);
        assert!(f.writes.is_empty());
    }
}
#[test]
fn foreign_enforce_request_is_never_cleared() {
    let mut f = Fake::new();
    f.h2me |= 0x1000;
    let old = f.h2me;
    assert_eq!(
        power::managed_exit(&mut f, Device::I218V3),
        Err(Error::RequestBusy)
    );
    assert!(f.writes.is_empty());
    assert_eq!(f.h2me, old);
}
#[test]
fn timeout_is_finite_and_cleanup_runs() {
    let mut f = Fake::new();
    f.clear_after = None;
    assert_eq!(
        power::managed_exit(&mut f, Device::I218V3),
        Err(Error::Timeout)
    );
    assert_eq!(f.delays.len(), 30);
    assert_eq!(f.delays.iter().sum::<u32>(), 300_000);
    assert_eq!(f.reads, 32);
    assert_eq!(f.h2me & 0x1800, 0);
}
#[test]
fn firmware_disappearance_is_not_completion() {
    let mut f = Fake::new();
    f.lose_at = Some(2);
    assert_eq!(
        power::managed_exit(&mut f, Device::I218V3),
        Err(Error::FirmwareLost)
    );
    assert_eq!(f.h2me & 0x1000, 0);
}
#[test]
fn delay_failure_still_releases_our_request() {
    let mut f = Fake::new();
    f.delay_fail = true;
    assert_eq!(
        power::managed_exit(&mut f, Device::I218V3),
        Err(Error::Io(IoError::Clock))
    );
    assert_eq!(f.h2me & 0x1000, 0);
}
#[test]
fn failed_write_may_have_latched_and_is_cleaned() {
    let mut f = Fake::new();
    f.write_fail = true;
    assert_eq!(
        power::managed_exit(&mut f, Device::I218V3),
        Err(Error::Io(IoError::Resource))
    );
    assert_eq!(f.writes.len(), 2);
    assert_eq!(f.h2me & 0x1000, 0);
}
#[test]
fn cleanup_error_overrides_timeout_and_never_claims_ready() {
    let mut f = Fake::new();
    f.cleanup_fail = true;
    f.clear_after = None;
    assert_eq!(
        power::managed_exit(&mut f, Device::I218V3),
        Err(Error::Cleanup)
    );
    assert_ne!(f.h2me & 0x1000, 0);
}
#[test]
fn concurrent_unrelated_bits_survive_cleanup() {
    let mut f = Fake::new();
    f.h2me = 0x800;
    f.extra_bit = true;
    power::managed_exit(&mut f, Device::I218V3).unwrap();
    assert_eq!(f.h2me, 0x20000000);
}
#[test]
fn floating_mmio_is_rejected_before_write() {
    let mut f = Fake::new();
    f.fwsm = u32::MAX;
    assert_eq!(
        power::managed_exit(&mut f, Device::I218V3),
        Err(Error::Io(IoError::Resource))
    );
    assert!(f.writes.is_empty());
    let mut f = Fake::new();
    f.h2me = u32::MAX;
    assert_eq!(
        power::managed_exit(&mut f, Device::I218V3),
        Err(Error::Io(IoError::Resource))
    );
    assert!(f.writes.is_empty());
}
#[test]
fn read_errors_before_and_after_request_fail_closed() {
    for n in 1..=3 {
        let mut f = Fake::new();
        f.read_fail = Some(n);
        assert_eq!(
            power::managed_exit(&mut f, Device::I218V3),
            Err(Error::Io(IoError::Resource))
        );
        if n == 1 {
            assert!(f.writes.is_empty());
        } else {
            assert_eq!(f.h2me & 0x1000, 0);
        }
    }
}
