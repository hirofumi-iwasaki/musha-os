// SPDX-License-Identifier: Apache-2.0
//! Bounded boot-mouse and byte-aligned relative-report subset, not generic HID.
use crate::{Speed, keyboard};
pub fn configuration(
    bytes: &[u8],
    speed: Speed,
) -> Result<Option<keyboard::Keyboard>, &'static str> {
    keyboard::configuration_for(bytes, speed, 2, 3)
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Layout {
    pub bytes: usize,
    pub wheel: bool,
    pub buttons: u8,
}
/// Recognize button-byte + signed8 relative X/Y[/wheel], without report IDs.
/// Unknown/absolute formats cannot enable wheel and fall back to HID boot mode.
pub fn layout(b: &[u8]) -> Option<Layout> {
    if b.is_empty() || b.len() > 256 {
        return None;
    }
    let (mut at, mut page, mut size, mut count, mut bits, mut depth) =
        (0, 0u32, 0u32, 0u32, 0u32, 0u32);
    let (mut usages, mut used) = ([0u32; 3], 0usize);
    let (mut logical_min, mut logical_max) = (0i32, 0i32);
    let (mut buttons, mut axes, mut wheel) = (0u8, false, false);
    while at < b.len() {
        let prefix = b[at];
        at += 1;
        if prefix == 0xfe {
            return None;
        }
        let n = match prefix & 3 {
            3 => 4,
            v => v as usize,
        };
        if n > 2 || at + n > b.len() {
            return None;
        }
        let mut value = 0u32;
        for i in 0..n {
            value |= (b[at + i] as u32) << (i * 8);
        }
        at += n;
        match prefix & 0xfc {
            0x04 => page = value,
            0x74 => size = value,
            0x94 => count = value,
            0x08 => {
                if used >= 3 {
                    return None;
                }
                usages[used] = value;
                used += 1;
            }
            0x18 | 0x28 => {}
            0x14 => {
                logical_min = if n == 1 {
                    value as u8 as i8 as i32
                } else {
                    value as u16 as i16 as i32
                }
            }
            0x24 => logical_max = value as i32, // usage/logical bounds; layout still checked below
            0xa0 => {
                depth += 1;
                if depth > 4 {
                    return None;
                }
                used = 0;
            }
            0xc0 => {
                depth = depth.checked_sub(1)?;
                used = 0;
            }
            0x80 => {
                let width = size.checked_mul(count)?;
                if value & 1 != 0 {
                    if bits + width != 8 || buttons == 0 {
                        return None;
                    }
                } else if page == 9
                    && bits == 0
                    && size == 1
                    && (1..=8).contains(&count)
                    && value == 2
                {
                    buttons = count as u8;
                } else if page == 1
                    && bits == 8
                    && size == 8
                    && (count == 2 || count == 3)
                    && matches!(logical_min, -127 | -128)
                    && logical_max == 127
                    && value == 6
                    && used == count as usize
                    && usages[0..2] == [0x30, 0x31]
                    && (count == 2 || usages[2] == 0x38)
                {
                    axes = true;
                    wheel = count == 3;
                } else {
                    return None;
                }
                bits = bits.checked_add(width)?;
                if bits > 32 {
                    return None;
                }
                used = 0;
            }
            _ => return None,
        }
    }
    if depth != 0 || !axes || buttons == 0 || bits != if wheel { 32 } else { 24 } {
        return None;
    }
    Some(Layout {
        bytes: (bits / 8) as usize,
        wheel,
        buttons,
    })
}
pub fn decode(layout: Layout, b: &[u8]) -> Option<([i32; 3], u32)> {
    if b.len() != layout.bytes
        || !(3..=4).contains(&layout.bytes)
        || layout.wheel != (layout.bytes == 4)
        || !(1..=8).contains(&layout.buttons)
    {
        return None;
    }
    if b[0] & !((1u16 << layout.buttons) - 1) as u8 != 0 {
        return None;
    }
    Some((
        [
            b[1] as i8 as i32,
            b[2] as i8 as i32,
            if layout.wheel { b[3] as i8 as i32 } else { 0 },
        ],
        b[0] as u32,
    ))
}
#[cfg(test)]
mod tests {
    extern crate std;
    use super::*;
    fn descriptor(wheel: bool) -> std::vec::Vec<u8> {
        let mut b = std::vec![
            5, 1, 9, 2, 0xa1, 1, 9, 1, 0xa1, 0, 5, 9, 0x19, 1, 0x29, 3, 0x15, 0, 0x25, 1, 0x75, 1,
            0x95, 3, 0x81, 2, 0x75, 5, 0x95, 1, 0x81, 1, 5, 1, 9, 0x30, 9, 0x31
        ];
        if wheel {
            b.extend_from_slice(&[9, 0x38]);
        }
        b.extend_from_slice(&[
            0x15,
            0x81,
            0x25,
            0x7f,
            0x75,
            8,
            0x95,
            if wheel { 3 } else { 2 },
            0x81,
            6,
            0xc0,
            0xc0,
        ]);
        b
    }
    #[test]
    fn wheel_and_boot_reports_are_distinct() {
        for wheel in [false, true] {
            let d = descriptor(wheel);
            let l = layout(&d).unwrap();
            assert_eq!(l.wheel, wheel);
            let b = [5, 255, 17, 254];
            let (v, buttons) = decode(l, &b[..l.bytes]).unwrap();
            assert_eq!((v, buttons), ([-1, 17, if wheel { -2 } else { 0 }], 5));
            assert!(decode(l, &b[..2]).is_none());
        }
    }
    #[test]
    fn absolute_reportid_truncation_and_unknown_are_refused() {
        let d = descriptor(true);
        for n in 0..d.len() {
            assert!(layout(&d[..n]).is_none());
        }
        let mut bad = d.clone();
        let i = bad.windows(2).position(|p| p == [0x81, 6]).unwrap();
        bad[i + 1] = 2;
        assert!(layout(&bad).is_none());
        bad = d.clone();
        bad.splice(0..0, [0x85, 1]);
        assert!(layout(&bad).is_none());
        bad = d.clone();
        bad.push(0xfe);
        assert!(layout(&bad).is_none());
    }
}
