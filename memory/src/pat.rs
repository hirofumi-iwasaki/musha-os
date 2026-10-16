// Copyright 2026 Hirofumi Iwasaki
// SPDX-License-Identifier: Apache-2.0
//! Select existing firmware PAT entries. Never modifies PAT or MTRRs.
use crate::Error;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Indices {
    pub wb: u8,
    pub uc: u8,
    pub framebuffer: u8,
}
impl Indices {
    /// RAM remains WB and DMA/device registers remain strong UC. Only the
    /// framebuffer may select an existing WC entry; otherwise it stays UC.
    pub fn select(pat: u64) -> Result<Self, Error> {
        let find =
            |kind: u8| (0u8..8).find(|i| ((pat >> (usize::from(*i) * 8)) & 255) == u64::from(kind));
        let wb = find(6).ok_or(Error::Unsupported)?;
        let uc = find(0).ok_or(Error::Unsupported)?;
        Ok(Self {
            wb,
            uc,
            framebuffer: find(1).unwrap_or(uc),
        })
    }
    pub fn framebuffer_wc(self) -> bool {
        self.framebuffer != self.uc
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    const DEFAULT: u64 = 0x0007_0406_0007_0406;
    #[test]
    fn reset_pat_falls_back_to_strong_uc_without_treating_uc_minus_as_wc() {
        assert_eq!(
            Indices::select(DEFAULT),
            Ok(Indices {
                wb: 0,
                uc: 3,
                framebuffer: 3
            })
        );
        assert!(!Indices::select(DEFAULT).unwrap().framebuffer_wc());
    }
    #[test]
    fn every_available_wc_slot_is_selected_without_changing_ram_or_device_indices() {
        for slot in 0..8 {
            let pat = (DEFAULT & !(255u64 << (slot * 8))) | (1u64 << (slot * 8));
            let selected = Indices::select(pat).unwrap();
            assert_eq!(selected.framebuffer, slot as u8);
            assert!(selected.framebuffer_wc());
            assert_eq!((pat >> (selected.wb * 8)) & 255, 6);
            assert_eq!((pat >> (selected.uc * 8)) & 255, 0);
        }
    }
    #[test]
    fn missing_required_ram_or_device_type_is_rejected() {
        assert_eq!(
            Indices::select(0x0101_0101_0101_0101),
            Err(Error::Unsupported)
        );
        assert_eq!(
            Indices::select(0x0606_0606_0606_0606),
            Err(Error::Unsupported)
        );
        assert_eq!(Indices::select(0), Err(Error::Unsupported));
    }
}
