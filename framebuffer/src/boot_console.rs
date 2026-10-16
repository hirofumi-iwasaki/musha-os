// Copyright 2026 Hirofumi Iwasaki
// SPDX-License-Identifier: Apache-2.0
//! Bounded GOP log. Use a static Console::new(); bind initializes in place.
use crate::{Framebuffer, glyph};
pub const MAX_COLUMNS: usize = 384;
pub const MAX_ROWS: usize = 128;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BindError {
    InvalidFramebuffer,
    TooSmall,
    GeometryTooLarge,
}
pub struct Console {
    cells: [[u8; MAX_COLUMNS]; MAX_ROWS],
    painted: [usize; MAX_ROWS],
    fb: Option<Framebuffer>,
    columns: usize,
    rows: usize,
    scale: usize,
    col: usize,
    row: usize,
    visible: bool,
    dirty: bool,
    pending_clear: bool,
}
impl Console {
    /// Fixed shadow about 49 KiB: initialize in static storage, not a boot stack.
    pub const fn new() -> Self {
        Self {
            cells: [[b' '; MAX_COLUMNS]; MAX_ROWS],
            painted: [0; MAX_ROWS],
            fb: None,
            columns: 0,
            rows: 0,
            scale: 1,
            col: 0,
            row: 0,
            visible: true,
            dirty: false,
            pending_clear: false,
        }
    }
    /// Never reads/writes framebuffer pixels. Rejects capacity overflow; 8K fits.
    /// # Safety
    /// GOP memory must remain writable and exclusively owned while bound.
    pub unsafe fn bind(&mut self, fb: Framebuffer) -> Result<(), BindError> {
        if !fb.valid() {
            return Err(BindError::InvalidFramebuffer);
        }
        let scale = (fb.height.saturating_add(300) / 600).clamp(1, 4);
        let columns = fb.width / (6 * scale);
        let rows = fb.height / (10 * scale);
        if columns < 3 || rows == 0 {
            return Err(BindError::TooSmall);
        }
        if columns > MAX_COLUMNS || rows > MAX_ROWS {
            return Err(BindError::GeometryTooLarge);
        }
        self.fb = Some(fb);
        self.columns = columns;
        self.rows = rows;
        self.scale = scale;
        self.col = 0;
        self.row = 0;
        self.visible = true;
        self.dirty = false;
        self.pending_clear = false;
        for row in &mut self.cells[..rows] {
            row[..columns].fill(b' ');
        }
        self.painted.fill(0);
        Ok(())
    }
    pub fn columns(&self) -> usize {
        self.columns
    }
    pub fn rows(&self) -> usize {
        self.rows
    }
    pub fn scale(&self) -> usize {
        self.scale
    }
    pub fn is_bound(&self) -> bool {
        self.fb.is_some()
    }
    pub fn is_visible(&self) -> bool {
        self.visible
    }
    pub fn line(&self, row: usize) -> Option<&[u8]> {
        (row < self.rows).then(|| &self.cells[row][..self.columns])
    }
    fn paint(&self, col: usize, row: usize) {
        let Some(fb) = self.fb else {
            return;
        };
        let bits = glyph(self.cells[row][col]);
        for y in 0..8 {
            for x in 0..6 {
                let on = y < 7 && x < 5 && bits[y] & (1 << (4 - x)) != 0;
                let color = if on { fb.color(255, 255, 255) } else { 0 };
                for dy in 0..self.scale {
                    for dx in 0..self.scale {
                        unsafe {
                            fb.pixel(
                                col * 6 * self.scale + x * self.scale + dx,
                                row * 10 * self.scale + y * self.scale + dy,
                                color,
                            );
                        }
                    }
                }
            }
        }
    }
    fn repaint(&self) {
        for row in 0..self.rows {
            for col in 0..self.painted[row] {
                self.paint(col, row);
            }
        }
    }
    fn newline(&mut self) {
        self.col = 0;
        if self.row + 1 < self.rows {
            self.row += 1;
            return;
        }
        for row in 0..self.rows - 1 {
            for col in 0..self.columns {
                self.cells[row][col] = self.cells[row + 1][col];
            }
            self.painted[row] = self.painted[row].max(self.painted[row + 1]);
        }
        self.cells[self.rows - 1][..self.columns].fill(b' ');
        self.dirty = true;
        if self.visible {
            self.repaint();
            self.dirty = false;
        }
    }
    fn put(&mut self, b: u8) {
        self.cells[self.row][self.col] = b;
        self.painted[self.row] = self.painted[self.row].max(self.col + 1);
        if self.visible {
            self.paint(self.col, self.row);
        } else {
            self.dirty = true;
        }
        self.col += 1;
    }
    pub fn append(&mut self, bytes: &[u8]) {
        if self.fb.is_none() {
            return;
        }
        for &b in bytes {
            if b == b'\n' {
                self.newline();
                continue;
            }
            if self.col == self.columns {
                self.newline();
                self.put(b' ');
                self.put(b' ');
            }
            self.put(if (32..=126).contains(&b) { b } else { b'.' });
        }
        if self.visible {
            self.fb.unwrap().flush_writes();
        }
    }
    fn fill_black(&self) {
        if let Some(fb) = self.fb {
            for y in 0..fb.height {
                for x in 0..fb.width {
                    unsafe {
                        fb.pixel(x, y, 0);
                    }
                }
            }
        }
    }
    pub fn set_visible(&mut self, visible: bool) {
        self.visible = visible;
        if visible {
            if self.pending_clear {
                self.fill_black();
                self.pending_clear = false;
            }
            if self.dirty {
                self.repaint();
                self.dirty = false;
            }
            if let Some(fb) = self.fb {
                fb.flush_writes();
            }
        }
    }
    /// Explicit viewer-to-app reset only. While hidden, defers pixels until shown.
    pub fn clear_black(&mut self) {
        for row in &mut self.cells[..self.rows] {
            row[..self.columns].fill(b' ');
        }
        self.painted.fill(0);
        self.col = 0;
        self.row = 0;
        self.dirty = false;
        self.pending_clear = true;
        if self.visible {
            self.fill_black();
            self.pending_clear = false;
            if let Some(fb) = self.fb {
                fb.flush_writes();
            }
        }
    }
}
impl Default for Console {
    fn default() -> Self {
        Self::new()
    }
}
#[cfg(test)]
mod tests {
    extern crate std;
    use super::*;
    use std::{boxed::Box, vec, vec::Vec};
    fn target(width: usize, height: usize, stride: usize) -> (Vec<u32>, Framebuffer) {
        let mut data = vec![0xdeadbeef; stride * height + 2];
        let fb = Framebuffer {
            base: unsafe { data.as_mut_ptr().add(1) } as usize,
            bytes: stride * height * 4,
            width,
            height,
            stride,
            format: 1,
        };
        (data, fb)
    }
    #[test]
    fn bind_does_not_clear_firmware_and_cell_paints_only_its_6_by_8_area() {
        let (data, fb) = target(30, 30, 32);
        let mut c = Box::new(Console::new());
        unsafe {
            c.bind(fb).unwrap();
        }
        assert!(data.iter().all(|p| *p == 0xdeadbeef));
        c.append(b"A");
        assert_eq!((c.columns(), c.rows(), c.scale()), (5, 3, 1));
        let mut white = 0;
        for y in 0..30 {
            for x in 0..32 {
                let value = data[1 + y * 32 + x];
                if x < 6 && y < 8 {
                    assert!(value == 0 || value == 0xffffff);
                    white += usize::from(value == 0xffffff);
                } else {
                    assert_eq!(value, 0xdeadbeef);
                }
            }
        }
        assert!(white > 0);
        assert_eq!(data[0], 0xdeadbeef);
        assert_eq!(*data.last().unwrap(), 0xdeadbeef);
    }
    #[test]
    fn wrapping_keeps_every_character_and_indents_only_continuations() {
        let (_data, fb) = target(30, 30, 30);
        let mut c = Box::new(Console::new());
        unsafe {
            c.bind(fb).unwrap();
        }
        c.append(b"123456789");
        assert_eq!(c.line(0), Some(&b"12345"[..]));
        assert_eq!(c.line(1), Some(&b"  678"[..]));
        assert_eq!(c.line(2), Some(&b"  9  "[..]));
        unsafe {
            c.bind(fb).unwrap();
        }
        c.append(b"ABCDE\nZ\x01");
        assert_eq!(c.line(1), Some(&b"Z.   "[..]));
    }
    #[test]
    fn scroll_uses_shadow_and_hidden_updates_do_not_touch_pixels() {
        let (mut data, fb) = target(30, 30, 32);
        let mut c = Box::new(Console::new());
        unsafe {
            c.bind(fb).unwrap();
        }
        c.set_visible(false);
        c.append(b"a\nbb\nccc\nz");
        assert_eq!(c.line(0), Some(&b"bb   "[..]));
        assert_eq!(c.line(1), Some(&b"ccc  "[..]));
        assert_eq!(c.line(2), Some(&b"z    "[..]));
        assert!(data.iter().all(|p| *p == 0xdeadbeef));
        // Screen contents do not participate in scrolling or retained text.
        data[1] = 0x123456;
        c.set_visible(true);
        assert_ne!(data[1], 0x123456);
        assert_eq!(data[1 + 30], 0xdeadbeef); // padded stride untouched
        assert_eq!(c.line(0), Some(&b"bb   "[..]));
    }
    #[test]
    fn explicit_black_clear_is_deferred_while_hidden_and_preserves_padding() {
        let (data, fb) = target(30, 30, 32);
        let mut c = Box::new(Console::new());
        unsafe {
            c.bind(fb).unwrap();
        }
        c.set_visible(false);
        c.clear_black();
        assert!(data.iter().all(|p| *p == 0xdeadbeef));
        c.set_visible(true);
        for y in 0..30 {
            for x in 0..32 {
                assert_eq!(data[1 + y * 32 + x], if x < 30 { 0 } else { 0xdeadbeef });
            }
        }
        assert_eq!(c.line(0), Some(&b"     "[..]));
    }
    #[test]
    fn four_k_geometry_and_capacity_rejection_are_explicit() {
        let (_data, fb) = target(3840, 2160, 3840);
        let mut c = Box::new(Console::new());
        unsafe {
            c.bind(fb).unwrap();
        }
        assert_eq!((c.columns(), c.rows(), c.scale()), (160, 54, 4));
        assert_eq!(
            unsafe { c.bind(Framebuffer { width: 0, ..fb }) },
            Err(BindError::InvalidFramebuffer)
        );
        let (_tiny, tiny) = target(12, 10, 12);
        assert_eq!(unsafe { c.bind(tiny) }, Err(BindError::TooSmall));
        let (_wide, wide) = target(2310, 10, 2310);
        assert_eq!(unsafe { c.bind(wide) }, Err(BindError::GeometryTooLarge));
    }
}
