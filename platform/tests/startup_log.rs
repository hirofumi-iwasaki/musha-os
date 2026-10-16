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
// One test owns the BSP globals for the entire matrix; no parallel API calls.
#[test]
fn single_log_levels_retained_ring_application_and_failure_boundaries() {
    use musha_platform::{
        boot_log::Level,
        config::{Config, LogLevel},
    };
    for (width, height) in [
        (320, 356),
        (640, 480),
        (1024, 768),
        (1920, 1080),
        (2048, 2048),
    ] {
        for log in [
            LogLevel::Quiet,
            LogLevel::Ready,
            LogLevel::Phases,
            LogLevel::Devices,
        ] {
            let mut pixels = vec![0x12345678u32; width * height + 16];
            let fb = musha_framebuffer::Framebuffer {
                base: pixels.as_mut_ptr() as usize,
                bytes: width * height * 4,
                width,
                height,
                stride: width,
                format: 0,
            };
            let config = Config {
                log,
                ..Config::default()
            };
            bootlog::configure(config);
            bootlog::initialize(
                fb,
                settings::Settings {
                    config,
                    state: settings::ReadState::Loaded,
                    forced: false,
                },
                false,
            );
            if matches!(log, LogLevel::Quiet | LogLevel::Ready) {
                assert!(
                    pixels.iter().all(|v| *v == 0x12345678),
                    "startup must not fill at {log:?}"
                );
            }
            pixels[..width * height].fill(0x12345678);
            bootlog::event(Level::Ready, "", format_args!("Musha-OS 0.4.0 ready"));
            if log == LogLevel::Quiet {
                assert!(pixels.iter().all(|v| *v == 0x12345678));
            } else {
                assert!(pixels[..width * height].iter().any(|v| *v != 0x12345678));
            }
            // No footer or overlay may repaint over the viewer's ownership.
            pixels[..width * height].fill(0x12345678);
            bootlog::overlay();
            assert!(pixels.iter().all(|v| *v == 0x12345678));
            bootlog::viewer(true);
            bootlog::phase("usb", "host ready");
            assert!(pixels.iter().all(|v| *v == 0x12345678));
            bootlog::viewer(false);
            pixels[..width * height].fill(0x12345678);
            bootlog::failure("clock", "missing");
            bootlog::halted();
            let changed: Vec<_> = pixels[..width * height]
                .iter()
                .copied()
                .filter(|p| *p != 0x12345678)
                .collect();
            assert!(!changed.is_empty(), "required failure hidden at {log:?}");
            assert!(
                changed.iter().all(|p| matches!(*p, 0 | 0xffffff)),
                "colored failure band returned"
            );
            assert_eq!(&pixels[width * height..], &[0x12345678; 16]);
        }
    }
    let mut pixels = vec![0x12345678u32; 640 * 480 + 16];
    let fb = musha_framebuffer::Framebuffer {
        base: pixels.as_mut_ptr() as usize,
        bytes: 640 * 480 * 4,
        width: 640,
        height: 480,
        stride: 640,
        format: 0,
    };
    let config = Config {
        log: LogLevel::Quiet,
        ..Config::default()
    };
    bootlog::configure(config);
    bootlog::initialize(
        fb,
        settings::Settings {
            config,
            state: settings::ReadState::Loaded,
            forced: false,
        },
        false,
    );
    appconsole::prepare(fb);
    assert!(pixels.iter().all(|p| *p == 0x12345678));
    // Quiet suppresses startup, never the application's own output or rejection.
    appconsole::append(b"application output\n");
    assert!(pixels[..640 * 480].iter().any(|p| *p != 0x12345678));
    appconsole::rejected(b"Invalid ELF or ABI NOTE");
    bootlog::ready(false);
    for i in 0..300 {
        timing::record(format_args!("stage {i}"));
    }
    bootlog::dump();
    let text = debug_text();
    assert!(text.contains("MUSHA: LOG Musha-OS 0.4.0 ready\n"));
    assert!(text.contains("MUSHA: LOG ERROR: clock: missing\n"));
    assert!(text.contains("MUSHA: LOG Musha-OS halted\n"));
    let retained = text.rsplit_once("MUSHA: RING_BEGIN\n").unwrap().1;
    assert!(retained.contains("MUSHA: RING_END\n"));
    assert!(retained.contains("settings: loaded log=quiet"));
    assert!(retained.contains("application output\n"));
    assert!(retained.contains("stage 299"));
    assert!(!retained.contains("TIMING LIMIT / MORE OMITTED"));
    assert_eq!(&pixels[640 * 480..], &[0x12345678; 16]);
    musha_framebuffer::set_output_enabled(true);
}
