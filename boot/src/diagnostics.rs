// SPDX-License-Identifier: Apache-2.0
//! Boot-CPU-only panel. Interrupts remain disabled; no reference escapes update.
use core::{
    cell::UnsafeCell,
    fmt::{self, Write},
};
use musha_framebuffer::Framebuffer;
use musha_platform::diagnostics::{Failure, Line};
struct Panel {
    fb: Option<Framebuffer>,
    page: Option<[Line; 42]>,
    journal: [Line; 512],
    journal_count: usize,
    lines: [Line; 42],
    usb: usize,
    cached_file: [Line; 4],
    file_controller: Option<usize>,
    history: [Line; 42],
    history_count: usize,
    last_history: Option<usize>,
    archived: [[Line; 42]; 8],
    keyboards: usize,
    storage: usize,
    device_class: [u8; 3],
    controller_ids: [u32; 8],
    controller_bdfs: [u16; 8],
    current_controller: usize,
    usb_fail: Failure,
    net_fail: Failure,
}
struct Shared(UnsafeCell<Panel>);
// SAFETY: only the boot CPU calls these functions with interrupts disabled;
// rendering never invokes callbacks or firmware, and updates cannot reenter.
unsafe impl Sync for Shared {}
static PANEL: Shared = Shared(UnsafeCell::new(Panel {
    fb: None,
    page: None,
    journal: [Line::EMPTY; 512],
    journal_count: 0,
    lines: [Line::EMPTY; 42],
    usb: 0,
    cached_file: [Line::EMPTY; 4],
    file_controller: None,
    history: [Line::EMPTY; 42],
    history_count: 0,
    last_history: None,
    archived: [[Line::EMPTY; 42]; 8],
    keyboards: 0,
    storage: 0,
    device_class: [0; 3],
    controller_ids: [0; 8],
    controller_bdfs: [u16::MAX; 8],
    current_controller: 0,
    usb_fail: Failure::EMPTY,
    net_fail: Failure::EMPTY,
}));
fn line(args: fmt::Arguments<'_>) -> Line {
    let mut l = Line::EMPTY;
    let _ = l.write_fmt(args);
    l
}
fn update(f: impl FnOnce(&mut Panel)) {
    unsafe {
        let p = &mut *PANEL.0.get();
        f(p);
        render(p);
    }
}
const DIAGNOSTIC_SCALE: usize = 6;
const DIAGNOSTIC_LINE_HEIGHT: usize = 50;

fn render(p: &Panel) {
    let Some(fb) = p.fb else { return };
    let x = if fb.width >= 1200 { 800 } else { 0 };
    let y = 24;
    let scale = DIAGNOSTIC_SCALE;
    let columns = (fb.width.saturating_sub(x + 12) / (6 * scale)).clamp(1, 64);
    let capacity = (fb.height.saturating_sub(y + 24) / DIAGNOSTIC_LINE_HEIGHT).clamp(1, 42);
    unsafe {
        for py in y..fb.height {
            for px in x..fb.width {
                fb.pixel(px, py, fb.color(12, 20, 32));
            }
        }
        let rows = p.page.as_ref().unwrap_or(&p.lines);
        let mut row = 0;
        for l in rows {
            for chunk in l.as_str().as_bytes().chunks(columns) {
                if row >= capacity {
                    break;
                }
                fb.text_scaled(
                    core::str::from_utf8(chunk).unwrap_or("?"),
                    x,
                    y + row * DIAGNOSTIC_LINE_HEIGHT,
                    fb.color(240, 220, 100),
                    scale,
                );
                row += 1;
            }
            if row >= capacity {
                break;
            }
        }
    }
}
pub fn start(fb: Framebuffer) {
    update(|p| {
        p.fb = Some(fb);
        p.lines[0] = line(format_args!(
            "{}",
            if cfg!(feature = "t2-diagnostics") {
                "MUSHA-OS 0.2.0 DEV T2-D1"
            } else {
                "MUSHA-OS 0.1.0 HARDWARE H19"
            }
        ));
    });
}
pub fn set(row: usize, args: fmt::Arguments<'_>) {
    update(|p| {
        p.lines[row] = line(args);
        if (36..=39).contains(&row) {
            p.cached_file[row - 36] = p.lines[row];
            p.file_controller = Some(p.current_controller);
        }
    });
}
pub fn usb_stage(s: &str) {
    observation(format_args!("USB STEP {}", s));
    set(4, format_args!("USB STEP {}", s));
}
pub fn net_stage(s: &str) {
    set(7, format_args!("NET STEP {}", s));
}
pub fn usb_failure(s: &str) {
    update(|p| {
        p.usb_fail.record(p.lines[4], line(format_args!("{}", s)));
        p.lines[5] = line(format_args!("FAIL AT {}", p.usb_fail.stage.as_str()));
        p.lines[6] = line(format_args!("USB ERROR {}", p.usb_fail.error.as_str()));
    });
}
pub fn net_failure(s: &str) {
    update(|p| {
        p.net_fail.record(p.lines[7], line(format_args!("{}", s)));
        p.lines[8] = line(format_args!("FAIL AT {}", p.net_fail.stage.as_str()));
        p.lines[9] = line(format_args!("NET ERROR {}", p.net_fail.error.as_str()));
    });
}
pub fn pci(row: usize, id: u32, bdf: u16) {
    set(
        row,
        format_args!(
            "{} PCI {:04X}:{:04X} BDF {:04X}",
            if row == 2 { "XHCI" } else { "LAN" },
            id as u16,
            id >> 16,
            bdf
        ),
    );
}
pub fn usb(
    port: usize,
    route: u32,
    slot: u8,
    vendor: u16,
    product: u16,
    speed: &str,
    class: [u8; 3],
) {
    update(|p| {
        p.device_class = class;
        if p.usb < 8 {
            p.lines[11 + p.usb * 2] = line(format_args!(
                "USB C{} ROOT {:02X} RT{:05X} SLOT {:02X} {:04X}:{:04X} {}",
                p.current_controller, port, route, slot, vendor, product, speed
            ));
        }
        if p.usb < 8 {
            p.lines[12 + p.usb * 2] = line(format_args!(
                "CLASS {:02X}/{:02X}/{:02X} {}",
                class[0],
                class[1],
                class[2],
                musha_platform::diagnostics::device_kind(class[0])
            ));
        }
        p.last_history = None;
        if p.history_count < 40 {
            let n = p.history_count;
            p.last_history = Some(n + 1);
            p.history[n] = line(format_args!(
                "C{} R{:02X} RT{:05X} S{} {:04X}:{:04X} {}",
                p.current_controller, port, route, slot, vendor, product, speed
            ));
            p.history[n + 1] = line(format_args!(
                "CLASS {:02X}/{:02X}/{:02X}",
                class[0], class[1], class[2]
            ));
            p.history_count += 2;
        } else {
            p.history[41] = line(format_args!("INVENTORY LIMIT 20 / MORE OMITTED"));
        }
        p.usb += 1;
        p.lines[10] = line(format_args!("USB DESCRIPTORS {} (FIRST 8 SHOWN)", p.usb));
    });
}
pub fn snapshot() {
    unsafe {
        let p = &*PANEL.0.get();
        for l in p
            .archived
            .iter()
            .flatten()
            .chain(p.lines.iter())
            .chain(p.history.iter())
            .chain(p.journal[..p.journal_count].iter())
        {
            if !l.as_str().is_empty() {
                crate::debug(b"MUSHA: DIAG ");
                crate::debug(l.as_str().as_bytes());
                crate::debug(b"\n");
            }
        }
    }
}

/// PCI inventory is read-only. The star identifies the firmware-selected BAR,
/// which need not be the first controller in PCI configuration scan order.
pub fn controller(index: usize, id: u32, bdf: u16, selected: bool) {
    if index < 8 {
        update(|p| {
            p.controller_ids[index] = id;
            p.controller_bdfs[index] = bdf;
        });
        set(
            27 + index,
            format_args!(
                "XHCI{} {:04X}:{:04X} BDF {:04X} {}",
                index,
                id as u16,
                id >> 16,
                bdf,
                if selected { "*ACTIVE" } else { "NOT PROBED" }
            ),
        );
    }
}
pub fn usb_kind(kind: &str) {
    update(|p| {
        if let Some(n) = p.last_history {
            p.history[n] = line(format_args!("{}", kind));
        }
        if (1..=8).contains(&p.usb) {
            p.lines[12 + (p.usb - 1) * 2] = line(format_args!(
                "DEV {:02X}/{:02X}/{:02X} {}",
                p.device_class[0], p.device_class[1], p.device_class[2], kind
            ));
        }
    });
}
pub fn supported_device(keyboard: bool) {
    update(|p| {
        if keyboard {
            p.keyboards += 1;
        } else {
            p.storage += 1;
        }
        p.lines[35] = line(format_args!(
            "THIS CONTROLLER BOOT KBD {} BOT STORAGE {}",
            p.keyboards, p.storage
        ));
    });
}
pub fn inventory_total(total: usize) {
    set(
        41,
        format_args!("XHCI TOTAL {} / FIRST 8 / *ACTIVE PORTS ONLY", total),
    );
    set(35, format_args!("FOUND BOOT KBD 0 BOT STORAGE 0"));
}

pub fn begin_controller(c: crate::pci::Controller) {
    observation(format_args!("CONTROLLER BDF {:04X}", c.bdf()));
    update(|p| {
        p.current_controller = p
            .controller_bdfs
            .iter()
            .position(|bdf| *bdf == c.bdf())
            .unwrap_or(0);
        p.usb = 0;
        p.keyboards = 0;
        p.storage = 0;
        p.usb_fail = Failure::EMPTY;
        p.lines[5] = Line::EMPTY;
        p.lines[6] = Line::EMPTY;
        for row in 10..27 {
            p.lines[row] = Line::EMPTY;
        }
        for row in 36..41 {
            p.lines[row] = Line::EMPTY;
        }
        p.lines[35] = line(format_args!("THIS CONTROLLER BOOT KBD 0 BOT STORAGE 0"));
        p.lines[27 + p.current_controller] = line(format_args!(
            "XHCI{} BDF {:04X} PROBING",
            p.current_controller,
            c.bdf()
        ));
    });
    pci(
        2,
        unsafe { crate::pci::read(c.bus, c.device, c.function, 0) },
        c.bdf(),
    );
}
pub fn controller_result(
    c: crate::pci::Controller,
    result: Option<(bool, usize)>,
    error: Option<&str>,
    input: bool,
) {
    update(|p| {
        let index = p
            .controller_bdfs
            .iter()
            .position(|bdf| *bdf == c.bdf())
            .unwrap_or(0);
        let id = p.controller_ids[index];
        p.lines[27 + index] = if let Some((kbd, storage)) = result {
            line(format_args!(
                "XHCI{} {:04X}:{:04X} BDF {:04X} {} K{} S{}",
                index,
                id as u16,
                id >> 16,
                c.bdf(),
                if input { "INPUT DONE" } else { "SCANNED" },
                kbd as u8,
                storage
            ))
        } else {
            line(format_args!(
                "XHCI{} BDF {:04X} ERROR {}",
                index,
                c.bdf(),
                error.unwrap_or("UNKNOWN")
            ))
        };
        p.archived[index] = p.lines;
    });
}
pub fn controller_limit(truncated: bool) {
    if truncated {
        set(41, format_args!("CONTROLLER LIMIT 8 / MORE NOT PROBED"));
    } else {
        set(
            41,
            format_args!("SEQUENTIAL SCAN / DETAILS CURRENT CONTROLLER"),
        );
    }
}

/// Preserve the hub's USB generation in the normal panel for physical triage.
pub fn unsupported_hub(version: u16) {
    update(|p| {
        if (1..=8).contains(&p.usb) {
            p.lines[12 + (p.usb - 1) * 2] = line(format_args!(
                "DEV {:02X}/{:02X}/{:02X} HUB USB{:04X} UNSUPPORTED",
                p.device_class[0], p.device_class[1], p.device_class[2], version
            ));
        }
    });
    crate::debug(b"MUSHA: HUB_GENERATION_UNSUPPORTED USB=");
    crate::debug(&crate::cpu::hex(version as u64));
    crate::debug(b"\n");
}

/// Last in-flight control request; retains details when cleanup updates stage.
pub fn control_request(slot: u8, request: u32, index: u16, bytes: u32) {
    set(
        17,
        format_args!(
            "CONTROL S{} REQ {:08X} IDX {:04X} LEN {}",
            slot, request, index, bytes
        ),
    );
}
pub fn transfer_failure(words: [u32; 4], expected: usize, slot: u8, endpoint: u8) {
    set(
        18,
        format_args!(
            "EVENT CC {} RES {} SLOT {} EP {}",
            words[2] >> 24,
            words[2] & 0xffffff,
            words[3] >> 24,
            (words[3] >> 16) & 31
        ),
    );
    set(
        19,
        format_args!("EXPECT S{} EP{} TRB {:016X}", slot, endpoint, expected),
    );
    set(20, format_args!("RAW {:08X} {:08X}", words[0], words[1]));
    set(21, format_args!("RAW {:08X} {:08X}", words[2], words[3]));
}

/// Bounded chronological observations. Repeated unchanged polling is coalesced.
pub fn observation(args: fmt::Arguments<'_>) {
    update(|p| {
        let entry = line(args);
        if p.journal_count > 0
            && p.journal[p.journal_count.min(p.journal.len()) - 1].as_str() == entry.as_str()
        {
            return;
        }
        if p.journal_count < p.journal.len() - 1 {
            p.journal[p.journal_count] = entry;
            p.journal_count += 1;
        } else {
            p.journal[p.journal.len() - 1] = line(format_args!("JOURNAL LIMIT / MORE OMITTED"));
            p.journal_count = p.journal.len();
        }
    });
}
/// Rotate retained diagnostics after all controller work has finished. No USB access.
pub fn page(index: usize) -> usize {
    let mut total = 1;
    update(|p| {
        let Some(fb) = p.fb else { return };
        let x = if fb.width >= 1200 { 800 } else { 0 };
        let columns = (fb.width.saturating_sub(x + 12) / (6 * DIAGNOSTIC_SCALE)).clamp(1, 64);
        let capacity = (fb.height.saturating_sub(48) / DIAGNOSTIC_LINE_HEIGHT).clamp(2, 42) - 1;
        let controllers = p.controller_bdfs.iter().filter(|b| **b != u16::MAX).count();
        let logs = p.journal_count.div_ceil(40);
        let source_count = 2 + controllers + logs;
        let source = |n: usize| {
            let mut rows = [Line::EMPTY; 42];
            if n == 0 {
                rows = p.lines;
                rows[3] = line(format_args!(
                    "CURRENT DETAILS C{} / SUMMARY ALL CONTROLLERS",
                    p.current_controller
                ));
                if let Some(owner) = p.file_controller {
                    rows[35] = line(format_args!(
                        "CACHED FILE RESULT FROM CONTROLLER C{}",
                        owner
                    ));
                    rows[36..40].copy_from_slice(&p.cached_file);
                }
            } else if n == 1 {
                rows[2..42].copy_from_slice(&p.history[..40]);
                rows[1] = p.history[41];
            } else if n < 2 + controllers {
                rows = p.archived[n - 2];
                rows[1] = line(format_args!("SAVED DETAILS C{} / SUMMARY FINAL", n - 2));
                rows[27..35].copy_from_slice(&p.lines[27..35]);
            } else {
                let first = (n - 2 - controllers) * 40;
                for i in 0..40 {
                    if first + i < p.journal_count {
                        rows[i + 2] = p.journal[first + i];
                    }
                }
            }
            rows
        };
        let wrapped_count = |rows: &[Line; 42]| -> usize {
            rows[1..]
                .iter()
                .filter(|l| !l.as_str().is_empty())
                .map(|l| l.as_str().len().div_ceil(columns))
                .sum::<usize>()
                .max(1)
        };
        total = (0..source_count)
            .map(|n| wrapped_count(&source(n)).div_ceil(capacity))
            .sum();
        let mut selected = index % total;
        let mut display = [Line::EMPTY; 42];
        for n in 0..source_count {
            let rows = source(n);
            let pages = wrapped_count(&rows).div_ceil(capacity);
            if selected >= pages {
                selected -= pages;
                continue;
            }
            let first = selected * capacity;
            let mut count = 0;
            for l in &rows[1..] {
                for chunk in l.as_str().as_bytes().chunks(columns) {
                    if count >= first && count < first + capacity {
                        display[1 + count - first] = line(format_args!(
                            "{}",
                            core::str::from_utf8(chunk).unwrap_or("?")
                        ));
                    }
                    count += 1;
                }
            }
            break;
        }
        display[0] = line(format_args!(
            "{} {}/{}",
            if cfg!(feature = "t2-diagnostics") {
                "T2-D1"
            } else {
                "H19"
            },
            index % total + 1,
            total
        ));
        p.page = Some(display);
    });
    total
}
