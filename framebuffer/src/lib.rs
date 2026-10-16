// Copyright 2026 Hirofumi Iwasaki
// SPDX-License-Identifier: Apache-2.0
#![no_std]

pub mod metrics;
use core::sync::atomic::{AtomicBool, Ordering};
static OUTPUT_ENABLED: AtomicBool = AtomicBool::new(true);
/// BSP startup policy. Dimensions and address validation are never changed.
pub fn set_output_enabled(enabled: bool) {
    OUTPUT_ENABLED.store(enabled, Ordering::Relaxed);
}
pub struct OutputGuard(bool);
impl OutputGuard {
    pub fn allow() -> Self {
        Self(OUTPUT_ENABLED.swap(true, Ordering::Relaxed))
    }
}
impl Drop for OutputGuard {
    fn drop(&mut self) {
        OUTPUT_ENABLED.store(self.0, Ordering::Relaxed);
    }
}

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
    /// Complete a drawing batch before handing off the framebuffer or halting.
    /// Volatile pixels preserve values but do not drain x86 WC store buffers.
    /// This is not display scanout/vsync synchronization. Call once per batch,
    /// never once per pixel. It does not touch the framebuffer address.
    pub fn flush_writes(&self) {
        flush_writes();
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
        if !OUTPUT_ENABLED.load(Ordering::Relaxed) {
            return;
        }
        if let Some(offset) = self.offset(x, y) {
            // SAFETY: caller guarantees a mapped framebuffer; checked offset
            // and 4-byte alignment keep this volatile store inside its bounds.
            unsafe {
                core::ptr::write_volatile((self.base + offset) as *mut u32, color);
            }
        }
    }
    /// Draw bounded grid text, excluding the two-cell screen margin.
    /// # Safety
    /// Same mapping/ownership requirements as pixel.
    pub unsafe fn text_grid(&self, text: &str, col: usize, row: usize, color: u32) {
        let m = metrics::Metrics::new(self.width, self.height);
        if row < metrics::MARGIN
            || row >= m.rows.saturating_sub(metrics::MARGIN)
            || col < metrics::MARGIN
            || col >= m.columns.saturating_sub(metrics::MARGIN)
        {
            return;
        }
        let shown = if text.len() == 16 && text.bytes().all(|b| b.is_ascii_hexdigit()) {
            let s = text.trim_start_matches('0');
            if s.is_empty() { "0" } else { s }
        } else {
            text
        };
        let end = m
            .columns
            .saturating_sub(metrics::MARGIN + col)
            .min(shown.len());
        let mut end = end;
        while !shown.is_char_boundary(end) {
            end -= 1;
        }
        unsafe {
            self.text_scaled(&shown[..end], m.x(col), m.y(row), color, m.scale);
        }
    }
    /// Legacy internal pixel API, without a diagnostic coordinate transform.
    /// # Safety
    /// Same mapping/ownership requirements as pixel.
    pub unsafe fn text(&self, text: &str, x: usize, y: usize, color: u32) {
        let scale = metrics::scale_for_height(self.height);
        unsafe {
            self.text_scaled(text, x, y, color, scale);
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
/// Complete an OS drawing batch without exposing framebuffer memory.
pub fn flush_writes() {
    #[cfg(target_arch = "x86_64")]
    unsafe {
        core::arch::asm!("sfence", options(nostack, preserves_flags));
    }
}
// Original minimal 5x7 bitmap glyphs for this boot milestone, not a third-party font.
fn glyph(ch: u8) -> [u8; 7] {
    match ch {
        98 => [16, 16, 22, 25, 17, 17, 30],
        99 => [0, 0, 14, 16, 16, 17, 14],
        100 => [1, 1, 13, 19, 17, 17, 15],
        102 => [6, 9, 8, 28, 8, 8, 8],
        103 => [0, 14, 17, 17, 15, 1, 14],
        105 => [4, 0, 12, 4, 4, 4, 14],
        106 => [2, 0, 6, 2, 2, 18, 12],
        107 => [16, 16, 18, 20, 24, 20, 18],
        109 => [0, 0, 26, 21, 21, 21, 21],
        110 => [0, 0, 22, 25, 17, 17, 17],
        112 => [0, 30, 17, 17, 30, 16, 16],
        113 => [0, 15, 17, 17, 15, 1, 1],
        114 => [0, 0, 22, 25, 16, 16, 16],
        116 => [8, 8, 28, 8, 8, 9, 6],
        118 => [0, 0, 17, 17, 17, 10, 4],
        119 => [0, 0, 17, 17, 21, 21, 10],
        120 => [0, 0, 17, 10, 4, 10, 17],
        121 => [0, 17, 17, 17, 15, 1, 14],
        122 => [0, 0, 31, 2, 4, 8, 31],
        34 => [10, 10, 10, 0, 0, 0, 0],
        35 => [10, 31, 10, 10, 31, 10, 0],
        36 => [4, 15, 20, 14, 5, 30, 4],
        37 => [25, 25, 2, 4, 8, 19, 19],
        38 => [12, 18, 20, 8, 21, 18, 13],
        39 => [4, 4, 8, 0, 0, 0, 0],
        40 => [2, 4, 8, 8, 8, 4, 2],
        41 => [8, 4, 2, 2, 2, 4, 8],
        42 => [0, 21, 14, 31, 14, 21, 0],
        43 => [0, 4, 4, 31, 4, 4, 0],
        44 => [0, 0, 0, 0, 4, 4, 8],
        59 => [0, 4, 4, 0, 4, 4, 8],
        60 => [1, 2, 4, 8, 4, 2, 1],
        62 => [16, 8, 4, 2, 4, 8, 16],
        63 => [14, 17, 1, 2, 4, 0, 4],
        64 => [14, 17, 23, 21, 23, 16, 14],
        91 => [14, 8, 8, 8, 8, 8, 14],
        92 => [16, 16, 8, 4, 2, 1, 1],
        93 => [14, 2, 2, 2, 2, 2, 14],
        94 => [4, 10, 17, 0, 0, 0, 0],
        95 => [0, 0, 0, 0, 0, 0, 31],
        96 => [8, 4, 2, 0, 0, 0, 0],
        123 => [2, 4, 4, 8, 4, 4, 2],
        124 => [4, 4, 4, 4, 4, 4, 4],
        125 => [8, 4, 4, 2, 4, 4, 8],
        126 => [0, 0, 8, 21, 2, 0, 0],

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
    fn printable_ascii_is_visible() {
        for c in 33..=126 {
            assert_ne!(glyph(c), [0; 7], "ASCII {c}");
        }
        assert_eq!(glyph(b' '), [0; 7]);
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

pub mod mode;

pub mod boot_console;
