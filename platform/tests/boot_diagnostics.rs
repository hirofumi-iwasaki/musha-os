// SPDX-License-Identifier: Apache-2.0
#![allow(dead_code, unexpected_cfgs)]
fn debug(_: &[u8]) {}
mod cpu {
    pub fn hex(_: u64) -> [u8; 16] {
        [b'0'; 16]
    }
}
mod pci {
    #[derive(Clone, Copy)]
    pub struct Controller {
        pub bus: u8,
        pub device: u8,
        pub function: u8,
    }
    impl Controller {
        pub fn bdf(self) -> u16 {
            (u16::from(self.bus) << 8) | (u16::from(self.device) << 3) | u16::from(self.function)
        }
    }
    pub unsafe fn read(_: u8, _: u8, _: u8, _: u32) -> u32 {
        panic!("diagnostic view accessed PCI")
    }
}
#[path = "../../boot/src/diagnostics.rs"]
mod diagnostics;
#[path = "../../boot/src/timing.rs"]
mod timing;

mod bootlog {
    use std::sync::Mutex;
    pub static EVENTS: Mutex<Vec<String>> = Mutex::new(Vec::new());
    pub fn detail(tag: &str, args: core::fmt::Arguments<'_>) {
        EVENTS.lock().unwrap().push(format!("detail {tag}: {args}"));
    }
    pub fn device(tag: &str, args: core::fmt::Arguments<'_>) {
        EVENTS.lock().unwrap().push(format!("device {tag}: {args}"));
    }
    pub fn device_once(_: u128, tag: &str, args: core::fmt::Arguments<'_>) {
        device(tag, args);
    }
    pub fn phase(tag: &str, text: &str) {
        EVENTS.lock().unwrap().push(format!("phase {tag}: {text}"));
    }
    pub fn failure(tag: &str, text: &str) {
        EVENTS.lock().unwrap().push(format!("error {tag}: {text}"));
    }
    pub fn dump() {
        EVENTS.lock().unwrap().push("dump".into());
    }
}

mod bootstate {
    pub fn report() -> musha_platform::readiness::Report {
        musha_platform::readiness::Report::NEW
    }
    pub fn epoch_now() -> Option<u64> {
        Some(0)
    }
}

#[test]
fn producers_preserve_order_fields_and_never_read_pci_or_draw_a_panel() {
    let mut pixels = [0x12345678u32; 600];
    let fb = musha_framebuffer::Framebuffer {
        base: pixels.as_mut_ptr() as usize,
        bytes: pixels.len() * 4,
        width: 30,
        height: 20,
        stride: 30,
        format: 0,
    };
    let c = pci::Controller {
        bus: 3,
        device: 5,
        function: 2,
    };
    diagnostics::start(fb);
    diagnostics::set(
        99,
        format_args!("range 0x{:x} bytes={}", 0x123400usize, 4096),
    );
    diagnostics::usb_stage("enumerate");
    diagnostics::net_stage("reset");
    diagnostics::pci(12, 0x9cb18086, c.bdf(), 3);
    diagnostics::controller(2, 0x9cb18086, c.bdf(), 3, true);
    diagnostics::usb(4, 0x12345, 7, 0x046d, 0xc31c, "high", [3, 1, 1]);
    diagnostics::usb_kind("keyboard");
    diagnostics::supported_device(true);
    diagnostics::supported_device(false);
    diagnostics::inventory_total(3);
    diagnostics::begin_controller(c);
    let token = diagnostics::suspend_controller();
    diagnostics::resume_controller(token);
    diagnostics::controller_result(c, Some((true, 2)), None, true);
    diagnostics::controller_result(c, None, Some("timeout"), false);
    diagnostics::controller_limit(false);
    diagnostics::controller_limit(true);
    diagnostics::control_request(7, 0x12345678, 0x4321, 512);
    diagnostics::transfer_failure([1, 2, 3, 4], 0x123400, 7, 3);
    diagnostics::usb_failure("descriptor timeout");
    diagnostics::net_failure("not available");
    diagnostics::failure("storage", "missing media");
    diagnostics::observation(format_args!("marker bytes={} hash={:016x}", 99, 0xabcdu64));
    diagnostics::unsupported_hub(0x0310);
    diagnostics::flush();
    diagnostics::snapshot();
    assert_eq!(
        *bootlog::EVENTS.lock().unwrap(),
        [
            "detail device: range 0x123400 bytes=4096",
            "detail usb: stage enumerate",
            "detail lan: stage reset",
            "detail pci: 03:05.2 8086:9cb1 revision=03",
            "detail usb: index=2 bdf=0x032a firmware-selected=true",
            "device usb: root=4 route=0x12345 slot=7 046d:c31c speed=high class=03/01/01",
            "detail usb: kind keyboard",
            "detail usb: supported boot keyboard",
            "detail usb: supported BOT storage",
            "detail usb: controllers=3 limit=8",
            "detail usb: begin bdf=0x032a",
            "detail usb: bdf=0x032a scanned keyboard=true storage=2 input=true",
            "detail usb: bdf=0x032a failed timeout",
            "detail usb: controller limit 8; remaining not probed",
            "detail usb: control slot=7 request=0x12345678 index=0x4321 bytes=512",
            "detail usb: transfer failure expected slot=7 endpoint=3 trb=0x123400 event=00000001 00000002 00000003 00000004",
            "detail usb: failure descriptor timeout",
            "phase lan: not available",
            "error storage: missing media",
            "detail device: marker bytes=99 hash=000000000000abcd",
            "detail usb: unsupported hub USB 0x0310",
            "dump",
        ]
    );
    assert!(
        pixels.iter().all(|p| *p == 0x12345678),
        "diagnostic producer drew pixels"
    );
}
