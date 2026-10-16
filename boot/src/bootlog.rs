// SPDX-License-Identifier: Apache-2.0
//! Single BSP console and firmware-independent startup history.
use core::{
    cell::UnsafeCell,
    fmt::{self, Write},
    sync::atomic::{AtomicBool, AtomicU8, AtomicU64, Ordering},
};
use musha_framebuffer::{Framebuffer, boot_console::Console};
use musha_platform::{
    boot_log::{Level, Ring},
    config::{Config, Display, LogLevel},
};
struct State {
    ring: Ring,
    console: Console,
    config: Config,
    ready: bool,
    early: bool,
    device_keys: [u128; 128],
    device_count: usize,
}
static MEDIA_SEEN: AtomicBool = AtomicBool::new(false);
static LAST_MS: AtomicU64 = AtomicU64::new(0);
static ERROR_SEEN: AtomicBool = AtomicBool::new(false);
static LOG_MODE: AtomicU8 = AtomicU8::new(LogLevel::Ready as u8);
static BUSY: AtomicBool = AtomicBool::new(false);
struct Lease;
impl Drop for Lease {
    fn drop(&mut self) {
        BUSY.store(false, Ordering::Release);
    }
}
fn lease() -> Option<Lease> {
    BUSY.compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
        .ok()
        .map(|_| Lease)
}
struct Shared(UnsafeCell<State>);
unsafe impl Sync for Shared {} // BSP only, interrupts disabled; no references escape.
static STATE: Shared = Shared(UnsafeCell::new(State {
    ring: Ring::new(),
    console: Console::new(),
    config: Config {
        log: LogLevel::Ready,
        display: Display::Max,
        pair: false,
    },
    ready: false,
    early: true,
    device_keys: [0; 128],
    device_count: 0,
}));
fn visible(config: LogLevel, level: Level) -> bool {
    match level {
        Level::Error | Level::Application => true,
        Level::Ready => config != LogLevel::Quiet,
        Level::Phases => matches!(
            config,
            LogLevel::Phases | LogLevel::Devices | LogLevel::Trace
        ),
        Level::Devices => matches!(config, LogLevel::Devices | LogLevel::Trace),
        Level::Trace => config == LogLevel::Trace,
    }
}
struct Text {
    bytes: [u8; 4096],
    used: usize,
}
impl Write for Text {
    fn write_str(&mut self, value: &str) -> fmt::Result {
        let end = self
            .used
            .checked_add(value.len())
            .filter(|n| *n < self.bytes.len())
            .ok_or(fmt::Error)?;
        self.bytes[self.used..end].copy_from_slice(value.as_bytes());
        self.used = end;
        Ok(())
    }
}
pub fn event(level: Level, tag: &str, args: fmt::Arguments<'_>) {
    event_inner(level, tag, args, false);
}
fn event_inner(level: Level, tag: &str, args: fmt::Arguments<'_>, summary: bool) {
    let mut text = Text {
        bytes: [0; 4096],
        used: 0,
    };
    // Fatal/NMI paths never reenter a possibly borrowed PM-clock state.
    let ms = if level == Level::Error {
        LAST_MS.load(Ordering::Relaxed)
    } else {
        let now = crate::bootstate::epoch_now().unwrap_or(0);
        LAST_MS.store(now, Ordering::Relaxed);
        now
    };
    let timestamp = !tag.is_empty()
        && (matches!(level, Level::Devices | Level::Trace)
            || (devices() && level != Level::Application));
    let result = if timestamp {
        if tag.is_empty() {
            write!(text, "[{:>3}.{:03}] {}", ms / 1000, ms % 1000, args)
        } else {
            write!(
                text,
                "[{:>3}.{:03}] {}: {}",
                ms / 1000,
                ms % 1000,
                tag,
                args
            )
        }
    } else if tag.is_empty() {
        text.write_fmt(args)
    } else {
        write!(text, "{}: {}", tag, args)
    };
    let level = if result.is_err() { Level::Error } else { level };
    if result.is_err() {
        text.used = 0;
        let _ = text.write_str("ERROR: log: record exceeds 4096 bytes");
    }
    if text.used == 0 || text.bytes[text.used - 1] != b'\n' {
        text.bytes[text.used] = b'\n';
        text.used += 1;
    }
    emit(level, ms, &text.bytes[..text.used], summary);
}
fn emit(level: Level, ms: u64, bytes: &[u8], summary: bool) {
    if level == Level::Error {
        ERROR_SEEN.store(true, Ordering::Relaxed);
    }
    let Some(_lease) = lease() else {
        // A fault during rendering must not create a second mutable reference.
        crate::debug(b"MUSHA: LOG_REENTRANT ");
        crate::debug(bytes);
        return;
    };
    unsafe {
        let s = &mut *STATE.0.get();
        s.ring.push(level, ms, s.early, bytes);
        if visible(s.config.log, level) && !(summary && s.config.log == LogLevel::Devices) {
            if level == Level::Error {
                s.console.set_visible(true);
            }
            s.console.append(bytes);
        }
    }
    crate::debug(b"MUSHA: LOG ");
    crate::debug(bytes);
    if !bytes.ends_with(b"\n") {
        crate::debug(b"\n");
    }
}
pub fn detail(tag: &str, args: fmt::Arguments<'_>) {
    event(Level::Trace, tag, args);
}
pub fn device(tag: &str, args: fmt::Arguments<'_>) {
    event(Level::Devices, tag, args);
}
pub fn device_once(key: u128, tag: &str, args: fmt::Arguments<'_>) {
    let mut repeated = true;
    if let Some(_lease) = lease() {
        unsafe {
            let state = &mut *STATE.0.get();
            if !state.device_keys[..state.device_count].contains(&key)
                && state.device_count < state.device_keys.len()
            {
                state.device_keys[state.device_count] = key;
                state.device_count += 1;
                repeated = false;
            }
        }
    }
    event(
        if repeated {
            Level::Trace
        } else {
            Level::Devices
        },
        tag,
        args,
    );
}
pub fn app(bytes: &[u8]) {
    emit(
        Level::Application,
        crate::bootstate::epoch_now().unwrap_or(0),
        bytes,
        false,
    );
}
pub fn configure(config: Config) {
    let Some(_lease) = lease() else {
        return;
    };
    LOG_MODE.store(config.log as u8, Ordering::Relaxed);
    unsafe {
        (*STATE.0.get()).config = config;
    }
    musha_framebuffer::set_output_enabled(true);
}
pub fn pair_requested() -> bool {
    unsafe { (*STATE.0.get()).config.pair }
}
pub fn devices() -> bool {
    matches!(LOG_MODE.load(Ordering::Relaxed), v if v == LogLevel::Devices as u8 || v == LogLevel::Trace as u8)
}
pub fn trace() -> bool {
    LOG_MODE.load(Ordering::Relaxed) == LogLevel::Trace as u8
}
pub fn bind(fb: Framebuffer) {
    let Some(_lease) = lease() else {
        return;
    };
    unsafe {
        let _ = (*STATE.0.get()).console.bind(fb);
    }
}
pub fn initialize(fb: Framebuffer, settings: crate::settings::Settings, fallback: bool) {
    bind(fb);
    event(Level::Phases, "", format_args!("Musha-OS 0.4.0 starting"));
    summary_phase(
        "settings",
        match settings.state {
            crate::settings::ReadState::Loaded => "CONFIG.TXT loaded",
            crate::settings::ReadState::Malformed => "CONFIG.TXT malformed; defaults",
            crate::settings::ReadState::NonFat32 => "boot device is not FAT32; defaults",
            _ => "CONFIG.TXT unavailable; defaults",
        },
    );
    event_inner(
        Level::Phases,
        "display",
        format_args!(
            "GOP {}x{} stride={} format={}",
            fb.width, fb.height, fb.stride, fb.format
        ),
        true,
    );
    device(
        "settings",
        format_args!(
            "{:?} log={:?} display={:?} pair={}",
            settings.state, settings.config.log, settings.config.display, settings.config.pair
        ),
    );
    device(
        "display",
        format_args!(
            "{}x{} stride={} format={} base=0x{:x} bytes={}",
            fb.width, fb.height, fb.stride, fb.format, fb.base, fb.bytes
        ),
    );
    if settings.forced {
        phase("settings", "trace selected by F8/8");
    }
    if fallback {
        phase("display", "requested mode unavailable; maximum selected");
    }
}
pub fn firmware_exited() {
    unsafe {
        (*STATE.0.get()).early = false;
    }
}
pub fn summary_phase(name: &str, status: &str) {
    event_inner(Level::Phases, name, format_args!("{}", status), true);
}
pub fn phase(name: &str, status: &str) {
    event(Level::Phases, name, format_args!("{}", status));
}
pub fn media_seen() -> bool {
    MEDIA_SEEN.load(Ordering::Acquire)
}
// A boot-volume observation outlives its scoped USB file handles.
pub fn retain_media_observation() {
    MEDIA_SEEN.store(true, Ordering::Release);
}
pub fn media_ready(exfat: bool) {
    retain_media_observation();
    summary_phase(
        "storage",
        if exfat {
            "exFAT readable"
        } else {
            "FAT32 readable"
        },
    );
}
pub fn keyboard(available: bool) {
    phase(
        "keyboard",
        if available {
            "USB keyboard available"
        } else {
            "unavailable"
        },
    );
}
pub fn ready(application: bool) {
    unsafe {
        if (*STATE.0.get()).ready {
            return;
        }
        (*STATE.0.get()).ready = true;
    }
    if !application {
        phase("viewer", "starting");
    }
    event(
        Level::Ready,
        "",
        format_args!("Musha-OS {} ready", include_str!("../../VERSION").trim()),
    );
    crate::debug(b"MUSHA: STARTUP_READY\n");
    #[cfg(feature = "startup-config-probe")]
    dump();
    #[cfg(feature = "startup-config-probe")]
    if option_env!("MUSHA_BOOT_PROBE_HOLD") == Some("ready") {
        loop {
            core::hint::spin_loop();
        }
    }
}
pub fn viewer(hidden: bool) {
    musha_framebuffer::flush_writes();
    let Some(_lease) = lease() else {
        return;
    };
    unsafe {
        (*STATE.0.get()).console.set_visible(!hidden);
    }
}
pub fn application_screen() {
    let Some(_lease) = lease() else {
        return;
    };
    unsafe {
        let s = &mut *STATE.0.get();
        s.console.clear_black();
        s.console.set_visible(true);
    }
}
/// No footer exists; retained only while safe-point callers are migrated.
pub fn overlay() {}
pub fn failure(stage: &str, reason: &str) {
    event(Level::Error, "ERROR", format_args!("{}: {}", stage, reason));
}
pub fn halted() {
    if !ERROR_SEEN.load(Ordering::Relaxed) {
        return;
    }
    event(Level::Error, "", format_args!("Musha-OS halted"));
}

/// Dump retained records without changing pixels, device state or timestamps.
pub fn dump() {
    let Some(_lease) = lease() else {
        return;
    };
    unsafe {
        let s = &*STATE.0.get();
        let mut metadata = Text {
            bytes: [0; 4096],
            used: 0,
        };
        let _ = write!(
            metadata,
            "MUSHA: RING_META bytes={} lost={}\n",
            s.ring.used(),
            s.ring.lost()
        );
        crate::debug(&metadata.bytes[..metadata.used]);
        crate::debug(b"MUSHA: RING_BEGIN\n");
        for record in s.ring.records() {
            crate::debug(b"MUSHA: RING ");
            let mut last = b'\n';
            for byte in record.bytes() {
                crate::debug(&[byte]);
                last = byte;
            }
            if last != b'\n' {
                crate::debug(b"\n");
            }
        }
        crate::debug(b"MUSHA: RING_END\n");
    }
}

#[cfg(test)]
pub fn test_record_levels(mut record: impl FnMut(Level, u64)) {
    let Some(_lease) = lease() else {
        return;
    };
    unsafe {
        for item in (*STATE.0.get()).ring.records() {
            record(item.level(), item.milliseconds());
        }
    }
}
