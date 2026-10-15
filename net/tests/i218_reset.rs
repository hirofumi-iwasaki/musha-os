// Copyright 2026 Hirofumi Iwasaki
// SPDX-License-Identifier: Apache-2.0
use musha_net::i218::{self, Error, Io, reset};
use std::collections::BTreeMap;
struct Fake {
    regs: BTreeMap<usize, u32>,
    events: Vec<String>,
    pending: u8,
    corrupt: bool,
    bme: bool,
    fail_reset: bool,
    fail_delay: bool,
    no_init: bool,
    held_after: bool,
}
impl Fake {
    fn new() -> Self {
        Self {
            regs: BTreeMap::from([(0, 0x10), (8, 0x602), (0x5b54, 0x8040), (0x5b50, 0x800)]),
            events: vec![],
            pending: 0,
            corrupt: false,
            bme: false,
            fail_reset: false,
            fail_delay: false,
            no_init: false,
            held_after: false,
        }
    }
}
impl Io for Fake {
    fn read(&mut self, r: usize) -> Result<u32, Error> {
        assert_eq!(self.pending, 0, "MMIO before reset synchronization");
        self.events.push(format!("read:{r:x}"));
        if r == i218::MDIC {
            let c = *self.regs.get(&r).unwrap();
            let n = (c >> 16) & 31;
            let v = if c & (1 << 27) != 0 {
                match n {
                    2 => 0x0154,
                    3 => 0x00a1,
                    _ => 0xffff,
                }
            } else {
                c & 0xffff
            };
            return Ok((c & !0xffff) | v | (1 << 28));
        }
        Ok(*self.regs.get(&r).unwrap_or(&0))
    }
    fn write(&mut self, r: usize, v: u32) -> Result<(), Error> {
        assert_eq!(self.pending, 0);
        self.events.push(format!("write:{r:x}:{v:x}"));
        self.regs.insert(r, v);
        Ok(())
    }
    fn delay_us(&mut self, us: u32) -> Result<(), Error> {
        self.events.push(format!("delay:{us}"));
        if self.pending == 1 {
            assert_eq!(us, 20_000);
            if self.fail_delay {
                return Err(Error::Clock);
            }
            self.pending = 2;
        }
        Ok(())
    }
}
impl reset::Io for Fake {
    fn pci_id(&mut self) -> Result<u32, Error> {
        self.events.push("pci".into());
        if self.pending == 2 {
            self.pending = 3;
        } else {
            assert_eq!(self.pending, 0);
        }
        Ok(0x15a38086)
    }
    fn bus_master(&mut self) -> Result<bool, Error> {
        Ok(self.bme)
    }
    fn raw_write(&mut self, r: usize, v: u32) -> Result<(), Error> {
        self.events.push(format!("raw:{r:x}:{v:x}"));
        if r == 0 {
            assert_ne!(v & (1 << 26), 0);
            assert_eq!(self.pending, 0);
            self.pending = 1;
            if self.fail_reset {
                return Err(Error::Resource);
            }
        } else {
            assert_eq!(r, 0xc);
            if self.pending == 3 {
                self.pending = 0;
                self.regs.insert(0, 0x10);
                self.regs
                    .insert(i218::EXTCNF_CTRL, if self.held_after { 0x20 } else { 0 });
                self.regs.insert(8, if self.no_init { 0 } else { 0x602 });
            }
        }
        Ok(())
    }
    fn nvm_word(&mut self, n: u16) -> Result<u16, Error> {
        Ok(if n == 63 {
            0xbaba ^ u16::from(self.corrupt)
        } else {
            0
        })
    }
}
#[test]
fn reset_sync_has_no_mmio_flush_and_post_phy_cleanup() {
    let mut f = Fake::new();
    reset::initialize(&mut f).unwrap();
    assert!(f.events.iter().any(|s| s == "write:f00:80"));
    let at = f
        .events
        .iter()
        .position(|x| x.starts_with("raw:0:"))
        .unwrap();
    assert_eq!(
        &f.events[at + 1..at + 4],
        ["delay:20000", "pci", "raw:c:8086"]
    );
    assert_eq!(f.regs[&i218::EXTCNF_CTRL] & 0x20, 0);
    assert_eq!(f.regs[&0x3004] & 0x50000, 0x50000);
    assert_eq!(f.regs[&0x400], 8);
    assert_eq!(f.regs[&0x100], 0);
    assert!(f.events.iter().any(|s| s == "write:20:431ffef"));
}
#[test]
fn software_nvm_is_not_skipped() {
    let mut f = Fake::new();
    f.regs.insert(0x28, 1 << 27);
    assert_eq!(
        reset::initialize(&mut f),
        Err(reset::Error::SoftwareNvmRequired)
    );
    assert!(
        !f.events
            .iter()
            .any(|s| s.starts_with("write:") || s.starts_with("raw:"))
    );
}
#[test]
fn invalid_nvm_checksum_is_not_bank_zero_fallback() {
    let mut f = Fake::new();
    f.corrupt = true;
    assert_eq!(reset::initialize(&mut f), Err(reset::Error::NvmChecksum));
    assert!(!f.events.iter().any(|s| s.starts_with("write:")));
}
#[test]
fn dma_active_is_rejected_before_hardware_reset() {
    let mut f = Fake::new();
    f.bme = true;
    assert_eq!(reset::initialize(&mut f), Err(reset::Error::DmaActive));
    assert!(!f.events.iter().any(|s| s.starts_with("write:")));
}
#[test]
fn blocked_reset_and_master_pending_are_bounded() {
    let mut f = Fake::new();
    f.regs.insert(0x5b54, 0x8000);
    assert_eq!(reset::initialize(&mut f), Err(reset::Error::ResetBlocked));
    assert!(!f.events.iter().any(|s| s.starts_with("raw:0:")));
    let mut f = Fake::new();
    f.regs.insert(8, 1 << 19);
    assert_eq!(reset::initialize(&mut f), Err(reset::Error::Timeout));
}
#[test]
fn reset_write_and_delay_failures_are_quarantined_without_mmio() {
    for delay in [false, true] {
        let mut f = Fake::new();
        f.fail_reset = !delay;
        f.fail_delay = delay;
        assert_eq!(
            reset::initialize(&mut f),
            Err(reset::Error::ResetUnconfirmed)
        );
        assert_eq!(f.pending, 1);
    }
}
#[test]
fn reset_ownership_or_missing_autoload_never_yields_ready() {
    let mut f = Fake::new();
    f.held_after = true;
    assert_eq!(
        reset::initialize(&mut f),
        Err(reset::Error::ResetUnconfirmed)
    );
    let mut f = Fake::new();
    f.no_init = true;
    assert_eq!(reset::initialize(&mut f), Err(reset::Error::Timeout));
}
#[test]
fn software_ulp_exits_without_me_and_preserves_no_dma() {
    let mut f = Fake::new();
    f.regs.insert(0x5b54, 0x40);
    f.regs.insert(0x18, 4);
    reset::initialize(&mut f).unwrap();
    assert!(f.events.iter().any(|s| s == "write:3c:8000000"));
    assert!(f.events.iter().any(|s| s == "delay:1000"));
    assert!(f.events.iter().any(|s| s == "delay:50000"));
    assert!(!f.bme);
}
#[test]
fn link_configures_fd_autoneg_and_validates_mac_without_dma() {
    let mut f = Fake::new();
    f.regs.insert(8, 3);
    f.regs.insert(0x5400, 0x44332202);
    f.regs.insert(0x5404, 0x80006655);
    assert_eq!(
        i218::link::initialize(&mut f),
        Ok([2, 0x22, 0x33, 0x44, 0x55, 0x66])
    );
    assert_eq!(f.regs[&0x3828] & 0x013f003f, 0x0101001f);
    assert_eq!(f.regs[&0] & ((1 << 11) | (1 << 12) | 4), 0);
    assert_eq!(f.regs[&0x2160], 0);
    assert_eq!(f.regs[&0x2168], 0);
    assert_eq!(f.regs[&i218::EXTCNF_CTRL] & 0x20, 0);
    assert!(!f.bme);
}
#[test]
fn link_failure_never_initializes_dma_or_returns_mac() {
    let mut f = Fake::new();
    f.regs.insert(8, 2);
    assert_eq!(i218::link::initialize(&mut f), Err(Error::LinkMode));
    let mut f = Fake::new();
    f.regs.insert(8, 0);
    assert_eq!(i218::link::initialize(&mut f), Err(Error::LinkTimeout));
    let mut f = Fake::new();
    f.regs.insert(8, 3);
    assert_eq!(i218::link::initialize(&mut f), Err(Error::InvalidMac));
    let mut f = Fake::new();
    f.bme = true;
    assert_eq!(i218::link::initialize(&mut f), Err(Error::Resource));
    assert_eq!(f.events, ["pci"]);
}
#[test]
fn master_stop_requires_bme_off_and_drained_device_before_reuse() {
    let mut f = Fake::new();
    assert_eq!(i218::link::stop_master(&mut f, false), Err(Error::Resource));
    assert!(f.events.is_empty());
    let mut f = Fake::new();
    i218::link::stop_master(&mut f, true).unwrap();
    assert_eq!(f.regs[&0x100], 0);
    assert_eq!(f.regs[&0x400], 0);
    let mut f = Fake::new();
    f.regs.insert(8, 1 << 19);
    assert_eq!(
        i218::link::stop_master(&mut f, true),
        Err(Error::MasterPending)
    );
}
