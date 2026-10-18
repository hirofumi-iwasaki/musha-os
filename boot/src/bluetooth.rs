// Copyright 2026 Hirofumi Iwasaki
// SPDX-License-Identifier: Apache-2.0
//! BSP-owned request routing. No Ring3 entry point or arbitrary pairing trigger.
use core::sync::atomic::{AtomicBool, Ordering};
static REQUEST: AtomicBool = AtomicBool::new(false);
static ACTIVE: AtomicBool = AtomicBool::new(false);
pub fn present() -> bool {
    ACTIVE.load(Ordering::Relaxed)
}
pub fn active(yes: bool) {
    ACTIVE.store(yes, Ordering::Relaxed);
}
pub fn request() -> Result<(), &'static str> {
    if !ACTIVE.load(Ordering::Relaxed) {
        return Err("Bluetooth pairing unavailable: no live USB HCI adapter.");
    }
    if !crate::entropy::available() {
        return Err(crate::entropy::reason());
    }
    CONFIG_REQUEST.store(false, Ordering::Relaxed);
    REQUEST.store(true, Ordering::Relaxed);
    Ok(())
}
pub fn take_request() -> bool {
    if !RECORD_READY.load(Ordering::Relaxed) {
        return false;
    }
    if CONFIG_REQUEST.load(Ordering::Relaxed)
        && crate::settings::pairing_config_hash()
            .is_some_and(|hash| unsafe { *CONSUMED.0.get() == Some(hash) })
    {
        REQUEST.store(false, Ordering::Relaxed);
        return false;
    }
    REQUEST.swap(false, Ordering::Relaxed)
}
pub fn defer_boot_request() {
    if FORCE_NEXT.swap(false, Ordering::Relaxed) {
        CONFIG_REQUEST.store(false, Ordering::Relaxed);
        REQUEST.store(true, Ordering::Relaxed);
        return;
    }
    if crate::bootlog::pair_requested() {
        CONFIG_REQUEST.store(true, Ordering::Relaxed);
        REQUEST.store(true, Ordering::Relaxed);
    }
}

use core::cell::UnsafeCell;
enum Ui {
    Display(u64, u32),
    Enter(u64),
    Status(&'static str),
}
struct Shared<T>(UnsafeCell<T>);
unsafe impl<T> Sync for Shared<T> {} // BSP-only; no borrowed value survives calls.
static UI: Shared<Option<Ui>> = Shared(UnsafeCell::new(None));
static ACK: Shared<Option<u64>> = Shared(UnsafeCell::new(None));
static ENTRY: Shared<Option<(u64, Option<musha_input::passkey::Secret>)>> =
    Shared(UnsafeCell::new(None));
static ATTEMPTS: core::sync::atomic::AtomicU8 = core::sync::atomic::AtomicU8::new(0);
pub fn begin_attempt() -> bool {
    ATTEMPTS
        .try_update(Ordering::Relaxed, Ordering::Relaxed, |n| {
            (n < 8).then_some(n + 1)
        })
        .is_ok()
}
pub fn stopped() {
    CAPTURE.store(false, Ordering::Relaxed);
    INPUT_OVERFLOW.store(false, Ordering::Relaxed);
    set_peer(None);
    RECORD_READY.store(false, Ordering::Relaxed);
    unsafe {
        (&mut *STORE.0.get()).clear();
        let pad = &mut *PAD.0.get();
        pad.1 = 0;
        pad.2 = false;
        (&mut *PENDING_SAVE.0.get()).take();
        *SAVED.0.get() = None;
        *SAVE_FAILURE.0.get() = "BT BOND SAVE UNKNOWN";
        *KEYS.0.get() = ([(0, false); 32], 0, u64::MAX, false);
        // Queued old reports are discarded, then only genuine held-state
        // releases/axis neutralization are delivered to an existing r2 consumer.
        let (buttons, axes) = *GAMEPAD_HELD.0.get();
        *GAMEPAD_HELD.0.get() = (0, [0; 6]);
        for code in 0..32 {
            if buttons & (1 << code) != 0 {
                gamepad(musha_input::gamepad::Change::Button(code, false));
            }
        }
        for (code, value) in axes.into_iter().enumerate() {
            if value != 0 {
                gamepad(musha_input::gamepad::Change::Axis(code as u8, 0));
            }
        }
        (&mut *UI.0.get()).take();
        (&mut *ENTRY.0.get()).take();
        *ACK.0.get() = None;
    }
    status("Bluetooth session stopped; input and pairing disabled.");
}
static CAPTURE: AtomicBool = AtomicBool::new(false);
pub fn captures_input() -> bool {
    CAPTURE.load(Ordering::Relaxed)
}
static SENSITIVE: AtomicBool = AtomicBool::new(false);
pub fn sensitive_input() -> bool {
    SENSITIVE.load(Ordering::Relaxed)
}
pub fn display(generation: u64, value: u32) {
    unsafe {
        *UI.0.get() = Some(Ui::Display(generation, value));
    }
}
pub fn enter(generation: u64) {
    CAPTURE.store(true, Ordering::Relaxed);
    SENSITIVE.store(true, Ordering::Relaxed);
    unsafe {
        *UI.0.get() = Some(Ui::Enter(generation));
    }
}
pub fn status(message: &'static str) {
    unsafe {
        *UI.0.get() = Some(Ui::Status(message));
    }
}
pub fn displayed(generation: u64) -> bool {
    unsafe {
        let ack = &mut *ACK.0.get();
        if *ack == Some(generation) {
            *ack = None;
            true
        } else {
            false
        }
    }
}
pub fn take_entry(generation: u64) -> Option<Option<musha_input::passkey::Secret>> {
    unsafe {
        let entry = &mut *ENTRY.0.get();
        if entry.as_ref().is_some_and(|e| e.0 == generation) {
            entry.take().map(|e| e.1)
        } else {
            None
        }
    }
}
pub fn service(viewer: &mut musha_viewer::Viewer, now: u64) {
    #[cfg(feature = "bluetooth-restart-probe")]
    {
        static DONE: AtomicBool = AtomicBool::new(false);
        if viewer.painted() && !DONE.swap(true, Ordering::Relaxed) {
            restart_after_stop(false);
            crate::debug(b"MUSHA: BT_RESTART_VIEWER_SOFTWARE_FIXTURE\n");
        }
    }

    static BOOT_NOTICE: AtomicBool = AtomicBool::new(false);
    if crate::bootlog::pair_requested()
        && !BOOT_NOTICE.swap(true, Ordering::Relaxed)
        && !ACTIVE.load(Ordering::Relaxed)
    {
        viewer.pairing_status("Bluetooth pairing unavailable: no live USB HCI adapter.");
        crate::debug(b"MUSHA: BT_PAIR_REQUEST_BLOCKED NO_HCI_ADAPTER\n");
    }

    #[cfg(feature = "bluetooth-passkey-probe")]
    {
        static ONCE: AtomicBool = AtomicBool::new(false);
        if !ONCE.swap(true, Ordering::Relaxed) {
            if !crypto_probe() {
                crate::debug(b"MUSHA: BT_CRYPTO_KAT_FAILED\n");
                return;
            }
            enter(0xbeef);
            crate::debug(b"MUSHA: BT_PASSKEY_FIXTURE_BEGIN\n");
        }
    }

    // Stage lines would otherwise stay hidden behind the viewer.
    static TAIL_SEEN: core::sync::atomic::AtomicU64 = core::sync::atomic::AtomicU64::new(0);
    let seen = TAIL_SEEN.load(Ordering::Relaxed);
    TAIL_SEEN.store(
        crate::bootlog::bluetooth_tail(seen, |line| viewer.push_log(line)),
        Ordering::Relaxed,
    );
    unsafe {
        let p = &mut *PEER.0.get();
        if p.2 {
            p.2 = false;
            viewer.set_peer(core::str::from_utf8(&p.0[..p.1]).unwrap_or(""));
        }
    }
    let message = unsafe { (&mut *UI.0.get()).take() };
    if let Some(message) = message {
        match message {
            Ui::Display(generation, value) => {
                viewer.display_passkey(value);
                unsafe {
                    *ACK.0.get() = Some(generation);
                }
            }
            Ui::Enter(generation) => {
                if !viewer.begin_passkey_input(generation, now) {
                    unsafe {
                        *ENTRY.0.get() = Some((generation, None));
                    }
                }
            }
            Ui::Status(message) => viewer.pairing_status(message),
        }
    }
    if let Some((generation, outcome)) = viewer.take_passkey() {
        CAPTURE.store(false, Ordering::Relaxed);
        #[cfg(feature = "bluetooth-passkey-probe")]
        if generation == 0xbeef {
            crate::debug(match &outcome {
                musha_input::passkey::Outcome::Submitted(_) => b"MUSHA: BT_PASSKEY_SUBMITTED\n",
                musha_input::passkey::Outcome::Cancelled => b"MUSHA: BT_PASSKEY_CANCELLED\n",
                musha_input::passkey::Outcome::TimedOut => b"MUSHA: BT_PASSKEY_TIMEOUT\n",
            });
            return;
        }
        let value = match outcome {
            musha_input::passkey::Outcome::Submitted(secret) => Some(secret),
            _ => None,
        };
        unsafe {
            *ENTRY.0.get() = Some((generation, value));
        }
    }
}
static PENDING_SAVE: Shared<Option<(u64, musha_platform::bluetooth::legacy::VerifiedBond)>> =
    Shared(UnsafeCell::new(None));
static SAVED: Shared<Option<(u64, bool)>> = Shared(UnsafeCell::new(None));
static RECORD_READY: AtomicBool = AtomicBool::new(false);
static CONFIG_REQUEST: AtomicBool = AtomicBool::new(false);
static CONSUMED: Shared<Option<[u8; 32]>> = Shared(UnsafeCell::new(None));
static KEYS: Shared<([(u8, bool); 32], usize, u64, bool)> =
    Shared(UnsafeCell::new(([(0, false); 32], 0, 0, false)));
pub fn storage_ready() -> bool {
    RECORD_READY.load(Ordering::Relaxed)
}
pub fn record_ready(hash: Option<[u8; 32]>) {
    unsafe {
        *CONSUMED.0.get() = hash;
    }
    RECORD_READY.store(true, Ordering::Relaxed);
}
pub fn save(
    generation: u64,
    bond: musha_platform::bluetooth::legacy::VerifiedBond,
) -> Result<(), &'static str> {
    unsafe {
        let p = &mut *PENDING_SAVE.0.get();
        if p.is_some() {
            return Err("BT SAVE BUSY");
        }
        *p = Some((generation, bond));
    }
    Ok(())
}
pub fn pending_save() -> Option<(u64, musha_platform::bluetooth::legacy::VerifiedBond)> {
    unsafe { (&mut *PENDING_SAVE.0.get()).take() }
}
pub fn save_result(
    generation: u64,
    result: Result<Option<[u8; 32]>, musha_platform::bluetooth::bonding::SaveError>,
) {
    let ok = result.is_ok();
    if let Ok(hash) = result {
        unsafe {
            *CONSUMED.0.get() = hash;
        }
    }
    unsafe {
        *SAVED.0.get() = Some((generation, ok));
    }
}
/// Connected peer shown on its own viewer row while connecting (metadata only:
/// two address octets and the address class). None clears the row.
static PEER: Shared<([u8; 80], usize, bool)> = Shared(UnsafeCell::new(([0; 80], 0, false)));
pub fn set_peer(line: Option<core::fmt::Arguments<'_>>) {
    struct Line<'a>(&'a mut [u8; 80], usize);
    impl core::fmt::Write for Line<'_> {
        fn write_str(&mut self, s: &str) -> core::fmt::Result {
            let n = s.len().min(self.0.len() - self.1);
            self.0[self.1..self.1 + n].copy_from_slice(&s.as_bytes()[..n]);
            self.1 += n;
            Ok(())
        }
    }
    unsafe {
        let p = &mut *PEER.0.get();
        let mut out = Line(&mut p.0, 0);
        if let Some(args) = line {
            let _ = core::fmt::write(&mut out, args);
        }
        p.1 = out.1;
        p.2 = true;
    }
}
/// Why the last bond save failed; the Bluetooth poller reports it on isolation.
static SAVE_FAILURE: Shared<&'static str> = Shared(UnsafeCell::new("BT BOND SAVE UNKNOWN"));
pub fn save_failed(reason: &'static str) {
    unsafe {
        *SAVE_FAILURE.0.get() = reason;
    }
}
pub fn save_failure() -> &'static str {
    unsafe { *SAVE_FAILURE.0.get() }
}
pub fn saved(generation: u64) -> Option<bool> {
    unsafe {
        let saved = &mut *SAVED.0.get();
        if saved.as_ref().is_some_and(|s| s.0 == generation) {
            saved.take().map(|s| s.1)
        } else {
            None
        }
    }
}
pub fn key(generation: u64, key: u8, down: bool) {
    unsafe {
        let s = &mut *KEYS.0.get();
        if s.1 != 0 && s.2 != generation {
            s.3 = true;
            return;
        }
        s.2 = generation;
        if s.1 == s.0.len() {
            s.3 = true;
        } else {
            s.0[s.1] = (key, down);
            s.1 += 1;
        }
    }
}
/// Overflowed Bluetooth input is dropped and reported to the Bluetooth poller,
/// which isolates only Bluetooth; stopped() then releases held keys and axes.
static INPUT_OVERFLOW: AtomicBool = AtomicBool::new(false);
pub fn take_input_overflow() -> bool {
    INPUT_OVERFLOW.swap(false, Ordering::Relaxed)
}
pub fn drain_keys() -> Result<(u64, [(u8, bool); 32], usize), &'static str> {
    unsafe {
        let s = &mut *KEYS.0.get();
        if s.3 {
            INPUT_OVERFLOW.store(true, Ordering::Relaxed);
            s.1 = 0;
            s.3 = false;
            return Ok((s.2, s.0, 0));
        }
        let result = (s.2, s.0, s.1);
        s.1 = 0;
        Ok(result)
    }
}

#[cfg(feature = "bluetooth-passkey-probe")]
fn crypto_probe() -> bool {
    // Published P-256 sample2. Fixed scalar exists only in this opt-in test.
    struct Vector;
    impl musha_platform::bluetooth::entropy::Random for Vector {
        fn fill(&mut self, b: &mut [u8]) -> Result<(), musha_platform::bluetooth::Error> {
            if b.len() != 32 {
                return Err(musha_platform::bluetooth::Error::Length);
            }
            b.copy_from_slice(&PRIVATE);
            Ok(())
        }
    }
    const PRIVATE: [u8; 32] = [
        0x06, 0xa5, 0x16, 0x69, 0x3c, 0x9a, 0xa3, 0x1a, 0x60, 0x84, 0x54, 0x5d, 0x0c, 0x5d, 0xb6,
        0x41, 0xb4, 0x85, 0x72, 0xb9, 0x72, 0x03, 0xdd, 0xff, 0xb7, 0xac, 0x73, 0xf7, 0xd0, 0x45,
        0x76, 0x63,
    ];
    const X: [u8; 32] = [
        0xf4, 0x65, 0xe4, 0x3f, 0xf2, 0x3d, 0x3f, 0x1b, 0x9d, 0xc7, 0xdf, 0xc0, 0x4d, 0xa8, 0x75,
        0x81, 0x84, 0xdb, 0xc9, 0x66, 0x20, 0x47, 0x96, 0xec, 0xcf, 0x0d, 0x6c, 0xf5, 0xe1, 0x65,
        0x00, 0xcc,
    ];
    const Y: [u8; 32] = [
        0x02, 0x01, 0xd0, 0x48, 0xbc, 0xbb, 0xd8, 0x99, 0xee, 0xef, 0xc4, 0x24, 0x16, 0x4e, 0x33,
        0xc2, 0x01, 0xc2, 0xb0, 0x10, 0xca, 0x6b, 0x4d, 0x43, 0xa8, 0xa1, 0x55, 0xca, 0xd8, 0xec,
        0xb2, 0x79,
    ];
    const DH: [u8; 32] = [
        0xab, 0x85, 0x84, 0x3a, 0x2f, 0x6d, 0x88, 0x3f, 0x62, 0xe5, 0x68, 0x4b, 0x38, 0xe3, 0x07,
        0x33, 0x5f, 0xe6, 0xe1, 0x94, 0x5e, 0xcd, 0x19, 0x60, 0x41, 0x05, 0xc6, 0xf2, 0x32, 0x21,
        0xeb, 0x69,
    ];
    let Ok(key) = musha_platform::bluetooth::sc_crypto::Key::generate(&mut Vector) else {
        return false;
    };
    let mut peer = [0; 64];
    peer[..32].copy_from_slice(&X);
    peer[32..].copy_from_slice(&Y);
    peer[..32].reverse();
    peer[32..].reverse();
    let valid = key.dh(&peer) == Ok(DH);
    if valid {
        crate::debug(b"MUSHA: BT_CRYPTO_ECDH_KAT_OK\n");
    }
    valid
}

static STORE: Shared<musha_platform::bluetooth::reconnect::Store> = Shared(UnsafeCell::new(
    musha_platform::bluetooth::reconnect::Store::new(),
));
pub fn load_record(record: Option<&[u8]>) -> Result<(), &'static str> {
    let hash = unsafe {
        let store = &mut *STORE.0.get();
        store.clear();
        match record {
            Some(bytes) => store.load(bytes).map_err(|_| "BT RECORD CORRUPT")?,
            None => None,
        }
    };
    record_ready(hash);
    Ok(())
}
pub fn stored_any() -> bool {
    unsafe { (&*STORE.0.get()).any() }
}
pub fn stored_matches(kind: u8, address: &[u8; 6]) -> bool {
    unsafe { (&*STORE.0.get()).matches(kind, address) }
}
/// True when the stored bond matched through its IRK, not its identity.
pub fn stored_resolved(kind: u8, address: &[u8; 6]) -> bool {
    unsafe { (&*STORE.0.get()).resolved(kind, address) }
}
pub fn stored_connectable(event_type: u8, kind: u8, address: &[u8; 6]) -> bool {
    unsafe { (&*STORE.0.get()).connectable(event_type, kind, address) }
}
pub fn reconnect(
    generation: u64,
    handle: u16,
    now: u64,
    kind: u8,
    address: [u8; 6],
) -> Result<musha_platform::bluetooth::reconnect::Pending, musha_platform::bluetooth::Error> {
    unsafe { (&*STORE.0.get()).begin(generation, handle, now, kind, address) }
}
pub fn reused(generation: u64) {
    unsafe {
        *SAVED.0.get() = Some((generation, true));
    }
}
static PAD: Shared<([musha_platform::user_io::Event; 64], usize, bool)> =
    Shared(UnsafeCell::new((
        [musha_platform::user_io::Event {
            kind: 0,
            flags: 0,
            timestamp_ms: 0,
            device: 0,
            code: 0,
            modifiers: 0,
            values: [0; 3],
            buttons: 0,
        }; 64],
        0,
        false,
    )));
static GAMEPAD_HELD: Shared<(u32, [i32; 6])> = Shared(UnsafeCell::new((0, [0; 6])));
pub fn gamepad(change: musha_input::gamepad::Change) {
    use musha_platform::user_io::{Event, GAMEPAD_AXIS, GAMEPAD_BUTTON};
    let mut event = Event {
        device: 2,
        timestamp_ms: crate::bootstate::epoch_now().unwrap_or(0),
        ..Event::default()
    };
    match change {
        musha_input::gamepad::Change::Button(code, down) => {
            if code < 32 {
                unsafe {
                    let held = &mut *GAMEPAD_HELD.0.get();
                    if down {
                        held.0 |= 1 << code;
                    } else {
                        held.0 &= !(1 << code);
                    }
                }
            }
            event.kind = GAMEPAD_BUTTON;
            event.code = code as u16;
            event.flags = down as u32;
        }
        musha_input::gamepad::Change::Axis(code, value) => {
            if let Some(axis) = unsafe { (&mut *GAMEPAD_HELD.0.get()).1.get_mut(code as usize) } {
                *axis = value;
            }
            event.kind = GAMEPAD_AXIS;
            event.code = code as u16;
            event.values[0] = value;
        }
    }
    unsafe {
        let s = &mut *PAD.0.get();
        if s.1 == s.0.len() {
            s.2 = true;
        } else {
            s.0[s.1] = event;
            s.1 += 1;
        }
    }
}
pub fn drain_gamepad(
    mut emit: impl FnMut(musha_platform::user_io::Event),
) -> Result<(), &'static str> {
    unsafe {
        let s = &mut *PAD.0.get();
        if s.2 {
            INPUT_OVERFLOW.store(true, Ordering::Relaxed);
            s.1 = 0;
            s.2 = false;
            return Ok(());
        }
        for e in &s.0[..s.1] {
            emit(*e);
        }
        s.1 = 0;
    }
    Ok(())
}

#[cfg(feature = "bluetooth-gamepad-probe")]
pub fn gamepad_probe() {
    // Software report fixture only. Never a fake production radio/bond token.
    static ONCE: AtomicBool = AtomicBool::new(false);
    if ONCE.swap(true, Ordering::Relaxed) {
        return;
    }
    let map = [
        5, 1, 9, 5, 0xa1, 1, 0x85, 6, 5, 9, 9, 1, 0x15, 0, 0x25, 1, 0x75, 1, 0x95, 1, 0x81, 2,
        0x75, 7, 0x95, 1, 0x81, 3, 5, 1, 9, 0x30, 0x15, 0, 0x26, 255, 0, 0x75, 8, 0x95, 1, 0x81, 2,
        0xc0,
    ];
    let layout = musha_input::gamepad::Layout::parse(&map).expect("GAMEPAD FIXTURE MAP");
    let mut decoder = musha_input::gamepad::Decoder::new(layout);
    decoder
        .update(&[6, 1, 128], gamepad)
        .expect("GAMEPAD FIXTURE PRESS");
    decoder
        .update(&[6, 0, 64], gamepad)
        .expect("GAMEPAD FIXTURE RELEASE");
    crate::debug(b"MUSHA: BT_GAMEPAD_SOFTWARE_FIXTURE_OK\n");
}

static RESTART: AtomicBool = AtomicBool::new(false);
static RESTART_SAFE: AtomicBool = AtomicBool::new(false);
static FORCE_NEXT: AtomicBool = AtomicBool::new(false);
static RESTARTS: core::sync::atomic::AtomicU8 = core::sync::atomic::AtomicU8::new(0);
pub fn restart_after_stop(force_pair: bool) -> bool {
    if RESTART.load(Ordering::Relaxed) {
        return true;
    }
    if RESTARTS
        .try_update(Ordering::Relaxed, Ordering::Relaxed, |n| {
            (n < 8).then_some(n + 1)
        })
        .is_err()
    {
        return false;
    }
    FORCE_NEXT.store(force_pair, Ordering::Relaxed);
    RESTART_SAFE.store(false, Ordering::Relaxed);
    RESTART.store(true, Ordering::Relaxed);
    true
}
/// A successfully isolated optional adapter must not request a whole USB restart.
pub fn isolated() {
    RESTART.store(false, Ordering::Relaxed);
    RESTART_SAFE.store(false, Ordering::Relaxed);
    FORCE_NEXT.store(false, Ordering::Relaxed);
    REQUEST.store(false, Ordering::Relaxed);
    CONFIG_REQUEST.store(false, Ordering::Relaxed);
    ACTIVE.store(false, Ordering::Relaxed);
}
pub fn wants_restart() -> bool {
    RESTART.load(Ordering::Relaxed)
}
/// Does not release references. Only caller's normal return through every Host,
/// file session and Ring3 loan scope can authorize take_restart in external::finish.
pub fn confirm_restart(stopped: bool) {
    if wants_restart() {
        RESTART_SAFE.store(stopped, Ordering::Relaxed);
        if stopped {
            crate::debug(b"MUSHA: BT_RESTART_STOP_CONFIRMED\n");
        }
    }
}
pub fn take_restart() -> bool {
    if !RESTART_SAFE.load(Ordering::Relaxed) {
        return false;
    }
    RESTART.swap(false, Ordering::Relaxed)
}
#[cfg(feature = "bluetooth-loan-restart-probe")]
pub fn loan_probe() -> bool {
    static DONE: AtomicBool = AtomicBool::new(false);
    static START: core::sync::atomic::AtomicU64 = core::sync::atomic::AtomicU64::new(u64::MAX);
    let Some(now) = crate::bootstate::epoch_now() else {
        return false;
    };
    let start = START.load(Ordering::Relaxed);
    if start == u64::MAX {
        START.store(now, Ordering::Relaxed);
        return false;
    }
    if now.saturating_sub(start) >= 1000 && !DONE.swap(true, Ordering::Relaxed) {
        crate::debug(b"MUSHA: BT_RESTART_RING3_SOFTWARE_FIXTURE\n");
        return restart_after_stop(false);
    }
    false
}
