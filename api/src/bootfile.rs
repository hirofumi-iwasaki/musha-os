// SPDX-License-Identifier: Apache-2.0
//! One-shot boot-file reader over the bounded stream service. No DMA or allocation.
use crate::stream::{Error, Files, Handle, MAX_READ};
pub const NAME: &[u8] = b"AUTORUN.ELF";
pub const MAX_BYTES: usize = 16 * 1024 * 1024;
pub const TIMEOUT_MS: u64 = 5000;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rejection {
    Size,
    Duplicate,
    Timeout,
    Stream(Error),
    Interrupted,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    Searching,
    Reading,
    Absent,
    Ready(usize),
    Rejected(Rejection),
}
pub struct Reader {
    state: State,
    handle: Option<Handle>,
    expected: usize,
    used: usize,
    started: Option<u64>,
}
impl Default for Reader {
    fn default() -> Self {
        Self::new()
    }
}
impl Reader {
    pub const fn new() -> Self {
        Self {
            state: State::Searching,
            handle: None,
            expected: 0,
            used: 0,
            started: None,
        }
    }
    /// A filesystem rejected the matching name before publishing its catalogue.
    pub fn duplicate(&mut self) {
        if self.state == State::Searching {
            self.state = State::Rejected(Rejection::Duplicate);
        }
    }
    pub fn state(&self) -> State {
        self.state
    }
    /// Terminal means loading claimed the boot; Absent instead permits the viewer.
    pub fn claimed(&self) -> bool {
        matches!(
            self.state,
            State::Reading | State::Ready(_) | State::Rejected(_)
        )
    }
    /// Called after the device session ends; does not itself quiesce hardware.
    pub fn interrupted(&mut self) {
        if self.claimed() {
            self.state = State::Rejected(Rejection::Interrupted);
        }
    }
    /// Keep the same output contents across calls; only Ready exposes a complete file.
    pub fn step(&mut self, files: &mut Files<'_>, output: &mut [u8], now: u64) -> State {
        let result = self.advance(files, output, now);
        if let Err(reason) = result {
            self.state = State::Rejected(reason);
        }
        if matches!(self.state, State::Ready(_) | State::Rejected(_)) {
            if let Some(h) = self.handle.take() {
                let _ = files.close(h);
            }
        }
        self.state
    }
    fn advance(
        &mut self,
        files: &mut Files<'_>,
        output: &mut [u8],
        now: u64,
    ) -> Result<(), Rejection> {
        if self.state == State::Searching {
            let mut cursor = files.list_begin().map_err(Rejection::Stream)?;
            let mut found = None;
            while let Some((id, entry)) = files.list_next(&mut cursor).map_err(Rejection::Stream)? {
                if entry.name_len as usize == NAME.len()
                    && entry.name[..NAME.len()]
                        .iter()
                        .zip(NAME)
                        .all(|(&a, &b)| a <= 127 && (a as u8).to_ascii_uppercase() == b)
                {
                    if found.is_some() {
                        return Err(Rejection::Duplicate);
                    }
                    found = Some((id, entry.bytes));
                }
            }
            let Some((id, bytes)) = found else {
                self.state = State::Absent;
                return Ok(());
            };
            if bytes < 64 || bytes > MAX_BYTES.min(output.len()) as u64 {
                return Err(Rejection::Size);
            }
            self.expected = bytes as usize;
            self.handle = Some(files.open_id(id).map_err(Rejection::Stream)?);
            self.started = Some(now);
            self.state = State::Reading;
        }
        if self.state != State::Reading {
            return Ok(());
        }
        if now
            .checked_sub(self.started.unwrap())
            .is_none_or(|elapsed| elapsed >= TIMEOUT_MS)
        {
            return Err(Rejection::Timeout);
        }
        if output.len() < self.expected {
            return Err(Rejection::Size);
        }
        let end = self.expected.min(self.used + MAX_READ);
        match files.read(self.handle.unwrap(), &mut output[self.used..end]) {
            Err(Error::Again) => (),
            Err(e) => return Err(Rejection::Stream(e)),
            Ok(0) => return Err(Rejection::Stream(Error::Corrupt)),
            Ok(n) => {
                self.used += n;
                if self.used == self.expected {
                    self.state = State::Ready(self.used);
                }
            }
        }
        Ok(())
    }
}
