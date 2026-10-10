// SPDX-License-Identifier: Apache-2.0
//! Firmware-discovered controller scalars and independently reserved DMA slices.
pub const MAX_CONTROLLERS: usize = 8;
pub const DMA_SLICE: usize = 1024 * 1024;
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(C)]
pub struct Controller {
    pub base: usize,
    pub bytes: usize,
    pub bus: u8,
    pub device: u8,
    pub function: u8,
}
impl Controller {
    pub const NONE: Self = Self {
        base: 0,
        bytes: 0,
        bus: 0,
        device: 0,
        function: 0,
    };
    pub fn bdf(self) -> u16 {
        ((self.bus as u16) << 8) | ((self.device as u16) << 3) | self.function as u16
    }
}
#[derive(Clone, Copy)]
#[repr(C)]
pub struct Controllers {
    pub entries: [Controller; MAX_CONTROLLERS],
    pub count: usize,
    pub truncated: bool,
}
impl Controllers {
    pub const EMPTY: Self = Self {
        entries: [Controller::NONE; MAX_CONTROLLERS],
        count: 0,
        truncated: false,
    };
    pub fn insert(&mut self, c: Controller) -> Result<(), &'static str> {
        if c.base == 0
            || c.bytes == 0
            || c.base % 4096 != 0
            || c.bytes % 4096 != 0
            || c.base.checked_add(c.bytes).is_none()
            || c.device > 31
            || c.function > 7
        {
            return Err("CONTROLLER RANGE");
        }
        if let Some(old) = self.entries[..self.count]
            .iter()
            .find(|old| old.bdf() == c.bdf())
        {
            return if *old == c {
                Ok(())
            } else {
                Err("CONFLICTING BDF")
            };
        }
        if self.count == MAX_CONTROLLERS {
            self.truncated = true;
            return Ok(());
        }
        self.entries[self.count] = c;
        self.count += 1;
        self.entries[..self.count].sort_unstable_by_key(|c| c.bdf());
        Ok(())
    }
}
pub fn dma_slice(base: usize, total: usize, index: usize, count: usize) -> Option<usize> {
    if base == 0
        || base % 4096 != 0
        || !(1..=MAX_CONTROLLERS).contains(&count)
        || index >= count
        || total < count.checked_mul(DMA_SLICE)?
        || base.checked_add(total)? > 1usize << 32
    {
        return None;
    }
    base.checked_add(index.checked_mul(DMA_SLICE)?)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ordered_bounded_deduplicated_firmware_records() {
        let mut list = Controllers::EMPTY;
        for bus in (0..8).rev() {
            list.insert(Controller {
                base: 0x100000 + bus as usize * 4096,
                bytes: 4096,
                bus,
                device: 0,
                function: 0,
            })
            .unwrap();
        }
        assert_eq!(list.count, 8);
        assert_eq!(list.entries[0].bus, 0);
        assert_eq!(list.entries[7].bus, 7);
        list.insert(list.entries[0]).unwrap();
        assert!(!list.truncated);
        list.insert(Controller {
            base: 0x200000,
            bytes: 4096,
            bus: 8,
            device: 0,
            function: 0,
        })
        .unwrap();
        assert!(list.truncated);
        assert!(
            list.insert(Controller {
                base: 0x300000,
                ..list.entries[0]
            })
            .is_err()
        );
        let mut empty = Controllers::EMPTY;
        assert!(empty.insert(Controller::NONE).is_err());
    }
    #[test]
    fn pools_are_disjoint_and_below_four_gib() {
        let base = 0x1000000;
        for i in 0..8 {
            assert_eq!(
                dma_slice(base, 8 * DMA_SLICE, i, 8),
                Some(base + i * DMA_SLICE)
            );
        }
        assert_eq!(dma_slice(base, 8 * DMA_SLICE, 8, 8), None);
        assert_eq!(dma_slice(base, DMA_SLICE, 1, 2), None);
        assert_eq!(dma_slice(0xfffff000, DMA_SLICE, 0, 1), None);
        assert_eq!(dma_slice(0, DMA_SLICE, 0, 1), None);
    }
}
