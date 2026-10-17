// Copyright 2026 Hirofumi Iwasaki
// SPDX-License-Identifier: Apache-2.0
//! LE host groundwork only. No hardware access, SMP crypto, pairing or HOGP yet.
//! USB carries HCI bytes without UART/H4 packet indicators. Caller assembles one
//! complete HCI packet across USB transactions before passing a borrowed slice.
#![forbid(unsafe_code)]
pub mod acl;
pub mod att;
#[cfg(feature = "bluetooth-legacy")]
pub mod bonding;
#[cfg(feature = "bluetooth-security")]
pub mod entropy;
#[cfg(feature = "bluetooth-legacy")]
pub mod hogp;
#[cfg(feature = "bluetooth-legacy")]
pub mod legacy;
pub mod lifecycle;
pub mod probe;
#[cfg(feature = "bluetooth-security")]
pub mod reconnect;
#[cfg(feature = "bluetooth-security")]
pub mod sc;
#[cfg(feature = "bluetooth-security")]
pub mod sc_crypto;
pub mod scan;
pub mod stream;
pub mod usb;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Length,
    Unsupported,
    Invalid,
    Busy,
    Stale,
    Timeout,
    Controller(u8),
    Entropy,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Event<'a> {
    pub code: u8,
    pub parameters: &'a [u8],
}
impl<'a> Event<'a> {
    pub fn parse(bytes: &'a [u8]) -> Result<Self, Error> {
        if bytes.len() < 2 || bytes.len() != 2 + bytes[1] as usize {
            return Err(Error::Length);
        }
        Ok(Self {
            code: bytes[0],
            parameters: &bytes[2..],
        })
    }
    pub fn complete(self) -> Result<(u8, u16, &'a [u8]), Error> {
        if self.code != 0x0e {
            return Err(Error::Unsupported);
        }
        if self.parameters.len() < 3 {
            return Err(Error::Length);
        }
        Ok((
            self.parameters[0],
            word(&self.parameters[1..3]),
            &self.parameters[3..],
        ))
    }
}
fn word(b: &[u8]) -> u16 {
    u16::from_le_bytes([b[0], b[1]])
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Advertisement<'a> {
    pub event_type: u8,
    pub address_type: u8,
    /// HCI byte order; not assumed to be a stable bonded identity.
    pub address: [u8; 6],
    pub data: &'a [u8],
    /// None is the specified unavailable sentinel 127.
    pub rssi: Option<i8>,
}
fn advertisement(bytes: &[u8]) -> Result<(Advertisement<'_>, usize), Error> {
    if bytes.len() < 10 {
        return Err(Error::Length);
    }
    let size = bytes[8] as usize;
    if size > 31 || bytes.len() < 10 + size {
        return Err(Error::Length);
    }
    if bytes[0] > 4 || bytes[1] > 1 {
        return Err(Error::Unsupported);
    }
    let rssi = bytes[9 + size] as i8;
    if rssi != 127 && !(-127..=20).contains(&rssi) {
        return Err(Error::Invalid);
    }
    Ok((
        Advertisement {
            event_type: bytes[0],
            address_type: bytes[1],
            address: bytes[2..8].try_into().unwrap(),
            data: &bytes[9..9 + size],
            rssi: (rssi != 127).then_some(rssi),
        },
        10 + size,
    ))
}
/// Validate the whole multi-report event before exposing any reports. Unsupported
/// extended/anonymous formats fail explicitly; this is legacy LE advertising.
pub fn advertisements(
    event: Event<'_>,
    mut emit: impl FnMut(Advertisement<'_>),
) -> Result<(), Error> {
    if event.code != 0x3e || event.parameters.first() != Some(&2) {
        return Err(Error::Unsupported);
    }
    let count = *event.parameters.get(1).ok_or(Error::Length)? as usize;
    if count == 0 {
        return Err(Error::Invalid);
    }
    let reports = &event.parameters[2..];
    let mut at = 0;
    for _ in 0..count {
        at += advertisement(&reports[at..])?.1;
    }
    if at != reports.len() {
        return Err(Error::Length);
    }
    at = 0;
    for _ in 0..count {
        let (item, n) = advertisement(&reports[at..])?;
        emit(item);
        at += n;
    }
    Ok(())
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AdField<'a> {
    pub kind: u8,
    pub value: &'a [u8],
}
/// Length-delimited advertising data; a zero terminator permits only zero padding.
pub fn ad_fields(bytes: &[u8], mut emit: impl FnMut(AdField<'_>)) -> Result<(), Error> {
    fn validate(bytes: &[u8]) -> Result<(), Error> {
        let mut at = 0;
        while at < bytes.len() {
            let n = bytes[at] as usize;
            if n == 0 {
                return if bytes[at..].iter().all(|b| *b == 0) {
                    Ok(())
                } else {
                    Err(Error::Invalid)
                };
            }
            if n > bytes.len() - at - 1 {
                return Err(Error::Length);
            }
            at += 1 + n;
        }
        Ok(())
    }
    if bytes.len() > 31 {
        return Err(Error::Length);
    }
    validate(bytes)?;
    let mut at = 0;
    while at < bytes.len() && bytes[at] != 0 {
        let end = at + 1 + bytes[at] as usize;
        emit(AdField {
            kind: bytes[at + 1],
            value: &bytes[at + 2..end],
        });
        at = end;
    }
    Ok(())
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Acl<'a> {
    pub handle: u16,
    pub boundary: u8,
    pub payload: &'a [u8],
}
impl<'a> Acl<'a> {
    pub fn parse(bytes: &'a [u8]) -> Result<Self, Error> {
        if bytes.len() < 4 || bytes.len() != 4 + word(&bytes[2..4]) as usize {
            return Err(Error::Length);
        }
        let bits = word(bytes);
        if bits & 0xc000 != 0 || bits & 0x0fff > 0x0eff {
            return Err(Error::Invalid);
        }
        let boundary = ((bits >> 12) & 3) as u8;
        if boundary == 3 {
            return Err(Error::Unsupported);
        }
        Ok(Self {
            handle: bits & 0x0fff,
            boundary,
            payload: &bytes[4..],
        })
    }
    /// Only complete first-fragment LE fixed-channel PDUs. Reassembly, ACL credit
    /// management and connection authorization must precede runtime delivery.
    pub fn fixed_channel(self) -> Result<(u16, &'a [u8]), Error> {
        if self.boundary == 1 {
            return Err(Error::Unsupported);
        }
        if self.payload.len() < 4 || self.payload.len() != 4 + word(self.payload) as usize {
            return Err(Error::Length);
        }
        let channel = word(&self.payload[2..4]);
        if channel != 4 && channel != 6 {
            return Err(Error::Unsupported);
        }
        Ok((channel, &self.payload[4..]))
    }
}
