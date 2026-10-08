// SPDX-License-Identifier: Apache-2.0
//! Boot-CPU-only panel. Interrupts remain disabled; no reference escapes update.
use core::{
    cell::UnsafeCell,
    fmt::{self, Write},
};
use musha_framebuffer::Framebuffer;
use musha_platform::diagnostics::{Failure, Line, layout};
struct Panel {
    fb: Option<Framebuffer>,
    lines: [Line; 24],
    usb: usize,
    usb_fail: Failure,
    net_fail: Failure,
}
struct Shared(UnsafeCell<Panel>);
// SAFETY: only the boot CPU calls these functions with interrupts disabled;
// rendering never invokes callbacks or firmware, and updates cannot reenter.
unsafe impl Sync for Shared {}
static PANEL: Shared = Shared(UnsafeCell::new(Panel {
    fb: None,
    lines: [Line::EMPTY; 24],
    usb: 0,
    usb_fail: Failure::EMPTY,
    net_fail: Failure::EMPTY,
}));
fn line(args: fmt::Arguments<'_>) -> Line {
    let mut l = Line::EMPTY;
    let _ = l.write_fmt(args);
    l
}
fn update(f: impl FnOnce(&mut Panel)) {
    unsafe {
        let p = &mut *PANEL.0.get();
        f(p);
        render(p);
    }
}
fn render(p: &Panel) {
    let Some(fb) = p.fb else { return };
    let Some((x, y, full)) = layout(fb.width, fb.height) else {
        return;
    };
    unsafe {
        let height = if full { 240 } else { 7 };
        for py in y..y + height {
            for px in x..(x + 384).min(fb.width) {
                fb.pixel(px, py, fb.color(12, 20, 32));
            }
        }
        if full {
            for (i, l) in p.lines.iter().enumerate() {
                fb.text_small(l.as_str(), x, y + i * 10, fb.color(240, 220, 100));
            }
        } else {
            let l = if !p.usb_fail.error.as_str().is_empty() {
                line(format_args!(
                    "USB {} {}",
                    p.usb_fail.stage.as_str(),
                    p.usb_fail.error.as_str()
                ))
            } else if !p.net_fail.error.as_str().is_empty() {
                line(format_args!(
                    "NET {} {}",
                    p.net_fail.stage.as_str(),
                    p.net_fail.error.as_str()
                ))
            } else {
                p.lines[1]
            };
            fb.text_small(l.as_str(), x, y, fb.color(240, 220, 100));
        }
    }
}
pub fn start(fb: Framebuffer) {
    update(|p| {
        p.fb = Some(fb);
        p.lines[0] = line(format_args!("MUSHA-OS 0.1.0 HARDWARE"));
    });
}
pub fn set(row: usize, args: fmt::Arguments<'_>) {
    update(|p| p.lines[row] = line(args));
}
pub fn usb_stage(s: &str) {
    set(4, format_args!("USB STEP {}", s));
}
pub fn net_stage(s: &str) {
    set(7, format_args!("NET STEP {}", s));
}
pub fn usb_failure(s: &str) {
    update(|p| {
        p.usb_fail.record(p.lines[4], line(format_args!("{}", s)));
        p.lines[5] = line(format_args!("FAIL AT {}", p.usb_fail.stage.as_str()));
        p.lines[6] = line(format_args!("USB ERROR {}", p.usb_fail.error.as_str()));
    });
}
pub fn net_failure(s: &str) {
    update(|p| {
        p.net_fail.record(p.lines[7], line(format_args!("{}", s)));
        p.lines[8] = line(format_args!("FAIL AT {}", p.net_fail.stage.as_str()));
        p.lines[9] = line(format_args!("NET ERROR {}", p.net_fail.error.as_str()));
    });
}
pub fn pci(row: usize, id: u32, bdf: u16) {
    set(
        row,
        format_args!(
            "{} PCI {:04X}:{:04X} BDF {:04X}",
            if row == 2 { "XHCI" } else { "LAN" },
            id as u16,
            id >> 16,
            bdf
        ),
    );
}
pub fn usb(port: usize, slot: u8, vendor: u16, product: u16, speed: &str) {
    update(|p| {
        if p.usb < 8 {
            p.lines[11 + p.usb] = line(format_args!(
                "USB PORT {:02X} SLOT {:02X} {:04X}:{:04X} {}",
                port, slot, vendor, product, speed
            ));
        }
        p.usb += 1;
        p.lines[10] = line(format_args!("USB DESCRIPTORS {} (FIRST 8 SHOWN)", p.usb));
    });
}
pub fn snapshot() {
    unsafe {
        let p = &*PANEL.0.get();
        for l in &p.lines {
            if !l.as_str().is_empty() {
                crate::debug(b"MUSHA: DIAG ");
                crate::debug(l.as_str().as_bytes());
                crate::debug(b"\n");
            }
        }
    }
}
