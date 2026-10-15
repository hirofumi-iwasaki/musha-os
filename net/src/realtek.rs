// Copyright 2026 Hirofumi Iwasaki
// SPDX-License-Identifier: Apache-2.0
//! Independent RTL8168 descriptor/reset prototype. No upstream code translated.
//! No measured/reviewed RTL8111H PHY/OCP profile exists: admission stays closed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Identity,
    UnverifiedSilicon,
    Io,
    Timeout,
    DmaActive,
    Length,
    Owned,
    DrainUnverified,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Identity {
    pub pci: u32,
    pub revision: Option<u8>,
    pub tx_config: u32,
}
pub struct Permit {
    _private: (),
}
/// PCI ID/revision alone do not authorize revision-dependent PHY/OCP writes.
/// Future admission requires a measured profile and its reviewed stop sequence.
pub fn admit(id: Identity) -> Result<Permit, Error> {
    if id.pci != 0x816810ec || id.tx_config == u32::MAX {
        return Err(Error::Identity);
    }
    Err(Error::UnverifiedSilicon)
}
pub trait Io {
    fn read8(&mut self, offset: u16) -> Result<u8, Error>;
    fn write8(&mut self, offset: u16, value: u8) -> Result<(), Error>;
    fn write16(&mut self, offset: u16, value: u16) -> Result<(), Error>;
    fn delay_us(&mut self, us: u32) -> Result<(), Error>;
    fn bus_master(&mut self) -> Result<bool, Error>;
    fn disable_bus_master(&mut self) -> Result<(), Error>;
}
pub fn reset(io: &mut impl Io, _permit: &Permit) -> Result<(), Error> {
    if io.bus_master()? {
        return Err(Error::DmaActive);
    }
    io.write16(0x3c, 0)?;
    io.write8(0x37, 0x10)?;
    for _ in 0..1000 {
        let c = io.read8(0x37)?;
        if c == 0xff {
            return Err(Error::Io);
        }
        if c & 0x10 == 0 {
            return Ok(());
        }
        io.delay_us(10)?;
    }
    Err(Error::Timeout)
}
/// Even successful register/BME disable is NOT proof that PCI writes drained.
/// The missing variant-specific drain proof forbids DMA-buffer reuse.
pub fn stop(io: &mut impl Io) -> Result<(), Error> {
    let result = (|| {
        io.write16(0x3c, 0)?;
        let c = io.read8(0x37)?;
        if c == 0xff {
            return Err(Error::Io);
        }
        io.write8(0x37, c & !0x0c)?;
        Ok(())
    })();
    let off = io.disable_bus_master().and_then(|_| {
        if io.bus_master()? {
            Err(Error::DmaActive)
        } else {
            Ok(())
        }
    });
    result?;
    off?;
    Err(Error::DrainUnverified)
}
/// One-buffer, no-offload, little-endian descriptor. CPU access and fences are
/// the future adapter's responsibility; OWN is published only after buffer data.
#[repr(C, align(16))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Descriptor {
    pub words: [u32; 4],
}
impl Descriptor {
    pub fn tx(address: u64, length: usize, end: bool) -> Result<Self, Error> {
        if address == 0
            || !(14..=1514).contains(&length)
            || address.checked_add(length as u64).is_none()
        {
            return Err(Error::Length);
        }
        Ok(Self {
            words: [
                0xb0000000 | if end { 0x40000000 } else { 0 } | length as u32,
                0,
                address as u32,
                (address >> 32) as u32,
            ],
        })
    }
    pub fn rx(address: u64, capacity: usize, end: bool) -> Result<Self, Error> {
        if address == 0
            || !(1518..=2048).contains(&capacity)
            || address.checked_add(capacity as u64).is_none()
        {
            return Err(Error::Length);
        }
        Ok(Self {
            words: [
                0x80000000 | if end { 0x40000000 } else { 0 } | capacity as u32,
                0,
                address as u32,
                (address >> 32) as u32,
            ],
        })
    }
    pub fn received(&self, capacity: usize) -> Result<usize, Error> {
        let w = self.words[0];
        if w & 0x80000000 != 0 {
            return Err(Error::Owned);
        }
        let n = (w & 0x3fff) as usize;
        if w & 0x30000000 != 0x30000000
            || w & (1 << 21) != 0
            || !(18..=1518).contains(&n)
            || n > capacity
        {
            return Err(Error::Length);
        }
        Ok(n - 4) // strip Ethernet FCS; lwIP receives the original Ethernet frame.
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    struct Fake {
        reads: u32,
        writes: u32,
        master: bool,
        stuck: bool,
        fail: bool,
    }
    impl Io for Fake {
        fn read8(&mut self, _: u16) -> Result<u8, Error> {
            self.reads += 1;
            Ok(if self.stuck { 0x10 } else { 0 })
        }
        fn write8(&mut self, _: u16, _: u8) -> Result<(), Error> {
            self.writes += 1;
            if self.fail { Err(Error::Io) } else { Ok(()) }
        }
        fn write16(&mut self, _: u16, _: u16) -> Result<(), Error> {
            self.writes += 1;
            Ok(())
        }
        fn delay_us(&mut self, _: u32) -> Result<(), Error> {
            Ok(())
        }
        fn bus_master(&mut self) -> Result<bool, Error> {
            Ok(self.master)
        }
        fn disable_bus_master(&mut self) -> Result<(), Error> {
            self.master = false;
            Ok(())
        }
    }
    fn fake() -> Fake {
        Fake {
            reads: 0,
            writes: 0,
            master: false,
            stuck: false,
            fail: false,
        }
    }
    #[test]
    fn unmeasured_identity_never_authorizes_writes() {
        for revision in [None, Some(0), Some(0x15), Some(0xff)] {
            assert!(matches!(
                admit(Identity {
                    pci: 0x816810ec,
                    revision,
                    tx_config: 0x54100000
                }),
                Err(Error::UnverifiedSilicon)
            ));
        }
        assert!(matches!(
            admit(Identity {
                pci: 0x10d38086,
                revision: Some(1),
                tx_config: 0
            }),
            Err(Error::Identity)
        ));
    }
    #[test]
    fn bounded_reset_and_active_dma_rejection() {
        let p = Permit { _private: () };
        let mut f = fake();
        reset(&mut f, &p).unwrap();
        assert_eq!(f.writes, 2);
        f.master = true;
        let writes = f.writes;
        assert_eq!(reset(&mut f, &p), Err(Error::DmaActive));
        assert_eq!(f.writes, writes);
        f.master = false;
        f.stuck = true;
        f.reads = 0;
        assert_eq!(reset(&mut f, &p), Err(Error::Timeout));
        assert_eq!(f.reads, 1000);
    }
    #[test]
    fn shutdown_never_invents_drain_proof_and_attempts_bme_off_on_error() {
        let mut f = fake();
        f.master = true;
        assert_eq!(stop(&mut f), Err(Error::DrainUnverified));
        assert!(!f.master);
        f.master = true;
        f.fail = true;
        assert_eq!(stop(&mut f), Err(Error::Io));
        assert!(!f.master);
    }
    #[test]
    fn descriptors_bound_addresses_lengths_ownership_and_fcs() {
        let d = Descriptor::tx(0x1234567800, 1514, true).unwrap();
        assert_eq!(d.words, [0xf00005ea, 0, 0x34567800, 0x12]);
        assert!(Descriptor::tx(u64::MAX, 14, false).is_err());
        assert!(Descriptor::tx(1, 1515, false).is_err());
        let mut r = Descriptor::rx(0x1000, 2048, true).unwrap();
        assert_eq!(r.received(2048), Err(Error::Owned));
        r.words[0] = 0x70000000 | 1518;
        assert_eq!(r.received(2048), Ok(1514));
        for w in [
            0x30000000 | 17,
            0x30000000 | 1519,
            0x30200000 | 64,
            0x10000000 | 64,
        ] {
            r.words[0] = w;
            assert!(r.received(2048).is_err());
        }
    }
}
