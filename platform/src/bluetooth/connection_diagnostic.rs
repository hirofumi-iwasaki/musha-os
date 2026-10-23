// Copyright 2026 Hirofumi Iwasaki
// SPDX-License-Identifier: Apache-2.0
//! Keyless observations only: does not govern connection timeouts or retries.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Fresh,
    Stored,
}
impl Mode {
    pub fn from_pairing(pairing: bool) -> Self {
        if pairing { Self::Fresh } else { Self::Stored }
    }
    pub fn name(self) -> &'static str {
        match self {
            Self::Fresh => "fresh",
            Self::Stored => "stored",
        }
    }
    pub fn stored_key(self) -> bool {
        self == Self::Stored
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WaitSnapshot {
    pub mode: Mode,
    pub handle: u16,
    pub elapsed: u64,
    /// Host-local sample time; not the boot-log timestamp epoch.
    pub sampled_at: u64,
    pub deadline: u64,
    pub accepted: bool,
    pub completions: [u32; 3],
    pub received_events: u32,
    pub last_event: Option<(u8, u8, u64)>,
}
pub struct Wait {
    state: Option<(Mode, u16, u64, u64, [u32; 3])>,
    accepted: bool,
    reported: bool,
    sampled_at: u64,
    received_events: u32,
    last_event: Option<(u8, u8, u64)>,
}
impl Wait {
    pub const fn new() -> Self {
        Self {
            state: None,
            accepted: false,
            reported: false,
            sampled_at: 0,
            received_events: 0,
            last_event: None,
        }
    }
    pub fn begin(&mut self, mode: Mode, handle: u16, now: u64, deadline: u64, baseline: [u32; 3]) {
        self.state = Some((mode, handle, now, deadline, baseline));
        self.sampled_at = now;
        self.accepted = false;
        self.reported = false;
        self.received_events = 0;
        self.last_event = None;
    }
    /// Record only the public HCI event header while encryption is outstanding.
    /// The event payload can contain private controller or peer data.
    pub fn received(&mut self, now: u64, event: super::Event<'_>) {
        if self.state.is_some() {
            self.received_events = self.received_events.saturating_add(1);
            self.last_event = Some((event.code, event.parameters.len() as u8, now));
        }
    }
    pub fn accepted(&mut self) {
        if self.state.is_some() {
            self.accepted = true;
        }
    }
    fn snapshot(&self, now: u64, counts: [u32; 3]) -> Option<WaitSnapshot> {
        let (mode, handle, start, deadline, base) = self.state?;
        Some(WaitSnapshot {
            mode,
            handle,
            elapsed: now.saturating_sub(start),
            sampled_at: now,
            deadline,
            accepted: self.accepted,
            completions: core::array::from_fn(|i| counts[i].saturating_sub(base[i])),
            received_events: self.received_events,
            last_event: self.last_event,
        })
    }
    pub fn due(&mut self, now: u64, counts: [u32; 3]) -> Option<WaitSnapshot> {
        self.sampled_at = now;
        let s = self.snapshot(now, counts)?;
        if self.reported || s.elapsed < 5000 {
            return None;
        }
        self.reported = true;
        Some(s)
    }
    pub fn finish(&mut self, now: u64, counts: [u32; 3]) -> Option<WaitSnapshot> {
        self.sampled_at = now;
        self.finish_observed(counts)
    }
    /// Error path uses the last host-local observation, never an unrelated epoch.
    /// It can precede a blocking transfer failure; sampled_at makes that explicit.
    pub fn finish_observed(&mut self, counts: [u32; 3]) -> Option<WaitSnapshot> {
        let s = self.snapshot(self.sampled_at, counts);
        *self = Self::new();
        s
    }
    pub fn clear(&mut self) {
        *self = Self::new();
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EventMetadata {
    pub status: u8,
    pub handle: u16,
    pub detail: u8,
}
pub fn event_metadata(event: super::Event<'_>) -> Option<EventMetadata> {
    let p = event.parameters;
    if !matches!(event.code, 5 | 8) || p.len() != 4 {
        return None;
    }
    Some(EventMetadata {
        status: p[0],
        handle: u16::from_le_bytes([p[1], p[2]]),
        detail: p[3],
    })
}
