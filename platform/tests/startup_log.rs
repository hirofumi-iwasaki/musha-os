// SPDX-License-Identifier: Apache-2.0
#![allow(dead_code, unexpected_cfgs)]
fn debug(_: &[u8]) {}
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
fn startup_policy_clipping_application_space_and_normal_timing_bounds() {
    use musha_platform::config::{Config, LogLevel};
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
            if log == LogLevel::Quiet || log == LogLevel::Ready {
                assert!(pixels.iter().all(|v| *v == 0x12345678));
            }
            bootlog::media_ready(false);
            bootlog::keyboard(true);
            bootlog::ready(false);
            // Emulate a complete application/viewer repaint, then restore startup footer.
            pixels[..width * height].fill(0x12345678);
            bootlog::overlay();
            if log == LogLevel::Quiet {
                assert!(pixels.iter().all(|v| *v == 0x12345678));
            } else {
                assert!(
                    pixels[..width * height].iter().any(|v| *v != 0x12345678),
                    "missing ready at {width}x{height}"
                );
                let m = musha_framebuffer::metrics::Metrics::new(width, height);
                let v = musha_framebuffer::metrics::Metrics::viewer(width, height);
                let bottom = (32 * v.line_pitch).max(26 * m.line_pitch);
                assert!(
                    pixels[..bottom * width].iter().all(|v| *v == 0x12345678),
                    "application grid overwritten"
                );
            }
            assert_eq!(&pixels[width * height..], &[0x12345678; 16]);
            bootlog::configure(Config {
                log: LogLevel::Quiet,
                ..config
            });
            pixels[..width * height].fill(0x12345678);
            bootlog::failure("CLOCK", "missing");
            assert!(pixels[..width * height].iter().any(|v| *v != 0x12345678));
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
    assert!(
        pixels.iter().all(|p| *p == 0x12345678),
        "prepare must not draw at quiet"
    );
    appconsole::rejected(b"Invalid ELF or ABI NOTE");
    assert!(
        pixels[..640 * 480].iter().any(|p| *p != 0x12345678),
        "quiet rejection must be visible"
    );
    assert_eq!(&pixels[640 * 480..], &[0x12345678; 16]);
    for i in 0..300 {
        timing::record(format_args!("stage {i}"));
    }
    assert_eq!(timing::count(), 256);
    assert_eq!(timing::row(255).as_str(), "TIMING LIMIT / MORE OMITTED");
    musha_framebuffer::set_output_enabled(true);
}
