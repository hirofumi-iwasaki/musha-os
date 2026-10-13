// SPDX-License-Identifier: Apache-2.0
//! Bounded kernel console model. Bytes outside printable ASCII become '.'.
use crate::Framebuffer;
pub const COLS: usize = 80;
pub const ROWS: usize = 24;
pub struct Console {
    cells: [[u8; COLS]; ROWS],
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
            col: 0,
            row: 0,
        }
    }
    pub fn line(&self, row: usize) -> &[u8] {
        &self.cells[row]
    }
    fn newline(&mut self) {
        self.col = 0;
        if self.row + 1 == ROWS {
            self.cells.copy_within(1..ROWS, 0);
            self.cells[ROWS - 1].fill(b' ');
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
            if self.col == COLS {
                self.newline();
            }
            self.cells[self.row][self.col] = if (32..=126).contains(&b) { b } else { b'.' };
            self.col += 1;
        }
    }
    /// Caller exclusively owns a valid identity-mapped framebuffer. No user pointers.
    pub unsafe fn render(&self, fb: Framebuffer) {
        // Fixed 80x24 cells, scale 2 where it fits, else scale 1; pixel() clips.
        let scale = if fb.width >= 992 && fb.height >= 416 {
            2
        } else {
            1
        };
        let width = COLS * 6 * scale;
        let height = ROWS * 8 * scale;
        for y in 16..(16 + height).min(fb.height) {
            for x in 16..(16 + width).min(fb.width) {
                unsafe {
                    fb.pixel(x, y, fb.color(12, 20, 32));
                }
            }
        }
        for row in 0..ROWS {
            let text = unsafe { core::str::from_utf8_unchecked(&self.cells[row]) };
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
