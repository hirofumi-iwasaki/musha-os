// Copyright 2026 Hirofumi Iwasaki
// SPDX-License-Identifier: Apache-2.0
//! Validated data-sector plans. Metadata planning is separate from data I/O.
use crate::Error;
#[derive(Clone, Copy, Default)]
struct Piece {
    lba: u64,
    skip: usize,
    take: usize,
    destination: usize,
}
/// A 4096-byte read intersects at most nine 512-byte sectors. Invalid exFAT tail
/// bytes have no pieces and remain zero. Plans require an unchanged mounted medium.
pub struct Plan {
    pieces: [Piece; 9],
    count: usize,
    bytes: usize,
    size: usize,
}
impl Plan {
    pub(crate) fn new(bytes: usize, size: usize) -> Self {
        Self {
            pieces: [Piece::default(); 9],
            count: 0,
            bytes,
            size,
        }
    }
    pub fn len(&self) -> usize {
        self.bytes
    }
    pub fn is_empty(&self) -> bool {
        self.bytes == 0
    }
    pub(crate) fn push(
        &mut self,
        lba: u64,
        skip: usize,
        take: usize,
        destination: usize,
    ) -> Result<(), Error> {
        if self.count == 9
            || skip.checked_add(take).is_none_or(|n| n > self.size)
            || destination.checked_add(take).is_none_or(|n| n > self.bytes)
        {
            return Err(Error::Limit);
        }
        self.pieces[self.count] = Piece {
            lba,
            skip,
            take,
            destination,
        };
        self.count += 1;
        Ok(())
    }
    pub(crate) fn execute<F: FnMut(u64, &mut [u8]) -> Result<(), Error>>(
        &self,
        read: &mut F,
        out: &mut [u8],
    ) -> Result<usize, Error> {
        if out.len() < self.bytes {
            return Err(Error::TooLarge);
        }
        let mut result = [0; 4096];
        let mut sector = [0; 4096];
        for piece in &self.pieces[..self.count] {
            read(piece.lba, &mut sector[..self.size])?;
            result[piece.destination..piece.destination + piece.take]
                .copy_from_slice(&sector[piece.skip..piece.skip + piece.take]);
        }
        out[..self.bytes].copy_from_slice(&result[..self.bytes]);
        Ok(self.bytes)
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Request {
    serial: u64,
    lba: u64,
    bytes: usize,
}
impl Request {
    pub fn lba(self) -> u64 {
        self.lba
    }
    pub fn bytes(self) -> usize {
        self.bytes
    }
}
/// One operation per executor, with tokens that never repeat for its lifetime.
/// Tokens belong only to their originating executor; never move them between executors.
/// Caller buffers are only used by finish, after all sectors have completed.
pub struct Executor {
    plan: Option<Plan>,
    next: usize,
    serial: u64,
    pending: Option<Request>,
    canceled: bool,
    error: Option<Error>,
    result: [u8; 4096],
}
impl Default for Executor {
    fn default() -> Self {
        Self::new()
    }
}
impl Executor {
    pub const fn new() -> Self {
        Self {
            plan: None,
            next: 0,
            serial: 0,
            pending: None,
            canceled: false,
            error: None,
            result: [0; 4096],
        }
    }
    /// Busy/canceled hardware operations must quiesce before starting another.
    pub fn start(&mut self, plan: Plan) -> Result<(), Error> {
        if self.plan.is_some() || self.pending.is_some() {
            return Err(Error::Limit);
        }
        self.plan = Some(plan);
        self.next = 0;
        self.canceled = false;
        self.error = None;
        self.result.fill(0);
        Ok(())
    }
    /// Returns a sector at most once. None also means pending, complete or idle.
    pub fn take_request(&mut self) -> Result<Option<Request>, Error> {
        if let Some(error) = self.error {
            return Err(error);
        }
        if self.pending.is_some() || self.canceled {
            return Ok(None);
        }
        let Some(plan) = self.plan.as_ref() else {
            return Ok(None);
        };
        if self.next == plan.count {
            return Ok(None);
        }
        let Some(serial) = self.serial.checked_add(1) else {
            self.error = Some(Error::Limit);
            return Err(Error::Limit);
        };
        self.serial = serial;
        let request = Request {
            serial,
            lba: plan.pieces[self.next].lba,
            bytes: plan.size,
        };
        self.pending = Some(request);
        Ok(Some(request))
    }
    /// Transport calls this only after the requested sector's DMA has quiesced.
    /// Wrong/stale tokens do not retire the active request or change any bytes.
    pub fn complete(&mut self, request: Request, result: Result<&[u8], Error>) -> bool {
        if self.pending != Some(request) {
            return false;
        }
        self.pending = None;
        if self.canceled {
            self.plan = None;
            return true;
        }
        let plan = self.plan.as_ref().unwrap();
        match result {
            Ok(bytes) if bytes.len() == plan.size => {
                let piece = plan.pieces[self.next];
                self.result[piece.destination..piece.destination + piece.take]
                    .copy_from_slice(&bytes[piece.skip..piece.skip + piece.take]);
                self.next += 1;
            }
            Ok(_) => self.error = Some(Error::Corrupt),
            Err(error) => self.error = Some(error),
        }
        true
    }
    /// None means pending; terminal errors and pending preserve all caller bytes.
    /// An undersized output can be retried without losing the completed result.
    pub fn finish(&mut self, out: &mut [u8]) -> Result<Option<usize>, Error> {
        if self.canceled {
            return Err(Error::Io);
        }
        if let Some(error) = self.error {
            self.plan = None;
            self.result.fill(0);
            return Err(error);
        }
        let plan = self.plan.as_ref().ok_or(Error::NotFound)?;
        if self.pending.is_some() || self.next < plan.count {
            return Ok(None);
        }
        if out.len() < plan.bytes {
            return Err(Error::TooLarge);
        }
        let bytes = plan.bytes;
        out[..bytes].copy_from_slice(&self.result[..bytes]);
        self.plan = None;
        self.result.fill(0);
        Ok(Some(bytes))
    }
    pub fn cancel(&mut self) {
        self.canceled = true;
        self.result.fill(0);
        if self.pending.is_none() {
            self.plan = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn plan() -> Plan {
        let mut p = Plan::new(5, 512);
        p.push(10, 511, 1, 0).unwrap();
        p.push(20, 0, 2, 1).unwrap();
        p
    }
    #[test]
    fn pending_atomic_owned_output_and_zero_tail() {
        let mut e = Executor::new();
        e.start(plan()).unwrap();
        let mut out = [0xa5; 8];
        let a = e.take_request().unwrap().unwrap();
        assert_eq!(a.lba(), 10);
        for _ in 0..100 {
            assert_eq!(e.take_request(), Ok(None));
            assert_eq!(e.finish(&mut out), Ok(None));
            assert_eq!(out, [0xa5; 8]);
        }
        let mut sector = [7; 512];
        assert!(e.complete(a, Ok(&sector)));
        sector.fill(0);
        let b = e.take_request().unwrap().unwrap();
        assert!(!e.complete(a, Err(Error::Io)));
        assert!(e.complete(b, Ok(&[8; 512])));
        assert_eq!(e.finish(&mut [0; 4]), Err(Error::TooLarge));
        assert_eq!(e.finish(&mut out), Ok(Some(5)));
        assert_eq!(out, [7, 8, 8, 0, 0, 0xa5, 0xa5, 0xa5]);
        assert_eq!(e.finish(&mut out), Err(Error::NotFound));
    }
    #[test]
    fn errors_cancel_and_stale_completion_do_not_publish() {
        for failed in [false, true] {
            let mut e = Executor::new();
            e.start(plan()).unwrap();
            let a = e.take_request().unwrap().unwrap();
            assert!(e.complete(a, Ok(&[7; 512])));
            let b = e.take_request().unwrap().unwrap();
            assert!(e.complete(
                b,
                if failed {
                    Err(Error::Io)
                } else {
                    Ok(&[8; 511])
                }
            ));
            let mut out = [0xa5; 5];
            assert_eq!(
                e.finish(&mut out),
                Err(if failed { Error::Io } else { Error::Corrupt })
            );
            assert_eq!(out, [0xa5; 5]);
            e.start(plan()).unwrap();
            let c = e.take_request().unwrap().unwrap();
            assert!(!e.complete(b, Ok(&[8; 512])));
            e.cancel();
            assert_eq!(e.start(plan()), Err(Error::Limit));
            assert!(e.complete(c, Ok(&[9; 512])));
            assert_eq!(e.finish(&mut out), Err(Error::Io));
            assert_eq!(out, [0xa5; 5]);
            e.start(plan()).unwrap();
            let d = e.take_request().unwrap().unwrap();
            assert_ne!(c, d);
        }
    }
    #[test]
    fn nine_sector_unaligned_read_preserves_full_range() {
        let mut p = Plan::new(4096, 512);
        p.push(1, 511, 1, 0).unwrap();
        for i in 0..7 {
            p.push(2 + i, 0, 512, 1 + i as usize * 512).unwrap();
        }
        p.push(9, 0, 511, 3585).unwrap();
        let mut e = Executor::new();
        e.start(p).unwrap();
        let mut expected = [0; 4096];
        let mut copied = 0;
        let mut count = 0;
        while let Some(r) = e.take_request().unwrap() {
            count += 1;
            let sector = [r.lba() as u8; 512];
            assert!(e.complete(r, Ok(&sector)));
            let n = if r.lba() == 1 {
                1
            } else if r.lba() == 9 {
                511
            } else {
                512
            };
            expected[copied..copied + n].fill(r.lba() as u8);
            copied += n;
        }
        let mut out = [0; 4096];
        assert_eq!(count, 9);
        assert_eq!(e.finish(&mut out), Ok(Some(4096)));
        assert_eq!(out, expected);
    }
    #[test]
    fn empty_zero_filled_reads_and_token_exhaustion() {
        let mut e = Executor::new();
        e.start(Plan::new(0, 512)).unwrap();
        assert_eq!(e.take_request(), Ok(None));
        assert_eq!(e.finish(&mut []), Ok(Some(0)));
        e.start(Plan::new(5, 4096)).unwrap();
        let mut out = [1; 5];
        assert_eq!(e.finish(&mut out), Ok(Some(5)));
        assert_eq!(out, [0; 5]);
        e.serial = u64::MAX;
        e.start(plan()).unwrap();
        assert_eq!(e.take_request(), Err(Error::Limit));
        assert_eq!(e.finish(&mut out), Err(Error::Limit));
        assert!(core::mem::size_of::<Executor>() < 8192);
    }
}
