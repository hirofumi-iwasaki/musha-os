// Copyright 2026 Hirofumi Iwasaki
// SPDX-License-Identifier: Apache-2.0
//! Decoded gamepad state only; no report payload or security material.
use musha_input::gamepad::Change;
#[derive(Clone, Copy)]
pub struct Snapshot {
    pub buttons: u32,
    pub pressed: u32,
    pub released: u32,
    pub axes: [i32; 12],
    pub seen: u16,
    pub reports: u32,
    pub changes: u32,
    pub dropped: u32,
    pub id: u8,
    pub bytes: u16,
}
impl Snapshot {
    pub fn line(&self, row: usize, out: &mut impl core::fmt::Write) -> core::fmt::Result {
        match row {
            0 => write!(
                out,
                "PAD id={} len={} reports={} changes={} queue-full={} buttons={:08x} pressed={:08x} released={:08x}",
                self.id,
                self.bytes,
                self.reports,
                self.changes,
                self.dropped,
                self.buttons,
                self.pressed,
                self.released
            ),
            1 => write!(
                out,
                "PAD axes-seen={:03x} X={} Y={} Z={} Rx={} Ry={} Rz={}",
                self.seen,
                self.axes[0],
                self.axes[1],
                self.axes[2],
                self.axes[3],
                self.axes[4],
                self.axes[5]
            ),
            _ => write!(
                out,
                "PAD hat={} Brake={} Accelerator={} Slider={} Dial={} Wheel={}",
                self.axes[9],
                self.axes[10],
                self.axes[11],
                self.axes[6],
                self.axes[7],
                self.axes[8]
            ),
        }
    }
}
pub struct Diagnostic {
    state: Snapshot,
    dirty: bool,
    last: Option<u64>,
}
impl Diagnostic {
    pub const fn new() -> Self {
        Self {
            state: Snapshot {
                buttons: 0,
                pressed: 0,
                released: 0,
                axes: [0, 0, 0, 0, 0, 0, 0, 0, 0, -1, 0, 0],
                seen: 0,
                reports: 0,
                changes: 0,
                dropped: 0,
                id: 0,
                bytes: 0,
            },
            dirty: false,
            last: None,
        }
    }
    pub fn accepted(&mut self, id: u8, bytes: u16) {
        self.dirty |= self.state.reports == 0;
        self.state.reports = self.state.reports.saturating_add(1);
        self.state.id = id;
        self.state.bytes = bytes;
    }
    /// Called at the actual event queue handoff, with its acceptance result.
    pub fn change(&mut self, change: Change, queued: bool) {
        match change {
            Change::Button(code, down) if code < 32 => {
                let bit = 1 << code;
                if down {
                    self.state.buttons |= bit;
                    self.state.pressed |= bit;
                } else {
                    self.state.buttons &= !bit;
                    self.state.released |= bit;
                }
            }
            Change::Axis(code, value) if code < 12 => {
                self.state.axes[code as usize] = value;
                self.state.seen |= 1 << code;
            }
            _ => return,
        }
        self.state.changes = self.state.changes.saturating_add(1);
        if !queued {
            self.state.dropped = self.state.dropped.saturating_add(1);
        }
        self.dirty = true;
    }
    /// Service timer flushes trailing releases without needing another packet.
    pub fn poll(&mut self, now: u64) -> Option<Snapshot> {
        if !self.dirty || self.last.is_some_and(|last| now.saturating_sub(last) < 250) {
            return None;
        }
        self.last = Some(now);
        self.dirty = false;
        let state = self.state;
        self.state.pressed = 0;
        self.state.released = 0;
        Some(state)
    }
}
