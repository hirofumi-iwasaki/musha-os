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

/// Pre-connection command wait. Payload is returned only for the correlated CC;
/// callers must never format it. No deadlines or command credits are modified.
pub enum WarmupEvent<'a> {
    Complete { credits: u8, result: &'a [u8] },
    Pending(&'static str),
    LateReset { credits: u8 },
}
pub fn warmup_event<'a>(
    bytes: &'a [u8],
    expected: u16,
    deadline: u64,
    now: u64,
    events: &mut u8,
) -> Result<WarmupEvent<'a>, &'static str> {
    if now >= deadline {
        return Err("BT ENTROPY COMMAND TIMEOUT");
    }
    *events = events.checked_add(1).ok_or("BT ENTROPY EVENT LIMIT")?;
    if *events > 64 {
        return Err("BT ENTROPY EVENT LIMIT");
    }
    let e = musha_platform::bluetooth::Event::parse(bytes).map_err(|_| "BT ENTROPY EVENT")?;
    match e.code {
        0x0e => {
            let (credits, opcode, result) = e.complete().map_err(|_| "BT ENTROPY COMPLETE")?;
            if credits == 0 {
                return Err("BT ENTROPY CREDIT");
            }
            if result.is_empty() && opcode != 0 {
                return Err("BT COMMAND STATUS MISSING");
            }
            if !command_result_shape(opcode, result) {
                return Err("BT COMMAND RETURN LENGTH");
            }
            if opcode != expected {
                if opcode == 0x0c03 {
                    if result != [0] {
                        return Err("BT LATE RESET REJECTED");
                    }
                    return Ok(WarmupEvent::LateReset { credits });
                }
                return Ok(WarmupEvent::Pending("unrelated-command-complete"));
            }
            Ok(WarmupEvent::Complete { credits, result })
        }
        0x0f => {
            let p = e.parameters;
            if p.len() != 4 {
                return Err("BT COMMAND STATUS");
            }
            if p[1] == 0 {
                return Err("BT ENTROPY CREDIT");
            }
            if u16::from_le_bytes([p[2], p[3]]) != expected {
                return Err("BT ENTROPY COMMAND STATUS OPCODE");
            }
            if p[0] != 0 {
                return Err("BT ENTROPY COMMAND STATUS REJECTED");
            }
            Ok(WarmupEvent::Pending("command-status-awaiting-complete"))
        }
        // No connection/ACL packet has been submitted in warm-up. Validate the
        // shared completed-packets format, but do not credit a future ACL queue.
        0x13 => {
            let p = e.parameters;
            if p.is_empty() || p[0] == 0 || p.len() != 1 + usize::from(p[0]) * 4 {
                return Err("BT COMPLETED PACKETS FORMAT");
            }
            for (i, b) in p[1..].chunks_exact(4).enumerate() {
                let handle = u16::from_le_bytes([b[0], b[1]]);
                if handle > 0x0eff
                    || p[1..1 + i * 4]
                        .chunks_exact(4)
                        .any(|old| u16::from_le_bytes([old[0], old[1]]) == handle)
                {
                    return Err("BT COMPLETED PACKETS HANDLE");
                }
            }
            Ok(WarmupEvent::Pending("preconnection-completed-packets"))
        }
        0x05 => {
            let p = e.parameters;
            if p.len() != 4 || u16::from_le_bytes([p[1], p[2]]) > 0x0eff {
                return Err("BT DISCONNECT FORMAT");
            }
            Ok(WarmupEvent::Pending("preconnection-disconnection"))
        }
        0x3e if e.parameters.first() == Some(&2) => {
            musha_platform::bluetooth::advertisements(e, |_| {})
                .map_err(|_| "BT ADVERTISING FORMAT")?;
            Ok(WarmupEvent::Pending("preconnection-advertising"))
        }
        0x10 => Err("BT CONTROLLER HARDWARE ERROR"),
        _ => Ok(WarmupEvent::Pending("unknown-wellformed-discard")),
    }
}

/// Pure production drain clock: no timeout clears a pending DMA loan.
pub struct Drain {
    start: u64,
    last: u64,
    count: u8,
}
impl Drain {
    pub fn new(now: u64) -> Self {
        Self {
            start: now,
            last: now,
            count: 0,
        }
    }
    pub fn event(&mut self, now: u64) -> Result<(), &'static str> {
        if now < self.last {
            return Err("BT DRAIN CLOCK");
        }
        self.count = self.count.checked_add(1).ok_or("BT DRAIN EVENT LIMIT")?;
        if self.count > 64 {
            return Err("BT DRAIN EVENT LIMIT");
        }
        self.last = now;
        Ok(())
    }
    pub fn finished(&self, now: u64) -> Result<bool, &'static str> {
        let elapsed = now.checked_sub(self.start).ok_or("BT DRAIN CLOCK")?;
        let quiet = now.checked_sub(self.last).ok_or("BT DRAIN CLOCK")?;
        Ok(elapsed >= 200 || (quiet >= 50 && (self.count == 0 || elapsed >= 100)))
    }
    pub fn count(&self) -> u8 {
        self.count
    }
}
pub fn retry_after_reset(
    expected: u16,
    deadline: u64,
    now: u64,
    used: &mut bool,
) -> Result<(), &'static str> {
    if now >= deadline {
        return Err("BT ENTROPY COMMAND TIMEOUT");
    }
    if *used || !matches!(expected, 0x1001 | 0x2018) {
        return Err("BT RESET RESYNC RETRY LIMIT");
    }
    *used = true;
    Ok(())
}

pub fn valid_version(result: &[u8]) -> bool {
    result.len() == 9 && result[0] == 0
}

pub fn command_result_shape(opcode: u16, result: &[u8]) -> bool {
    match opcode {
        0x0c03 => result.len() == 1,
        0x1001 | 0x2018 => result
            .first()
            .is_some_and(|status| result.len() == if *status == 0 { 9 } else { 1 }),
        _ => true,
    }
}
