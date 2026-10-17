// Copyright 2026 Hirofumi Iwasaki
// SPDX-License-Identifier: Apache-2.0
//! Exact xHCI context words shared by interrupt-only probe and live HCI.
use super::{Error, usb::Transport};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Endpoint {
    pub dci: u8,
    pub words: [u32; 5],
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Plan {
    pub endpoints: [Endpoint; 3],
    pub count: usize,
    pub add_flags: u32,
    pub context_entries: u8,
}
pub fn plan(t: Transport, interval: u8, rings: [u64; 3], event_only: bool) -> Result<Plan, Error> {
    let descriptors = [t.event, t.acl_in, t.acl_out];
    let mut endpoints = [Endpoint {
        dci: 0,
        words: [0; 5],
    }; 3];
    for i in 0..3 {
        if rings[i] & 15 != 0 {
            return Err(Error::Invalid);
        }
        let e = descriptors[i];
        let incoming = e.address & 0x80 != 0;
        if e.address & 15 == 0 || e.address & 0x70 != 0 || e.max_packet == 0 || incoming != (i != 2)
        {
            return Err(Error::Invalid);
        }
        endpoints[i] = Endpoint {
            dci: (e.address & 15) * 2 + u8::from(incoming),
            words: [
                if i == 0 { (interval as u32) << 16 } else { 0 },
                (3 << 1)
                    | ((if i == 0 {
                        7
                    } else if i == 1 {
                        6
                    } else {
                        2
                    }) << 3)
                    | ((e.max_packet as u32) << 16),
                rings[i] as u32 | 1,
                (rings[i] >> 32) as u32,
                if i == 0 { 257 | (257 << 16) } else { 512 },
            ],
        };
    }
    let count = if event_only { 1 } else { 3 };
    let active = &endpoints[..count];
    Ok(Plan {
        endpoints,
        count,
        add_flags: active.iter().fold(1, |f, e| f | (1 << e.dci)),
        context_entries: active.iter().map(|e| e.dci).max().ok_or(Error::Invalid)?,
    })
}
