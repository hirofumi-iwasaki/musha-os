// Copyright 2026 Hirofumi Iwasaki
// SPDX-License-Identifier: Apache-2.0
//! Standard, parameter-free HCI identification commands only. No vendor opcode.
use super::{Error, Event};
pub const COMMANDS: [u16; 5] = [0x0c03, 0x1001, 0x1003, 0x2003, 0x1009];
#[derive(Debug, PartialEq, Eq)]
pub struct Reply<'a> {
    pub credits: u8,
    pub opcode: u16,
    pub status: u8,
    pub data: &'a [u8],
}
pub fn reply(expected: u16, packet: &[u8]) -> Result<Reply<'_>, Error> {
    if !COMMANDS.contains(&expected) {
        return Err(Error::Unsupported);
    }
    let (credits, opcode, result) = Event::parse(packet)?.complete()?;
    if opcode != expected || result.is_empty() {
        return Err(Error::Invalid);
    }
    let status = result[0];
    let size = match expected {
        0x0c03 => 1,
        0x1001 => 9,
        0x1003 | 0x2003 => 9,
        0x1009 => 7,
        _ => unreachable!(),
    };
    if (status == 0 && result.len() != size)
        || (status != 0 && result.len() != 1 && result.len() != size)
    {
        return Err(Error::Length);
    }
    Ok(Reply {
        credits,
        opcode,
        status,
        data: if status == 0 { &result[1..] } else { &[] },
    })
}
