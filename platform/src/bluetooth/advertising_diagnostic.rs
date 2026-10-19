// Copyright 2026 Hirofumi Iwasaki
// SPDX-License-Identifier: Apache-2.0
//! Current per-report pairing classification, without scan-response merging.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Candidate {
    Keyboard,
    Gamepad,
}
#[derive(Clone, Copy, Debug)]
pub struct Classification {
    pub hid: bool,
    pub appearance: Option<u16>,
    pub ad_valid: bool,
    pub candidate: Option<Candidate>,
    pub reason: &'static str,
    pub flags: Flags,
}
pub fn classify(event_type: u8, data: &[u8]) -> Classification {
    let mut flags = Flags::Missing;
    let mut hid = false;
    let mut appearance = None;
    let mut odd_uuid = false;
    let mut bad_appearance = false;
    let ad_valid = super::ad_fields(data, |field| {
        if field.kind == 1 {
            flags = match (flags, field.value) {
                (Flags::Missing, [value]) => Flags::Value(*value),
                _ => Flags::Invalid,
            };
        }
        if field.kind == 0x19 {
            if field.value.len() == 2 {
                appearance = Some(u16::from_le_bytes(field.value.try_into().unwrap()));
            } else {
                bad_appearance = true;
            }
        }
        if matches!(field.kind, 2 | 3) {
            if field.value.len() % 2 == 0 {
                // Preserve the existing last well-formed UUID-list field rule.
                hid = field.value.chunks_exact(2).any(|b| b == [0x12, 0x18]);
            } else {
                odd_uuid = true;
            }
        }
    })
    .is_ok();
    let candidate = if matches!(event_type, 0 | 1) && ad_valid && hid {
        match appearance {
            Some(0x3c1) => Some(Candidate::Keyboard),
            Some(0x3c4) => Some(Candidate::Gamepad),
            _ => None,
        }
    } else {
        None
    };
    let reason = if event_type == 4 {
        "scan-response"
    } else if !matches!(event_type, 0 | 1) {
        "nonconnectable"
    } else if !ad_valid {
        "malformed-ad"
    } else if !hid {
        if odd_uuid {
            "uuid-width"
        } else {
            "no-hid-uuid"
        }
    } else if candidate.is_some() {
        "candidate"
    } else if appearance.is_some() {
        "unsupported-appearance"
    } else if bad_appearance {
        "appearance-width"
    } else {
        "missing-appearance"
    };
    Classification {
        hid,
        appearance,
        ad_valid,
        candidate,
        reason,
        flags: if ad_valid { flags } else { Flags::Invalid },
    }
}
/// Per Device/pairing scan session: first24 reports plus16 further HID-shaped
/// reports. Never reset on automatic disconnect/rescan. No payload retention.
pub struct Budget {
    general: u8,
    relevant: u8,
    suppressed: u32,
}
impl Budget {
    pub const fn new() -> Self {
        Self {
            general: 0,
            relevant: 0,
            suppressed: 0,
        }
    }
    pub fn admits(&mut self, c: Classification) -> bool {
        if self.general < 24 {
            self.general += 1;
            true
        } else if (c.hid || c.appearance.is_some()) && self.relevant < 16 {
            self.relevant += 1;
            true
        } else {
            self.suppressed = self.suppressed.saturating_add(1);
            false
        }
    }
    pub fn suppressed(&self) -> u32 {
        self.suppressed
    }
}

/// Flags are reference metadata, never a pairing-mode authorization predicate.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Flags {
    Missing,
    Invalid,
    Value(u8),
}
impl Flags {
    pub fn limited(self) -> Option<bool> {
        match self {
            Self::Value(v) => Some(v & 1 != 0),
            _ => None,
        }
    }
    pub fn general(self) -> Option<bool> {
        match self {
            Self::Value(v) => Some(v & 2 != 0),
            _ => None,
        }
    }
}
impl core::fmt::Display for Flags {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Missing => f.write_str("none"),
            Self::Invalid => f.write_str("invalid"),
            Self::Value(v) => write!(f, "{:02x}", v),
        }
    }
}
