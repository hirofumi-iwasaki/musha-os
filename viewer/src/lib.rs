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
    bytes: u64,
    name: Text<48>,
}
impl Item {
    const EMPTY: Self = Self {
        id: None,
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
    keyboard_missing: bool,
    bootfile_absent: bool,
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
            keyboard_missing: false,
            bootfile_absent: false,
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
        }
    }
    /// Set only after the boot-file reader reports State::Absent, never on I/O failure.
    pub fn bootfile_absent(&mut self) {
        if !self.bootfile_absent {
            self.bootfile_absent = true;
            self.repaint();
        }
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
    /// At most one catalog entry or one bounded read attempt per call, except the
    /// selected-name lookup (at most 128 in-memory entries, no disk access).
    pub fn service(&mut self, files: &mut Files<'_>, now: u64) {
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
            7 => {
                let _ = match self.view {
                    View::Unavailable => out.write_str(if self.keyboard_missing {
                        "USB keyboard not detected. Connect one, then restart."
                    } else if self.ended {
                        "Device session ended. Restart to try again."
                    } else {
                        "No supported FAT32/exFAT medium. External USB keyboard required."
                    }),
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
            27 => {
                let _ =
                    out.write_str("256 bytes/page. Home: first page. Non-ASCII names use \\uXXXX.");
            }
            _ => {}
        }
        out
    }
    pub fn render_step(&mut self, ctx: &mut Context<'_>) -> Result<bool, musha_api::Error> {
        if !self.dirty {
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
        let scale = ((w.saturating_sub(32) / 480).min(h.saturating_sub(32) / 224)).clamp(1, 4);
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
        let y = 16 + self.line * 8 * scale;
        if w > 16 && y < h {
            ctx.text_at(text.as_str(), 16, y, color, scale)?;
        }
        self.line += 1;
        if self.line == 28 {
            self.dirty = false;
            return Ok(true);
        }
        Ok(false)
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
}
