// SPDX-License-Identifier: Apache-2.0
//! Boot-CPU-only panel. Interrupts remain disabled; no reference escapes update.
use core::{
    cell::UnsafeCell,
    fmt::{self, Write},
};
use musha_framebuffer::Framebuffer;
use musha_platform::diagnostics::{Failure, Line};
struct Panel {
    hidden: bool,
    painted: [Line; 42],
    paint_valid: bool,
    paint_rows: u64,
    omitted: u64,
    first_failure: Option<Line>,
    last_failure: Option<Line>,
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
    hidden: false,
    painted: [Line::EMPTY; 42],
    paint_valid: false,
    paint_rows: 0,
    omitted: 0,
    first_failure: None,
    last_failure: None,
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
    }
}
const VERSION: &str = include_str!("../../VERSION");
fn test_name() -> &'static str {
    if cfg!(feature = "t2-diagnostics") {
        "T2-D1"
    } else if cfg!(feature = "ring3-lifecycle-probe") {
        "RING3 LIFECYCLE"
    } else if cfg!(feature = "ring3-elf-probe") {
        "RING3 ELF"
    } else if cfg!(feature = "ring3-probe") {
        "RING3"
    } else {
        "EXTERNAL ELF"
    }
}

pub fn hide(hidden: bool) {
    update(|p| {
        if p.hidden != hidden {
            p.hidden = hidden;
            p.paint_valid = false;
        }
    });
}
/// Explicit BSP safe point. Record updates and view selection never touch devices.
pub fn flush() {
    flush_reserved(0);
}

/// Explicit devices-level startup capture; preserve hidden state and leave footer space.
pub fn startup_snapshot() {
    let hidden = unsafe { (*PANEL.0.get()).hidden };
    hide(false);
    flush_reserved(4);
    hide(hidden);
}

fn flush_reserved(reserved: usize) {
    unsafe {
        let p = &mut *PANEL.0.get();
        if p.hidden || cfg!(feature = "diagnostics-off") {
            return;
        }
        let Some(fb) = p.fb else { return };
        let m = musha_framebuffer::metrics::Metrics::new(fb.width, fb.height);
        let scale = m.scale;
        let line_height = m.line_pitch;
        let x = m.x(m.panel_column);
        let columns = m.columns.saturating_sub(m.panel_column + 2).clamp(1, 64);
        let capacity = m.rows.saturating_sub(m.panel_row + 2 + reserved).min(42);
        let mut display = [Line::EMPTY; 42];
        let mut row = 0;
        for l in p.page.as_ref().unwrap_or(&p.lines) {
            for chunk in l.as_str().as_bytes().chunks(columns) {
                if row >= capacity {
                    break;
                }
                display[row] = line(format_args!(
                    "{}",
                    core::str::from_utf8(chunk).unwrap_or("?")
                ));
                row += 1;
            }
            if row >= capacity {
                break;
            }
        }
        for (row, value) in display.iter().enumerate().take(capacity) {
            if p.paint_valid && p.painted[row] == *value {
                continue;
            }
            let y = m.y(row + m.panel_row);
            for py in y..(y + line_height).min(fb.height) {
                for px in x..fb.width {
                    fb.pixel(px, py, fb.color(12, 20, 32));
                }
            }
            fb.text_scaled(value.as_str(), x, y, fb.color(240, 220, 100), scale);
            p.paint_rows += 1;
        }
        p.painted = display;
        p.paint_valid = true;
    }
}
pub fn start(fb: Framebuffer) {
    update(|p| {
        p.fb = Some(fb);
        p.paint_valid = false;
        p.lines[0] = line(format_args!("MUSHA-OS {} {}", VERSION.trim(), test_name()));
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
/// Bounded failure summary independent of optional detail retention.
pub fn failure(stage: &str, error: &str) {
    crate::bootlog::failure(stage, error);
    update(|p| {
        p.first_failure
            .get_or_insert(line(format_args!("{}", stage)));
        p.last_failure = Some(line(format_args!("{}", error)));
    });
}
pub fn usb_failure(s: &str) {
    update(|p| {
        p.usb_fail.record(p.lines[4], line(format_args!("{}", s)));
        p.first_failure.get_or_insert(p.lines[4]);
        p.last_failure = Some(line(format_args!("{}", s)));
        p.lines[5] = line(format_args!("FAIL AT {}", p.usb_fail.stage.as_str()));
        p.lines[6] = line(format_args!("USB ERROR {}", p.usb_fail.error.as_str()));
    });
}
pub fn net_failure(s: &str) {
    update(|p| {
        p.net_fail.record(p.lines[7], line(format_args!("{}", s)));
        p.first_failure.get_or_insert(p.lines[7]);
        p.last_failure = Some(line(format_args!("{}", s)));
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
        crate::debug(b"MUSHA: DIAG_PAINT_ROWS=");
        crate::debug(&crate::cpu::hex(p.paint_rows));
        crate::debug(b" OMITTED=");
        crate::debug(&crate::cpu::hex(p.omitted));
        crate::debug(b"\n");
        for (label, value) in [
            (b"MUSHA: FIRST_FAILURE=".as_slice(), p.first_failure),
            (b"MUSHA: LAST_FAILURE=".as_slice(), p.last_failure),
        ] {
            if let Some(value) = value {
                crate::debug(label);
                crate::debug(value.as_str().as_bytes());
                crate::debug(b"\n");
            }
        }
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
// Preserve the outer storage view while a separate input controller is live.
// Its lines use the existing archive; only small counters travel on the stack.
pub struct ControllerState {
    index: usize,
    usb: usize,
    keyboards: usize,
    storage: usize,
    class: [u8; 3],
    failure: Failure,
}
pub fn suspend_controller() -> ControllerState {
    let mut saved = ControllerState {
        index: 0,
        usb: 0,
        keyboards: 0,
        storage: 0,
        class: [0; 3],
        failure: Failure::EMPTY,
    };
    update(|p| {
        p.archived[p.current_controller] = p.lines;
        saved = ControllerState {
            index: p.current_controller,
            usb: p.usb,
            keyboards: p.keyboards,
            storage: p.storage,
            class: p.device_class,
            failure: p.usb_fail,
        };
    });
    saved
}
pub fn resume_controller(saved: ControllerState) {
    update(|p| {
        let mut summary = [Line::EMPTY; 8];
        summary.copy_from_slice(&p.lines[27..35]);
        let mut net = [Line::EMPTY; 3];
        net.copy_from_slice(&p.lines[7..10]);
        p.lines = p.archived[saved.index];
        p.lines[27..35].copy_from_slice(&summary);
        p.lines[7..10].copy_from_slice(&net);
        p.current_controller = saved.index;
        p.usb = saved.usb;
        p.keyboards = saved.keyboards;
        p.storage = saved.storage;
        p.device_class = saved.class;
        p.usb_fail = saved.failure;
    });
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
            p.omitted = p.omitted.saturating_add(1);
            p.journal[p.journal.len() - 1] = line(format_args!("JOURNAL LIMIT / MORE OMITTED"));
            p.journal_count = p.journal.len();
        }
    });
}
/// Rotate retained diagnostics after all controller work has finished. No USB access.
#[allow(dead_code)] // On-demand detailed view; never a boot-time wait loop.
pub fn page(index: usize) -> usize {
    let mut total = 1;
    update(|p| {
        let Some(fb) = p.fb else { return };
        let m = musha_framebuffer::metrics::Metrics::new(fb.width, fb.height);
        let columns = m.columns.saturating_sub(m.panel_column + 2).clamp(1, 64);
        let (session, rescan, cycle) = crate::timing::display_state();
        let header_rows = if cycle > 0 { 2 } else { 1 };
        let capacity = m.rows.saturating_sub(m.panel_row + 2).clamp(3, 42) - header_rows;
        let controllers = p.controller_bdfs.iter().filter(|b| **b != u16::MAX).count();
        let logs = p.journal_count.div_ceil(40);
        let timing_pages = crate::timing::count().div_ceil(40);
        let source_count = 2 + controllers + logs + timing_pages;
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
            } else if n >= 2 + controllers + logs {
                rows[1] = line(format_args!("PM EPOCH TIMINGS / MS / BDF FFFF = APP"));
                let first = (n - 2 - controllers - logs) * 40;
                for i in 0..40 {
                    if first + i < crate::timing::count() {
                        rows[i + 2] = crate::timing::row(first + i);
                    }
                }
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
                        display[header_rows + count - first] = line(format_args!(
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
            "T+{} SESSION {} RESCAN {} {}/{}",
            crate::bootstate::epoch_now().unwrap_or(0) / 1000,
            session,
            rescan,
            index % total + 1,
            total
        ));
        if cycle > 0 {
            display[1] = line(format_args!("DIAGNOSTIC HOLD (no retry) cycle {}", cycle));
        }
        p.page = Some(display);
    });
    total
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn records_views_overflow_and_small_screen() {
        let mut pixels = std::vec![0xdeadbeefu32;320*356+16];
        let fb = Framebuffer {
            base: pixels.as_mut_ptr() as usize,
            bytes: 320 * 356 * 4,
            width: 320,
            height: 356,
            stride: 320,
            format: 0,
        };
        start(fb);
        set(1, format_args!("CORE READY"));
        usb_stage("RESET");
        usb_failure("TIMEOUT");
        for i in 0..600 {
            observation(format_args!("EVENT {}", i));
        }
        assert!(pixels.iter().all(|p| *p == 0xdeadbeef));
        unsafe {
            let p = &*PANEL.0.get();
            assert!(p.omitted > 0);
            assert_eq!(p.first_failure.unwrap().as_str(), "USB STEP RESET");
            assert_eq!(p.last_failure.unwrap().as_str(), "TIMEOUT");
        }
        flush();
        let before = pixels.clone();
        let rows = unsafe { (*PANEL.0.get()).paint_rows };
        set(1, format_args!("CORE READY"));
        flush();
        assert_eq!(pixels, before);
        assert_eq!(unsafe { (*PANEL.0.get()).paint_rows }, rows);
        hide(true);
        page(3);
        set(1, format_args!("APP RUNNING"));
        flush();
        assert_eq!(pixels, before);
        hide(false);
        page(0);
        flush();
        assert_eq!(&pixels[320 * 356..], &[0xdeadbeef; 16]);
        assert_eq!(
            unsafe { (*PANEL.0.get()).first_failure.unwrap() }.as_str(),
            "USB STEP RESET"
        );
        crate::timing::begin_session();
        crate::timing::set_rescan(1);
        crate::timing::hold_cycle(0);
        page(0);
        let first_evidence = unsafe { (*PANEL.0.get()).page.unwrap()[1] };
        crate::timing::hold_cycle(2);
        page(0);
        let held = unsafe { (*PANEL.0.get()).page.unwrap() };
        assert!(held[0].as_str().starts_with("T+0 SESSION 1 RESCAN 1"));
        assert_eq!(held[1].as_str(), "DIAGNOSTIC HOLD (no retry) cycle 2");
        assert_eq!(held[2].as_str(), first_evidence.as_str());
        crate::timing::hold_cycle(0);
        hide(true);
        startup_snapshot();
        assert!(unsafe { (*PANEL.0.get()).hidden });
        assert_eq!(&pixels[320 * 356..], &[0xdeadbeef; 16]);
    }
}
