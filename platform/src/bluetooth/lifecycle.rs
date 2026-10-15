// Copyright 2026 Hirofumi Iwasaki
// SPDX-License-Identifier: Apache-2.0
//! Recovery requests never authorize DMA reuse; the outer stopped-host boundary does.
use super::{Error, Event};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Loss {
    Hardware,
    Disconnected,
}
pub fn loss(event: Event<'_>, handle: u16) -> Result<Option<Loss>, Error> {
    match event.code {
        0x10 if event.parameters.len() == 1 => Ok(Some(Loss::Hardware)),
        0x10 => Err(Error::Length),
        5 => {
            let p = event.parameters;
            if p.len() != 4 {
                return Err(Error::Length);
            }
            if p[0] != 0 || handle > 0xeff || u16::from_le_bytes([p[1], p[2]]) != handle {
                return Err(Error::Stale);
            }
            Ok(Some(Loss::Disconnected))
        }
        _ => Ok(None),
    }
}
