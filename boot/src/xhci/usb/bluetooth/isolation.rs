// Copyright 2026 Hirofumi Iwasaki
// SPDX-License-Identifier: Apache-2.0
//! Shared production/host-fixture failure boundary. No buffer reclamation API.
pub trait SlotStop {
    fn retire(&mut self, slot: u8) -> Result<(), &'static str>;
    fn disable(&mut self, slot: u8) -> Result<(), &'static str>;
    fn detach(&mut self, slot: u8);
}
pub fn disable(io: &mut impl SlotStop, slot: u8) -> Result<(), &'static str> {
    io.retire(slot)?;
    io.disable(slot).map_err(|_| "BT ISOLATION FAILED")?;
    io.detach(slot);
    Ok(())
}
pub fn local_failure(reason: &str) -> bool {
    // Controller/ring/clock faults are never converted into optional BT failure.
    (reason.starts_with("BT ") && !matches!(reason, "BT CONTEXT COMMAND" | "BT REPAIR REQUEST"))
        || matches!(reason, "TRANSFER TIMEOUT" | "SHORT CONTROL WRITE")
}
pub fn remaining(deadline: u64, now: u64) -> Result<u64, &'static str> {
    deadline
        .checked_sub(now)
        .filter(|n| *n != 0)
        .map(|n| n.min(1000))
        .ok_or("BT INITIALIZATION TIMEOUT")
}

/// Metadata only. Command parameters/return payloads may contain secrets.
pub fn command_event(
    bytes: &[u8],
) -> Result<Option<(&'static str, u8, u16, Option<u8>)>, &'static str> {
    let e = musha_platform::bluetooth::Event::parse(bytes).map_err(|_| "BT EVENT FORMAT")?;
    match e.code {
        0x0e => {
            let (credits, opcode, result) = e.complete().map_err(|_| "BT COMMAND COMPLETE")?;
            if result.is_empty() && opcode != 0 {
                return Err("BT COMMAND STATUS MISSING");
            }
            Ok(Some(("CC", credits, opcode, result.first().copied())))
        }
        0x0f if e.parameters.len() == 4 => Ok(Some((
            "CS",
            e.parameters[1],
            u16::from_le_bytes([e.parameters[2], e.parameters[3]]),
            Some(e.parameters[0]),
        ))),
        0x0f => Err("BT COMMAND STATUS"),
        _ => Ok(None),
    }
}
