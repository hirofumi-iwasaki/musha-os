// SPDX-License-Identifier: Apache-2.0
//! Bounded kernel console model. Bytes outside printable ASCII become '.'.
use crate::Framebuffer;
/// Keep the current QEMU 2048-high GOP mode at the native regression scale.
pub const fn scale_for_height(height: usize) -> usize {
    if height < 2160 {
        1
    } else if height < 2880 {
        2
    } else {
        3
    }
}
pub const COLS: usize = 80;
pub const ROWS: usize = 24;
pub struct Console {
    cells: [[u8; COLS]; ROWS],
    scale: usize,
    columns: usize,
    rows: usize,
    col: usize,
    row: usize,
}
impl Default for Console {
    fn default() -> Self {
        Self::new()
    }
}
impl Console {
    pub const fn new() -> Self {
        Self {
            cells: [[b' '; COLS]; ROWS],
            scale: 1,
            columns: COLS,
            rows: ROWS,
            col: 0,
            row: 0,
        }
    }
    pub fn for_framebuffer(fb: Framebuffer) -> Self {
        let mut c = Self::new();
        c.scale = scale_for_height(fb.height);
        c.columns = (fb.width.saturating_sub(16) / (6 * c.scale)).clamp(1, COLS);
        c.rows = (fb.height.saturating_sub(16) / (8 * c.scale)).clamp(1, ROWS);
        c
    }
    pub fn line(&self, row: usize) -> &[u8] {
        &self.cells[row]
    }
    fn newline(&mut self) {
        self.col = 0;
        if self.row + 1 == self.rows {
            self.cells.copy_within(1..self.rows, 0);
            self.cells[self.rows - 1].fill(b' ');
        } else {
            self.row += 1;
        }
    }
    pub fn append(&mut self, bytes: &[u8]) {
        for &b in bytes {
            if b == b'\n' {
                self.newline();
                continue;
            }
            if self.col == self.columns {
                self.newline();
            }
            self.cells[self.row][self.col] = if (32..=126).contains(&b) { b } else { b'.' };
            self.col += 1;
        }
    }
    /// Caller exclusively owns a valid identity-mapped framebuffer. No user pointers.
    pub unsafe fn render(&self, fb: Framebuffer) {
        // Resolution-aware text; wrap/scroll within the visible cell grid.
        let scale = self.scale;
        let width = self.columns * 6 * scale;
        let height = self.rows * 8 * scale;
        for y in 16..(16 + height).min(fb.height) {
            for x in 16..(16 + width).min(fb.width) {
                unsafe {
                    fb.pixel(x, y, fb.color(12, 20, 32));
                }
            }
        }
        for row in 0..self.rows {
            let text = unsafe { core::str::from_utf8_unchecked(&self.cells[row][..self.columns]) };
            unsafe {
                fb.text_scaled(
                    text,
                    16,
                    16 + row * 8 * scale,
                    fb.color(240, 240, 240),
                    scale,
                );
            }
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn wrap_and_newline_at_boundary() {
        let mut c = Console::new();
        c.append(&[b'A'; 80]);
        c.append(b"\nB");
        assert_eq!(c.line(0), &[b'A'; 80]);
        assert_eq!(c.line(1)[0], b'B');
        c.append(&[b'C'; 80]);
        assert_eq!(c.line(2)[0], b'C');
    }
    #[test]
    fn oldest_row_is_discarded() {
        let mut c = Console::new();
        for _ in 0..24 {
            c.append(b"old\n");
        }
        c.append(b"new");
        assert_eq!(&c.line(0)[..3], b"old");
        assert_eq!(&c.line(23)[..3], b"new");
    }
    #[test]
    fn replacements_and_4096_bytes() {
        let mut c = Console::new();
        c.append("A\t\r\0日本\nB".as_bytes());
        assert_eq!(&c.line(0)[..10], b"A.........");
        assert_eq!(c.line(1)[0], b'B');
        c.append(&[b'Z'; 4096]);
        assert!(c.line(0).iter().all(|&b| b == b'Z'));
        assert_eq!(c.line(23)[16], b'Z');
        assert_eq!(c.line(23)[17], b' ');
    }
    #[test]
    fn visible_grid_wraps_and_scrolls_at_log_scale() {
        let fb = Framebuffer {
            base: 0,
            bytes: 0,
            width: 1600,
            height: 1200,
            stride: 1600,
            format: 0,
        };
        let mut c = Console::for_framebuffer(fb);
        assert_eq!((c.columns, c.rows, c.scale), (80, 24, 1));
        c.append(&[b'A'; 80]);
        c.append(b"B");
        assert_eq!(c.line(1)[0], b'B');
        for _ in 0..24 {
            c.append(b"\n");
        }
        c.append(b"MUSHA.ELF exited: status 0");
        assert_eq!(
            &c.line(23)[..b"MUSHA.ELF exited: status 0".len()],
            b"MUSHA.ELF exited: status 0"
        );
        assert_eq!(scale_for_height(2048), 1);
        assert_eq!(scale_for_height(2159), 1);
        assert_eq!(scale_for_height(2160), 2);
        assert_eq!(scale_for_height(2879), 2);
        assert_eq!(scale_for_height(2880), 3);
        let tiny = Console::for_framebuffer(Framebuffer {
            width: 1,
            height: 1,
            ..fb
        });
        assert_eq!((tiny.columns, tiny.rows), (1, 1));
    }
    #[test]
    fn rendering_clips_and_preserves_outside_region() {
        extern crate std;
        let mut pixels = std::vec![0xdeadbeefu32;303*201+4];
        let fb = Framebuffer {
            base: pixels.as_mut_ptr() as usize,
            bytes: 303 * 201 * 4,
            width: 300,
            height: 201,
            stride: 303,
            format: 0,
        };
        let mut c = Console::new();
        c.append(b"Hello\nworld");
        unsafe { c.render(fb) };
        assert_eq!(pixels[0], 0xdeadbeef);
        for y in 0..201 {
            assert!(
                pixels[y * 303 + 300..(y + 1) * 303]
                    .iter()
                    .all(|&p| p == 0xdeadbeef)
            );
        }
        assert_eq!(&pixels[303 * 201..], &[0xdeadbeef; 4]);
    }
}
