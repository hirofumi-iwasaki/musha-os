// Copyright (c) 2026 Abdelkader Boudih <freebsd@seuros.com>
// Copyright (c) 2026 Hirofumi Iwasaki (Rust adaptation and model)
// SPDX-License-Identifier: BSD-2-Clause
//! Hardware-independent BCE subset. No MMIO, DMA memory, interrupts or I/O.
//! Wire layout provenance: third_party/freebsd-apple-bce/UPSTREAM.
//! Queue quarantine is a host policy; this model cannot stop real DMA.
#![no_std]
#![forbid(unsafe_code)]
pub mod mailbox;
pub mod queue;
pub mod wire;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Invalid,
    Busy,
    Timeout,
    ClockBackwards,
    PollLimit,
    UnexpectedReply,
    Stopped,
    QueueFull,
    QueueId,
    Slot,
    Duplicate,
    Length,
    Status,
    Flags,
    NotComplete,
    Ticket,
    SequenceOverflow,
}
