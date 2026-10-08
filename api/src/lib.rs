// Copyright 2026 Hirofumi Iwasaki
// SPDX-License-Identifier: Apache-2.0
#![no_std]
use musha_framebuffer::Framebuffer;
pub const API_VERSION: u32 = 1;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Invalid,
    Unsupported,
    Io,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Step {
    Continue,
    Complete,
}
/// Statically linked Rust application contract; this is not a binary FFI ABI.
pub trait Application {
    fn init(&mut self, context: &mut Context<'_>) -> Result<(), Error>;
    fn step(&mut self, context: &mut Context<'_>) -> Result<Step, Error>;
    fn shutdown(&mut self, context: &mut Context<'_>);
}
pub struct Context<'a> {
    arena: &'a mut [u8],
    screen: Framebuffer,
    now_ms: u64,
}
impl<'a> Context<'a> {
    /// # Safety
    /// screen must remain exclusively drawable mapped memory for this lifetime,
    /// disjoint from arena. The runtime must not draw while an app method runs.
    pub unsafe fn new(arena: &'a mut [u8], screen: Framebuffer) -> Result<Self, Error> {
        if arena.is_empty() || !screen.valid() {
            return Err(Error::Invalid);
        }
        Ok(Self {
            arena,
            screen,
            now_ms: 0,
        })
    }
    pub fn version(&self) -> u32 {
        API_VERSION
    }
    pub fn now_ms(&self) -> u64 {
        self.now_ms
    }
    /// Runtime updates time before each cooperative callback; stale samples fail.
    pub fn advance(&mut self, now: u64) -> Result<(), Error> {
        if now < self.now_ms {
            return Err(Error::Invalid);
        }
        self.now_ms = now;
        Ok(())
    }
    pub fn arena(&mut self) -> &mut [u8] {
        self.arena
    }
    pub fn arena_bytes(&self) -> usize {
        self.arena.len()
    }
    pub fn pixel(&mut self, x: usize, y: usize, rgb: [u8; 3]) -> Result<(), Error> {
        if self.screen.offset(x, y).is_none() {
            return Err(Error::Invalid);
        }
        unsafe {
            self.screen
                .pixel(x, y, self.screen.color(rgb[0], rgb[1], rgb[2]));
        }
        Ok(())
    }
    pub fn rectangle(
        &mut self,
        x: usize,
        y: usize,
        width: usize,
        height: usize,
        rgb: [u8; 3],
    ) -> Result<(), Error> {
        let end_x = x.checked_add(width).ok_or(Error::Invalid)?;
        let end_y = y.checked_add(height).ok_or(Error::Invalid)?;
        if end_x > self.screen.width || end_y > self.screen.height {
            return Err(Error::Invalid);
        }
        for row in y..end_y {
            for col in x..end_x {
                self.pixel(col, row, rgb)?;
            }
        }
        Ok(())
    }
    pub fn text(&mut self, text: &str, x: usize, y: usize, rgb: [u8; 3]) {
        // SAFETY: constructor contract persists and &mut self serializes drawing.
        unsafe {
            self.screen
                .text(text, x, y, self.screen.color(rgb[0], rgb[1], rgb[2]));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn monotonic_time_and_clipped_drawing() {
        let mut arena = [0u8; 16];
        let mut pixels = [0xa5a5a5a5u32; 8];
        let screen = Framebuffer {
            base: pixels.as_mut_ptr() as usize,
            bytes: 24,
            width: 2,
            height: 2,
            stride: 3,
            format: 0,
        };
        let mut ctx = unsafe { Context::new(&mut arena, screen) }.unwrap();
        assert_eq!(ctx.advance(100), Ok(()));
        assert_eq!(ctx.advance(99), Err(Error::Invalid));
        assert_eq!(ctx.now_ms(), 100);
        ctx.text("A", usize::MAX, usize::MAX, [1, 2, 3]);
        assert!(pixels.iter().all(|v| *v == 0xa5a5a5a5));
        assert_eq!(ctx.rectangle(1, 1, 2, 1, [1, 2, 3]), Err(Error::Invalid));
        assert_eq!(ctx.pixel(2, 0, [1, 2, 3]), Err(Error::Invalid));
        assert_eq!(ctx.rectangle(0, 0, 2, 2, [1, 2, 3]), Ok(()));
        assert_eq!(pixels[0], 0x030201);
        ctx.text("A", 0, 0, [1, 2, 3]);
        assert_eq!(pixels[2], 0xa5a5a5a5);
        assert_eq!(pixels[5], 0xa5a5a5a5);
        assert_eq!(pixels[6], 0xa5a5a5a5);
        assert_eq!(pixels[7], 0xa5a5a5a5);
        assert_eq!(ctx.arena_bytes(), 16);
    }
}
