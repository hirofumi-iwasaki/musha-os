// SPDX-License-Identifier: Apache-2.0
//! BSP-only external application display. No mutable references survive callbacks.
use core::{
    cell::UnsafeCell,
    fmt::{self, Write},
};
use musha_framebuffer::{Framebuffer, console::Console};
struct State {
    console: Console,
    fb: Option<Framebuffer>,
}
struct Shared(UnsafeCell<State>);
// Single boot CPU, interrupts disabled. Fatal exception path uses no console state.
unsafe impl Sync for Shared {}
static STATE: Shared = Shared(UnsafeCell::new(State {
    console: Console::new(),
    fb: None,
}));
pub fn start(fb: Framebuffer, text: &[u8]) {
    unsafe {
        for y in 0..fb.height {
            for x in 0..fb.width {
                fb.pixel(x, y, fb.color(12, 20, 32));
            }
        }
        let s = &mut *STATE.0.get();
        s.fb = Some(fb);
        s.console = Console::for_framebuffer(fb);
        s.console.append(text);
        s.console.render(fb);
    }
}
pub fn append(bytes: &[u8]) {
    unsafe {
        let s = &mut *STATE.0.get();
        s.console.append(bytes);
        if let Some(fb) = s.fb {
            s.console.render(fb);
        }
    }
}
pub fn message(args: fmt::Arguments<'_>) {
    struct Text {
        bytes: [u8; 256],
        used: usize,
    }
    impl Write for Text {
        fn write_str(&mut self, s: &str) -> fmt::Result {
            let n = s.len().min(self.bytes.len() - self.used);
            self.bytes[self.used..self.used + n].copy_from_slice(&s.as_bytes()[..n]);
            self.used += n;
            Ok(())
        }
    }
    let mut text = Text {
        bytes: [0; 256],
        used: 0,
    };
    let _ = text.write_fmt(args);
    append(&text.bytes[..text.used]);
}
pub fn rejected(reason: &[u8]) {
    message(format_args!(
        "\nAUTORUN.ELF rejected: {}\n",
        core::str::from_utf8(reason).unwrap_or("Unknown reason")
    ));
    crate::bootstate::update(|r, _| r.finish(false));
}
/// Never borrows STATE: even a fault during drawing can use the fatal display.
pub unsafe fn fault(fb: Framebuffer, vector: u64, error: u64, rip: u64, rsp: u64) {
    let mut c = Console::for_framebuffer(fb);
    c.append(b"AUTORUN.ELF faulted\n");
    for (label, value) in [
        (b"VECTOR " as &[u8], vector),
        (b"ERROR ", error),
        (b"RIP ", rip),
        (b"RSP ", rsp),
    ] {
        c.append(label);
        c.append(&crate::cpu::hex(value));
        c.append(b"\n");
    }
    unsafe { c.render(fb) };
    crate::bootstate::update(|r, _| r.finish(false));
}
