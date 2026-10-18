// Copyright 2026 Hirofumi Iwasaki
// SPDX-License-Identifier: Apache-2.0
//! Encrypted ATT notification to the existing descriptor-driven keyboard decoder.
//! This supports only that decoder's documented subset, not arbitrary HOGP maps.
use super::{Error, att::InputReport, legacy::VerifiedBond};
pub struct Keyboard {
    binding: InputReport,
    state: musha_input::hid::Decoder,
    id: u8,
    active: bool,
}
impl Keyboard {
    pub fn bind(
        bond: &VerifiedBond,
        value: u16,
        reference: &[u8],
        map: &[u8],
        properties: u8,
    ) -> Result<Self, Error> {
        if reference.len() != 2 {
            return Err(Error::Length);
        }
        let layout = musha_input::hid::Layout::parse(map).map_err(|_| Error::Unsupported)?;
        let id = reference[0];
        let bytes = layout.report_bytes(id).ok_or(Error::Unsupported)?;
        let bytes = bytes
            .checked_sub(usize::from(id != 0))
            .ok_or(Error::Invalid)?;
        let binding = InputReport::bind(bond, value, reference, bytes, properties)?;
        Ok(Self {
            binding,
            state: musha_input::hid::Decoder::new(layout),
            id,
            active: true,
        })
    }
    pub fn notification(
        &mut self,
        generation: u64,
        handle: u16,
        p: &[u8],
        emit: impl FnMut(u8, bool),
    ) -> Result<(), Error> {
        if !self.active {
            return Err(Error::Stale);
        }
        let (_, data) = self.binding.notification(generation, handle, p)?;
        let mut report = [0; 513];
        let offset = usize::from(self.id != 0);
        report[0] = self.id;
        report[offset..offset + data.len()].copy_from_slice(data);
        self.state
            .update(&report[..offset + data.len()], emit)
            .map_err(|_| Error::Invalid)
    }
    /// Caller must detach this binding before emitting source releases. Old
    /// generation notifications must never be routed to the current binding.
    pub fn disconnect(&mut self, emit: impl FnMut(u8, bool)) {
        self.active = false;
        self.state.release_all(emit);
    }
}

pub const MAX_REPORTS: usize = 8;
#[derive(Clone, Copy)]
pub struct ReportSpec {
    pub value: u16,
    pub end: u16,
    pub properties: u8,
    pub reference: u16,
    pub cccd: u16,
    pub id: u8,
    pub kind: u8,
}
const SPEC: ReportSpec = ReportSpec {
    value: 0,
    end: 0,
    properties: 0,
    reference: 0,
    cccd: 0,
    id: 0,
    kind: 0,
};
struct Binding {
    wire: InputReport,
    value: u16,
    cccd: u16,
    id: u8,
    gamepad: bool,
}
/// One decoder per physical device: report IDs retain independent keyboard state.
pub struct Reports {
    bindings: [Option<Binding>; MAX_REPORTS],
    count: usize,
    // Input reports neither decoder maps (e.g. battery): not subscribed, and a
    // notification on them is accepted without events.
    unmapped: [u16; MAX_REPORTS],
    unmapped_count: usize,
    keyboard: Option<musha_input::hid::Decoder>,
    gamepad: Option<musha_input::gamepad::Decoder>,
    active: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Change {
    Key(u8, bool),
    Gamepad(musha_input::gamepad::Change),
}
impl Reports {
    pub fn bind(bond: &VerifiedBond, map: &[u8], specs: &[ReportSpec]) -> Result<Self, Error> {
        if specs.is_empty() || specs.len() > MAX_REPORTS {
            return Err(Error::Length);
        }
        let keyboard = match musha_input::hid::Layout::parse(map) {
            Ok(l) => Some(l),
            Err(musha_input::hid::Error::NoKeyboard) => None,
            Err(_) => return Err(Error::Unsupported),
        };
        let gamepad = match musha_input::gamepad::Layout::parse(map) {
            Ok(l) => Some(l),
            Err(musha_input::gamepad::Error::NoGamepad) => None,
            Err(_) => return Err(Error::Unsupported),
        };
        let mut bindings = core::array::from_fn(|_| None);
        let mut count = 0;
        let mut unmapped = [0; MAX_REPORTS];
        let mut unmapped_count = 0;
        for s in specs.iter().filter(|s| s.kind == 1) {
            if s.cccd == 0 || s.reference == 0 || s.value >= s.cccd || s.cccd > s.end {
                return Err(Error::Invalid);
            }
            let kb = keyboard
                .as_ref()
                .filter(|l| l.has_keys(s.id))
                .and_then(|l| l.report_bytes(s.id));
            let gp = gamepad.as_ref().and_then(|l| l.report_bytes(s.id));
            // Mixed keyboard/gamepad bits inside one ID are outside this subset.
            let (bytes, pad) = match (kb, gp) {
                (Some(n), None) => (n, false),
                (None, Some(n)) => (n, true),
                (None, None) => {
                    unmapped[unmapped_count] = s.value;
                    unmapped_count += 1;
                    continue;
                }
                _ => return Err(Error::Unsupported),
            };
            if specs
                .iter()
                .filter(|other| {
                    other.kind == 1
                        && (other.value == s.value || other.id == s.id || other.cccd == s.cccd)
                })
                .count()
                != 1
            {
                return Err(Error::Invalid);
            }
            let bytes = bytes
                .checked_sub(usize::from(s.id != 0))
                .ok_or(Error::Length)?;
            bindings[count] = Some(Binding {
                wire: InputReport::bind(bond, s.value, &[s.id, 1], bytes, s.properties)?,
                value: s.value,
                cccd: s.cccd,
                id: s.id,
                gamepad: pad,
            });
            count += 1;
        }
        if count == 0 {
            return Err(Error::Unsupported);
        }
        Ok(Self {
            bindings,
            count,
            unmapped,
            unmapped_count,
            keyboard: keyboard.map(musha_input::hid::Decoder::new),
            gamepad: gamepad.map(musha_input::gamepad::Decoder::new),
            active: true,
        })
    }
    pub fn count(&self) -> usize {
        self.count
    }
    /// Input reports discovered but not mapped by either decoder.
    pub fn unmapped(&self) -> usize {
        self.unmapped_count
    }
    pub fn cccd(&self, index: usize) -> Option<u16> {
        self.bindings.get(index)?.as_ref().map(|b| b.cccd)
    }
    /// Determine the reference device kind only from the authenticated GATT map.
    /// Mixed/empty classes remain outside the normal reference-device path.
    pub fn single_kind(&self) -> Result<bool, Error> {
        match (self.supports(false), self.supports(true)) {
            (true, false) => Ok(false),
            (false, true) => Ok(true),
            _ => Err(Error::Unsupported),
        }
    }
    pub fn supports(&self, pad: bool) -> bool {
        self.bindings[..self.count]
            .iter()
            .flatten()
            .any(|b| b.gamepad == pad)
    }
    pub fn notification(
        &mut self,
        generation: u64,
        handle: u16,
        p: &[u8],
        mut emit: impl FnMut(Change),
    ) -> Result<(), Error> {
        if !self.active {
            return Err(Error::Stale);
        }
        if p.len() < 3 {
            return Err(Error::Length);
        }
        let value = u16::from_le_bytes([p[1], p[2]]);
        if p[0] == 0x1b && self.unmapped[..self.unmapped_count].contains(&value) {
            return Ok(());
        }
        let b = self.bindings[..self.count]
            .iter()
            .flatten()
            .find(|b| b.value == value)
            .ok_or(Error::Invalid)?;
        let (_, data) = b.wire.notification(generation, handle, p)?;
        let mut report = [0; 513];
        let offset = usize::from(b.id != 0);
        report[0] = b.id;
        report[offset..offset + data.len()].copy_from_slice(data);
        if b.gamepad {
            self.gamepad
                .as_mut()
                .ok_or(Error::Invalid)?
                .update(&report[..offset + data.len()], |c| emit(Change::Gamepad(c)))
                .map_err(|_| Error::Invalid)
        } else {
            self.keyboard
                .as_mut()
                .ok_or(Error::Invalid)?
                .update(&report[..offset + data.len()], |k, d| {
                    emit(Change::Key(k, d))
                })
                .map_err(|_| Error::Invalid)
        }
    }
    pub fn disconnect(&mut self, mut emit: impl FnMut(Change)) {
        self.active = false;
        if let Some(k) = self.keyboard.as_mut() {
            k.release_all(|k, d| emit(Change::Key(k, d)));
        }
        if let Some(g) = self.gamepad.as_mut() {
            g.disconnect(|c| emit(Change::Gamepad(c)));
        }
    }
}
/// Bounded service/characteristic/each-report descriptor/reference discovery.
/// ATT default MTU23; up to2048 Report Map bytes and224 responses, no allocation.
pub struct Discovery {
    stage: u8,
    end: u16,
    cursor: u16,
    map_handle: u16,
    reports: [ReportSpec; MAX_REPORTS],
    count: usize,
    index: usize,
    map: [u8; 2048],
    used: usize,
    pages: u16,
    skipped: u16,
}
/// Notifications/indications or other non-responses skipped during discovery.
pub const MAX_SKIPPED: u16 = 256;
pub enum DiscoveryAction {
    Send([u8; 7], usize),
    Ready(Reports),
    /// A non-response PDU was skipped; nothing is sent and no state changes.
    Ignore,
    /// An indication was skipped; send Handle Value Confirmation (0x1e).
    Confirm,
}
/// Public ATT metadata only: never contains Report Map, report or security payload bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Diagnostic {
    pub stage: &'static str,
    pub request: u8,
    pub response: Option<u8>,
    pub length: usize,
    pub cursor: u16,
    pub request_handle: u16,
    pub map_bytes: usize,
    pub pages: u16,
    pub att_error: Option<(u8, u16, u8)>,
}
impl core::fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "HOGP stage={} request={:02x} handle={:04x} response=",
            self.stage, self.request, self.request_handle
        )?;
        match self.response {
            Some(op) => write!(f, "{:02x}", op)?,
            None => f.write_str("n/a")?,
        }
        write!(
            f,
            " len={} map-bytes={} pages={}",
            self.length, self.map_bytes, self.pages
        )?;
        if let Some((op, handle, code)) = self.att_error {
            write!(
                f,
                " ATT-error request={:02x} handle={:04x} code={:02x}",
                op, handle, code
            )?;
        }
        Ok(())
    }
}
impl Diagnostic {
    pub fn reason(self, error: Error) -> &'static str {
        if self.pages >= 224 {
            return "response-limit";
        }
        if self.att_error.is_some() {
            return "ATT-error-or-required-attribute-missing";
        }
        match (self.stage, error) {
            ("report-map", Error::Length) => "map-length-or-response-format",
            ("services", Error::Unsupported) => "HID-service-missing-or-service-format",
            ("characteristics", Error::Unsupported) => "report-map-or-notify-report-missing",
            ("descriptors", Error::Unsupported) => "reference-CCCD-missing-or-descriptor-format",
            ("reference", _) => "reference-format-or-report-map-binding",
            (_, Error::Length) => "bounded-length-or-count",
            (_, Error::Invalid) => "invalid-format-handle-order-or-duplicate",
            (_, Error::Stale) => "bond-generation-mismatch",
            _ => "discovery-rejected",
        }
    }
}
impl Discovery {
    pub fn new() -> Self {
        Self {
            stage: 0,
            end: 0,
            cursor: 1,
            map_handle: 0,
            reports: [SPEC; MAX_REPORTS],
            count: 0,
            index: 0,
            map: [0; 2048],
            used: 0,
            pages: 0,
            skipped: 0,
        }
    }
    pub fn skipped(&self) -> u16 {
        self.skipped
    }
    /// The response opcode the outstanding request expects (Error Response aside).
    fn expected(&self) -> u8 {
        match self.stage {
            0 => 0x11,
            1 => 0x09,
            2 => 0x05,
            3 if self.used != 0 => 0x0d,
            3 | 4 => 0x0b,
            _ => 0,
        }
    }
    /// Stage name for public diagnostics.
    pub fn stage_name(&self) -> &'static str {
        match self.stage {
            0 => "services",
            1 => "characteristics",
            2 => "descriptors",
            3 => "report-map",
            4 => "reference",
            5 => "complete",
            _ => "failed",
        }
    }
    pub fn diagnostic(&self, p: &[u8]) -> Diagnostic {
        let (stage, request) = match self.stage {
            0 => ("services", 0x10),
            1 => ("characteristics", 0x08),
            2 => ("descriptors", 0x04),
            3 => ("report-map", if self.used == 0 { 0x0a } else { 0x0c }),
            4 => ("reference", 0x0a),
            5 => ("complete", 0),
            _ => ("failed", 0),
        };
        Diagnostic {
            stage,
            request,
            response: p.first().copied(),
            length: p.len(),
            request_handle: match self.stage {
                3 => self.map_handle,
                4 if self.index < self.count => self.reports[self.index].reference,
                _ => self.cursor,
            },
            cursor: self.cursor,
            map_bytes: self.used,
            pages: self.pages,
            att_error: if p.len() == 5 && p[0] == 1 {
                Some((p[1], u16::from_le_bytes([p[2], p[3]]), p[4]))
            } else {
                None
            },
        }
    }
    pub fn request(&self) -> DiscoveryAction {
        Self::range(0x10, 1, u16::MAX, Some(0x2800))
    }
    fn packet(b: &[u8]) -> DiscoveryAction {
        let mut p = [0; 7];
        p[..b.len()].copy_from_slice(b);
        DiscoveryAction::Send(p, b.len())
    }
    fn range(code: u8, start: u16, end: u16, uuid: Option<u16>) -> DiscoveryAction {
        let mut p = [0; 7];
        p[0] = code;
        p[1..3].copy_from_slice(&start.to_le_bytes());
        p[3..5].copy_from_slice(&end.to_le_bytes());
        if let Some(uuid) = uuid {
            p[5..].copy_from_slice(&uuid.to_le_bytes());
        }
        DiscoveryAction::Send(p, if uuid.is_some() { 7 } else { 5 })
    }
    fn read(h: u16) -> DiscoveryAction {
        let mut p = [0x0a, 0, 0];
        p[1..].copy_from_slice(&h.to_le_bytes());
        Self::packet(&p)
    }
    fn descriptor(&mut self) -> Result<DiscoveryAction, Error> {
        let s = self.reports[self.index];
        self.cursor = s.value.checked_add(1).ok_or(Error::Invalid)?;
        if self.cursor > s.end {
            return Err(Error::Invalid);
        }
        Ok(Self::range(4, self.cursor, s.end, None))
    }
    pub fn receive(&mut self, p: &[u8], bond: &VerifiedBond) -> Result<DiscoveryAction, Error> {
        if self.stage == 255 {
            return Err(Error::Invalid);
        }
        let result = self.receive_inner(p, bond);
        if result.is_err() {
            self.stage = 255;
        }
        result
    }
    fn receive_inner(&mut self, p: &[u8], bond: &VerifiedBond) -> Result<DiscoveryAction, Error> {
        // Peripherals may notify before their CCCD is written (game controllers
        // commonly do). Skip anything that is neither the awaited response nor an
        // Error Response, without counting it as a discovery page.
        if let Some(&opcode) = p.first()
            && opcode != 0x01
            && opcode != self.expected()
        {
            self.skipped = self.skipped.checked_add(1).ok_or(Error::Length)?;
            if self.skipped > MAX_SKIPPED {
                return Err(Error::Length);
            }
            return Ok(if opcode == 0x1d {
                DiscoveryAction::Confirm
            } else {
                DiscoveryAction::Ignore
            });
        }
        self.pages = self.pages.checked_add(1).ok_or(Error::Length)?;
        if self.pages > 224 {
            return Err(Error::Length);
        }
        let opcode = match self.stage {
            0 => 0x10,
            1 => 8,
            2 => 4,
            _ => 0,
        };
        let ended = p.len() == 5
            && p[0] == 1
            && p[1] == opcode
            && p[4] == 0x0a
            && u16::from_le_bytes([p[2], p[3]]) == self.cursor;
        match self.stage {
            0 => {
                if ended {
                    return Err(Error::Unsupported);
                }
                let mut found = None;
                let mut last = 0;
                super::att::services(p, |s| {
                    last = s.end;
                    if s.uuid16 == Some(0x1812) {
                        found = Some(s);
                    }
                })?;
                if let Some(s) = found {
                    self.end = s.end;
                    self.cursor = s.start;
                    self.stage = 1;
                    return Ok(Self::range(8, s.start, s.end, Some(0x2803)));
                }
                if last < self.cursor || last == u16::MAX {
                    return Err(Error::Unsupported);
                }
                self.cursor = last + 1;
                Ok(Self::range(0x10, self.cursor, u16::MAX, Some(0x2800)))
            }
            1 => {
                if !ended {
                    let mut error = None;
                    let mut last = 0;
                    super::att::characteristics(p, self.cursor, self.end, |c| {
                        last = c.value;
                        for s in &mut self.reports[..self.count] {
                            if s.end == 0 {
                                s.end = c.declaration - 1;
                            }
                        }
                        if c.uuid16 == Some(0x2a4b) {
                            if self.map_handle != 0 {
                                error = Some(Error::Invalid);
                            }
                            self.map_handle = c.value;
                        }
                        if c.uuid16 == Some(0x2a4d) && c.properties & 0x10 != 0 {
                            if self.count == MAX_REPORTS {
                                error = Some(Error::Length);
                            } else {
                                self.reports[self.count] = ReportSpec {
                                    value: c.value,
                                    properties: c.properties,
                                    ..SPEC
                                };
                                self.count += 1;
                            }
                        }
                    })?;
                    if let Some(e) = error {
                        return Err(e);
                    }
                    if last < self.cursor {
                        return Err(Error::Invalid);
                    }
                    if last < self.end {
                        self.cursor = last + 1;
                        return Ok(Self::range(8, self.cursor, self.end, Some(0x2803)));
                    }
                }
                if self.map_handle == 0 || self.count == 0 {
                    return Err(Error::Unsupported);
                }
                for s in &mut self.reports[..self.count] {
                    if s.end == 0 {
                        s.end = self.end;
                    }
                }
                self.stage = 3;
                Ok(Self::read(self.map_handle))
            }
            3 => {
                let eof = p.len() == 5
                    && p[0] == 1
                    && p[1] == 0x0c
                    && u16::from_le_bytes([p[2], p[3]]) == self.map_handle
                    && matches!(p[4], 7 | 0x0b)
                    && self.used > 0;
                if !eof {
                    if p.is_empty()
                        || p[0] != if self.used == 0 { 0x0b } else { 0x0d }
                        || p.len() > 23
                    {
                        return Err(Error::Length);
                    }
                    let data = &p[1..];
                    if self.used + data.len() > self.map.len() {
                        return Err(Error::Length);
                    }
                    self.map[self.used..self.used + data.len()].copy_from_slice(data);
                    self.used += data.len();
                    if data.len() == 22 {
                        let mut q = [0x0c, 0, 0, 0, 0];
                        q[1..3].copy_from_slice(&self.map_handle.to_le_bytes());
                        q[3..].copy_from_slice(&(self.used as u16).to_le_bytes());
                        return Ok(Self::packet(&q));
                    }
                }
                if self.used == 0 {
                    return Err(Error::Unsupported);
                }
                self.stage = 2;
                self.index = 0;
                self.descriptor()
            }
            2 => {
                let end = self.reports[self.index].end;
                if !ended {
                    if p.len() < 6 || p[0] != 5 || p[1] != 1 || (p.len() - 2) % 4 != 0 {
                        return Err(Error::Unsupported);
                    }
                    let mut last = 0;
                    for b in p[2..].chunks_exact(4) {
                        let h = u16::from_le_bytes([b[0], b[1]]);
                        let uuid = u16::from_le_bytes([b[2], b[3]]);
                        if h < self.cursor || h > end || h <= last {
                            return Err(Error::Invalid);
                        }
                        last = h;
                        let s = &mut self.reports[self.index];
                        match uuid {
                            0x2803 => return Err(Error::Invalid),
                            0x2908 => {
                                if s.reference != 0 {
                                    return Err(Error::Invalid);
                                }
                                s.reference = h;
                            }
                            0x2902 => {
                                if s.cccd != 0 {
                                    return Err(Error::Invalid);
                                }
                                s.cccd = h;
                            }
                            _ => {}
                        }
                    }
                    if last < end {
                        self.cursor = last + 1;
                        return Ok(Self::range(4, self.cursor, end, None));
                    }
                }
                let s = self.reports[self.index];
                if s.reference == 0 || s.cccd == 0 {
                    return Err(Error::Unsupported);
                }
                self.stage = 4;
                Ok(Self::read(s.reference))
            }
            4 => {
                if p.len() != 3 || p[0] != 0x0b || !(1..=3).contains(&p[2]) {
                    return Err(Error::Invalid);
                }
                self.reports[self.index].id = p[1];
                self.reports[self.index].kind = p[2];
                self.index += 1;
                if self.index < self.count {
                    self.stage = 2;
                    self.descriptor()
                } else {
                    let reports =
                        Reports::bind(bond, &self.map[..self.used], &self.reports[..self.count])?;
                    self.stage = 5;
                    Ok(DiscoveryAction::Ready(reports))
                }
            }
            _ => Err(Error::Invalid),
        }
    }
}

#[cfg(test)]
mod diagnostic_tests {
    use super::*;
    #[test]
    fn stages_limits_and_error_header_are_bounded_public_metadata() {
        let mut d = Discovery::new();
        let stages = [
            (0, "services", 0x10),
            (1, "characteristics", 8),
            (2, "descriptors", 4),
            (3, "report-map", 0x0a),
            (4, "reference", 0x0a),
            (5, "complete", 0),
            (255, "failed", 0),
        ];
        for (stage, name, op) in stages {
            d.stage = stage;
            let m = d.diagnostic(&[1, 8, 0x34, 0x12, 5]);
            assert_eq!((m.stage, m.request), (name, op));
            assert_eq!(m.att_error, Some((8, 0x1234, 5)));
            assert!(m.reason(Error::Invalid).contains("ATT-error"));
        }
        d.stage = 3;
        d.used = 2048;
        let m = d.diagnostic(&[0x0d]);
        assert_eq!(m.request, 0x0c);
        assert_eq!(m.reason(Error::Length), "map-length-or-response-format");
        d.pages = 224;
        assert_eq!(d.diagnostic(&[]).reason(Error::Length), "response-limit");
        for p in [&[][..], &[1, 8][..], &[1, 8, 0, 0, 5, 0][..]] {
            assert_eq!(d.diagnostic(p).att_error, None);
        }
    }
}
