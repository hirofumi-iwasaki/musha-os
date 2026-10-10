// SPDX-License-Identifier: Apache-2.0
//! Checked USB hub descriptors and topology. No hardware access or driver claim.
use crate::Speed;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Usb2,
    Super,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Descriptor {
    pub kind: Kind,
    pub ports: u8,
    pub power_good_ms: u16,
    pub power_switching: u8,
    pub tt_think_time: u8,
}
impl Descriptor {
    pub fn parse(bytes: &[u8], kind: Kind) -> Option<Self> {
        if bytes.len() < 7 || bytes[0] as usize != bytes.len() {
            return None;
        }
        let ports = bytes[2];
        if !(1..=15).contains(&ports) {
            return None;
        }
        let required = match kind {
            Kind::Usb2 => {
                if bytes[1] != 0x29 {
                    return None;
                }
                7 + 2 * (ports as usize + 1).div_ceil(8)
            }
            Kind::Super => {
                if bytes[1] != 0x2a {
                    return None;
                }
                12
            }
        };
        if bytes.len() != required {
            return None;
        }
        let flags = u16::from_le_bytes([bytes[3], bytes[4]]);
        let power_switching = (flags & 3) as u8;
        if power_switching == 3 || (kind == Kind::Super && power_switching >= 2) {
            return None;
        }
        Some(Self {
            kind,
            ports,
            power_good_ms: bytes[5] as u16 * 2,
            power_switching,
            tt_think_time: if kind == Kind::Usb2 {
                ((flags >> 5) & 3) as u8
            } else {
                0
            },
        })
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PortStatus {
    pub status: u16,
    pub changes: u16,
    pub kind: Kind,
}
impl PortStatus {
    pub fn parse(bytes: &[u8], kind: Kind) -> Option<Self> {
        if bytes.len() != 4 {
            return None;
        }
        Some(Self {
            status: u16::from_le_bytes([bytes[0], bytes[1]]),
            changes: u16::from_le_bytes([bytes[2], bytes[3]]),
            kind,
        })
    }
    pub fn connected(self) -> bool {
        self.status & 1 != 0
    }
    pub fn overcurrent(self) -> bool {
        self.status & 8 != 0
    }
    pub fn powered(self) -> bool {
        self.status
            & (if self.kind == Kind::Usb2 {
                1 << 8
            } else {
                1 << 9
            })
            != 0
    }
    pub fn ready_speed(self) -> Option<Speed> {
        if !self.connected()
            || self.status & 2 == 0
            || self.overcurrent()
            || self.status & 16 != 0
            || !self.powered()
        {
            return None;
        }
        match self.kind {
            Kind::Usb2 => match (self.status >> 9) & 3 {
                0 => Some(Speed::Full),
                1 => Some(Speed::Low),
                2 => Some(Speed::High),
                _ => None,
            },
            // Initial SS scope: U0 and the defined 5Gbps status encoding only.
            Kind::Super if !self.configuration_error() && self.status & (0x01e0 | 0x1c00) == 0 => {
                Some(Speed::Super)
            }
            Kind::Super => None,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Path {
    pub root_port: u8,
    pub route: u32,
    pub depth: u8,
    pub tt_slot: u8,
    pub tt_port: u8,
    pub multi_tt: bool,
}
impl Path {
    fn valid(self) -> bool {
        if self.depth > 5 || self.root_port == 0 {
            return false;
        }
        self.route >> (self.depth as u32 * 4) == 0
            && (0..self.depth).all(|tier| (self.route >> (tier as u32 * 4)) & 15 != 0)
            && self.tt_slot <= 8
            && self.tt_port <= 15
            && (self.tt_slot == 0) == (self.tt_port == 0)
            && (self.depth != 0 || self.tt_slot == 0)
            && (!self.multi_tt || self.tt_slot != 0)
    }
    pub fn root(port: u8) -> Option<Self> {
        if port == 0 {
            return None;
        }
        Some(Self {
            root_port: port,
            route: 0,
            depth: 0,
            tt_slot: 0,
            tt_port: 0,
            multi_tt: false,
        })
    }
    pub fn child(
        self,
        port: u8,
        parent_slot: u8,
        parent_speed: Speed,
        child_speed: Speed,
        multi_tt: bool,
    ) -> Option<Self> {
        if !self.valid()
            || self.depth >= 5
            || !(1..=15).contains(&port)
            || !(1..=8).contains(&parent_slot)
            || self.route >> (self.depth as u32 * 4) != 0
        {
            return None;
        }
        if (parent_speed == Speed::Super) != (child_speed == Speed::Super)
            || parent_speed == Speed::Low
        {
            return None;
        }
        if parent_speed == Speed::Full && child_speed == Speed::High {
            return None;
        }
        let mut result = Self {
            route: self.route | ((port as u32) << (self.depth as u32 * 4)),
            depth: self.depth + 1,
            ..self
        };
        if matches!(child_speed, Speed::Low | Speed::Full) && parent_speed == Speed::High {
            result.tt_slot = parent_slot;
            result.tt_port = port;
            result.multi_tt = multi_tt;
        } else if matches!(child_speed, Speed::High | Speed::Super) {
            result.tt_slot = 0;
            result.tt_port = 0;
            result.multi_tt = false;
        }
        Some(result)
    }
    pub fn slot_words(
        self,
        speed_id: u8,
        packet: u16,
        ring: usize,
    ) -> Option<([u32; 4], [u32; 5])> {
        if !self.valid() {
            return None;
        }
        let (mut slot, ep) = crate::initial_context(self.root_port, speed_id, packet, ring)?;
        slot[0] |= self.route | if self.multi_tt { 1 << 25 } else { 0 };
        slot[2] = self.tt_slot as u32 | ((self.tt_port as u32) << 8);
        Some((slot, ep))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn descriptor_bitmap_lengths_and_kind() {
        let d = [9, 0x29, 4, 0x21, 0, 10, 0, 0, 255];
        let h = Descriptor::parse(&d, Kind::Usb2).unwrap();
        assert_eq!((h.ports, h.power_good_ms, h.tt_think_time), (4, 20, 1));
        assert!(Descriptor::parse(&d[..8], Kind::Usb2).is_none());
        assert!(Descriptor::parse(&d, Kind::Super).is_none());
        let mut ss = [12, 0x2a, 15, 1, 0, 255, 0, 0, 0, 0, 0, 0];
        assert_eq!(
            Descriptor::parse(&ss, Kind::Super).unwrap().power_good_ms,
            510
        );
        ss[2] = 16;
        assert!(Descriptor::parse(&ss, Kind::Super).is_none());
        let mut wide = [11, 0x29, 8, 2, 0, 0, 0, 0, 0, 255, 255];
        assert!(Descriptor::parse(&wide, Kind::Usb2).is_some());
        wide[3] = 3;
        assert!(Descriptor::parse(&wide, Kind::Usb2).is_none());
    }
    #[test]
    fn status_speed_power_and_faults() {
        let p = PortStatus::parse(&[3, 5, 16, 0], Kind::Usb2).unwrap();
        assert_eq!(p.ready_speed(), Some(Speed::High));
        assert_eq!(p.changes, 16);
        let low = PortStatus::parse(&[3, 3, 0, 0], Kind::Usb2).unwrap();
        assert_eq!(low.ready_speed(), Some(Speed::Low));
        let ss = PortStatus::parse(&[3, 2, 0, 0], Kind::Super).unwrap();
        assert_eq!(ss.ready_speed(), Some(Speed::Super));
        assert_eq!(
            PortStatus {
                status: 0x20 | ss.status,
                ..ss
            }
            .ready_speed(),
            None
        );
        assert_eq!(
            PortStatus {
                status: 8 | p.status,
                ..p
            }
            .ready_speed(),
            None
        );
        assert_eq!(
            PortStatus {
                status: 16 | p.status,
                ..p
            }
            .ready_speed(),
            None
        );
        assert_eq!(PortStatus { status: 0x703, ..p }.ready_speed(), None);
        assert_eq!(PortStatus { status: 0x403, ..p }.ready_speed(), None);
        assert!(PortStatus::parse(&[0; 3], Kind::Usb2).is_none());
    }
    #[test]
    fn routes_and_tt_ancestry() {
        let root = Path::root(6).unwrap();
        let child = root.child(3, 1, Speed::High, Speed::Full, false).unwrap();
        let leaf = child.child(4, 2, Speed::Full, Speed::Low, false).unwrap();
        assert_eq!(
            (leaf.root_port, leaf.route, leaf.tt_slot, leaf.tt_port),
            (6, 0x43, 1, 3)
        );
        let (slot, _) = leaf.slot_words(2, 8, 4096).unwrap();
        assert_eq!(slot[2], 0x301);
        let mut path = root;
        for _ in 0..5 {
            path = path
                .child(15, 1, Speed::Super, Speed::Super, false)
                .unwrap();
        }
        assert_eq!(path.route, 0xfffff);
        assert!(
            path.child(1, 1, Speed::Super, Speed::Super, false)
                .is_none()
        );
        assert!(root.child(0, 1, Speed::High, Speed::Full, false).is_none());
        assert!(root.child(16, 1, Speed::High, Speed::Full, false).is_none());
        assert!(root.child(1, 1, Speed::Super, Speed::High, false).is_none());
        assert!(Path { route: 1, ..root }.slot_words(3, 64, 4096).is_none());
        assert!(
            Path {
                route: 1,
                depth: 2,
                ..root
            }
            .slot_words(3, 64, 4096)
            .is_none()
        );
    }
}
/// Alternate-zero hub interface and its unarmed status endpoint.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Configuration {
    pub kind: Kind,
    pub value: u8,
    pub endpoint: u8,
    pub packet: u16,
    pub interval: u8,
}
pub fn configuration(bytes: &[u8], speed: Speed) -> Result<Option<Configuration>, &'static str> {
    if bytes.len() < 9
        || bytes.len() > 1024
        || bytes[0] != 9
        || bytes[1] != 2
        || u16::from_le_bytes([bytes[2], bytes[3]]) as usize != bytes.len()
    {
        return Err("HUB CONFIGURATION");
    }
    let mut offset = 9;
    let mut active = false;
    let mut count = 0;
    let mut hub = false;
    let mut endpoint = None;
    while offset < bytes.len() {
        if offset + 2 > bytes.len() {
            return Err("HUB CONFIGURATION");
        }
        let n = bytes[offset] as usize;
        if n < 2 || offset + n > bytes.len() {
            return Err("HUB CONFIGURATION");
        }
        let d = &bytes[offset..offset + n];
        match d[1] {
            4 => {
                if n != 9 {
                    return Err("HUB INTERFACE");
                }
                active = false;
                if d[3] == 0 {
                    count += 1;
                    if d[5] == 9 {
                        if !matches!(speed, Speed::Full | Speed::High | Speed::Super)
                            || d[6] != 0
                            || (if speed == Speed::Super {
                                d[7] != 0
                            } else {
                                d[7] > 1
                            })
                            || d[4] != 1
                            || hub
                        {
                            return Err("HUB PROTOCOL");
                        }
                        active = true;
                        hub = true;
                    }
                }
            }
            5 if active => {
                if n != 7
                    || endpoint.is_some()
                    || d[2] & 0xf0 != 0x80
                    || d[2] & 15 == 0
                    || !(d[3] == 3 || (speed == Speed::Super && d[3] == 0x13))
                {
                    return Err("HUB ENDPOINT");
                }
                let packet = u16::from_le_bytes([d[4], d[5]]);
                let interval = match speed {
                    Speed::Full if d[6] != 0 => 3 + (7 - d[6].leading_zeros()) as u8,
                    Speed::High | Speed::Super if (1..=16).contains(&d[6]) => d[6] - 1,
                    _ => return Err("HUB INTERVAL"),
                };
                if packet == 0 || packet > if speed == Speed::Full { 64 } else { 1024 } {
                    return Err("HUB PACKET");
                }
                if speed == Speed::Super {
                    let next = offset + n;
                    if packet != 2
                        || next + 6 > bytes.len()
                        || bytes[next..next + 6] != [6, 48, 0, 0, 2, 0]
                        || (d[3] == 0x13 && d[6] < 8)
                    {
                        return Err("HUB SS COMPANION");
                    }
                }
                endpoint = Some(Configuration {
                    kind: if speed == Speed::Super {
                        Kind::Super
                    } else {
                        Kind::Usb2
                    },
                    value: bytes[5],
                    endpoint: d[2] & 15,
                    packet,
                    interval,
                });
            }
            _ => {}
        }
        offset += n;
    }
    if !hub {
        return Ok(None);
    }
    if count != 1 || bytes[4] != 1 || bytes[5] == 0 {
        return Err("HUB CONFIGURATION");
    }
    endpoint.map(Some).ok_or("HUB ENDPOINT")
}
#[cfg(test)]
mod configuration_tests {
    use super::*;
    #[test]
    fn usb2_single_interface_and_alternate_policy() {
        let mut c = [
            9, 2, 25, 0, 1, 1, 0, 0x80, 0, 9, 4, 0, 0, 1, 9, 0, 1, 0, 7, 5, 0x81, 3, 2, 0, 255,
        ];
        assert_eq!(
            configuration(&c, Speed::Full),
            Ok(Some(Configuration {
                kind: Kind::Usb2,
                value: 1,
                endpoint: 1,
                packet: 2,
                interval: 10
            }))
        );
        assert!(configuration(&c, Speed::Super).is_err());
        c[16] = 2;
        assert!(configuration(&c, Speed::High).is_err());
        c[16] = 1;
        c[12] = 1;
        assert_eq!(configuration(&c, Speed::Full), Ok(None));
        c[12] = 0;
        c[9] = 10;
        assert!(configuration(&c, Speed::Full).is_err());
    }
    #[test]
    fn endpoints_and_truncation() {
        let mut c = [
            9, 2, 25, 0, 1, 1, 0, 0x80, 0, 9, 4, 0, 0, 1, 9, 0, 1, 0, 7, 5, 0x81, 3, 2, 0, 10,
        ];
        for n in 0..c.len() {
            assert!(configuration(&c[..n], Speed::Full).is_err());
        }
        c[20] = 0x01;
        assert!(configuration(&c, Speed::Full).is_err());
        c[20] = 0x81;
        c[24] = 0;
        assert!(configuration(&c, Speed::Full).is_err());
        c[24] = 10;
        c[22] = 0;
        assert!(configuration(&c, Speed::Full).is_err());
    }
}
impl Kind {
    pub fn descriptor_type(self) -> u8 {
        match self {
            Self::Usb2 => 0x29,
            Self::Super => 0x2a,
        }
    }
    /// Depth is a hub's own route depth, not its child's or root-port number.
    pub fn depth_request(self, depth: u8) -> Option<u32> {
        if self == Self::Super && depth <= 4 {
            Some(((depth as u32) << 16) | 0x0c20)
        } else {
            None
        }
    }
    pub fn reset_feature(self) -> u16 {
        if self == Self::Super { 28 } else { 4 }
    }
}
impl PortStatus {
    /// Only defined change bits map to ClearFeature selectors.
    pub fn change_features(self) -> [Option<u16>; 8] {
        let selectors = match self.kind {
            Kind::Usb2 => [
                Some(16),
                Some(17),
                Some(18),
                Some(19),
                Some(20),
                None,
                None,
                None,
            ],
            Kind::Super => [
                Some(16),
                None,
                None,
                Some(19),
                Some(20),
                Some(29),
                Some(25),
                Some(26),
            ],
        };
        core::array::from_fn(|i| {
            if self.changes & (1 << i) != 0 {
                selectors[i]
            } else {
                None
            }
        })
    }
    pub fn reset_complete(self) -> bool {
        let mask = if self.kind == Kind::Super { 0x30 } else { 0x10 };
        self.changes & mask == mask && self.ready_speed().is_some()
    }
    pub fn configuration_error(self) -> bool {
        self.kind == Kind::Super && self.changes & 0x80 != 0
    }
    pub fn connection_changed(self) -> bool {
        self.changes & if self.kind == Kind::Super { 1 } else { 3 } != 0
    }
}
/// Initial SS scope avoids assuming a USB3.1 extended-status speed/rank.
pub fn device_supported(bytes: &[u8], speed: Speed) -> bool {
    bytes.len() == 18
        && bytes[0] == 18
        && bytes[1] == 1
        && bytes[4] == 9
        && bytes[5] == 0
        && match speed {
            Speed::Super => bytes[2..4] == [0, 3] && bytes[6] == 3,
            Speed::High => bytes[6] <= 2,
            Speed::Full => bytes[6] == 0,
            Speed::Low => false,
        }
}
#[cfg(test)]
mod ss_policy_tests {
    use super::*;
    #[test]
    fn depth_and_change_requests_are_protocol_specific() {
        for depth in 0..=4 {
            assert_eq!(
                Kind::Super.depth_request(depth),
                Some(((depth as u32) << 16) | 0x0c20)
            );
        }
        assert_eq!(Kind::Super.depth_request(5), None);
        assert_eq!(Kind::Usb2.depth_request(0), None);
        assert_eq!(Kind::Super.descriptor_type(), 0x2a);
        assert_eq!(Kind::Super.reset_feature(), 28);
        assert_eq!(Kind::Usb2.reset_feature(), 4);
        let ss = PortStatus {
            status: 0x203,
            changes: 0xffff,
            kind: Kind::Super,
        };
        assert_eq!(
            ss.change_features(),
            [
                Some(16),
                None,
                None,
                Some(19),
                Some(20),
                Some(29),
                Some(25),
                Some(26)
            ]
        );
        assert!(ss.configuration_error());
        let usb2 = PortStatus {
            kind: Kind::Usb2,
            ..ss
        };
        assert_eq!(
            usb2.change_features(),
            [
                Some(16),
                Some(17),
                Some(18),
                Some(19),
                Some(20),
                None,
                None,
                None
            ]
        );
    }
    #[test]
    fn warm_reset_requires_both_changes_and_u0() {
        let mut ss = PortStatus {
            status: 0x203,
            changes: 0x30,
            kind: Kind::Super,
        };
        assert!(ss.reset_complete());
        for changes in [0, 0x10, 0x20] {
            ss.changes = changes;
            assert!(!ss.reset_complete());
        }
        ss.changes = 0x30;
        for status in [0x103, 0x213, 0x202, 0x201, 0x20b, 0x603, 0x2c3] {
            ss.status = status;
            assert!(!ss.reset_complete());
        }
        ss.status = 0x203;
        ss.changes = 0x80;
        assert!(ss.configuration_error());
        ss.changes = 2;
        assert!(!ss.connection_changed());
        ss.kind = Kind::Usb2;
        assert!(ss.connection_changed());
    }
    #[test]
    fn ss_device_generation_is_not_guessed() {
        let mut d = [18, 1, 0, 3, 9, 0, 3, 9, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1];
        assert!(device_supported(&d, Speed::Super));
        d[2] = 0x10;
        assert!(!device_supported(&d, Speed::Super));
        d[2] = 0;
        d[6] = 0;
        assert!(!device_supported(&d, Speed::Super));
        assert!(device_supported(&d, Speed::Full));
        assert!(!device_supported(&d[..17], Speed::Full));
    }
}
#[cfg(test)]
mod ss_configuration_tests {
    use super::*;
    fn fixture() -> [u8; 31] {
        [
            9, 2, 31, 0, 1, 1, 0, 0xc0, 0, 9, 4, 0, 0, 1, 9, 0, 0, 0, 7, 5, 0x81, 0x13, 2, 0, 8, 6,
            48, 0, 0, 2, 0,
        ]
    }
    #[test]
    fn ss_notification_and_companion_are_checked() {
        let mut c = fixture();
        assert_eq!(
            configuration(&c, Speed::Super),
            Ok(Some(Configuration {
                kind: Kind::Super,
                value: 1,
                endpoint: 1,
                packet: 2,
                interval: 7
            }))
        );
        c[21] = 3;
        c[24] = 1;
        assert!(configuration(&c, Speed::Super).is_ok());
        c[21] = 0x13;
        assert!(configuration(&c, Speed::Super).is_err());
        c[24] = 8;
        for (field, value) in [
            (16, 1),
            (20, 1),
            (21, 0x23),
            (22, 1),
            (24, 17),
            (25, 5),
            (26, 0),
            (27, 1),
            (28, 1),
            (29, 1),
        ] {
            let mut bad = c;
            bad[field] = value;
            assert!(configuration(&bad, Speed::Super).is_err(), "field {field}");
        }
        for n in 0..c.len() {
            assert!(configuration(&c[..n], Speed::Super).is_err());
        }
        let mut short = [0; 25];
        short.copy_from_slice(&c[..25]);
        short[2] = 25;
        assert!(configuration(&short, Speed::Super).is_err());
    }
    #[test]
    fn ss_descriptor_power_modes_and_types() {
        let mut d = [12, 0x2a, 4, 1, 0, 5, 0, 0, 0, 0, 0, 0];
        assert!(Descriptor::parse(&d, Kind::Super).is_some());
        d[3] = 2;
        assert!(Descriptor::parse(&d, Kind::Super).is_none());
        d[3] = 3;
        assert!(Descriptor::parse(&d, Kind::Super).is_none());
        d[3] = 0;
        d[1] = 0x29;
        assert!(Descriptor::parse(&d, Kind::Super).is_none());
    }
}
