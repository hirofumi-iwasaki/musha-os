// Copyright 2026 Hirofumi Iwasaki
// SPDX-License-Identifier: Apache-2.0
//! Non-secret pairing-stage metadata for the devices log. Never keys, nonces,
//! confirm/random values, DHKey checks or full addresses.
#![forbid(unsafe_code)]

/// SMP PDU code name (Core Vol 3 Part H 3.3). The PDU body is never logged.
pub fn smp_name(code: u8) -> &'static str {
    match code {
        0x01 => "Pairing Request",
        0x02 => "Pairing Response",
        0x03 => "Pairing Confirm",
        0x04 => "Pairing Random",
        0x05 => "Pairing Failed",
        0x06 => "Encryption Information",
        0x07 => "Central Identification",
        0x08 => "Identity Information",
        0x09 => "Identity Address Information",
        0x0a => "Signing Information",
        0x0b => "Security Request",
        0x0c => "Pairing Public Key",
        0x0d => "Pairing DHKey Check",
        0x0e => "Pairing Keypress Notification",
        _ => "unknown",
    }
}

/// Lowest two address octets in display order (HCI order is little-endian).
pub fn address_tail(address: &[u8; 6]) -> [u8; 2] {
    [address[1], address[0]]
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AdvertisingSummary {
    pub elapsed: u64,
    pub events: u32,
    pub reports: u32,
    pub hid: u32,
    pub unsupported: u32,
    pub last: Option<[u8; 2]>,
}
/// One-second advertising aggregation; individual reports are never emitted.
pub struct Advertising {
    started: Option<u64>,
    events: u32,
    reports: u32,
    hid: u32,
    unsupported: u32,
    last: Option<[u8; 2]>,
}
impl Default for Advertising {
    fn default() -> Self {
        Self::new()
    }
}
impl Advertising {
    pub const fn new() -> Self {
        Self {
            started: None,
            events: 0,
            reports: 0,
            hid: 0,
            unsupported: 0,
            last: None,
        }
    }
    pub fn event(&mut self, now: u64) {
        self.started.get_or_insert(now);
        self.events = self.events.saturating_add(1);
    }
    pub fn report(&mut self, address: &[u8; 6]) {
        self.reports = self.reports.saturating_add(1);
        self.last = Some(address_tail(address));
    }
    /// HID service advertised and accepted as a keyboard/gamepad candidate.
    pub fn hid(&mut self) {
        self.hid = self.hid.saturating_add(1);
    }
    /// HID service advertised with an appearance this host does not pair.
    pub fn unsupported(&mut self) {
        self.unsupported = self.unsupported.saturating_add(1);
    }
    /// Returns and resets the window once at least 1000 ms have passed.
    pub fn due(&mut self, now: u64) -> Option<AdvertisingSummary> {
        self.take(now, 1000)
    }
    /// Returns and resets a non-empty window early, e.g. when scanning ends.
    pub fn flush(&mut self, now: u64) -> Option<AdvertisingSummary> {
        self.take(now, 0)
    }
    fn take(&mut self, now: u64, minimum: u64) -> Option<AdvertisingSummary> {
        let started = self.started?;
        let elapsed = now.checked_sub(started)?;
        if elapsed < minimum {
            return None;
        }
        let summary = AdvertisingSummary {
            elapsed,
            events: self.events,
            reports: self.reports,
            hid: self.hid,
            unsupported: self.unsupported,
            last: self.last,
        };
        *self = Self::new();
        Some(summary)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ConnectionComplete {
    pub status: u8,
    pub handle: u16,
    pub role: u8,
    pub peer_type: u8,
    pub peer_tail: [u8; 2],
    /// Units of 1.25 ms.
    pub interval: u16,
    pub latency: u16,
    /// Units of 10 ms.
    pub timeout: u16,
}
/// LE Connection Complete parameters including the 0x01 subevent byte.
pub fn connection_complete(p: &[u8]) -> Option<ConnectionComplete> {
    if p.len() != 19 || p[0] != 1 {
        return None;
    }
    let word = |i: usize| u16::from_le_bytes([p[i], p[i + 1]]);
    Some(ConnectionComplete {
        status: p[1],
        handle: word(2),
        role: p[4],
        peer_type: p[5],
        peer_tail: [p[7], p[6]],
        interval: word(12),
        latency: word(14),
        timeout: word(16),
    })
}

/// Bluetooth failure reason for an SMP Pairing Failed code (Core Vol 3 Part H
/// 3.5.5). The code is public; the reason is shown on the viewer status line.
pub fn pairing_failed(code: u8) -> &'static str {
    match code {
        0x01 => "BT PAIRING FAILED PASSKEY ENTRY",
        0x02 => "BT PAIRING FAILED OOB NOT AVAILABLE",
        0x03 => "BT PAIRING FAILED AUTHENTICATION REQUIREMENTS",
        0x04 => "BT PAIRING FAILED CONFIRM VALUE",
        0x05 => "BT PAIRING FAILED NOT SUPPORTED",
        0x06 => "BT PAIRING FAILED ENCRYPTION KEY SIZE",
        0x07 => "BT PAIRING FAILED COMMAND NOT SUPPORTED",
        0x08 => "BT PAIRING FAILED UNSPECIFIED",
        0x09 => "BT PAIRING FAILED REPEATED ATTEMPTS",
        0x0a => "BT PAIRING FAILED INVALID PARAMETERS",
        0x0b => "BT PAIRING FAILED DHKEY CHECK",
        0x0c => "BT PAIRING FAILED NUMERIC COMPARISON",
        0x0d => "BT PAIRING FAILED BR/EDR IN PROGRESS",
        0x0e => "BT PAIRING FAILED CROSS-TRANSPORT KEY",
        0x0f => "BT PAIRING FAILED KEY REJECTED",
        _ => "BT PAIRING FAILED OTHER",
    }
}
/// Viewer guidance when a controller does not answer a Pairing Request: it may
/// still hold a bond this host failed to save. ASCII only (viewer font).
pub const PAIRING_NO_REPLY: &str =
    "No pairing reply: hold the controller pair button until it blinks fast.";
pub const PAIRING_TIMED_OUT: &str =
    "Pairing timed out: hold the controller pair button until it blinks fast.";
