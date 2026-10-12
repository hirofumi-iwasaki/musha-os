// Copyright 2026 Hirofumi Iwasaki
// SPDX-License-Identifier: Apache-2.0
//! One bounded file operation across metadata and data sector requests.
//! Tokens belong to this instance and must be routed with the parent API request.
use crate::{Error, planner, read_plan};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Token {
    Metadata(planner::Request),
    Data(read_plan::Request),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Request(Token);
impl Request {
    pub fn lba(self) -> u64 {
        match self.0 {
            Token::Metadata(r) => r.lba(),
            Token::Data(r) => r.lba(),
        }
    }
    pub fn bytes(self) -> usize {
        match self.0 {
            Token::Metadata(r) => r.bytes(),
            Token::Data(r) => r.bytes(),
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Progress {
    Yield,
    Pending,
    Read(Request),
    Complete,
}
/// CPU-owned state only; does not retain caller buffers or perform any I/O.
/// Cancellation must be followed by transport quiescence before dropping/reusing
/// the operation. A pending token survives cancel until complete acknowledges it.
pub struct Read {
    planner: Option<planner::Planner>,
    executor: read_plan::Executor,
    pending: Option<Request>,
    ready: bool,
    canceled: bool,
    error: Option<Error>,
    bytes: usize,
    result: [u8; 4096],
}
impl Read {
    pub fn new(planner: planner::Planner) -> Self {
        Self {
            planner: Some(planner),
            executor: read_plan::Executor::new(),
            pending: None,
            ready: false,
            canceled: false,
            error: None,
            bytes: 0,
            result: [0; 4096],
        }
    }
    pub fn poll(&mut self) -> Result<Progress, Error> {
        if self.canceled {
            return Err(Error::Io);
        }
        if let Some(e) = self.error {
            return Err(e);
        }
        if self.pending.is_some() {
            return Ok(Progress::Pending);
        }
        if self.ready {
            return Ok(Progress::Complete);
        }
        let result = self.step();
        if let Err(e) = result {
            self.error = Some(e);
        }
        result
    }
    fn step(&mut self) -> Result<Progress, Error> {
        if let Some(planner) = self.planner.as_mut() {
            match planner.poll()? {
                planner::Progress::Read(r) => {
                    let request = Request(Token::Metadata(r));
                    self.pending = Some(request);
                    Ok(Progress::Read(request))
                }
                planner::Progress::Complete => {
                    self.executor
                        .start(planner.take_plan()?.ok_or(Error::Corrupt)?)?;
                    self.planner = None;
                    Ok(Progress::Yield)
                }
                planner::Progress::Yield => Ok(Progress::Yield),
                planner::Progress::Pending => Err(Error::Corrupt),
            }
        } else if let Some(r) = self.executor.take_request()? {
            let request = Request(Token::Data(r));
            self.pending = Some(request);
            Ok(Progress::Read(request))
        } else {
            self.bytes = self
                .executor
                .finish(&mut self.result)?
                .ok_or(Error::Corrupt)?;
            self.ready = true;
            Ok(Progress::Complete)
        }
    }
    /// Caller guarantees DMA for this token has completed/stopped. Duplicate or
    /// stale tokens never retire the current request or publish partial results.
    pub fn complete(&mut self, request: Request, result: Result<&[u8], Error>) -> bool {
        if self.pending != Some(request) {
            return false;
        }
        let accepted = match request.0 {
            Token::Metadata(r) => self.planner.as_mut().is_some_and(|p| p.complete(r, result)),
            Token::Data(r) => self.executor.complete(r, result),
        };
        if accepted {
            self.pending = None;
        }
        accepted
    }
    pub fn finish(&self, out: &mut [u8]) -> Result<Option<usize>, Error> {
        if self.canceled {
            return Err(Error::Io);
        }
        if let Some(e) = self.error {
            return Err(e);
        }
        if !self.ready {
            return Ok(None);
        }
        if out.len() < self.bytes {
            return Err(Error::TooLarge);
        }
        out[..self.bytes].copy_from_slice(&self.result[..self.bytes]);
        Ok(Some(self.bytes))
    }
    pub fn cancel(&mut self) {
        self.canceled = true;
        self.result.fill(0);
        if let Some(p) = self.planner.as_mut() {
            p.cancel();
        }
        self.executor.cancel();
    }
    pub fn is_quiescent(&self) -> bool {
        self.pending.is_none()
    }
}
