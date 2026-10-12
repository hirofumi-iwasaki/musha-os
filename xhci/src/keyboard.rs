// Copyright 2026 Hirofumi Iwasaki
// SPDX-License-Identifier: Apache-2.0
//! Bounded USB configuration parsing and HID boot report state.
use crate::Speed;
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Keyboard {
    pub configuration: u8,
    pub interface: u8,
    pub endpoint: u8,
    pub packet: u16,
    pub interval: u8,
}
pub fn configuration(b: &[u8], speed: Speed) -> Result<Option<Keyboard>, &'static str> {
    if b.len() < 9
        || b.len() > 1024
        || b[0] != 9
        || b[1] != 2
        || u16::from_le_bytes([b[2], b[3]]) as usize != b.len()
        || b[5] == 0
    {
        return Err("CONFIG DESCRIPTOR");
    }
    let mut offset = 9;
    let mut active = None;
    let mut found = None;
    let mut interface_endpoint = false;
    let mut hid = false;
    let mut endpoints = 0u8;
    let mut expected = 0u8;
    while offset < b.len() {
        if offset + 2 > b.len() {
            return Err("CONFIG TRUNCATED");
        }
        let length = b[offset] as usize;
        if length < 2 || offset + length > b.len() {
            return Err("CONFIG LENGTH");
        }
        let d = &b[offset..offset + length];
        match d[1] {
            4 => {
                if active.is_some() && (endpoints != expected || !hid) {
                    return Err("HID INTERFACE");
                }
                if length != 9 {
                    return Err("INTERFACE LENGTH");
                }
                interface_endpoint = false;
                active = if d[3] == 0 && d[5..8] == [3, 1, 1] {
                    Some(d[2])
                } else {
                    None
                };
                hid = false;
                endpoints = 0;
                expected = d[4];
            }
            0x21 if active.is_some() => {
                if length < 9
                    || d[5] == 0
                    || length != 6 + d[5] as usize * 3
                    || d[6] != 0x22
                    || u16::from_le_bytes([d[7], d[8]]) == 0
                    || hid
                {
                    return Err("HID DESCRIPTOR");
                }
                hid = true;
            }
            5 if active.is_some() => {
                if length != 7 {
                    return Err("ENDPOINT LENGTH");
                }
                endpoints = endpoints.checked_add(1).ok_or("ENDPOINT COUNT")?;
                if d[2] & 0x80 != 0 && d[3] & 3 == 3 {
                    if interface_endpoint || d[2] & 0x70 != 0 || d[2] & 15 == 0 {
                        return Err("KEYBOARD ENDPOINT");
                    }
                    let packet = u16::from_le_bytes([d[4], d[5]]);
                    let interval = match speed {
                        Speed::Low | Speed::Full if d[6] != 0 => {
                            3 + (7 - d[6].leading_zeros()) as u8
                        }
                        Speed::High if (1..=16).contains(&d[6]) => d[6] - 1,
                        _ => return Err("KEYBOARD SPEED"),
                    };
                    let max = match speed {
                        Speed::Low => 8,
                        Speed::Full => 64,
                        _ => 1024,
                    };
                    if packet < 8 || packet > max {
                        return Err("KEYBOARD PACKET");
                    }
                    interface_endpoint = true;
                    let candidate = Keyboard {
                        configuration: b[5],
                        interface: active.unwrap(),
                        endpoint: d[2] & 15,
                        packet,
                        interval,
                    };
                    if found.is_none() {
                        found = Some(candidate);
                    }
                }
            }
            _ => {}
        }
        offset += length;
    }
    if active.is_some() && (endpoints != expected || !hid) {
        return Err("HID INTERFACE");
    }
    Ok(found)
}
// Compatibility export: existing xHCI callers keep the same interface.
pub use musha_input::BootKeyboardState as State;
#[cfg(test)]
mod tests {
    extern crate std;
    use super::*;
    fn fixture() -> [u8; 34] {
        [
            9, 2, 34, 0, 1, 1, 0, 0x80, 50, 9, 4, 0, 0, 1, 3, 1, 1, 0, 9, 0x21, 0x11, 1, 0, 1,
            0x22, 63, 0, 7, 5, 0x81, 3, 8, 0, 10,
        ]
    }
    #[test]
    fn malformed_configuration_is_bounded() {
        let mut b = fixture();
        assert_eq!(configuration(&b, Speed::Full).unwrap().unwrap().interval, 6);
        for n in 0..b.len() {
            assert!(configuration(&b[..n], Speed::Full).is_err());
        }
        b[18] = 0;
        assert!(configuration(&b, Speed::Full).is_err());
        b = fixture();
        b[31] = 7;
        assert!(configuration(&b, Speed::Full).is_err());
        b = fixture();
        b[16] = 2;
        assert_eq!(configuration(&b, Speed::Full), Ok(None));
    }
    #[test]
    fn composite_selects_first_but_validates_later_interfaces() {
        let mut b = std::vec::Vec::from(fixture());
        let mut second = fixture()[9..].to_vec();
        second[2] = 1;
        second[20] = 0x82;
        b.extend_from_slice(&second);
        let length = b.len() as u16;
        b[2..4].copy_from_slice(&length.to_le_bytes());
        b[4] = 2;
        assert_eq!(
            configuration(&b, Speed::Full).unwrap().unwrap().interface,
            0
        );
        let end = b.len();
        b[end - 3] = 7;
        assert!(configuration(&b, Speed::Full).is_err());
    }
    #[test]
    fn duplicate_endpoint_in_one_interface_is_rejected() {
        let mut b = std::vec::Vec::from(fixture());
        b.extend_from_slice(&fixture()[27..]);
        b[13] = 2;
        let length = b.len() as u16;
        b[2..4].copy_from_slice(&length.to_le_bytes());
        assert_eq!(configuration(&b, Speed::Full), Err("KEYBOARD ENDPOINT"));
    }
    #[test]
    fn reports_preserve_rollover_and_deduplicate_keys() {
        let mut state = State::default();
        let mut events = std::vec::Vec::new();
        state.update([2, 99, 4, 4, 0, 0, 0, 0], |k, d| events.push((k, d)));
        assert_eq!(events, [(0xe1, true), (4, true)]);
        events.clear();
        assert!(!state.update([0, 0, 1, 1, 1, 1, 1, 1], |k, d| events.push((k, d))));
        assert!(events.is_empty());
        state.update([0; 8], |k, d| events.push((k, d)));
        assert_eq!(events, [(0xe1, false), (4, false)]);
    }
    #[test]
    fn endpoint_id_and_residual_are_checked() {
        let w = [0x1000, 0, 1 << 24, (32 << 10) | (3 << 16) | (2 << 24) | 1];
        assert!(crate::endpoint_completion(w, 0x1000, 2, 3));
        assert!(!crate::endpoint_completion(w, 0x1000, 2, 1));
        let mut short = w;
        short[2] = (13 << 24) | 1;
        assert!(!crate::endpoint_completion(short, 0x1000, 2, 3));
    }
}
