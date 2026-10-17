// Copyright 2026 Hirofumi Iwasaki
// SPDX-License-Identifier: Apache-2.0
//! Strict primary alternate-zero Bluetooth USB interface selection; no DMA.
use super::Error;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Endpoint {
    pub address: u8,
    pub max_packet: u16,
    pub interval: u8,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Transport {
    pub interface: u8,
    pub event: Endpoint,
    pub acl_in: Endpoint,
    pub acl_out: Endpoint,
}
/// Full configuration bytes, no UART H4 prefix, no SCO/ISO support. Composite
/// The first complete E0/01/01 alternate-zero HCI interface wins. Later
/// interfaces/alternates (including SCO) are not activated; descriptor framing
/// remains validated. An incomplete first HCI interface is an error.
pub fn select(bytes: &[u8]) -> Result<Transport, Error> {
    if bytes.len() < 9
        || bytes[0] != 9
        || bytes[1] != 2
        || u16::from_le_bytes([bytes[2], bytes[3]]) as usize != bytes.len()
    {
        return Err(Error::Length);
    }
    let mut selected = None;
    let mut active = false;
    let mut endpoints = [None; 3];
    let mut interface = 0;
    let mut at = 9;
    fn finish(interface: u8, endpoints: [Option<Endpoint>; 3]) -> Result<Transport, Error> {
        Ok(Transport {
            interface,
            event: endpoints[0].ok_or(Error::Invalid)?,
            acl_in: endpoints[1].ok_or(Error::Invalid)?,
            acl_out: endpoints[2].ok_or(Error::Invalid)?,
        })
    }
    while at < bytes.len() {
        if bytes.len() - at < 2 {
            return Err(Error::Length);
        }
        let n = bytes[at] as usize;
        if n < 2 || n > bytes.len() - at {
            return Err(Error::Length);
        }
        let d = &bytes[at..at + n];
        match d[1] {
            4 => {
                if n != 9 {
                    return Err(Error::Length);
                }
                if active {
                    selected = Some(finish(interface, endpoints)?);
                }
                // Finalize the primary transport before considering a later
                // interface. SCO alternate zero may declare zero-sized isochronous
                // endpoints: it is neither a second HCI candidate nor activated.
                active = selected.is_none() && d[3] == 0 && d[5..8] == [0xe0, 1, 1];
                if active {
                    if d[4] != 3 {
                        return Err(Error::Invalid);
                    }
                    interface = d[2];
                    endpoints = [None; 3];
                }
            }
            5 if active => {
                if n != 7 {
                    return Err(Error::Length);
                }
                let address = d[2];
                if address & 0x70 != 0 || address & 15 == 0 {
                    return Err(Error::Invalid);
                }
                let kind = match (d[3], address & 0x80 != 0) {
                    (3, true) => 0,
                    (2, true) => 1,
                    (2, false) => 2,
                    _ => return Err(Error::Unsupported),
                };
                let size = u16::from_le_bytes([d[4], d[5]]);
                if size == 0
                    || size > if kind == 0 { 64 } else { 512 }
                    || kind == 0 && d[6] == 0
                    || endpoints[kind].is_some()
                    || endpoints.iter().flatten().any(|e| e.address == address)
                {
                    return Err(Error::Invalid);
                }
                endpoints[kind] = Some(Endpoint {
                    address,
                    max_packet: size,
                    interval: d[6],
                });
            }
            _ => {}
        }
        at += n;
    }
    if active {
        selected = Some(finish(interface, endpoints)?);
    }
    selected.ok_or(Error::Unsupported)
}

impl Transport {
    /// Validate one HCI command and build only its USB Setup bytes. The actual
    /// control transfer must retain the command/DMA loan through completion.
    pub fn command_setup(&self, command: &[u8], composite: bool) -> Result<[u8; 8], Error> {
        if command.len() < 3 || command.len() != 3 + command[2] as usize {
            return Err(Error::Length);
        }
        let size = command.len() as u16;
        Ok([
            if composite { 0x21 } else { 0x20 },
            0,
            0,
            0,
            if composite { self.interface } else { 0 },
            0,
            size as u8,
            (size >> 8) as u8,
        ])
    }
}
