// Copyright 2026 Hirofumi Iwasaki
// SPDX-License-Identifier: Apache-2.0
#![no_std]

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Framebuffer {
    pub base: usize,
    pub bytes: usize,
    pub width: usize,
    pub height: usize,
    pub stride: usize,
    pub format: u32,
}
impl Framebuffer {
    pub fn valid(&self) -> bool {
        self.base != 0
            && self.base % 4 == 0
            && self.width > 0
            && self.height > 0
            && self.stride >= self.width
            && self.format <= 1
            && self
                .stride
                .checked_mul(self.height)
                .and_then(|v| v.checked_mul(4))
                .is_some_and(|v| v <= self.bytes && self.base.checked_add(v).is_some())
    }
    pub fn offset(&self, x: usize, y: usize) -> Option<usize> {
        if !self.valid() || x >= self.width || y >= self.height {
            return None;
        }
        y.checked_mul(self.stride)?.checked_add(x)?.checked_mul(4)
    }
    pub fn color(&self, r: u8, g: u8, b: u8) -> u32 {
        if self.format == 0 {
            u32::from(r) | u32::from(g) << 8 | u32::from(b) << 16
        } else {
            u32::from(b) | u32::from(g) << 8 | u32::from(r) << 16
        }
    }
    /// # Safety
    /// base must refer to writable, identity-mapped GOP memory of bytes length.
    /// The caller exclusively controls drawing and keeps the mapping alive.
    pub unsafe fn pixel(&self, x: usize, y: usize, color: u32) {
        if let Some(offset) = self.offset(x, y) {
            // SAFETY: caller guarantees a mapped framebuffer; checked offset
            // and 4-byte alignment keep this volatile store inside its bounds.
            unsafe {
                core::ptr::write_volatile((self.base + offset) as *mut u32, color);
            }
        }
    }
    /// # Safety
    /// Same mapping/ownership requirements as pixel.
    pub unsafe fn text(&self, text: &str, x: usize, y: usize, color: u32) {
        unsafe {
            // Presentation coordinates retain the existing application layout contract.
            // Keep 42-pixel glyphs even on 768/800-high GOP modes.
            let py = y.saturating_mul(self.height.saturating_sub(48).min(1100)) / 550;
            let px = if x >= 240 { x.saturating_mul(2) } else { x };
            // Counts/addresses on the visitor panel need not carry sixteen zeros.
            let shown = if text.len() == 16 && text.bytes().all(|b| b.is_ascii_hexdigit()) {
                let short = text.trim_start_matches('0');
                if short.is_empty() { "0" } else { short }
            } else {
                text
            };
            self.text_scaled(shown, px, py, color, 6);
        }
    }
    /// # Safety
    /// Same mapping/ownership requirements as pixel.
    pub unsafe fn text_small(&self, text: &str, x: usize, y: usize, color: u32) {
        unsafe {
            self.text_scaled(text, x, y, color, 1);
        }
    }
    pub unsafe fn text_scaled(&self, text: &str, x: usize, y: usize, color: u32, scale: usize) {
        for (i, ch) in text.bytes().enumerate() {
            let Some(left) = i.checked_mul(6 * scale).and_then(|v| x.checked_add(v)) else {
                break;
            };
            for (row, bits) in glyph(ch).iter().enumerate() {
                for col in 0..5 {
                    if bits & (1 << (4 - col)) != 0 {
                        for dy in 0..scale {
                            for dx in 0..scale {
                                if let (Some(px), Some(py)) = (
                                    left.checked_add(col * scale + dx),
                                    y.checked_add(row * scale + dy),
                                ) {
                                    unsafe {
                                        self.pixel(px, py, color);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
// Original minimal 5x7 bitmap glyphs for this boot milestone, not a third-party font.
fn glyph(ch: u8) -> [u8; 7] {
    match ch {
        b'G' => [14, 17, 16, 23, 17, 17, 14],
        b'J' => [7, 2, 2, 2, 18, 18, 12],
        b'Q' => [14, 17, 17, 17, 21, 18, 13],
        b'Z' => [31, 1, 2, 4, 8, 16, 31],
        b'.' => [0, 0, 0, 0, 0, 4, 4],
        b':' => [0, 4, 4, 0, 4, 4, 0],
        b'/' => [1, 1, 2, 4, 8, 16, 16],
        b'=' => [0, 0, 31, 0, 31, 0, 0],
        b'H' => [17, 17, 17, 31, 17, 17, 17],
        b'e' => [0, 0, 14, 17, 31, 16, 14],
        b'l' => [12, 4, 4, 4, 4, 4, 14],
        b'o' => [0, 0, 14, 17, 17, 17, 14],
        b'M' => [17, 27, 21, 21, 17, 17, 17],
        b'u' => [0, 0, 17, 17, 17, 19, 13],
        b's' => [0, 0, 15, 16, 14, 1, 30],
        b'h' => [16, 16, 22, 25, 17, 17, 17],
        b'a' => [0, 0, 14, 1, 15, 17, 15],
        b'-' => [0, 0, 0, 31, 0, 0, 0],
        b'O' => [14, 17, 17, 17, 17, 17, 14],
        b'S' => [15, 16, 16, 14, 1, 1, 30],
        b'!' => [4, 4, 4, 4, 4, 0, 4],
        b'R' => [30, 17, 17, 30, 20, 18, 17],
        b'U' => [17, 17, 17, 17, 17, 17, 14],
        b'N' => [17, 25, 25, 21, 19, 19, 17],
        b'T' => [31, 4, 4, 4, 4, 4, 4],
        b'I' => [14, 4, 4, 4, 4, 4, 14],
        b'E' => [31, 16, 16, 30, 16, 16, 31],
        b'A' => [14, 17, 17, 31, 17, 17, 17],
        b'D' => [30, 17, 17, 17, 17, 17, 30],
        b'Y' => [17, 17, 10, 4, 4, 4, 4],
        b'B' => [30, 17, 17, 30, 17, 17, 30],
        b'F' => [31, 16, 16, 30, 16, 16, 16],
        b'C' => [14, 17, 16, 16, 16, 17, 14],
        b'V' => [17, 17, 17, 17, 17, 10, 4],
        b'W' => [17, 17, 17, 21, 21, 27, 17],
        b'X' => [17, 17, 10, 4, 10, 17, 17],
        b'P' => [30, 17, 17, 30, 16, 16, 16],
        b'L' => [16, 16, 16, 16, 16, 16, 31],
        b'K' => [17, 18, 20, 24, 20, 18, 17],
        b'0' => [14, 17, 19, 21, 25, 17, 14],
        b'1' => [4, 12, 4, 4, 4, 4, 14],
        b'2' => [14, 17, 1, 2, 4, 8, 31],
        b'3' => [30, 1, 1, 14, 1, 1, 30],
        b'4' => [2, 6, 10, 18, 31, 2, 2],
        b'5' => [31, 16, 16, 30, 1, 1, 30],
        b'6' => [14, 16, 16, 30, 17, 17, 14],
        b'7' => [31, 1, 2, 4, 8, 8, 8],
        b'8' => [14, 17, 17, 14, 17, 17, 14],
        b'9' => [14, 17, 17, 15, 1, 1, 14],
        _ => [0; 7],
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn fb() -> Framebuffer {
        Framebuffer {
            base: 4096,
            bytes: 48,
            width: 3,
            height: 2,
            stride: 6,
            format: 0,
        }
    }
    #[test]
    fn padded_stride_and_bounds() {
        let f = fb();
        assert!(f.valid());
        assert_eq!(f.offset(2, 1), Some(32));
        assert_eq!(f.offset(3, 0), None);
        assert_eq!(f.offset(0, 2), None);
    }
    #[test]
    fn rejects_invalid_layouts() {
        for f in [
            Framebuffer { bytes: 47, ..fb() },
            Framebuffer { stride: 2, ..fb() },
            Framebuffer {
                base: usize::MAX - 3,
                ..fb()
            },
            Framebuffer {
                stride: usize::MAX,
                ..fb()
            },
            Framebuffer { format: 3, ..fb() },
            Framebuffer { base: 1, ..fb() },
        ] {
            assert!(!f.valid());
        }
    }
    #[test]
    fn drawing_preserves_padding_and_guard_words() {
        let mut memory = [0xdeadbeefu32; 14];
        let f = Framebuffer {
            base: memory.as_mut_ptr() as usize,
            ..fb()
        };
        unsafe {
            f.pixel(2, 1, 0x123456);
            f.pixel(3, 0, 0);
            f.pixel(0, 2, 0);
        }
        assert_eq!(memory[8], 0x123456);
        for (i, value) in memory.iter().enumerate() {
            if i != 8 {
                assert_eq!(*value, 0xdeadbeef);
            }
        }
    }
    #[test]
    fn small_text_clips_without_touching_padding_or_guards() {
        let mut memory = [0xdeadbeefu32; 42];
        let f = Framebuffer {
            base: memory.as_mut_ptr() as usize,
            bytes: 40 * 4,
            width: 3,
            height: 8,
            stride: 5,
            format: 0,
        };
        unsafe {
            f.text_small("GQ:Z", 1, 2, 0x123456);
            f.text_small("A", usize::MAX, usize::MAX, 0);
        }
        assert!(memory[..40].contains(&0x123456));
        for y in 0..8 {
            assert_eq!(&memory[y * 5 + 3..y * 5 + 5], &[0xdeadbeef; 2]);
        }
        assert_eq!(&memory[40..], &[0xdeadbeef; 2]);
    }
    #[test]
    fn pixel_formats() {
        assert_eq!(fb().color(0x12, 0x34, 0x56), 0x563412);
        assert_eq!(
            Framebuffer { format: 1, ..fb() }.color(0x12, 0x34, 0x56),
            0x123456
        );
    }
}
