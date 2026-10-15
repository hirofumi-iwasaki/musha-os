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
            0
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
    pub fn failure(_: &str, _: &str) {}
}
mod bootstate {
    pub fn report() -> musha_platform::readiness::Report {
        musha_platform::readiness::Report::NEW
    }
    pub fn epoch_now() -> Option<u64> {
        Some(0)
    }
}
