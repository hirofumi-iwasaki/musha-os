// Copyright 2026 Hirofumi Iwasaki
// SPDX-License-Identifier: Apache-2.0
//! Measurement-only UEFI-target library. Never linked into the boot image.
#![no_std]
use musha_input::hid::{Decoder, Error, Layout};
#[unsafe(no_mangle)]
pub static HID_LAYOUT_SIZE: usize = core::mem::size_of::<Layout>();
#[unsafe(no_mangle)]
pub static HID_DECODER_SIZE: usize = core::mem::size_of::<Decoder>();
#[unsafe(no_mangle)]
pub static HID_LAYOUT_ALIGN: usize = core::mem::align_of::<Layout>();
#[unsafe(no_mangle)]
pub static HID_DECODER_ALIGN: usize = core::mem::align_of::<Decoder>();
#[unsafe(no_mangle)]
#[inline(never)]
pub fn hid_probe_construct(bytes: &[u8]) -> Result<Decoder, Error> {
    Ok(Decoder::new(Layout::parse(bytes)?))
}
#[unsafe(no_mangle)]
#[inline(never)]
pub fn hid_probe_update(decoder: &mut Decoder, bytes: &[u8]) -> Result<usize, Error> {
    let mut count = 0;
    decoder.update(bytes, |_, _| count += 1)?;
    Ok(count)
}
#[unsafe(no_mangle)]
#[inline(never)]
pub fn hid_probe_release(decoder: &mut Decoder) -> usize {
    let mut count = 0;
    decoder.release_all(|_, _| count += 1);
    count
}
