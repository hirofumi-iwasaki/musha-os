// SPDX-License-Identifier: Apache-2.0
//! BSP-only startup log. Application/viewer rendering is explicitly outside this policy.
use core::{cell::UnsafeCell, fmt::Write};
use musha_framebuffer::{Framebuffer, OutputGuard, metrics};
use musha_platform::{
    config::{Config, LogLevel},
    diagnostics::Line,
};
struct State {
    fb: Option<Framebuffer>,
    config: Config,
    row: usize,
    ready: bool,
    media: bool,
    keyboard: bool,
}
struct Shared(UnsafeCell<State>);
unsafe impl Sync for Shared {} // BSP only; no mutable reference is retained across callbacks.
static STATE: Shared = Shared(UnsafeCell::new(State {
    fb: None,
    config: Config {
        log: LogLevel::Ready,
        display: musha_platform::config::Display::Max,
        pair: false,
    },
    row: 0,
    ready: false,
    media: false,
    keyboard: false,
}));
pub fn configure(config: Config) {
    unsafe {
        (*STATE.0.get()).config = config;
    }
    musha_framebuffer::set_output_enabled(config.log == LogLevel::Devices);
}
pub fn devices() -> bool {
    unsafe { (*STATE.0.get()).config.log == LogLevel::Devices }
}
pub fn initialize(fb: Framebuffer, settings: crate::settings::Settings, fallback: bool) {
    unsafe {
        let s = &mut *STATE.0.get();
        s.fb = Some(fb);
        s.row = 0;
        s.ready = false;
        s.media = false;
        s.keyboard = false;
    }
    phase("UEFI", "configuration copied; firmware files closed");
    phase(
        "CONFIG",
        match settings.state {
            crate::settings::ReadState::Loaded => "loaded",
            crate::settings::ReadState::MissingDirectory => "SETTINGS absent; defaults",
            crate::settings::ReadState::MissingFile => "CONFIG absent; defaults",
            crate::settings::ReadState::Malformed => "malformed; all defaults",
            crate::settings::ReadState::NonFat32 => "boot device not FAT32; defaults",
            _ => "firmware read unavailable; defaults",
        },
    );
    if settings.forced {
        phase("LOG", "devices forced by F8/8");
    }
    if fallback {
        phase("DISPLAY", "requested mode unavailable; max fallback");
    }
    crate::debug(b"MUSHA: CONFIG LOG/DISPLAY/PAIR ");
    let mut line = Line::EMPTY;
    let _ = write!(
        line,
        "{:?} {:?} {} {:?}\n",
        settings.config.log, settings.config.display, settings.config.pair, settings.state
    );
    crate::debug(line.as_str().as_bytes());
}
pub fn phase(name: &str, status: &str) {
    let (config, fb, row) = unsafe {
        let s = &mut *STATE.0.get();
        let row = s.row;
        s.row = s.row.saturating_add(1);
        (s.config, s.fb, row)
    };
    let mut line = Line::EMPTY;
    let _ = write!(line, "{}: {}", name, status);
    crate::debug(b"MUSHA: PHASE ");
    crate::debug(line.as_str().as_bytes());
    crate::debug(b"\n");
    if !matches!(config.log, LogLevel::Phases | LogLevel::Devices) {
        return;
    }
    if let Some(fb) = fb {
        let _visible = OutputGuard::allow();
        let m = metrics::Metrics::new(fb.width, fb.height);
        if config.log == LogLevel::Devices {
            // Latest phase occupies a dedicated footer, below the existing diagnostic grid.
            let y = fb.height.saturating_sub(40 * m.scale);
            unsafe {
                for py in y..(y + 8 * m.scale).min(fb.height) {
                    for x in 0..fb.width {
                        fb.pixel(x, py, fb.color(12, 20, 32));
                    }
                }
                fb.text_scaled(line.as_str(), 8, y, fb.color(190, 210, 230), m.scale);
            }
            return;
        }
        // Log-only lines; no whole-background repaint at phases level.
        unsafe {
            fb.text_grid(
                line.as_str(),
                metrics::LABEL,
                metrics::MARGIN + row.min(m.rows.saturating_sub(5)),
                fb.color(190, 210, 230),
            );
        }
    }
}
pub fn media_ready(exfat: bool) {
    unsafe {
        (*STATE.0.get()).media = true;
    }
    phase(
        "Boot media",
        if exfat {
            "exFAT data readable"
        } else {
            "FAT32 readable"
        },
    );
}
pub fn keyboard(available: bool) {
    unsafe {
        (*STATE.0.get()).keyboard = available;
    }
    phase(
        "Keyboard",
        if available {
            "USB boot keyboard found"
        } else {
            "not found (application may be keyboard-free)"
        },
    );
}
pub fn ready(application: bool) {
    let (media, keyboard, already, pair) = unsafe {
        let s = &*STATE.0.get();
        (s.media, s.keyboard, s.ready, s.config.pair)
    };
    if already {
        return;
    }
    phase(
        "Bluetooth",
        if pair {
            "not implemented; pair=on retained"
        } else {
            "not implemented in this stage"
        },
    );
    phase(
        if application { "Application" } else { "Viewer" },
        "starting",
    );
    if !media {
        failure("BOOT MEDIA", "unavailable; recovery UI only");
        return;
    }
    if !application && !keyboard {
        failure("KEYBOARD", "required for viewer; unavailable");
        return;
    }
    unsafe {
        (*STATE.0.get()).ready = true;
    }
    if devices() {
        crate::diagnostics::startup_snapshot();
    }
    phase("Ready", "required device discovery complete");
    overlay();
    crate::debug(b"MUSHA: STARTUP_READY\n");
    #[cfg(feature = "startup-config-probe")]
    if option_env!("MUSHA_BOOT_PROBE_HOLD") == Some("ready") {
        crate::debug(b"MUSHA: STARTUP READY HOLD\n");
        loop {
            core::hint::spin_loop();
        }
    }
}
/// Only unused screen space below both existing fixed UIs is eligible for the footer.
pub fn overlay() {
    let (fb, config, ready) = unsafe {
        let s = &*STATE.0.get();
        (s.fb, s.config, s.ready)
    };
    if !ready || config.log == LogLevel::Quiet {
        return;
    }
    let Some(fb) = fb else {
        return;
    };
    let m = metrics::Metrics::new(fb.width, fb.height);
    let viewer = metrics::Metrics::viewer(fb.width, fb.height);
    let ui_bottom = (32 * viewer.line_pitch).max(26 * m.line_pitch);
    let y = fb.height.saturating_sub(16 * m.scale);
    if y.saturating_sub(8 * m.scale) <= ui_bottom {
        return;
    }
    let _visible = OutputGuard::allow();
    unsafe {
        for py in y..(y + 8 * m.scale).min(fb.height) {
            for px in 0..(200 * m.scale).min(fb.width) {
                fb.pixel(px, py, fb.color(12, 20, 32));
            }
        }
        fb.text_scaled("Musha-OS ready.", 8, y, fb.color(160, 240, 160), m.scale);
    }
    if devices() {
        crate::timing::summary(fb, y.saturating_sub(8 * m.scale), m.scale);
    }
}
pub fn failure(stage: &str, error: &str) {
    let fb = unsafe { (*STATE.0.get()).fb };
    crate::debug(b"MUSHA: STARTUP FAILURE ");
    crate::debug(stage.as_bytes());
    crate::debug(b": ");
    crate::debug(error.as_bytes());
    crate::debug(b"\n");
    if let Some(fb) = fb {
        let _visible = OutputGuard::allow();
        let m = metrics::Metrics::new(fb.width, fb.height);
        let y = fb.height.saturating_sub(24 * m.scale);
        let mut line = Line::EMPTY;
        let _ = write!(line, "FAILED {}: {}", stage, error);
        unsafe {
            fb.text_scaled(line.as_str(), 8, y, fb.color(255, 80, 80), m.scale);
        }
    }
}
