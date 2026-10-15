// Copyright 2026 Hirofumi Iwasaki
// SPDX-License-Identifier: Apache-2.0
use musha_net::i218::{
    Error,
    flash::{Io, Region},
};
struct Fake {
    mem: Vec<u8>,
    address: usize,
    status: u16,
    ctl: u16,
    region: u32,
    fail: bool,
    busy: bool,
    stuck: bool,
    cycles: usize,
}
impl Fake {
    fn new() -> Self {
        let mut mem = vec![0; 8192];
        mem[0x27] = 0x80;
        Self {
            mem,
            address: 0,
            status: 0x4000,
            ctl: 0xa000,
            region: 1 << 16,
            fail: false,
            busy: false,
            stuck: false,
            cycles: 0,
        }
    }
}
impl Io for Fake {
    fn read16(&mut self, r: usize) -> Result<u16, Error> {
        match r {
            4 => Ok(self.status | if self.busy { 0x20 } else { 0 }),
            6 => Ok(self.ctl),
            _ => panic!("read16"),
        }
    }
    fn write16(&mut self, r: usize, v: u16) -> Result<(), Error> {
        match r {
            4 => self.status &= !(v & 7),
            6 => {
                assert_eq!(v & 6, 0, "only READ cycle allowed");
                self.ctl = v;
                if v & 1 != 0 {
                    self.cycles += 1;
                    if !self.stuck {
                        self.status |= if self.fail { 3 } else { 1 };
                    }
                }
            }
            _ => panic!("write16"),
        };
        Ok(())
    }
    fn read32(&mut self, r: usize) -> Result<u32, Error> {
        match r {
            0 => Ok(self.region),
            0x10 => {
                let n = if self.ctl & 0x300 == 0 { 1 } else { 2 };
                assert!(self.address + n <= self.mem.len());
                let mut v = self.mem[self.address] as u32;
                if n == 2 {
                    v |= (self.mem[self.address + 1] as u32) << 8;
                }
                Ok(v)
            }
            _ => panic!("read32"),
        }
    }
    fn write32(&mut self, r: usize, v: u32) -> Result<(), Error> {
        assert_eq!(r, 8);
        self.address = v as usize;
        Ok(())
    }
    fn delay_us(&mut self, _: u32) -> Result<(), Error> {
        Ok(())
    }
}
#[test]
fn valid_bank_word_and_read_only_cycles() {
    let mut f = Fake::new();
    f.mem[12] = 0x34;
    f.mem[13] = 0x12;
    let r = Region::open(&mut f).unwrap();
    assert_eq!(r.word(&mut f, 6), Ok(0x1234));
    assert_eq!(f.cycles, 2);
    assert_eq!(f.ctl & 0xf800, 0xa000);
}
#[test]
fn second_bank_is_selected_without_invalid_fallback() {
    let mut f = Fake::new();
    f.mem[0x27] = 0;
    f.mem[4096 + 0x27] = 0x80;
    f.mem[4096] = 9;
    let r = Region::open(&mut f).unwrap();
    assert_eq!(r.word(&mut f, 0), Ok(9));
    let mut f = Fake::new();
    f.mem[0x27] = 0;
    assert!(Region::open(&mut f).is_err());
    assert_eq!(f.cycles, 2);
}
#[test]
fn invalid_region_and_word_bounds_rejected_before_cycle() {
    for v in [u32::MAX, 0, 2 | (1 << 16), 0x1fff | (0x1fff << 16)] {
        let mut f = Fake::new();
        f.region = v;
        assert!(Region::open(&mut f).is_err());
        assert_eq!(f.cycles, 0);
    }
    let mut f = Fake::new();
    let r = Region::open(&mut f).unwrap();
    let n = f.cycles;
    assert_eq!(r.word(&mut f, 2048), Err(Error::Nvm));
    assert_eq!(f.cycles, n);
}
#[test]
fn descriptor_error_cycle_error_and_timeouts_are_not_data() {
    let mut f = Fake::new();
    f.status = 0;
    assert!(Region::open(&mut f).is_err());
    let mut f = Fake::new();
    f.fail = true;
    assert!(Region::open(&mut f).is_err());
    let mut f = Fake::new();
    f.busy = true;
    assert!(matches!(Region::open(&mut f), Err(Error::NvmTimeout)));
    assert_eq!(f.cycles, 0);
    let mut f = Fake::new();
    f.stuck = true;
    assert!(matches!(Region::open(&mut f), Err(Error::NvmTimeout)));
    assert_eq!(f.cycles, 1);
}
