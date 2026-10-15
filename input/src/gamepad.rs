// Copyright 2026 Hirofumi Iwasaki
// SPDX-License-Identifier: Apache-2.0
//! Bounded HID Game Pad application subset, not an Xbox-specific descriptor.
//! Absolute variable buttons1..32 and GD axes0x30..0x39; values retain descriptor
//! logical units, hat null is -1. Unknown data fields fail closed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Truncated,
    Invalid,
    Unsupported,
    Limit,
    NoGamepad,
}
#[derive(Clone, Copy)]
struct Global {
    page: u16,
    min: i32,
    max: i32,
    size: u8,
    count: u16,
    id: u8,
}
const GLOBAL: Global = Global {
    page: 0,
    min: 0,
    max: 0,
    size: 0,
    count: 0,
    id: 0,
};
#[derive(Clone, Copy)]
struct Field {
    report: usize,
    offset: usize,
    size: u8,
    min: i32,
    max: i32,
    code: u8,
    button: bool,
    null: bool,
}
const FIELD: Field = Field {
    report: 0,
    offset: 0,
    size: 0,
    min: 0,
    max: 0,
    code: 0,
    button: false,
    null: false,
};
#[derive(Clone, Copy)]
struct Report {
    id: u8,
    bits: usize,
    mapped: bool,
}
const REPORT: Report = Report {
    id: 0,
    bits: 0,
    mapped: false,
};
pub struct Layout {
    reports: [Report; 8],
    count: usize,
    fields: [Field; 64],
    used: usize,
    ids: bool,
}
impl Layout {
    pub fn parse(bytes: &[u8]) -> Result<Self, Error> {
        if bytes.len() > 4096 {
            return Err(Error::Limit);
        }
        let mut out = Self {
            reports: [REPORT; 8],
            count: 0,
            fields: [FIELD; 64],
            used: 0,
            ids: false,
        };
        let mut g = GLOBAL;
        let mut stack = [GLOBAL; 4];
        let mut sp = 0;
        let mut collections = [false; 8];
        let mut depth = 0;
        let mut usages = [0u32; 256];
        let mut used = 0;
        let mut minimum = None;
        let mut at = 0;
        while at < bytes.len() {
            let prefix = bytes[at];
            at += 1;
            if prefix == 0xfe {
                return Err(Error::Unsupported);
            }
            let n = match prefix & 3 {
                3 => 4,
                x => x as usize,
            };
            let b = bytes.get(at..at + n).ok_or(Error::Truncated)?;
            at += n;
            let mut raw = [0; 4];
            raw[..n].copy_from_slice(b);
            let value = u32::from_le_bytes(raw);
            let signed = if n == 0 {
                0
            } else {
                ((value as i32) << (32 - n * 8)) >> (32 - n * 8)
            };
            let kind = (prefix >> 2) & 3;
            let tag = prefix >> 4;
            match (kind, tag) {
                (1, 0) => g.page = u16::try_from(value).map_err(|_| Error::Unsupported)?,
                (1, 1) => g.min = signed,
                (1, 2) => {
                    g.max = if g.min < 0 {
                        signed
                    } else {
                        i32::try_from(value).map_err(|_| Error::Unsupported)?
                    }
                }
                (1, 3..=6) => {}
                (1, 7) => {
                    g.size = u8::try_from(value).map_err(|_| Error::Limit)?;
                }
                (1, 8) => {
                    if n != 1 || value == 0 {
                        return Err(Error::Invalid);
                    }
                    g.id = value as u8;
                    out.ids = true;
                }
                (1, 9) => g.count = u16::try_from(value).map_err(|_| Error::Limit)?,
                (1, 10) => {
                    if n != 0 || sp == 4 {
                        return Err(Error::Limit);
                    }
                    stack[sp] = g;
                    sp += 1;
                }
                (1, 11) => {
                    if n != 0 || sp == 0 {
                        return Err(Error::Invalid);
                    }
                    sp -= 1;
                    g = stack[sp];
                }
                (2, 0..=2) => {
                    if n == 0 {
                        return Err(Error::Invalid);
                    }
                    let usage = if n == 4 {
                        value
                    } else {
                        ((g.page as u32) << 16) | value
                    };
                    match tag {
                        0 => {
                            if minimum.is_some() || used == 256 {
                                return Err(Error::Limit);
                            }
                            usages[used] = usage;
                            used += 1;
                        }
                        1 => {
                            if minimum.replace(usage).is_some() {
                                return Err(Error::Invalid);
                            }
                        }
                        _ => {
                            let min = minimum.take().ok_or(Error::Invalid)?;
                            if min > usage || usage - min >= 256 || min >> 16 != usage >> 16 {
                                return Err(Error::Limit);
                            }
                            for u in min..=usage {
                                if used == 256 {
                                    return Err(Error::Limit);
                                }
                                usages[used] = u;
                                used += 1;
                            }
                        }
                    }
                }
                (0, _) => {
                    if minimum.is_some() {
                        return Err(Error::Invalid);
                    }
                    match tag {
                        10 => {
                            if n != 1 || used == 0 || depth == 8 {
                                return Err(Error::Invalid);
                            }
                            collections[depth] = if value == 1 {
                                usages[0] == 0x10005
                            } else {
                                depth != 0 && collections[depth - 1]
                            };
                            depth += 1;
                        }
                        12 => {
                            if n != 0 || depth == 0 {
                                return Err(Error::Invalid);
                            }
                            depth -= 1;
                        }
                        8 => {
                            if depth == 0 || n == 0 || g.size == 0 || g.count == 0 {
                                return Err(Error::Invalid);
                            }
                            let bits = (g.size as usize)
                                .checked_mul(g.count as usize)
                                .ok_or(Error::Limit)?;
                            let r = match out.reports[..out.count].iter().position(|r| r.id == g.id)
                            {
                                Some(r) => r,
                                None => {
                                    if out.count == 8 {
                                        return Err(Error::Limit);
                                    }
                                    let r = out.count;
                                    out.count += 1;
                                    out.reports[r].id = g.id;
                                    r
                                }
                            };
                            let offset = out.reports[r].bits;
                            out.reports[r].bits = offset
                                .checked_add(bits)
                                .filter(|v| *v <= 4096)
                                .ok_or(Error::Limit)?;
                            if value & 1 == 0 && collections[depth - 1] {
                                if value & !0x42 != 0
                                    || value & 2 == 0
                                    || g.size > 32
                                    || g.min > g.max
                                    || used != g.count as usize
                                {
                                    return Err(Error::Unsupported);
                                }
                                let lo = if g.min < 0 {
                                    -(1i64 << (g.size - 1))
                                } else {
                                    0
                                };
                                let hi = if g.min < 0 {
                                    (1i64 << (g.size - 1)) - 1
                                } else {
                                    (1i64 << g.size) - 1
                                };
                                if (g.min as i64) < lo || (g.max as i64) > hi {
                                    return Err(Error::Unsupported);
                                }
                                for (i, &u) in usages[..used].iter().enumerate() {
                                    let page = u >> 16;
                                    let usage = u as u16;
                                    let (button, code) = if page == 9
                                        && (1..=32).contains(&usage)
                                        && g.size == 1
                                        && g.min == 0
                                        && g.max == 1
                                    {
                                        (true, (usage - 1) as u8)
                                    } else if page == 1 && (0x30..=0x39).contains(&usage) {
                                        (false, (usage - 0x30) as u8)
                                    } else if page == 2 && matches!(usage, 0xc4 | 0xc5) {
                                        (false, (usage - 0xc4 + 10) as u8)
                                    } else {
                                        return Err(Error::Unsupported);
                                    };
                                    if value & 0x40 != 0 && (button || code != 9) {
                                        return Err(Error::Unsupported);
                                    }
                                    if out.fields[..out.used]
                                        .iter()
                                        .any(|f| f.button == button && f.code == code)
                                    {
                                        return Err(Error::Unsupported);
                                    }
                                    if out.used == 64 {
                                        return Err(Error::Limit);
                                    }
                                    out.fields[out.used] = Field {
                                        report: r,
                                        offset: offset + i * g.size as usize,
                                        size: g.size,
                                        min: g.min,
                                        max: g.max,
                                        code,
                                        button,
                                        null: value & 0x40 != 0,
                                    };
                                    out.used += 1;
                                    out.reports[r].mapped = true;
                                }
                            }
                        }
                        9 | 11 => {
                            if depth == 0 || n == 0 {
                                return Err(Error::Invalid);
                            }
                        }
                        _ => return Err(Error::Unsupported),
                    }
                    used = 0;
                    usages.fill(0);
                }
                _ => return Err(Error::Unsupported),
            }
        }
        if depth != 0 || sp != 0 || minimum.is_some() {
            return Err(Error::Invalid);
        }
        if out.used == 0 {
            return Err(Error::NoGamepad);
        }
        if out.ids && out.reports[..out.count].iter().any(|r| r.id == 0) {
            return Err(Error::Invalid);
        }
        Ok(out)
    }
    pub fn report_bytes(&self, id: u8) -> Option<usize> {
        self.reports[..self.count]
            .iter()
            .find(|r| r.id == id && r.mapped)
            .map(|r| r.bits.div_ceil(8) + usize::from(self.ids))
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Change {
    Button(u8, bool),
    Axis(u8, i32),
}
pub struct Decoder {
    layout: Layout,
    previous: [Option<i32>; 64],
}
impl Decoder {
    pub fn new(layout: Layout) -> Self {
        Self {
            layout,
            previous: [None; 64],
        }
    }
    pub fn update(&mut self, report: &[u8], mut emit: impl FnMut(Change)) -> Result<(), Error> {
        let id = if self.layout.ids {
            *report.first().ok_or(Error::Truncated)?
        } else {
            0
        };
        let r = self.layout.reports[..self.layout.count]
            .iter()
            .position(|r| r.id == id && r.mapped)
            .ok_or(Error::Unsupported)?;
        if self.layout.report_bytes(id) != Some(report.len()) {
            return Err(Error::Invalid);
        }
        let bytes = &report[usize::from(self.layout.ids)..];
        let mut values = [0i32; 64];
        for (i, f) in self.layout.fields[..self.layout.used]
            .iter()
            .enumerate()
            .filter(|(_, f)| f.report == r)
        {
            let mut raw = 0u32;
            for bit in 0..f.size as usize {
                let p = f.offset + bit;
                raw |= (((bytes[p / 8] >> (p % 8)) & 1) as u32) << bit;
            }
            let value = if f.min < 0 {
                if f.size == 32 {
                    raw as i32
                } else {
                    ((raw << (32 - f.size)) as i32) >> (32 - f.size)
                }
            } else {
                i32::try_from(raw).map_err(|_| Error::Invalid)?
            };
            values[i] = if (f.min..=f.max).contains(&value) {
                value
            } else if f.null {
                -1
            } else {
                return Err(Error::Invalid);
            };
        }
        // Full report validates before any event/state update.
        for (i, f) in self.layout.fields[..self.layout.used]
            .iter()
            .enumerate()
            .filter(|(_, f)| f.report == r)
        {
            let value = values[i];
            if self.previous[i] != Some(value) {
                if !f.button || self.previous[i].is_some() || value != 0 {
                    emit(if f.button {
                        Change::Button(f.code, value != 0)
                    } else {
                        Change::Axis(f.code, value)
                    });
                }
                self.previous[i] = Some(value);
            }
        }
        Ok(())
    }
    pub fn disconnect(&mut self, mut emit: impl FnMut(Change)) {
        for (i, f) in self.layout.fields[..self.layout.used].iter().enumerate() {
            if f.button && self.previous[i].is_some_and(|v| v != 0) {
                emit(Change::Button(f.code, false));
            }
            self.previous[i] = None;
        }
    }
}
