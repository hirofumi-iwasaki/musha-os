// Copyright 2026 Hirofumi Iwasaki
// SPDX-License-Identifier: Apache-2.0
#![no_std]
use core::fmt::{self, Write};
use musha_api::{
    Context,
    stream::{Cursor, Error, Files, Format, Handle, Id, MAX_FILES},
};
pub const PAGE: usize = 256;
pub const READ_TIMEOUT_MS: u64 = 10_000;
const ROWS: usize = 10;
// Diagnostic tail: header row, then the newest lines wrapped to two rows max.
const LOG_FIRST: usize = 19;
const LOG_LAST: usize = 26;
const LOG_LINES: usize = 6;
const LOG_WIDTH: usize = 78;
const BG: [u8; 3] = [12, 20, 32];
const FG: [u8; 3] = [218, 228, 238];
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum View {
    Unavailable,
    Catalog,
    List,
    Loading,
    Page,
    Error,
}
#[derive(Clone, Copy)]
struct Item {
    id: Option<Id>,
    elf: bool,
    bytes: u64,
    name: Text<48>,
}
impl Item {
    const EMPTY: Self = Self {
        id: None,
        elf: false,
        bytes: 0,
        name: Text::new(),
    };
}
#[derive(Clone, Copy)]
pub struct Text<const N: usize> {
    data: [u8; N],
    len: usize,
}
impl<const N: usize> Text<N> {
    const fn new() -> Self {
        Self {
            data: [0; N],
            len: 0,
        }
    }
    pub fn as_str(&self) -> &str {
        core::str::from_utf8(&self.data[..self.len]).unwrap_or("")
    }
}
impl<const N: usize> Write for Text<N> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for b in s.bytes() {
            if self.len == N {
                return Err(fmt::Error);
            }
            self.data[self.len] = b;
            self.len += 1;
        }
        Ok(())
    }
}
/// Escape UTF-16 units, including surrogate units, without interpreting controls.
/// Backslashes are escaped too, so literal escape-looking names stay distinct.
pub fn name_text(name: &[u16]) -> Text<1530> {
    let mut out = Text::new();
    for &u in name {
        if (32..=126).contains(&u) && u != 92 {
            let _ = out.write_char(char::from_u32(u as u32).unwrap());
        } else {
            let _ = write!(out, "\\u{u:04X}");
        }
    }
    out
}
fn executable_name(name: &[u16]) -> bool {
    name.len() >= 4
        && name[name.len() - 4..]
            .iter()
            .zip(b".ELF")
            .all(|(&a, &b)| a <= 127 && (a as u8).to_ascii_uppercase() == b)
}
/// Bounded, CPU-only viewer. Catalog identities belong to the borrowed Files;
/// application buffers never escape callbacks. No full-file allocation or seek I/O.
pub struct Viewer {
    items: [Item; MAX_FILES],
    count: usize,
    cursor: Option<Cursor>,
    listed: bool,
    selected: usize,
    name: Text<1530>,
    name_page: usize,
    name_dirty: bool,
    format: Option<Format>,
    view: View,
    error: Option<Error>,
    ended: bool,
    ended_reason: Option<&'static str>,
    keyboard_missing: bool,
    bootfile_absent: bool,
    pairing_blocked: bool,
    pairing_request: bool,
    pairing_message: Option<&'static str>,
    passkey: Option<musha_input::passkey::Entry>,
    passkey_outcome: Option<musha_input::passkey::Outcome>,
    passkey_generation: u64,
    display_passkey: Option<u32>,
    returned: Option<(u64, u64)>,
    launch: Option<Id>,
    handle: Option<Handle>,
    closing: bool,
    position: u64,
    offset: u64,
    data: [u8; PAGE],
    len: usize,
    started: u64,
    exit: bool,
    // Rendering clears at most 8192 pixels or draws one bounded line per tick.
    clear: usize,
    line: usize,
    dirty: bool,
    // Metadata-only Bluetooth log tail; repainted alone, within the same budget.
    log: [Text<{ 2 * LOG_WIDTH }>; LOG_LINES],
    log_count: usize,
    log_dirty: bool,
    log_clear: usize,
    log_row: usize,
}
impl Default for Viewer {
    fn default() -> Self {
        Self::new()
    }
}
impl Viewer {
    pub const fn new() -> Self {
        Self {
            items: [Item::EMPTY; MAX_FILES],
            count: 0,
            cursor: None,
            listed: false,
            selected: 0,
            name: Text::new(),
            name_page: 0,
            name_dirty: false,
            format: None,
            view: View::Unavailable,
            error: None,
            ended: false,
            ended_reason: None,
            keyboard_missing: false,
            bootfile_absent: false,
            pairing_blocked: false,
            pairing_request: false,
            pairing_message: None,
            passkey: None,
            passkey_outcome: None,
            passkey_generation: 0,
            display_passkey: None,
            returned: None,
            launch: None,
            handle: None,
            closing: false,
            position: 0,
            offset: 0,
            data: [0; PAGE],
            len: 0,
            started: 0,
            exit: false,
            clear: 0,
            line: 0,
            dirty: true,
            log: [const { Text::new() }; LOG_LINES],
            log_count: 0,
            log_dirty: false,
            log_clear: 0,
            log_row: LOG_FIRST,
        }
    }
    /// Append one diagnostic line (metadata only, ASCII). Longer text is cut at
    /// two display rows. Only the log rows are repainted, never the whole screen.
    pub fn push_log(&mut self, line: &str) {
        let mut text = Text::new();
        for c in line.chars() {
            let c = if c.is_ascii_graphic() || c == ' ' {
                c
            } else {
                '?'
            };
            if text.write_char(c).is_err() {
                break;
            }
        }
        self.log[self.log_count % LOG_LINES] = text;
        self.log_count = self.log_count.saturating_add(1);
        self.log_dirty = true;
        self.log_clear = 0;
        self.log_row = LOG_FIRST;
    }
    fn log_shown(&self) -> bool {
        self.log_count != 0 && self.view != View::Page
    }
    fn log_line(&self, row: usize, out: &mut Text<80>) {
        if row == LOG_FIRST {
            let _ = out.write_str("Bluetooth log (devices, newest last):");
            return;
        }
        // Fill rows upward from the bottom with the newest lines first.
        let mut below = LOG_LAST - row;
        for k in 0..self.log_count.min(LOG_LINES) {
            let text = self.log[(self.log_count - 1 - k) % LOG_LINES].as_str();
            let rows = text.len().div_ceil(LOG_WIDTH).max(1);
            if below < rows {
                let segment = rows - 1 - below;
                let start = segment * LOG_WIDTH;
                let end = (start + LOG_WIDTH).min(text.len());
                let _ = out.write_str(if segment == 0 { "" } else { "  " });
                let _ = out.write_str(&text[start..end]);
                return;
            }
            below -= rows;
        }
    }
    /// Set only after the boot-file reader reports State::Absent, never on I/O failure.
    pub fn bootfile_absent(&mut self) {
        if !self.bootfile_absent {
            self.bootfile_absent = true;
            self.repaint();
        }
    }
    /// Runtime has no approved entropy/crypto/UI/transport service yet. This is
    /// a visible refusal, never a successful pairing-mode transition.
    pub fn begin_passkey_input(&mut self, generation: u64, now: u64) -> bool {
        let Some(entry) = musha_input::passkey::Entry::new(generation, now) else {
            return false;
        };
        self.passkey = Some(entry);
        self.passkey_generation = generation;
        self.passkey_outcome = None;
        self.pairing_blocked = false;
        self.repaint();
        true
    }
    pub fn take_passkey(&mut self) -> Option<(u64, musha_input::passkey::Outcome)> {
        self.passkey_outcome
            .take()
            .map(|v| (self.passkey_generation, v))
    }
    pub fn display_passkey(&mut self, value: u32) {
        self.display_passkey = Some(value);
        self.pairing_blocked = true;
        self.repaint();
    }
    fn passkey_tick(&mut self, now: u64) {
        if let Some(entry) = self.passkey.as_mut() {
            if let Some(outcome) = entry.tick(self.passkey_generation, now) {
                self.passkey = None;
                self.passkey_outcome = Some(outcome);
                self.pairing_blocked = true;
                self.repaint();
            }
        }
    }
    pub fn take_pairing_request(&mut self) -> bool {
        core::mem::replace(&mut self.pairing_request, false)
    }
    pub fn pairing_status(&mut self, message: &'static str) {
        self.passkey = None;
        self.passkey_outcome = None;
        self.display_passkey = None;
        self.pairing_message = Some(message);
        self.pairing_blocked = true;
        self.repaint();
    }
    pub fn pairing_unavailable(&mut self) {
        if !self.pairing_blocked {
            self.pairing_blocked = true;
            self.repaint();
        }
    }
    pub fn returned(&mut self, kind: u64, detail: u64) {
        if self.returned.is_none() {
            self.returned = Some((kind, detail));
            self.repaint();
        }
    }
    /// CPU-only request; runtime revalidates this opaque ID in the same mount.
    pub fn launch_request(&self) -> Option<Id> {
        self.launch
    }
    pub fn launch_consumed(&mut self) {
        self.launch = None;
    }
    fn repaint(&mut self) {
        self.clear = 0;
        self.line = 0;
        self.dirty = true;
    }
    pub fn view(&self) -> View {
        self.view
    }
    pub fn selected(&self) -> usize {
        self.selected
    }
    pub fn count(&self) -> usize {
        self.count
    }
    pub fn offset(&self) -> u64 {
        self.offset
    }
    pub fn data(&self) -> &[u8] {
        &self.data[..self.len]
    }
    pub fn error(&self) -> Option<Error> {
        self.error
    }
    pub fn exit_requested(&self) -> bool {
        self.exit
    }
    pub fn painted(&self) -> bool {
        !self.dirty
    }
    pub fn selected_name(&self) -> &str {
        self.name.as_str()
    }
    fn load(&mut self, offset: u64, now: u64) {
        if offset < self.position {
            self.closing = true;
        }
        self.offset = offset;
        self.started = now;
        self.len = 0;
        self.data.fill(0);
        self.error = None;
        self.view = View::Loading;
        self.repaint();
    }
    /// HID usage, press transitions only. A held key does not create repeats here.
    pub fn key(&mut self, usage: u8, now: u64) {
        if let Some(entry) = self.passkey.as_mut() {
            let outcome = entry.key(self.passkey_generation, usage, true, now);
            if let Some(outcome) = outcome {
                self.passkey = None;
                self.passkey_outcome = Some(outcome);
            }
            self.repaint();
            return;
        }

        if usage == 0x13 {
            self.pairing_request = true;
            self.pairing_unavailable();
            return;
        } // P
        if usage == 0x14 {
            self.exit = true;
            self.closing = true;
            return;
        } // Q
        if matches!(usage, 0x29 | 0x2a) {
            // Escape / Backspace
            if matches!(self.view, View::Page | View::Loading | View::Error) && self.listed {
                self.closing = true;
                self.view = View::List;
                self.error = None;
                self.len = 0;
                self.data.fill(0);
                self.repaint();
            }
            return;
        }
        match self.view {
            View::List if self.count != 0 => {
                let old = self.selected;
                match usage {
                    0x52 => self.selected = self.selected.saturating_sub(1),
                    0x51 => self.selected = (self.selected + 1).min(self.count - 1),
                    0x4b => self.selected = self.selected.saturating_sub(ROWS),
                    0x4e => self.selected = (self.selected + ROWS).min(self.count - 1),
                    0x4a => self.selected = 0,
                    0x4d => self.selected = self.count - 1,
                    0x28 => {
                        if self.items[self.selected].elf {
                            self.launch = self.items[self.selected].id;
                            self.closing = true;
                            return;
                        }
                        self.load(0, now);
                    }
                    _ => {}
                }
                if self.selected != old {
                    self.name_dirty = true;
                    self.name_page = 0;
                    self.repaint();
                }
            }
            View::Page => match usage {
                0x4e | 0x51 if self.offset + (PAGE as u64) < self.items[self.selected].bytes => {
                    self.load(self.offset + PAGE as u64, now)
                }
                0x4b | 0x52 if self.offset > 0 => {
                    self.load(self.offset.saturating_sub(PAGE as u64), now)
                }
                0x4a => self.load(0, now),
                _ => {}
            },
            _ => {}
        }
        if matches!(self.view, View::List | View::Page | View::Error) {
            let old = self.name_page;
            if usage == 0x4f && (self.name_page + 1) * 72 < self.name.len {
                self.name_page += 1;
            }
            if usage == 0x50 {
                self.name_page = self.name_page.saturating_sub(1);
            }
            if old != self.name_page {
                self.repaint();
            }
        }
    }
    fn fail(&mut self, e: Error) {
        self.error = Some(e);
        self.closing = true;
        self.len = 0;
        self.data.fill(0);
        self.view = View::Error;
        self.repaint();
    }
    pub fn closed(&mut self, keyboard_missing: bool) {
        self.keyboard_missing = keyboard_missing;
        self.launch = None;
        self.handle = None;
        self.cursor = None;
        self.closing = false;
        self.items.fill(Item::EMPTY);
        self.count = 0;
        self.listed = false;
        self.len = 0;
        self.data.fill(0);
        self.name = Text::new();
        self.ended = true;
        self.view = View::Unavailable;
        self.repaint();
    }
    /// Shown with the ended-session line; the reason is a fixed diagnostic string.
    pub fn session_reason(&mut self, reason: &'static str) {
        self.ended_reason = Some(reason);
        self.repaint();
    }
    /// At most one catalog entry or one bounded read attempt per call, except the
    /// selected-name lookup (at most 128 in-memory entries, no disk access).
    pub fn service(&mut self, files: &mut Files<'_>, now: u64) {
        self.passkey_tick(now);
        if self.ended {
            return;
        }
        if self.closing {
            if let Some(h) = self.handle.take() {
                let _ = files.close(h);
            }
            self.closing = false;
            self.position = 0;
            return;
        }
        if self.exit {
            return;
        }
        if !self.listed {
            if self.cursor.is_none() {
                match (files.list_begin(), files.format()) {
                    (Ok(c), Ok(f)) => {
                        self.cursor = Some(c);
                        self.format = Some(f);
                        self.view = View::Catalog;
                        self.repaint();
                    }
                    (Err(e), _) | (_, Err(e)) => {
                        self.fail(e);
                        self.ended = true;
                    }
                }
                return;
            }
            match files.list_next(self.cursor.as_mut().unwrap()) {
                Ok(Some((id, entry))) if self.count < MAX_FILES => {
                    let full = name_text(&entry.name[..entry.name_len as usize]);
                    let mut name = Text::new();
                    let n = full.len.min(47);
                    let _ = name.write_str(&full.as_str()[..n]);
                    if n < full.len {
                        let _ = name.write_char('~');
                    }
                    self.items[self.count] = Item {
                        id: Some(id),
                        bytes: entry.bytes,
                        elf: executable_name(&entry.name[..entry.name_len as usize]),
                        name,
                    };
                    self.count += 1;
                }
                Ok(None) => {
                    self.listed = true;
                    self.view = View::List;
                    self.name_dirty = true;
                    self.repaint();
                }
                Ok(Some(_)) => {
                    self.fail(Error::Limit);
                    self.ended = true;
                }
                Err(e) => {
                    self.fail(e);
                    self.ended = true;
                }
            }
            return;
        }
        if self.name_dirty && self.count != 0 {
            let result = (|| {
                let mut c = files.list_begin()?;
                for i in 0..=self.selected {
                    let (id, entry) = files.list_next(&mut c)?.ok_or(Error::Invalid)?;
                    if i == self.selected {
                        if Some(id) != self.items[i].id {
                            return Err(Error::Invalid);
                        }
                        self.name = name_text(&entry.name[..entry.name_len as usize]);
                    }
                }
                Ok(())
            })();
            self.name_dirty = false;
            if let Err(e) = result {
                self.fail(e);
            }
            self.repaint();
            return;
        }
        if self.view != View::Loading {
            return;
        }
        if now.saturating_sub(self.started) >= READ_TIMEOUT_MS {
            self.fail(Error::Limit);
            return;
        }
        if self.handle.is_none() {
            match files.open_id(self.items[self.selected].id.unwrap()) {
                Ok(h) => {
                    self.handle = Some(h);
                    self.position = 0;
                }
                Err(Error::Again) => {}
                Err(e) => self.fail(e),
            }
            return;
        }
        let mut scratch = [0; PAGE];
        let skip = self.position < self.offset;
        let limit = if skip {
            (self.offset - self.position).min(PAGE as u64) as usize
        } else {
            PAGE
        };
        match files.read(self.handle.unwrap(), &mut scratch[..limit]) {
            Err(Error::Again) => {}
            Err(e) => self.fail(e),
            Ok(n) => {
                self.position += n as u64;
                if skip {
                    if n == 0 {
                        self.fail(Error::Corrupt);
                    }
                } else {
                    self.data[..n].copy_from_slice(&scratch[..n]);
                    self.len = n;
                    self.view = View::Page;
                    self.repaint();
                }
            }
        }
    }
    pub fn line(&self, row: usize) -> Text<80> {
        let mut out = Text::new();
        match row {
            0 => {
                let _ = write!(
                    out,
                    "Musha-OS {}  |  USB file viewer  |  READ ONLY",
                    include_str!("../../VERSION").trim()
                );
            }
            2 => {
                let _ = write!(
                    out,
                    "{}  |  {} files  |  maximum 64 MiB per file",
                    match self.format {
                        Some(Format::Fat32) => "FAT32",
                        Some(Format::Exfat) => "exFAT",
                        None => "No medium",
                    },
                    self.count
                );
            }
            3 => {
                let _ = write!(
                    out,
                    "Up/Down select   Enter open   PgUp/PgDn page   Esc back   Q quit"
                );
            }
            4 if self.count > 0 => {
                let _ = write!(
                    out,
                    "Selected #{:03}  |  {} bytes  |  Name {}/{} (Left/Right)",
                    self.selected + 1,
                    self.items[self.selected].bytes,
                    self.name_page + 1,
                    self.name.len.max(1).div_ceil(72)
                );
            }
            5 => {
                let start = (self.name_page * 72).min(self.name.len);
                let _ = out.write_str(&self.name.as_str()[start..(start + 72).min(self.name.len)]);
            }
            6 if self.display_passkey.is_some() => {
                let _ = write!(
                    out,
                    "Bluetooth passkey: {:06}. Type on the Bluetooth keyboard.",
                    self.display_passkey.unwrap()
                );
            }
            6 if self.passkey.is_some() => {
                let _ = write!(
                    out,
                    "Bluetooth passkey: {} digits entered. Enter confirms; Esc cancels.",
                    self.passkey.as_ref().unwrap().digits()
                );
            }
            6 if self.pairing_blocked => {
                let _ = out
                    .write_str(self.pairing_message.unwrap_or(
                        "Bluetooth pairing unavailable: runtime service not connected.",
                    ));
            }
            6 if self.returned.is_some() => {
                let _ = match self.returned.unwrap() {
                    (0, status) => write!(out, "ELF exited: status {}.", status as i32),
                    (1, vector) => write!(out, "ELF faulted: vector {}.", vector),
                    _ => out.write_str("ELF refused."),
                };
            }
            7 => {
                let _ = match self.view {
                    View::Unavailable => {
                        if self.keyboard_missing {
                            out.write_str("USB keyboard not detected. Connect one, then restart.")
                        } else if let (true, Some(reason)) = (self.ended, self.ended_reason) {
                            write!(
                                out,
                                "Device session ended: {}. Restart to try again.",
                                reason
                            )
                        } else if self.ended {
                            out.write_str("Device session ended. Restart to try again.")
                        } else {
                            out.write_str(
                                "No supported FAT32/exFAT medium. External USB keyboard required.",
                            )
                        }
                    }
                    View::Catalog | View::List if self.returned.is_some() => {
                        out.write_str("Select an ELF and press Enter.")
                    }
                    View::Catalog | View::List if self.bootfile_absent => {
                        out.write_str("AUTORUN.ELF not found in root. Starting file viewer.")
                    }
                    View::Catalog => out.write_str("Reading file list..."),
                    View::List if self.count == 0 => {
                        out.write_str("This root directory contains no supported files.")
                    }
                    View::List => write!(
                        out,
                        "Root files  |  list page {}/{}  |  ~ means shortened name",
                        self.selected / ROWS + 1,
                        self.count.div_ceil(ROWS)
                    ),
                    View::Loading => write!(
                        out,
                        "Reading page at byte {}... Esc cancels. Time limit: 10 seconds.",
                        self.offset
                    ),
                    View::Page if self.items[self.selected].bytes == 0 => {
                        out.write_str("Empty file (0 bytes). Esc returns to the list.")
                    }
                    View::Page => write!(
                        out,
                        "Bytes {}..{} of {}{}",
                        self.offset,
                        self.offset + self.len as u64,
                        self.items[self.selected].bytes,
                        if self.offset + self.len as u64 >= self.items[self.selected].bytes {
                            "  |  EOF"
                        } else {
                            ""
                        }
                    ),
                    View::Error => write!(
                        out,
                        "Cannot read: {}. Esc returns to the list.",
                        match self.error {
                            Some(Error::TooLarge) => "file exceeds 64 MiB",
                            Some(Error::Corrupt) => "corrupt file",
                            Some(Error::Limit) => "read limit reached",
                            Some(Error::Disconnected) => "device disconnected",
                            Some(Error::Unsupported) => "unsupported file",
                            _ => "I/O error",
                        }
                    ),
                };
            }
            9..=18 if self.view == View::List => {
                let i = self.selected / ROWS * ROWS + row - 9;
                if i < self.count {
                    let item = &self.items[i];
                    let _ = write!(
                        out,
                        "{} {:03} {:48} {:>10} B",
                        if i == self.selected { ">" } else { " " },
                        i + 1,
                        item.name.as_str(),
                        item.bytes
                    );
                }
            }
            9..=24 if self.view == View::Page => {
                let start = (row - 9) * 16;
                if start < self.len {
                    let _ = write!(out, "{:08X}  ", self.offset + start as u64);
                    for i in start..start + 16 {
                        if i < self.len {
                            let _ = write!(out, "{:02X} ", self.data[i]);
                        } else {
                            let _ = out.write_str("   ");
                        }
                    }
                    let _ = out.write_char('|');
                    for &b in &self.data[start..(start + 16).min(self.len)] {
                        let _ = out.write_char(if (32..=126).contains(&b) {
                            b as char
                        } else {
                            '.'
                        });
                    }
                    let _ = out.write_char('|');
                }
            }
            LOG_FIRST..=LOG_LAST if self.log_shown() => self.log_line(row, &mut out),
            27 => {
                let _ =
                    out.write_str("256 bytes/page. Home: first page. Non-ASCII names use \\uXXXX.");
            }
            _ => {}
        }
        out
    }
    pub fn render_step(&mut self, ctx: &mut Context<'_>) -> Result<bool, musha_api::Error> {
        self.passkey_tick(ctx.now_ms());
        if !self.dirty {
            if self.log_dirty && self.log_shown() {
                self.log_step(ctx)?;
            }
            return Ok(false);
        }
        let (w, h) = ctx.screen_size();
        let total = w.checked_mul(h).ok_or(musha_api::Error::Invalid)?;
        if self.clear < total {
            let x = self.clear % w;
            let y = self.clear / w;
            let n = (w - x).min(8192);
            ctx.rectangle(x, y, n, 1, BG)?;
            self.clear += n;
            return Ok(false);
        }
        let m = musha_framebuffer::metrics::Metrics::viewer(w, h);
        let scale = m.scale;
        let text = self.line(self.line);
        let color = if self.line == 7 && matches!(self.view, View::Error | View::Unavailable) {
            [255, 180, 110]
        } else if self.line == 0
            || (self.view == View::List && self.line == 9 + self.selected % ROWS)
        {
            [100, 230, 220]
        } else {
            FG
        };
        let x = m.x(musha_framebuffer::metrics::MARGIN);
        let y = m.y(musha_framebuffer::metrics::MARGIN + self.line);
        if w > x && y + 8 * scale <= h {
            ctx.text_at(text.as_str(), x, y, color, scale)?;
        }
        self.line += 1;
        if self.line == 28 {
            self.dirty = false;
            return Ok(true);
        }
        Ok(false)
    }
}
impl Viewer {
    /// Clears the log band in at most 8192-pixel slices, then draws one row
    /// per call, like the full repaint.
    fn log_step(&mut self, ctx: &mut Context<'_>) -> Result<(), musha_api::Error> {
        let (w, h) = ctx.screen_size();
        let m = musha_framebuffer::metrics::Metrics::viewer(w, h);
        let top = m.y(musha_framebuffer::metrics::MARGIN + LOG_FIRST).min(h);
        let bottom = m
            .y(musha_framebuffer::metrics::MARGIN + LOG_LAST + 1)
            .min(h);
        let band = w * (bottom - top);
        if self.log_clear < band {
            let x = self.log_clear % w;
            let y = top + self.log_clear / w;
            let n = (w - x).min(8192);
            ctx.rectangle(x, y, n, 1, BG)?;
            self.log_clear += n;
            return Ok(());
        }
        let text = self.line(self.log_row);
        let x = m.x(musha_framebuffer::metrics::MARGIN);
        let y = m.y(musha_framebuffer::metrics::MARGIN + self.log_row);
        if w > x && y + 8 * m.scale <= h {
            ctx.text_at(text.as_str(), x, y, FG, m.scale)?;
        }
        self.log_row += 1;
        if self.log_row > LOG_LAST {
            self.log_dirty = false;
        }
        Ok(())
    }
}
const _: () = assert!(core::mem::size_of::<Viewer>() < 16 * 1024);

#[cfg(test)]
extern crate std;
#[cfg(test)]
mod tests {
    use super::*;
    use musha_api::stream::{Entry, MAX_FILE_BYTES};
    use std::vec::Vec;
    #[test]
    fn repeated_return_status_does_not_restart_every_render_slice() {
        let mut viewer = Viewer::new();
        viewer.returned(0, 42);
        viewer.dirty = false;
        viewer.clear = 8192;
        viewer.returned(0, 42);
        assert!(!viewer.dirty);
        assert_eq!(viewer.clear, 8192);
        viewer.view = View::List;
        assert_eq!(viewer.line(6).as_str(), "ELF exited: status 42.");
    }
    #[test]
    fn elf_selection_uses_complete_name_and_never_starts_a_page_read() {
        for name in [
            "demo.elf",
            "A-very-long-name-that-is-truncated-by-the-list-and-still-an-ELF.ELF",
        ] {
            let entries = [entry(name, 64)];
            let mut files = Files::new();
            files.mount(Format::Exfat, &entries).unwrap();
            let mut viewer = Viewer::new();
            catalog(&mut viewer, &mut files);
            viewer.key(0x28, 1);
            let id = viewer.launch_request().unwrap();
            assert_eq!(viewer.view(), View::List);
            assert!(files.take_request().is_none());
            assert!(files.open_id(id).is_ok());
            viewer.launch_consumed();
            assert!(viewer.launch_request().is_none());
            viewer.closed(false);
            assert!(viewer.launch_request().is_none());
        }
        assert!(!executable_name(
            &"sample.elf.txt".encode_utf16().collect::<Vec<_>>()
        ));
        assert!(!executable_name(
            &"sample.ＥＬＦ".encode_utf16().collect::<Vec<_>>()
        ));
    }
    #[test]
    fn absent_notice_requires_explicit_result_and_preserves_errors() {
        let mut v = Viewer::new();
        let unavailable = v.line(7);
        v.bootfile_absent();
        assert_eq!(v.line(7).as_str(), unavailable.as_str());
        v.view = View::List;
        assert_eq!(
            v.line(7).as_str(),
            "AUTORUN.ELF not found in root. Starting file viewer."
        );
        v.view = View::Error;
        assert!(v.line(7).as_str().starts_with("Cannot read:"));
        let mut ordinary = Viewer::new();
        ordinary.view = View::List;
        assert!(!ordinary.line(7).as_str().contains("AUTORUN.ELF"));
    }
    fn entry(name: &str, bytes: u64) -> Entry {
        let mut e = Entry {
            name: [0; 255],
            name_len: 0,
            bytes,
        };
        for (i, u) in name.encode_utf16().enumerate() {
            e.name[i] = u;
            e.name_len += 1;
        }
        e
    }
    fn catalog(v: &mut Viewer, f: &mut Files<'_>) {
        for _ in 0..132 {
            v.service(f, 0);
        }
        assert_eq!(v.view(), View::List);
    }
    fn page(v: &mut Viewer, f: &mut Files<'_>, payloads: &[&[u8]]) {
        for _ in 0..100 {
            v.service(f, 1);
            if let Some(r) = f.take_request() {
                let start = r.offset() as usize;
                assert!(r.length() <= PAGE);
                assert!(f.complete(r, Ok(&payloads[r.index()][start..start + r.length()])));
            }
            if v.view() == View::Page {
                return;
            }
        }
        panic!("no page: {:?}", v.view());
    }
    #[test]
    fn page_forward_backward_last_and_empty() {
        let bytes: Vec<u8> = (0..700).map(|n| (n % 251) as u8).collect();
        let entries = [entry("data.bin", 700), entry("empty", 0)];
        let mut f = Files::new();
        f.mount(Format::Exfat, &entries).unwrap();
        let mut v = Viewer::new();
        catalog(&mut v, &mut f);
        v.key(0x28, 0);
        page(&mut v, &mut f, &[&bytes, &[]]);
        assert_eq!(v.data(), &bytes[..256]);
        for offset in [256, 512] {
            v.key(0x4e, 0);
            page(&mut v, &mut f, &[&bytes, &[]]);
            assert_eq!(v.offset(), offset);
            assert_eq!(
                v.data(),
                &bytes[offset as usize..(offset as usize + 256).min(700)]
            );
        }
        v.key(0x4e, 0);
        assert_eq!(v.offset(), 512);
        assert!(v.line(7).as_str().contains("EOF"));
        v.key(0x4b, 0);
        page(&mut v, &mut f, &[&bytes, &[]]);
        assert_eq!(v.data(), &bytes[256..512]);
        v.key(0x4a, 0);
        page(&mut v, &mut f, &[&bytes, &[]]);
        assert_eq!(v.data(), &bytes[..256]);
        v.key(0x29, 0);
        v.key(0x51, 0);
        v.key(0x28, 0);
        page(&mut v, &mut f, &[&bytes, &[]]);
        assert!(v.data().is_empty());
        assert!(v.line(7).as_str().contains("Empty file"));
    }
    #[test]
    fn names_are_escaped_and_identified_by_id_even_when_preview_collides() {
        let prefix = "x".repeat(100);
        let entries = [
            entry(&(prefix.clone() + "a"), 1),
            entry(&(prefix + "b"), 1),
            entry("日本語\\u1234", 1),
        ];
        let mut f = Files::new();
        f.mount(Format::Fat32, &entries).unwrap();
        let mut v = Viewer::new();
        catalog(&mut v, &mut f);
        assert_eq!(v.items[0].name.as_str(), v.items[1].name.as_str());
        v.key(0x51, 0);
        v.service(&mut f, 0);
        v.key(0x4f, 0);
        assert!(v.line(5).as_str().ends_with('b'));
        v.key(0x28, 0);
        page(&mut v, &mut f, &[b"a", b"b", b"c"]);
        assert_eq!(v.data(), b"b");
        v.key(0x29, 0);
        v.key(0x51, 0);
        v.service(&mut f, 0);
        v.service(&mut f, 0);
        assert_eq!(v.selected_name(), "\\u65E5\\u672C\\u8A9E\\u005Cu1234");
        assert_eq!(
            name_text(&[0xd83d, 0xde00, 0]).as_str(),
            "\\uD83D\\uDE00\\u0000"
        );
    }
    #[test]
    fn full_catalog_navigation_and_empty_catalog() {
        let entries: Vec<_> = (0..128).map(|_| entry("same", 0)).collect();
        let mut f = Files::new();
        f.mount(Format::Exfat, &entries).unwrap();
        let mut v = Viewer::new();
        catalog(&mut v, &mut f);
        v.key(0x4e, 0);
        assert_eq!(v.selected(), 10);
        assert!(v.line(7).as_str().contains("2/13"));
        v.key(0x4d, 0);
        v.key(0x51, 0);
        assert_eq!(v.selected(), 127);
        v.key(0x4b, 0);
        assert_eq!(v.selected(), 117);
        v.key(0x4a, 0);
        v.key(0x52, 0);
        assert_eq!(v.selected(), 0);
        let mut f = Files::new();
        f.mount(Format::Fat32, &[]).unwrap();
        let mut v = Viewer::new();
        catalog(&mut v, &mut f);
        v.key(0x28, 0);
        assert_eq!(v.view(), View::List);
        assert!(v.line(7).as_str().contains("no supported files"));
    }
    #[test]
    fn cancel_pending_reopen_waits_for_ack_and_discards_old_bytes() {
        let entries = [entry("a", 1), entry("b", 1)];
        let mut f = Files::new();
        f.mount(Format::Exfat, &entries).unwrap();
        let mut v = Viewer::new();
        catalog(&mut v, &mut f);
        v.key(0x28, 0);
        v.service(&mut f, 0);
        v.service(&mut f, 0);
        let old = f.take_request().unwrap();
        v.key(0x29, 0);
        v.service(&mut f, 0);
        assert!(f.request_canceled(old));
        v.key(0x51, 0);
        v.key(0x28, 0);
        for _ in 0..5 {
            v.service(&mut f, 0);
        }
        assert!(f.take_request().is_none());
        assert_eq!(v.view(), View::Loading);
        f.complete(old, Ok(b"a"));
        page(&mut v, &mut f, &[b"a", b"b"]);
        assert_eq!(v.data(), b"b");
    }
    #[test]
    fn size_timeout_corruption_and_disconnect_never_publish_data() {
        let entries = [entry("large", MAX_FILE_BYTES + 1), entry("data", 1)];
        let mut f = Files::new();
        f.mount(Format::Exfat, &entries).unwrap();
        let mut v = Viewer::new();
        catalog(&mut v, &mut f);
        v.key(0x28, 0);
        v.service(&mut f, 0);
        assert_eq!(v.error(), Some(Error::TooLarge));
        assert!(f.take_request().is_none());
        v.key(0x29, 0);
        v.key(0x51, 0);
        v.key(0x28, 0);
        for _ in 0..4 {
            v.service(&mut f, 0);
        }
        let r = f.take_request().unwrap();
        v.service(&mut f, READ_TIMEOUT_MS);
        assert_eq!(v.error(), Some(Error::Limit));
        v.service(&mut f, READ_TIMEOUT_MS);
        assert!(f.request_canceled(r));
        f.complete(r, Ok(b"x"));
        assert!(v.data().is_empty());
        v.key(0x29, 0);
        v.key(0x28, 0);
        for _ in 0..3 {
            v.service(&mut f, 0);
        }
        let r = f.take_request().unwrap();
        f.complete(r, Err(Error::Corrupt));
        v.service(&mut f, 0);
        assert_eq!(v.error(), Some(Error::Corrupt));
        assert!(v.data().is_empty());
        f.disconnect();
        v.closed(false);
        assert_eq!(v.view(), View::Unavailable);
        assert_eq!(v.count(), 0);
        assert!(v.selected_name().is_empty());
        v.key(0x28, 0);
        assert_eq!(v.view(), View::Unavailable);
        v.key(0x14, 0);
        assert!(v.exit_requested());
    }
    #[test]
    fn closure_distinguishes_missing_keyboard_from_other_session_endings() {
        let mut v = Viewer::new();
        v.closed(true);
        assert_eq!(
            v.line(7).as_str(),
            "USB keyboard not detected. Connect one, then restart."
        );
        assert_eq!(v.view(), View::Unavailable);
        assert_eq!(v.count(), 0);
        v.closed(false);
        assert_eq!(
            v.line(7).as_str(),
            "Device session ended. Restart to try again."
        );
        v.session_reason("BAD TRANSFER");
        assert_eq!(
            v.line(7).as_str(),
            "Device session ended: BAD TRANSFER. Restart to try again."
        );
    }
    #[test]
    fn rendering_clips_to_screen_and_preserves_stride_padding() {
        for (w, h) in [(1, 1), (640, 480), (1280, 800)] {
            let stride = w + 3;
            let mut pixels = std::vec![0xdeadbeefu32; stride*h+4];
            let fb = musha_framebuffer::Framebuffer {
                base: pixels.as_mut_ptr() as usize,
                bytes: stride * h * 4,
                width: w,
                height: h,
                stride,
                format: 0,
            };
            let mut arena = [0; 1];
            let mut ctx = unsafe { Context::new(&mut arena, fb).unwrap() };
            let mut v = Viewer::new();
            for _ in 0..h + 30 {
                v.render_step(&mut ctx).unwrap();
            }
            assert!(v.painted());
            assert_eq!(
                ctx.text_at("A", 0, 0, [0; 3], 0),
                Err(musha_api::Error::Invalid)
            );
            assert_eq!(
                ctx.text_at("A", 0, 0, [0; 3], 5),
                Err(musha_api::Error::Invalid)
            );
            assert_eq!(
                ctx.text_at("A", w, 0, [0; 3], 1),
                Err(musha_api::Error::Invalid)
            );
            for y in 0..h {
                assert_eq!(&pixels[y * stride + w..(y + 1) * stride], &[0xdeadbeef; 3]);
            }
            assert_eq!(&pixels[stride * h..], &[0xdeadbeef; 4]);
        }
    }
    #[test]
    fn bluetooth_log_tail_fills_rows_upward_and_wraps_long_lines() {
        let mut v = Viewer::new();
        assert_eq!(v.line(19).as_str(), "");
        v.push_log("1.000 LE scan enabled");
        assert_eq!(v.line(19).as_str(), "Bluetooth log (devices, newest last):");
        assert_eq!(v.line(26).as_str(), "1.000 LE scan enabled");
        assert_eq!(v.line(25).as_str(), "");
        let long = std::format!("2.000 {}", "x".repeat(100));
        v.push_log(&long);
        assert_eq!(v.line(25).as_str(), &long[..78]);
        assert_eq!(v.line(26).as_str(), std::format!("  {}", &long[78..]));
        assert_eq!(v.line(24).as_str(), "1.000 LE scan enabled");
        // Longer than two rows is cut; control characters are replaced.
        v.push_log(&std::format!("3.000 \u{1}{}", "y".repeat(300)));
        assert!(v.line(26).as_str().len() <= 80);
        assert!(v.line(25).as_str().starts_with("3.000 ?"));
        // Six lines are retained; the oldest is dropped, newest stays at the bottom.
        for i in 0..7 {
            v.push_log(&std::format!("{}.000 stage {}", 10 + i, i));
        }
        assert_eq!(v.line(20).as_str(), "");
        assert_eq!(v.line(21).as_str(), "11.000 stage 1");
        assert_eq!(v.line(26).as_str(), "16.000 stage 6");
        // The hex page view keeps its own rows.
        v.view = View::Page;
        assert_eq!(v.line(26).as_str(), "");
    }
    #[test]
    fn a_new_log_line_repaints_only_the_log_rows() {
        let (w, h) = (640, 480);
        let mut pixels = std::vec![0u32; w * h];
        let fb = musha_framebuffer::Framebuffer {
            base: pixels.as_mut_ptr() as usize,
            bytes: w * h * 4,
            width: w,
            height: h,
            stride: w,
            format: 0,
        };
        let mut arena = [0; 1];
        let mut ctx = unsafe { Context::new(&mut arena, fb).unwrap() };
        let mut v = Viewer::new();
        v.push_log("1.000 first");
        while !v.painted() || v.log_dirty {
            v.render_step(&mut ctx).unwrap();
        }
        let m = musha_framebuffer::metrics::Metrics::viewer(w, h);
        let top = m.y(musha_framebuffer::metrics::MARGIN + LOG_FIRST);
        let bottom = m.y(musha_framebuffer::metrics::MARGIN + LOG_LAST + 1);
        let before = pixels.clone();
        v.push_log("2.000 connect stage=LE create connection phase=6");
        assert!(v.painted() && v.log_dirty);
        let mut steps = 0;
        while v.log_dirty {
            assert!(!v.render_step(&mut ctx).unwrap());
            steps += 1;
        }
        assert!(v.painted());
        assert!(steps > LOG_LAST - LOG_FIRST);
        assert_eq!(&pixels[..top * w], &before[..top * w]);
        assert_eq!(&pixels[bottom * w..], &before[bottom * w..]);
        assert_ne!(&pixels[top * w..bottom * w], &before[top * w..bottom * w]);
    }
    #[test]
    fn pairing_entry_is_a_visible_refusal_until_runtime_security_is_connected() {
        let mut v = Viewer::new();
        assert!(!v.line(6).as_str().contains("pairing"));
        v.key(0x13, 0);
        assert!(v.line(6).as_str().contains("pairing unavailable"));
        v.pairing_unavailable();
        assert!(v.line(6).as_str().contains("runtime service not connected"));
        assert!(!v.exit);
    }
}
