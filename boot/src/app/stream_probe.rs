// Copyright 2026 Hirofumi Iwasaki
// SPDX-License-Identifier: Apache-2.0
use musha_api::stream::{Cursor, Error, Files, Handle};
pub(super) struct Scan {
    cursor: Option<Cursor>,
    handle: Option<Handle>,
    index: usize,
    bytes: u64,
    hash: u64,
    first: (u64, u64),
    expected: u64,
    done: bool,
    #[cfg(feature = "file-app-close")]
    close: u8,
    #[cfg(feature = "file-app-close")]
    id: Option<musha_api::stream::Id>,
}
impl Scan {
    pub(super) const fn new() -> Self {
        Self {
            cursor: None,
            handle: None,
            index: 0,
            bytes: 0,
            hash: 0xcbf29ce484222325,
            first: (0, 0),
            expected: 0,
            done: false,
            #[cfg(feature = "file-app-close")]
            close: 0,
            #[cfg(feature = "file-app-close")]
            id: None,
        }
    }
    #[cfg(feature = "file-app-close")]
    pub(super) fn request_close(&mut self) {
        if self.close == 0 {
            self.close = 1;
        }
    }
    pub(super) fn step(&mut self, files: &mut Files<'_>) -> Result<(), musha_api::Error> {
        self.poll(files).map_err(|_| musha_api::Error::Io)
    }
    fn poll(&mut self, files: &mut Files<'_>) -> Result<(), Error> {
        if self.done {
            return Ok(());
        }
        #[cfg(feature = "file-app-close")]
        {
            if self.close == 1 {
                files.close(self.handle.take().ok_or(Error::Invalid)?)?;
                if files.open_id(self.id.ok_or(Error::Invalid)?) != Err(Error::Again) {
                    return Err(Error::Corrupt);
                }
                crate::debug(b"MUSHA: APP_FILE_CLOSE_QUARANTINED\n");
                self.close = 2;
                return Ok(());
            }
            if self.close == 2 {
                match files.open_id(self.id.ok_or(Error::Invalid)?) {
                    Err(Error::Again) => return Ok(()),
                    Err(e) => return Err(e),
                    Ok(h) => self.handle = Some(h),
                }
                self.close = 3;
                crate::debug(b"MUSHA: APP_FILE_REOPENED\n");
                return Ok(());
            }
        }
        if self.cursor.is_none() {
            self.cursor = Some(files.list_begin()?);
        }
        if self.handle.is_none() {
            match files.list_next(self.cursor.as_mut().unwrap())? {
                Some((id, entry)) => {
                    #[cfg(feature = "file-app-close")]
                    {
                        self.id = Some(id);
                    }
                    self.expected = entry.bytes;
                    self.bytes = 0;
                    self.hash = 0xcbf29ce484222325;
                    crate::debug(b"MUSHA: LIVE_FILE_NAME ");
                    for &u in &entry.name[..entry.name_len as usize] {
                        crate::debug(&crate::cpu::hex(u as u64));
                    }
                    crate::debug(b"\n");
                    self.handle = Some(files.open_id(id)?);
                }
                None => {
                    self.done = true;
                    crate::debug(b"MUSHA: LIVE_FILE_API_OK BYTES=");
                    crate::debug(&crate::cpu::hex(self.first.0));
                    crate::debug(b" HASH=");
                    crate::debug(&crate::cpu::hex(self.first.1));
                    crate::debug(b"\n");
                }
            }
            return Ok(());
        }
        let handle = self.handle.unwrap();
        let mut data = [0; 512];
        let n = if self.index == 0 { 7 } else { 512 };
        match files.read(handle, &mut data[..n]) {
            Err(Error::Again) => {}
            Err(e) => return Err(e),
            Ok(0) => {
                if self.bytes != self.expected || files.read(handle, &mut data[..1])? != 0 {
                    return Err(Error::Corrupt);
                }
                files.close(handle)?;
                self.handle = None;
                crate::debug(b"MUSHA: LIVE_FILE_ENTRY_OK INDEX=");
                crate::debug(&crate::cpu::hex(self.index as u64));
                crate::debug(b" BYTES=");
                crate::debug(&crate::cpu::hex(self.bytes));
                crate::debug(b" HASH=");
                crate::debug(&crate::cpu::hex(self.hash));
                crate::debug(b"\n");
                if self.index == 0 {
                    self.first = (self.bytes, self.hash);
                }
                self.index += 1;
            }
            Ok(n) => {
                self.bytes += n as u64;
                for &b in &data[..n] {
                    self.hash = (self.hash ^ b as u64).wrapping_mul(0x100000001b3);
                }
            }
        }
        Ok(())
    }
}
