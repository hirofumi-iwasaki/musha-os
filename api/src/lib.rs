// Copyright 2026 Hirofumi Iwasaki
// SPDX-License-Identifier: Apache-2.0
#![no_std]
mod files;
mod input;
pub use files::FileHandle;
use musha_framebuffer::Framebuffer;
pub const API_VERSION: u32 = 4;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Invalid,
    Unsupported,
    Io,
    Again,
    NotFound,
    NoMemory,
    Disconnected,
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
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KeyEvent {
    pub usage: u8,
    pub pressed: bool,
    pub timestamp_ms: u64,
}
pub struct Context<'a> {
    arena: &'a mut [u8],
    files: files::Files,
    screen: Framebuffer,
    now_ms: u64,
    input: input::Input,
    input_active: bool,
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
            files: files::Files::new(),
            screen,
            now_ms: 0,
            input: input::Input::new(),
            input_active: false,
        })
    }
    /// Runtime publishes a verified owned snapshot; only the first success wins.
    pub fn install_boot_file(&mut self, bytes: &[u8]) -> Result<bool, Error> {
        self.files.publish(bytes)
    }
    pub fn record_file_error(&mut self, error: Error) {
        self.files.unavailable(error);
    }
    pub fn finish_file_discovery(&mut self, failed: bool) {
        self.files.finish(failed);
    }
    pub fn invalidate_files(&mut self, error: Error) {
        self.files.invalidate(error);
    }
    /// Opens an absolute root 8.3 path. Initial cached file: /MUSHA.TXT.
    pub fn file_open(&mut self, path: &str) -> Result<FileHandle, Error> {
        self.files.open(path)
    }
    /// Copies at most out.len() bytes; zero means EOF or a zero-length buffer.
    pub fn file_read(&mut self, handle: FileHandle, out: &mut [u8]) -> Result<usize, Error> {
        self.files.read(handle, out)
    }
    pub fn file_close(&mut self, handle: FileHandle) -> Result<(), Error> {
        self.files.close(handle)
    }
    /// Runtime producer. Overflow preserves the older FIFO and records the latest
    /// held-key state. Further input coalesces until the consumer drains recovery.
    /// Intermediate transitions may be lost; final held state is reconciled.
    pub fn push_key(&mut self, usage: u8, pressed: bool) {
        self.input.push(usage, pressed, self.now_ms);
    }
    /// FIFO events retain timestamps; recovery events use the current context
    /// time and release keys before pressing keys. Drain until None each step.
    pub fn next_key(&mut self) -> Option<KeyEvent> {
        self.input.next(self.now_ms)
    }
    /// Number of producer events coalesced during overflow recovery (saturating).
    /// State recovery does not reconstruct lost clicks, text or their timestamps.
    pub fn lost_key_events(&self) -> u64 {
        self.input.lost()
    }
    pub fn set_input_active(&mut self, active: bool) {
        self.input_active = active;
    }
    pub fn input_active(&self) -> bool {
        self.input_active
    }
    pub fn screen_size(&self) -> (usize, usize) {
        (self.screen.width, self.screen.height)
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
    fn input_fifo_wrap_and_timestamps() {
        let mut arena = [0u8; 1];
        let mut pixels = [0u32; 1];
        let fb = Framebuffer {
            base: pixels.as_mut_ptr() as usize,
            bytes: 4,
            width: 1,
            height: 1,
            stride: 1,
            format: 0,
        };
        let mut ctx = unsafe { Context::new(&mut arena, fb) }.unwrap();
        ctx.advance(7).unwrap();
        for key in 0..64 {
            ctx.push_key(key, true);
        }
        assert_eq!(ctx.lost_key_events(), 0);
        for key in 0..32 {
            assert_eq!(
                ctx.next_key(),
                Some(KeyEvent {
                    usage: key,
                    pressed: true,
                    timestamp_ms: 7
                })
            );
        }
        ctx.advance(9).unwrap();
        for key in 64..96 {
            ctx.push_key(key, false);
        }
        for key in 32..96 {
            let e = ctx.next_key().unwrap();
            assert_eq!(e.usage, key);
            assert_eq!(e.pressed, key < 64);
            assert_eq!(e.timestamp_ms, if key < 64 { 7 } else { 9 });
        }
        assert_eq!(ctx.next_key(), None);
    }
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
