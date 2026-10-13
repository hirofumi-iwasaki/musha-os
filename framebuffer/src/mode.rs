// SPDX-License-Identifier: Apache-2.0
//! Firmware-independent GOP selection policy. Mode queries must release their buffers.
#[derive(Clone, Copy)]
pub struct Info {
    pub width: u32,
    pub height: u32,
    pub stride: u32,
    pub format: u32,
}
impl Info {
    fn area(self) -> Option<u64> {
        (self.format <= 1 && self.width > 0 && self.height > 0 && self.stride >= self.width)
            .then_some(u64::from(self.width) * u64::from(self.height))
    }
}
pub trait Modes {
    fn count(&self) -> u32;
    fn current(&self) -> u32;
    fn current_info(&self) -> Option<Info>;
    fn query(&mut self, index: u32) -> Option<Info>;
    fn set(&mut self, index: u32) -> bool;
}
/// Query every mode, prefer the current mode on ties, and attempt only the maximum.
/// A failed SetMode leaves the firmware's current mode in use; no lower-mode retry.
pub fn select(modes: &mut impl Modes) {
    let current = modes.current();
    let mut best = modes
        .current_info()
        .and_then(|i| i.area())
        .map(|a| (current, a));
    for index in 0..modes.count() {
        if let Some(area) = modes.query(index).and_then(|i| i.area()) {
            if best.is_none_or(|(_, old)| area > old) {
                best = Some((index, area));
            }
        }
    }
    if let Some((index, _)) = best {
        if index != current {
            let _ = modes.set(index);
        }
    }
}
#[cfg(test)]
mod tests {
    extern crate std;
    use super::*;
    use std::vec::Vec;
    struct Mock {
        entries: Vec<Option<Info>>,
        current: u32,
        queries: Vec<u32>,
        sets: Vec<u32>,
        fail: bool,
    }
    impl Modes for Mock {
        fn count(&self) -> u32 {
            self.entries.len() as u32
        }
        fn current(&self) -> u32 {
            self.current
        }
        fn current_info(&self) -> Option<Info> {
            self.entries[self.current as usize]
        }
        fn query(&mut self, i: u32) -> Option<Info> {
            self.queries.push(i);
            self.entries[i as usize]
        }
        fn set(&mut self, i: u32) -> bool {
            self.sets.push(i);
            if !self.fail {
                self.current = i;
            }
            !self.fail
        }
    }
    fn info(w: u32, h: u32, f: u32) -> Option<Info> {
        Some(Info {
            width: w,
            height: h,
            stride: w,
            format: f,
        })
    }
    fn mock(entries: Vec<Option<Info>>) -> Mock {
        Mock {
            entries,
            current: 0,
            queries: Vec::new(),
            sets: Vec::new(),
            fail: false,
        }
    }
    #[test]
    fn largest_rgb_bgr_not_largest_width() {
        let mut m = mock(std::vec![
            info(800, 600, 0),
            info(1920, 1080, 1),
            info(2560, 720, 0),
            info(4096, 2160, 2),
            info(8192, 4320, 3)
        ]);
        select(&mut m);
        assert_eq!(m.current, 1);
        assert_eq!(m.queries, std::vec![0, 1, 2, 3, 4]);
    }
    #[test]
    fn current_tie_avoids_reset() {
        let mut m = mock(std::vec![info(1024, 768, 1), info(1024, 768, 0)]);
        m.current = 1;
        select(&mut m);
        assert!(m.sets.is_empty());
        assert_eq!(m.queries.len(), 2);
    }
    #[test]
    fn failed_set_keeps_current_without_lower_retry() {
        let mut m = mock(std::vec![
            info(800, 600, 0),
            info(1280, 720, 0),
            info(1920, 1080, 1)
        ]);
        m.fail = true;
        select(&mut m);
        assert_eq!(m.current, 0);
        assert_eq!(m.sets, std::vec![2]);
    }
    #[test]
    fn failed_queries_and_invalid_layouts_are_skipped() {
        let mut m = mock(std::vec![
            info(800, 600, 0),
            None,
            info(0, 2000, 0),
            Some(Info {
                width: 4096,
                height: 2160,
                stride: 10,
                format: 0
            }),
            info(1024, 768, 1)
        ]);
        select(&mut m);
        assert_eq!(m.current, 4);
        assert_eq!(m.queries.len(), 5);
    }
    #[test]
    fn unavailable_queries_preserve_current() {
        let mut m = mock(std::vec![None, None]);
        select(&mut m);
        assert!(m.sets.is_empty());
        assert_eq!(m.current, 0);
    }
    #[test]
    fn area_uses_u64() {
        let mut m = mock(std::vec![info(65536, 65536, 0), info(100000, 100000, 1)]);
        select(&mut m);
        assert_eq!(m.current, 1);
    }
}
