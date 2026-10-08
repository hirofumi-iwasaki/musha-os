// SPDX-License-Identifier: Apache-2.0
//! Owned, bounded ASCII diagnostic lines; no allocation or borrowed error text.
use core::fmt;
#[derive(Clone, Copy)]
pub struct Line {
    bytes: [u8; 64],
    len: usize,
}
impl Line {
    pub const EMPTY: Self = Self {
        bytes: [0; 64],
        len: 0,
    };
    pub fn as_str(&self) -> &str {
        // Only printable ASCII is stored by Write.
        unsafe { core::str::from_utf8_unchecked(&self.bytes[..self.len]) }
    }
}
impl fmt::Write for Line {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for b in s.bytes() {
            if self.len == self.bytes.len() {
                break;
            }
            self.bytes[self.len] = if (32..=126).contains(&b) { b } else { b'?' };
            self.len += 1;
        }
        Ok(())
    }
}
#[derive(Clone, Copy)]
pub struct Failure {
    pub stage: Line,
    pub error: Line,
}
impl Failure {
    pub const EMPTY: Self = Self {
        stage: Line::EMPTY,
        error: Line::EMPTY,
    };
    pub fn record(&mut self, stage: Line, error: Line) {
        // Cleanup changes the current stage; repeated propagation must retain
        // the original failure stage. A different failure supersedes it.
        if self.error.as_str() != error.as_str() {
            self.stage = stage;
            self.error = error;
        }
    }
}
/// Full panel occupies 384x240 pixels. Compact status uses the last seven rows.
pub fn layout(width: usize, height: usize) -> Option<(usize, usize, bool)> {
    if width >= 1024 && height >= 264 {
        Some((640, 24, true))
    } else if width >= 408 && height >= 824 {
        Some((24, 560, true))
    } else if width >= 8 && height >= 8 {
        Some((0, height - 8, false))
    } else {
        None
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use core::fmt::Write;
    #[test]
    fn bounds_and_ascii() {
        let mut l = Line::EMPTY;
        write!(&mut l, "{:04X}:{:04X}\n", 0x8086, 0x15a3).unwrap();
        assert_eq!(l.as_str(), "8086:15A3?");
        l.write_str(&"X".repeat(100)).unwrap();
        assert_eq!(l.as_str().len(), 64);
    }
    #[test]
    fn cleanup_preserves_failure() {
        let mut stage = Line::EMPTY;
        stage.write_str("RESET").unwrap();
        let mut error = Line::EMPTY;
        error.write_str("TIMEOUT").unwrap();
        let mut f = Failure::EMPTY;
        f.record(stage, error);
        let mut stopped = Line::EMPTY;
        stopped.write_str("STOPPED").unwrap();
        f.record(stopped, error);
        assert_eq!(f.stage.as_str(), "RESET");
    }
    #[test]
    fn panel_stays_inside_screen() {
        for (w, h) in [(1280, 800), (1024, 768), (640, 824), (320, 356), (7, 7)] {
            if let Some((x, y, full)) = layout(w, h) {
                assert!(x < w && y + 7 <= h);
                if full {
                    assert!(x + 384 <= w && y + 240 <= h);
                }
            } else {
                assert_eq!((w, h), (7, 7));
            }
        }
    }
}
