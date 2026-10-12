// Copyright 2026 Hirofumi Iwasaki
// SPDX-License-Identifier: Apache-2.0
//! Bounded keyboard subset of USB HID 1.11 (6.2.2, 8). No device I/O.
//! Unsupported encodings fail explicitly; never infer T2's report format.
const REPORTS: usize = 8;
const FIELDS: usize = 32;
const MAX_BITS: usize = 4096;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Truncated,
    Malformed,
    Unsupported,
    Limit,
    NoKeyboard,
    ReportId,
    Length,
    Value,
    Rollover,
}
#[derive(Clone, Copy, Default)]
struct Global {
    page: u32,
    min: i64,
    max: i64,
    size: u32,
    count: u32,
    id: u8,
}
struct Local {
    usages: [u32; 256],
    count: usize,
    minimum: Option<u32>,
}
impl Default for Local {
    fn default() -> Self {
        Self {
            usages: [0; 256],
            count: 0,
            minimum: None,
        }
    }
}
impl Local {
    fn add(&mut self, usage: u32) -> Result<(), Error> {
        if self.count == 256 {
            return Err(Error::Limit);
        }
        self.usages[self.count] = usage;
        self.count += 1;
        Ok(())
    }
}
#[derive(Clone, Copy)]
struct Field {
    report: usize,
    offset: usize,
    size: usize,
    count: usize,
    variable: bool,
    min: u32,
    max: u32,
    usages: [u8; 256],
    usage_count: usize,
}
const EMPTY_FIELD: Field = Field {
    report: 0,
    offset: 0,
    size: 0,
    count: 0,
    variable: false,
    min: 0,
    max: 0,
    usages: [0; 256],
    usage_count: 0,
};
#[derive(Clone, Copy, Default)]
struct Report {
    id: u8,
    bits: usize,
}
pub struct Layout {
    reports: [Report; REPORTS],
    report_count: usize,
    fields: [Field; FIELDS],
    field_count: usize,
    ids: bool,
}
impl Layout {
    /// Max 4096 descriptor bytes, 8 input IDs, 32 keyboard fields, 512 bytes/ID.
    /// Supports unsigned absolute arrays (up to 8 bits) and 1-bit variables.
    pub fn parse(bytes: &[u8]) -> Result<Self, Error> {
        if bytes.len() > 4096 {
            return Err(Error::Limit);
        }
        let mut out = Self {
            reports: [Report::default(); REPORTS],
            report_count: 0,
            fields: [EMPTY_FIELD; FIELDS],
            field_count: 0,
            ids: false,
        };
        let mut g = Global::default();
        let mut stack = [g; 4];
        let mut sp = 0;
        let mut local = Local::default();
        let mut collections = [false; 8];
        let mut depth = 0;
        let mut pos = 0;
        while pos < bytes.len() {
            let prefix = bytes[pos];
            pos += 1;
            if prefix == 0xfe {
                return Err(Error::Unsupported);
            }
            let n = match prefix & 3 {
                3 => 4,
                v => v as usize,
            };
            let data = bytes.get(pos..pos + n).ok_or(Error::Truncated)?;
            pos += n;
            let mut raw = [0u8; 4];
            raw[..n].copy_from_slice(data);
            let value = u32::from_le_bytes(raw);
            let signed = if n == 0 {
                0
            } else {
                (value as i32)
                    .wrapping_shl((32 - n * 8) as u32)
                    .wrapping_shr((32 - n * 8) as u32) as i64
            };
            let kind = (prefix >> 2) & 3;
            let tag = prefix >> 4;
            match (kind, tag) {
                (1, 0) => {
                    if value > 0xffff {
                        return Err(Error::Unsupported);
                    }
                    g.page = value;
                }
                (1, 1) => g.min = signed,
                (1, 2) => g.max = if g.min < 0 { signed } else { value as i64 },
                (1, 3..=6) => {} // physical units do not affect key usages
                (1, 7) => g.size = value,
                (1, 8) => {
                    if n != 1 || value == 0 {
                        return Err(Error::Malformed);
                    }
                    g.id = value as u8;
                    out.ids = true;
                }
                (1, 9) => g.count = value,
                (1, 10) => {
                    if n != 0 {
                        return Err(Error::Malformed);
                    }
                    if sp == 4 {
                        return Err(Error::Limit);
                    }
                    stack[sp] = g;
                    sp += 1;
                }
                (1, 11) => {
                    if n != 0 || sp == 0 {
                        return Err(Error::Malformed);
                    }
                    sp -= 1;
                    g = stack[sp];
                }
                (2, 0..=2) => {
                    if n == 0 {
                        return Err(Error::Malformed);
                    }
                    let u = if n == 4 {
                        value
                    } else {
                        (g.page << 16) | value
                    };
                    match tag {
                        0 => {
                            if local.minimum.is_some() {
                                return Err(Error::Unsupported);
                            }
                            local.add(u)?;
                        }
                        1 => {
                            if local.minimum.replace(u).is_some() {
                                return Err(Error::Malformed);
                            }
                        }
                        _ => {
                            let min = local.minimum.take().ok_or(Error::Malformed)?;
                            if min > u || min >> 16 != u >> 16 {
                                return Err(Error::Malformed);
                            }
                            if u - min >= 256 {
                                return Err(Error::Limit);
                            }
                            for usage in min..=u {
                                local.add(usage)?;
                            }
                        }
                    }
                }
                (0, _) => {
                    if local.minimum.is_some() {
                        return Err(Error::Malformed);
                    }
                    match tag {
                        10 => {
                            if n != 1 || local.count == 0 {
                                return Err(Error::Malformed);
                            }
                            if depth == 8 {
                                return Err(Error::Limit);
                            }
                            let parent = depth != 0 && collections[depth - 1];
                            // Nested Application collections establish their own usage domain.
                            collections[depth] = if value == 1 {
                                local.usages[0] == 0x0001_0006
                            } else {
                                parent
                            };
                            depth += 1;
                        }
                        12 => {
                            if n != 0 || depth == 0 {
                                return Err(Error::Malformed);
                            }
                            depth -= 1;
                        }
                        8 => {
                            if depth == 0 || n == 0 || g.size == 0 || g.count == 0 {
                                return Err(Error::Malformed);
                            }
                            let bits = g.size.checked_mul(g.count).ok_or(Error::Limit)? as usize;
                            if bits > MAX_BITS {
                                return Err(Error::Limit);
                            }
                            let r = match out.reports[..out.report_count]
                                .iter()
                                .position(|r| r.id == g.id)
                            {
                                Some(r) => r,
                                None => {
                                    if out.report_count == REPORTS {
                                        return Err(Error::Limit);
                                    }
                                    let r = out.report_count;
                                    out.report_count += 1;
                                    out.reports[r].id = g.id;
                                    r
                                }
                            };
                            let offset = out.reports[r].bits;
                            let end = offset.checked_add(bits).ok_or(Error::Limit)?;
                            if end > MAX_BITS {
                                return Err(Error::Limit);
                            }
                            out.reports[r].bits = end;
                            if value & 1 == 0 && collections[depth - 1] {
                                if local.count == 0 {
                                    return Err(Error::Unsupported);
                                }
                                let keyboard =
                                    local.usages[..local.count].iter().any(|u| u >> 16 == 7);
                                if keyboard {
                                    if value & !2 != 0
                                        || g.min < 0
                                        || g.max < g.min
                                        || g.max > 255
                                        || g.size > 8
                                    {
                                        return Err(Error::Unsupported);
                                    }
                                    let variable = value & 2 != 0;
                                    if variable
                                        && (g.size != 1
                                            || g.min != 0
                                            || g.max != 1
                                            || local.count != g.count as usize)
                                    {
                                        return Err(Error::Unsupported);
                                    }
                                    if !variable && local.count != (g.max - g.min + 1) as usize {
                                        return Err(Error::Unsupported);
                                    }
                                    if g.max >= (1i64 << g.size) {
                                        return Err(Error::Malformed);
                                    }
                                    if out.field_count == FIELDS {
                                        return Err(Error::Limit);
                                    }
                                    let mut f = Field {
                                        report: r,
                                        offset,
                                        size: g.size as usize,
                                        count: g.count as usize,
                                        variable,
                                        min: g.min as u32,
                                        max: g.max as u32,
                                        usage_count: local.count,
                                        ..EMPTY_FIELD
                                    };
                                    for (i, u) in local.usages[..local.count].iter().enumerate() {
                                        if u >> 16 != 7 || u & 0xffff > 255 {
                                            return Err(Error::Unsupported);
                                        }
                                        f.usages[i] = *u as u8;
                                    }
                                    out.fields[out.field_count] = f;
                                    out.field_count += 1;
                                }
                            }
                        }
                        9 | 11 => {
                            if n == 0 || depth == 0 {
                                return Err(Error::Malformed);
                            }
                        } // output/feature have separate offsets
                        _ => return Err(Error::Unsupported),
                    }
                    local = Local::default();
                }
                _ => return Err(Error::Unsupported),
            }
        }
        if depth != 0 || sp != 0 || local.minimum.is_some() || local.count != 0 {
            return Err(Error::Malformed);
        }
        if out.ids && out.reports[..out.report_count].iter().any(|r| r.id == 0) {
            return Err(Error::Malformed);
        }
        if out.field_count == 0 {
            return Err(Error::NoKeyboard);
        }
        Ok(out)
    }
    /// Exact input length, including an ID byte when present.
    pub fn report_bytes(&self, id: u8) -> Option<usize> {
        self.reports[..self.report_count]
            .iter()
            .find(|r| r.id == id)
            .map(|r| r.bits.div_ceil(8) + usize::from(self.ids))
    }
}
#[derive(Clone, Copy, Default)]
struct Keys([u64; 4]);
impl Keys {
    fn set(&mut self, k: u8) {
        self.0[k as usize / 64] |= 1u64 << (k % 64);
    }
    fn has(self, k: usize) -> bool {
        self.0[k / 64] & (1u64 << (k % 64)) != 0
    }
}
fn changes(before: Keys, after: Keys, mut emit: impl FnMut(u8, bool)) {
    for down in [false, true] {
        for k in (0xe0..=0xe7).chain(4..0xe0).chain(0xe8..256) {
            if before.has(k) != after.has(k) && after.has(k) == down {
                emit(k as u8, down);
            }
        }
    }
}
pub struct Decoder {
    layout: Layout,
    previous: [Keys; REPORTS],
}
impl Decoder {
    pub fn new(layout: Layout) -> Self {
        Self {
            layout,
            previous: [Keys::default(); REPORTS],
        }
    }
    fn combined(&self) -> Keys {
        let mut all = Keys::default();
        for p in self.previous {
            for (a, b) in all.0.iter_mut().zip(p.0) {
                *a |= b;
            }
        }
        all
    }
    /// Validate the entire report before changing any held-key state.
    /// Invalid/unknown/truncated reports and rollover leave state unchanged.
    pub fn update(&mut self, bytes: &[u8], emit: impl FnMut(u8, bool)) -> Result<(), Error> {
        let (id, data) = if self.layout.ids {
            let (id, data) = bytes.split_first().ok_or(Error::Length)?;
            (*id, data)
        } else {
            (0, bytes)
        };
        let r = self.layout.reports[..self.layout.report_count]
            .iter()
            .position(|r| r.id == id)
            .ok_or(Error::ReportId)?;
        if data.len() != self.layout.reports[r].bits.div_ceil(8) {
            return Err(Error::Length);
        }
        let mut next = Keys::default();
        for f in self.layout.fields[..self.layout.field_count]
            .iter()
            .filter(|f| f.report == r)
        {
            for i in 0..f.count {
                let mut v = 0u32;
                for bit in 0..f.size {
                    let p = f.offset + i * f.size + bit;
                    v |= (((data[p / 8] >> (p % 8)) & 1) as u32) << bit;
                }
                if v < f.min || v > f.max {
                    return Err(Error::Value);
                }
                let usage = if f.variable {
                    if v == 0 {
                        continue;
                    }
                    f.usages[i]
                } else {
                    let index = (v - f.min) as usize;
                    if index >= f.usage_count {
                        return Err(Error::Value);
                    }
                    f.usages[index]
                };
                if (1..=3).contains(&usage) {
                    return Err(Error::Rollover);
                }
                if usage != 0 {
                    next.set(usage);
                }
            }
        }
        let before = self.combined();
        self.previous[r] = next;
        changes(before, self.combined(), emit);
        Ok(())
    }
    pub fn release_all(&mut self, emit: impl FnMut(u8, bool)) {
        let before = self.combined();
        self.previous = [Keys::default(); REPORTS];
        changes(before, Keys::default(), emit);
    }
}
