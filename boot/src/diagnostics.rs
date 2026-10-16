// SPDX-License-Identifier: Apache-2.0
//! Diagnostic producers feed the same sequential log; there is no panel or page.
use core::{
    fmt,
    sync::atomic::{AtomicU16, Ordering},
};
static CONTROLLER: AtomicU16 = AtomicU16::new(0);
use musha_framebuffer::Framebuffer;
pub fn flush() {}
pub fn start(_fb: Framebuffer) {}
pub fn set(_row: usize, args: fmt::Arguments<'_>) {
    crate::bootlog::detail("device", args);
}
pub fn usb_stage(s: &str) {
    crate::bootlog::detail("usb", format_args!("stage {}", s));
}
pub fn net_stage(s: &str) {
    crate::bootlog::detail("lan", format_args!("stage {}", s));
}
pub fn failure(stage: &str, error: &str) {
    crate::bootlog::failure(stage, error);
}
pub fn usb_failure(s: &str) {
    crate::bootlog::detail("usb", format_args!("failure {}", s));
}
pub fn net_failure(s: &str) {
    crate::bootlog::phase("lan", s);
}
pub fn pci(_row: usize, id: u32, bdf: u16, revision: u8) {
    crate::bootlog::detail(
        "pci",
        format_args!(
            "{:02x}:{:02x}.{} {:04x}:{:04x} revision={:02x}",
            bdf >> 8,
            (bdf >> 3) & 31,
            bdf & 7,
            id as u16,
            id >> 16,
            revision
        ),
    );
}
pub fn usb(
    port: usize,
    route: u32,
    slot: u8,
    vendor: u16,
    product: u16,
    speed: &str,
    class: [u8; 3],
) {
    crate::bootlog::device_once(
        (1u128 << 127)
            | ((CONTROLLER.load(Ordering::Relaxed) as u128) << 80)
            | ((port as u128) << 64)
            | ((route as u128) << 32)
            | ((vendor as u128) << 16)
            | product as u128,
        "usb",
        format_args!(
            "root={} route=0x{:05x} slot={} {:04x}:{:04x} speed={} class={:02x}/{:02x}/{:02x}",
            port, route, slot, vendor, product, speed, class[0], class[1], class[2]
        ),
    );
}
pub fn snapshot() {
    crate::bootlog::dump();
}
pub fn controller(index: usize, id: u32, bdf: u16, revision: u8, selected: bool) {
    detail_controller(index, id, bdf, revision, selected);
}
fn detail_controller(index: usize, id: u32, bdf: u16, _revision: u8, selected: bool) {
    let _ = id;
    crate::bootlog::detail(
        "usb",
        format_args!(
            "index={} bdf=0x{:04x} firmware-selected={}",
            index, bdf, selected
        ),
    );
}
pub fn usb_kind(kind: &str) {
    crate::bootlog::detail("usb", format_args!("kind {}", kind));
}
pub fn supported_device(keyboard: bool) {
    crate::bootlog::detail(
        "usb",
        format_args!(
            "supported {}",
            if keyboard {
                "boot keyboard"
            } else {
                "BOT storage"
            }
        ),
    );
}
pub fn inventory_total(total: usize) {
    crate::bootlog::detail("usb", format_args!("controllers={} limit=8", total));
}
pub fn begin_controller(c: crate::pci::Controller) {
    CONTROLLER.store(c.bdf(), Ordering::Relaxed);
    crate::bootlog::detail("usb", format_args!("begin bdf=0x{:04x}", c.bdf()));
}
/// No display state belongs to a device session. Retained as a zero-size lifetime token.
pub struct ControllerState;
pub fn suspend_controller() -> ControllerState {
    ControllerState
}
pub fn resume_controller(_saved: ControllerState) {}
pub fn controller_result(
    c: crate::pci::Controller,
    result: Option<(bool, usize)>,
    error: Option<&str>,
    input: bool,
) {
    match result {
        Some((keyboard, storage)) => crate::bootlog::detail(
            "usb",
            format_args!(
                "bdf=0x{:04x} scanned keyboard={} storage={} input={}",
                c.bdf(),
                keyboard,
                storage,
                input
            ),
        ),
        None => crate::bootlog::detail(
            "usb",
            format_args!(
                "bdf=0x{:04x} failed {}",
                c.bdf(),
                error.unwrap_or("unknown")
            ),
        ),
    }
}
pub fn controller_limit(truncated: bool) {
    if truncated {
        crate::bootlog::detail(
            "usb",
            format_args!("controller limit 8; remaining not probed"),
        );
    }
}
pub fn unsupported_hub(version: u16) {
    crate::bootlog::detail("usb", format_args!("unsupported hub USB 0x{:04x}", version));
    crate::debug(b"MUSHA: HUB_GENERATION_UNSUPPORTED USB=");
    crate::debug(&crate::cpu::hex(version as u64));
    crate::debug(b"\n");
}
pub fn control_request(slot: u8, request: u32, index: u16, bytes: u32) {
    crate::bootlog::detail(
        "usb",
        format_args!(
            "control slot={} request=0x{:08x} index=0x{:04x} bytes={}",
            slot, request, index, bytes
        ),
    );
}
pub fn transfer_failure(words: [u32; 4], expected: usize, slot: u8, endpoint: u8) {
    crate::bootlog::detail(
        "usb",
        format_args!(
            "transfer failure expected slot={} endpoint={} trb=0x{:x} event={:08x} {:08x} {:08x} {:08x}",
            slot, endpoint, expected, words[0], words[1], words[2], words[3]
        ),
    );
}
pub fn observation(args: fmt::Arguments<'_>) {
    crate::bootlog::detail("device", args);
}
