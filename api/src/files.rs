// Copyright 2026 Hirofumi Iwasaki
// SPDX-License-Identifier: Apache-2.0
use crate::Error;
const MAX_BYTES: usize = 4096;
const HANDLES: usize = 4;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FileHandle {
    index: u8,
    generation: u32,
}
#[derive(Clone, Copy)]
struct OpenFile {
    position: usize,
}
#[derive(Clone, Copy)]
enum State {
    Pending,
    Ready(usize),
    Unavailable(Error),
}
/// A bounded, runtime-owned snapshot of the first verified boot MUSHA.TXT.
/// No device DMA memory is borrowed and reads do not invoke device I/O.
pub(crate) struct Files {
    bytes: [u8; MAX_BYTES],
    state: State,
    discovery_finished: bool,
    handles: [Option<OpenFile>; HANDLES],
    generations: [u32; HANDLES],
}
impl Files {
    pub(crate) fn new() -> Self {
        Self {
            bytes: [0; MAX_BYTES],
            state: State::Pending,
            discovery_finished: false,
            handles: [None; HANDLES],
            generations: [1; HANDLES],
        }
    }
    pub(crate) fn publish(&mut self, bytes: &[u8]) -> Result<bool, Error> {
        if matches!(self.state, State::Ready(_)) {
            return Ok(false);
        }
        if bytes.len() > MAX_BYTES {
            return Err(Error::NoMemory);
        }
        self.bytes[..bytes.len()].copy_from_slice(bytes);
        self.state = State::Ready(bytes.len());
        Ok(true)
    }
    pub(crate) fn unavailable(&mut self, error: Error) {
        if !matches!(self.state, State::Ready(_)) {
            self.state = State::Unavailable(error);
        }
    }
    pub(crate) fn finish(&mut self, failed: bool) {
        self.discovery_finished = true;
        if failed {
            self.unavailable(Error::Io);
        } else if matches!(self.state, State::Pending) {
            self.state = State::Unavailable(Error::NotFound);
        }
    }
    pub(crate) fn invalidate(&mut self, error: Error) {
        self.discovery_finished = true;
        for i in 0..HANDLES {
            if self.handles[i].take().is_some() {
                self.generations[i] = self.generations[i].checked_add(1).unwrap_or(0);
            }
        }
        self.bytes.fill(0);
        self.state = State::Unavailable(error);
    }
    fn length(&self) -> Result<usize, Error> {
        match self.state {
            State::Pending => Err(Error::Again),
            State::Ready(length) => Ok(length),
            State::Unavailable(_) if !self.discovery_finished => Err(Error::Again),
            State::Unavailable(error) => Err(error),
        }
    }
    pub(crate) fn open(&mut self, path: &str) -> Result<FileHandle, Error> {
        // Initial scope: absolute, ASCII 8.3 names in the root directory.
        let name = path.strip_prefix('/').ok_or(Error::Invalid)?;
        let (base, ext) = name.split_once('.').ok_or(Error::Invalid)?;
        if base.is_empty()
            || base.len() > 8
            || ext.is_empty()
            || ext.len() > 3
            || !base
                .bytes()
                .chain(ext.bytes())
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-'))
        {
            return Err(Error::Invalid);
        }
        self.length()?;
        if !name.eq_ignore_ascii_case("MUSHA.TXT") {
            return Err(Error::NotFound);
        }
        let index = (0..HANDLES)
            .find(|&i| self.handles[i].is_none() && self.generations[i] != 0)
            .ok_or(Error::NoMemory)?;
        self.handles[index] = Some(OpenFile { position: 0 });
        Ok(FileHandle {
            index: index as u8,
            generation: self.generations[index],
        })
    }
    fn slot(&self, handle: FileHandle) -> Result<usize, Error> {
        let index = handle.index as usize;
        if index >= HANDLES
            || self.generations[index] != handle.generation
            || self.handles[index].is_none()
        {
            return Err(Error::Invalid);
        }
        Ok(index)
    }
    pub(crate) fn read(&mut self, handle: FileHandle, out: &mut [u8]) -> Result<usize, Error> {
        let index = self.slot(handle)?;
        let length = self.length()?;
        let file = self.handles[index].as_mut().unwrap();
        let count = out.len().min(length - file.position);
        out[..count].copy_from_slice(&self.bytes[file.position..file.position + count]);
        file.position += count;
        Ok(count)
    }
    pub(crate) fn close(&mut self, handle: FileHandle) -> Result<(), Error> {
        let index = self.slot(handle)?;
        self.handles[index] = None;
        // On generation overflow, permanently retire this slot instead of aliasing.
        self.generations[index] = self.generations[index].checked_add(1).unwrap_or(0);
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pending_snapshot_chunks_eof_and_caller_ownership() {
        let mut files = Files::new();
        assert_eq!(files.open("/MUSHA.TXT"), Err(Error::Again));
        let mut source = *b"abcdefg";
        assert_eq!(files.publish(&source), Ok(true));
        source.fill(b'X');
        let handle = files.open("/musha.txt").unwrap();
        let mut out = [0x5a; 4];
        assert_eq!(files.read(handle, &mut []), Ok(0));
        assert_eq!(files.read(handle, &mut out), Ok(4));
        assert_eq!(&out, b"abcd");
        assert_eq!(files.read(handle, &mut out), Ok(3));
        assert_eq!(&out, b"efgd");
        assert_eq!(files.read(handle, &mut out), Ok(0));
        assert_eq!(&out, b"efgd");
        assert_eq!(files.close(handle), Ok(()));
        let new = files.open("/MUSHA.TXT").unwrap();
        let other = files.open("/MUSHA.TXT").unwrap();
        assert_ne!(new, handle);
        assert_eq!(files.read(handle, &mut out), Err(Error::Invalid));
        assert_eq!(files.close(handle), Err(Error::Invalid));
        assert_eq!(files.read(new, &mut out), Ok(4));
        assert_eq!(&out, b"abcd");
        assert_eq!(files.read(other, &mut out[..2]), Ok(2));
        assert_eq!(&out[..2], b"ab");
        assert_eq!(files.read(new, &mut out), Ok(3));
        assert_eq!(&out[..3], b"efg");
    }
    #[test]
    fn limits_paths_errors_and_invalidation() {
        let mut files = Files::new();
        assert_eq!(files.publish(&[0; 4097]), Err(Error::NoMemory));
        assert_eq!(files.open("/MUSHA.TXT"), Err(Error::Again));
        files.unavailable(Error::Unsupported);
        assert_eq!(files.open("/MUSHA.TXT"), Err(Error::Again));
        files.finish(false);
        assert_eq!(files.open("/MUSHA.TXT"), Err(Error::Unsupported));
        files.publish(b"hello").unwrap();
        for path in [
            "MUSHA.TXT",
            "/../MUSHA.TXT",
            "/EFI/BOOT.TXT",
            "/longfilename.txt",
            "/日本語.txt",
            "/MUSHA.",
        ] {
            assert_eq!(files.open(path), Err(Error::Invalid), "{path}");
        }
        assert_eq!(files.open("/ABSENT.TXT"), Err(Error::NotFound));
        let mut handles = [files.open("/MUSHA.TXT").unwrap(); 4];
        for slot in &mut handles[1..] {
            *slot = files.open("/MUSHA.TXT").unwrap();
        }
        assert_eq!(files.open("/MUSHA.TXT"), Err(Error::NoMemory));
        files.invalidate(Error::Disconnected);
        assert_eq!(files.open("/MUSHA.TXT"), Err(Error::Disconnected));
        for handle in handles {
            assert_eq!(files.read(handle, &mut [0; 1]), Err(Error::Invalid));
        }
        files.publish(b"new").unwrap();
        let new = files.open("/MUSHA.TXT").unwrap();
        assert_ne!(new, handles[0]);
    }
    #[test]
    fn discovery_failures_first_success_and_generation_retirement() {
        let mut files = Files::new();
        files.finish(false);
        assert_eq!(files.open("/MUSHA.TXT"), Err(Error::NotFound));
        files.finish(true);
        assert_eq!(files.open("/MUSHA.TXT"), Err(Error::Io));
        files.publish(b"").unwrap();
        assert_eq!(files.publish(b"ignored"), Ok(false));
        files.finish(true); // The copied snapshot survives unrelated device failure.
        files.generations[0] = u32::MAX;
        let handle = files.open("/MUSHA.TXT").unwrap();
        assert_eq!(files.read(handle, &mut [0; 1]), Ok(0));
        files.close(handle).unwrap();
        let next = files.open("/MUSHA.TXT").unwrap();
        assert_eq!(next.index, 1);
    }
}
