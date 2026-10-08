// Copyright 2026 Hirofumi Iwasaki
// SPDX-License-Identifier: Apache-2.0
#![no_std]
//! Pure validation for the dedicated legacy 82574 descriptor layout.
pub const DMA_BYTES: usize = 65536;
pub const RX_COUNT: usize = 16;
pub const TX_COUNT: usize = 8;
pub const TX_RING: usize = 4096;
pub const RX_BUFFERS: usize = 8192;
pub const TX_BUFFERS: usize = 40960;
pub const BUFFER_BYTES: usize = 2048;
pub fn valid_dma(base: usize, bytes: usize) -> bool {
    base != 0
        && base % 4096 == 0
        && bytes >= DMA_BYTES
        && base
            .checked_add(DMA_BYTES)
            .is_some_and(|end| end as u64 <= 0x1_0000_0000)
}
pub fn valid_rx(status: u8, errors: u8, length: usize) -> bool {
    status & 3 == 3 && errors == 0 && (14..=1514).contains(&length)
}
pub fn tx_word(length: usize) -> Option<u64> {
    (14..=1514)
        .contains(&length)
        .then_some(length as u64 | (0x0b << 24))
}
/// Completion state retains buffer ownership until a clean DD observation.
#[derive(Debug, PartialEq, Eq)]
pub enum TxStatus {
    Pending,
    Complete,
    Error,
    Timeout,
}
pub struct TxFlight {
    deadline: u64,
    last: u64,
    observations: u32,
}
impl TxFlight {
    pub fn new(now: u64) -> Option<Self> {
        Some(Self {
            deadline: now.checked_add(100)?,
            last: now,
            observations: 0,
        })
    }
    pub fn observe(&mut self, status: u8, now: u64) -> TxStatus {
        if now < self.last {
            return TxStatus::Error;
        }
        self.last = now;
        self.observations = self.observations.saturating_add(1);
        if status & 1 != 0 {
            return if status & 14 != 0 {
                TxStatus::Error
            } else {
                TxStatus::Complete
            };
        }
        if now >= self.deadline || self.observations >= 5_000_000 {
            TxStatus::Timeout
        } else {
            TxStatus::Pending
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn layout_is_disjoint_and_within_reserved_region() {
        assert!(RX_COUNT * 16 <= TX_RING);
        assert!(TX_RING + TX_COUNT * 16 <= RX_BUFFERS);
        assert!(RX_BUFFERS + RX_COUNT * BUFFER_BYTES <= TX_BUFFERS);
        assert!(TX_BUFFERS + TX_COUNT * BUFFER_BYTES <= DMA_BYTES);
        assert!(valid_dma(0xffff0000, DMA_BYTES));
        for (base, bytes) in [
            (0, DMA_BYTES),
            (4097, DMA_BYTES),
            (4096, DMA_BYTES - 1),
            (0xffff1000, DMA_BYTES),
            (usize::MAX - 4095, DMA_BYTES),
        ] {
            assert!(!valid_dma(base, bytes));
        }
    }
    #[test]
    fn receive_requires_complete_clean_bounded_frame() {
        assert!(valid_rx(3, 0, 14));
        assert!(valid_rx(3, 0, 1514));
        for (status, errors, len) in [
            (0, 0, 60),
            (1, 0, 60),
            (2, 0, 60),
            (3, 1, 60),
            (3, 0, 13),
            (3, 0, 1515),
            (3, 0, usize::MAX),
        ] {
            assert!(!valid_rx(status, errors, len));
        }
    }
    #[test]
    fn transmit_requests_crc_end_of_packet_and_completion() {
        assert_eq!(tx_word(1514), Some(0x0b0005ea));
        for length in [0, 13, 1515, usize::MAX] {
            assert!(tx_word(length).is_none());
        }
    }
}

#[cfg(test)]
mod flight_tests {
    use super::*;
    #[test]
    fn pending_then_completion_and_hardware_errors() {
        let mut f = TxFlight::new(100).unwrap();
        assert_eq!(f.observe(0, 100), TxStatus::Pending);
        assert_eq!(f.observe(0, 199), TxStatus::Pending);
        assert_eq!(f.observe(1, 199), TxStatus::Complete);
        for status in [3, 5, 9] {
            assert_eq!(
                TxFlight::new(0).unwrap().observe(status, 1),
                TxStatus::Error
            );
        }
    }
    #[test]
    fn deadlines_stalled_clock_and_overflow() {
        assert!(TxFlight::new(u64::MAX).is_none());
        assert_eq!(TxFlight::new(0).unwrap().observe(0, 100), TxStatus::Timeout);
        assert_eq!(TxFlight::new(2).unwrap().observe(0, 1), TxStatus::Error);
        let mut f = TxFlight::new(0).unwrap();
        for _ in 0..4_999_999 {
            assert_eq!(f.observe(0, 0), TxStatus::Pending);
        }
        assert_eq!(f.observe(0, 0), TxStatus::Timeout);
    }
}
