// SPDX-License-Identifier: Apache-2.0
//! GOP calls are confined to Boot Services, before framebuffer capture and memory-map acquisition.
use musha_framebuffer::mode::{Info, Modes};
use r_efi::{efi, protocols::graphics_output as gop};
fn info(m: &gop::ModeInformation) -> Info {
    Info {
        width: m.horizontal_resolution,
        height: m.vertical_resolution,
        stride: m.pixels_per_scan_line,
        format: m.pixel_format,
    }
}
struct Firmware {
    protocol: *mut gop::Protocol,
    bs: *mut efi::BootServices,
}
impl Modes for Firmware {
    fn count(&self) -> u32 {
        unsafe { (*(*self.protocol).mode).max_mode }
    }
    fn current(&self) -> u32 {
        unsafe { (*(*self.protocol).mode).mode }
    }
    fn current_info(&self) -> Option<Info> {
        unsafe {
            let mode = &*(*self.protocol).mode;
            if mode.info.is_null()
                || mode.size_of_info < core::mem::size_of::<gop::ModeInformation>()
            {
                None
            } else {
                Some(info(&*mode.info))
            }
        }
    }
    fn query(&mut self, index: u32) -> Option<Info> {
        unsafe {
            let mut size = 0;
            let mut p = core::ptr::null_mut();
            let status = ((*self.protocol).query_mode)(self.protocol, index, &mut size, &mut p);
            let result = if !status.is_error()
                && !p.is_null()
                && size >= core::mem::size_of::<gop::ModeInformation>()
            {
                Some(info(&*p))
            } else {
                None
            };
            // QueryMode allocates a separate pool buffer; never retain it across EBS.
            if !p.is_null() {
                let _ = ((*self.bs).free_pool)(p.cast());
            }
            if let Some(i) = result {
                crate::debug(b"MUSHA: GOP CANDIDATE INDEX/WIDTH/HEIGHT/STRIDE/FORMAT ");
                for n in [index, i.width, i.height, i.stride, i.format] {
                    crate::debug(&crate::cpu::hex(n as u64));
                    crate::debug(b" ");
                }
                crate::debug(b"\n");
            }
            result
        }
    }
    fn set(&mut self, index: u32) -> bool {
        unsafe { !((*self.protocol).set_mode)(self.protocol, index).is_error() }
    }
}
/// Firmware owns valid protocol and Boot Services tables; current mode is initialized.
pub unsafe fn select(protocol: *mut gop::Protocol, bs: *mut efi::BootServices) {
    if unsafe { (*protocol).mode.is_null() } {
        return;
    }
    let mut modes = Firmware { protocol, bs };
    let before = modes.current();
    #[cfg(not(feature = "display-grid-probe"))]
    musha_framebuffer::mode::select(&mut modes);
    #[cfg(feature = "display-grid-probe")]
    {
        let wanted = option_env!("MUSHA_TEST_GOP").unwrap_or("1024x768");
        let (w, h) = wanted.split_once('x').unwrap();
        let w: u32 = w.parse().unwrap();
        let h: u32 = h.parse().unwrap();
        for index in 0..modes.count() {
            if modes
                .query(index)
                .is_some_and(|i| i.width == w && i.height == h && i.format <= 1)
            {
                let _ = modes.set(index);
                break;
            }
        }
    }
    crate::debug(b"MUSHA: GOP MODE BEFORE/AFTER ");
    crate::debug(&crate::cpu::hex(before as u64));
    crate::debug(b" ");
    crate::debug(&crate::cpu::hex(modes.current() as u64));
    crate::debug(b"\n");
}
