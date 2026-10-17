// SPDX-License-Identifier: Apache-2.0
#![allow(dead_code, unexpected_cfgs)]
static DEBUG: std::sync::Mutex<Vec<u8>> = std::sync::Mutex::new(Vec::new());
fn debug(bytes: &[u8]) {
    DEBUG.lock().unwrap().extend_from_slice(bytes);
}
fn debug_text() -> String {
    String::from_utf8(DEBUG.lock().unwrap().clone()).unwrap()
}
mod bluetooth {
    pub fn present() -> bool {
        false
    }
}
mod cpu {
    pub fn hex(_: u64) -> [u8; 16] {
        [b'0'; 16]
    }
}
mod diagnostics {
    pub fn startup_snapshot() {}
}
mod settings {
    #[derive(Clone, Copy, Debug)]
    pub enum ReadState {
        Loaded,
        MissingDirectory,
        MissingFile,
        Malformed,
        NonFat32,
        Unavailable,
    }
    pub struct Settings {
        pub config: musha_platform::config::Config,
        pub state: ReadState,
        pub forced: bool,
    }
}
mod bootstate {
    pub fn update(f: impl FnOnce(&mut musha_platform::readiness::Report, Option<u64>)) {
        let mut r = musha_platform::readiness::Report::NEW;
        f(&mut r, None);
    }
    pub fn epoch_now() -> Option<u64> {
        Some(100)
    }
    pub fn report() -> musha_platform::readiness::Report {
        musha_platform::readiness::Report::NEW
    }
}
#[path = "../../boot/src/appconsole.rs"]
mod appconsole;
#[path = "../../boot/src/bootlog.rs"]
mod bootlog;
#[path = "../../boot/src/timing.rs"]
mod timing;

#[test]
fn deferred_screen_retains_ring_and_restores_original_visibility_on_nested_early_return() {
    use musha_platform::config::{Config, LogLevel};
    let mut pixels = vec![0x12345678u32; 640 * 480];
    let fb = musha_framebuffer::Framebuffer {
        base: pixels.as_mut_ptr() as usize,
        bytes: pixels.len() * 4,
        width: 640,
        height: 480,
        stride: 640,
        format: 0,
    };
    bootlog::configure(Config {
        log: LogLevel::Devices,
        ..Config::default()
    });
    bootlog::initialize(
        fb,
        settings::Settings {
            config: Config {
                log: LogLevel::Devices,
                ..Config::default()
            },
            state: settings::ReadState::Loaded,
            forced: false,
        },
        false,
    );
    let before = pixels.clone();
    {
        let _outer = bootlog::defer_screen();
        bootlog::device("bluetooth", format_args!("deferred first"));
        fn early() {
            let _inner = bootlog::defer_screen();
            bootlog::device("bluetooth", format_args!("deferred second"));
        }
        early();
        assert!(pixels == before);
        bootlog::dump();
        assert!(debug_text().contains("deferred first"));
        assert!(debug_text().contains("deferred second"));
    }
    assert!(pixels != before);
    bootlog::viewer(true);
    let hidden = pixels.clone();
    {
        let _g = bootlog::defer_screen();
        bootlog::device("bluetooth", format_args!("hidden remains hidden"));
    }
    assert!(pixels == hidden);
    bootlog::viewer(false);
    assert!(pixels != hidden);
}
