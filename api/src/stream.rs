// Copyright 2026 Hirofumi Iwasaki
// SPDX-License-Identifier: Apache-2.0
//! Backend-neutral read-only file service. No disk calls or retained app buffers.
//! IDs, handles and requests belong exclusively to their originating service.

pub const MAX_FILES: usize = 128;
pub const MAX_READ: usize = 4096;
pub const MAX_FILE_BYTES: u64 = 64 * 1024 * 1024;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Invalid,
    Again,
    Disconnected,
    Corrupt,
    Unsupported,
    TooLarge,
    Limit,
    Io,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Format {
    Fat32,
    Exfat,
}
/// Validated root-file metadata, owned by the runtime for the mount lifetime.
/// Names are full UTF-16; display abbreviations must never be used as identity.
pub struct Entry {
    pub name: [u16; 255],
    pub name_len: u8,
    pub bytes: u64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Id {
    media: u64,
    index: usize,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Handle {
    media: u64,
    serial: u64,
}
/// An immutable catalog cursor; replacement media invalidates it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Cursor {
    media: u64,
    next: usize,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Request {
    serial: u64,
    handle: Handle,
    index: usize,
    offset: u64,
    length: usize,
}
impl Request {
    pub fn index(self) -> usize {
        self.index
    }
    pub fn offset(self) -> u64 {
        self.offset
    }
    pub fn length(self) -> usize {
        self.length
    }
}
#[derive(Clone, Copy)]
struct Open {
    handle: Handle,
    index: usize,
    position: u64,
}
enum Pending {
    Idle,
    Queued(Request),
    Running(Request, bool), // canceled requests retain the slot until quiescent
    Ready(Result<usize, Error>),
}
/// One viewer and one outstanding operation. Construct outside the UEFI stack.
/// Runtime DMA memory is separate; `complete` is called only after DMA quiesces.
pub struct Files<'a> {
    entries: &'a [Entry],
    format: Format,
    online: bool,
    media: u64,
    serial: u64,
    open: Option<Open>,
    pending: Pending,
    result: [u8; MAX_READ],
}
impl Default for Files<'_> {
    fn default() -> Self {
        Self::new()
    }
}
impl<'a> Files<'a> {
    pub const fn new() -> Self {
        Self {
            entries: &[],
            format: Format::Fat32,
            online: false,
            media: 0,
            serial: 0,
            open: None,
            pending: Pending::Idle,
            result: [0; MAX_READ],
        }
    }
    /// Publish a fully validated, immutable catalog. Backend indices match its order.
    /// Mount/replacement waits for any canceled hardware operation to quiesce.
    pub fn mount(&mut self, format: Format, entries: &'a [Entry]) -> Result<(), Error> {
        if matches!(self.pending, Pending::Running(..)) {
            return Err(Error::Again);
        }
        if entries.len() > MAX_FILES {
            return Err(Error::Limit);
        }
        if entries.iter().any(|e| e.name_len == 0) {
            return Err(Error::Invalid);
        }
        let media = self.media.checked_add(1).ok_or(Error::Limit)?;
        self.media = media;
        self.format = format;
        self.entries = entries;
        self.online = true;
        self.open = None;
        self.pending = Pending::Idle;
        self.result.fill(0);
        Ok(())
    }
    pub fn format(&self) -> Result<Format, Error> {
        self.connected()?;
        Ok(self.format)
    }
    fn connected(&self) -> Result<(), Error> {
        if self.online {
            Ok(())
        } else {
            Err(Error::Disconnected)
        }
    }
    pub fn list_begin(&self) -> Result<Cursor, Error> {
        self.connected()?;
        Ok(Cursor {
            media: self.media,
            next: 0,
        })
    }
    pub fn list_next(&self, cursor: &mut Cursor) -> Result<Option<(Id, &Entry)>, Error> {
        self.connected()?;
        if cursor.media != self.media {
            return Err(Error::Invalid);
        }
        let Some(entry) = self.entries.get(cursor.next) else {
            return Ok(None);
        };
        let id = Id {
            media: self.media,
            index: cursor.next,
        };
        cursor.next += 1;
        Ok(Some((id, entry)))
    }
    pub fn open_id(&mut self, id: Id) -> Result<Handle, Error> {
        self.connected()?;
        if id.media != self.media || id.index >= self.entries.len() {
            return Err(Error::Invalid);
        }
        if self.entries[id.index].bytes > MAX_FILE_BYTES {
            return Err(Error::TooLarge);
        }
        if self.open.is_some() || !matches!(self.pending, Pending::Idle) {
            return Err(Error::Again);
        }
        self.serial = self.serial.checked_add(1).ok_or(Error::Limit)?;
        let handle = Handle {
            media: self.media,
            serial: self.serial,
        };
        self.open = Some(Open {
            handle,
            index: id.index,
            position: 0,
        });
        Ok(handle)
    }
    fn validate(&self, handle: Handle) -> Result<&Open, Error> {
        self.connected()?;
        self.open
            .as_ref()
            .filter(|o| o.handle == handle)
            .ok_or(Error::Invalid)
    }
    /// Again and errors preserve position and caller bytes. Retries may use a
    /// different buffer, but it must fit the completed result (otherwise Invalid).
    /// Empty output is a no-op. EOF is zero for a nonempty output.
    pub fn read(&mut self, handle: Handle, out: &mut [u8]) -> Result<usize, Error> {
        let open = *self.validate(handle)?;
        if out.is_empty() {
            return Ok(0);
        }
        match self.pending {
            Pending::Ready(result) => {
                if let Ok(n) = result {
                    if out.len() < n {
                        return Err(Error::Invalid);
                    }
                    out[..n].copy_from_slice(&self.result[..n]);
                    self.open.as_mut().unwrap().position += n as u64;
                }
                self.pending = Pending::Idle;
                self.result.fill(0);
                result
            }
            Pending::Idle => {
                let left = self.entries[open.index].bytes - open.position;
                let length = left.min(out.len().min(MAX_READ) as u64) as usize;
                if length == 0 {
                    return Ok(0);
                }
                self.serial = self.serial.checked_add(1).ok_or(Error::Limit)?;
                self.pending = Pending::Queued(Request {
                    serial: self.serial,
                    handle,
                    index: open.index,
                    offset: open.position,
                    length,
                });
                Err(Error::Again)
            }
            _ => Err(Error::Again),
        }
    }
    /// Runtime consumes each queued request once. This never executes I/O.
    pub fn take_request(&mut self) -> Option<Request> {
        if let Pending::Queued(request) = self.pending {
            self.pending = Pending::Running(request, false);
            Some(request)
        } else {
            None
        }
    }
    /// Runtime reports an exact read or terminal error, after hardware has stopped
    /// touching its own buffers. Short results become Corrupt; Again is not terminal.
    /// False means stale/duplicate token or nonterminal status; slot is unchanged.
    pub fn complete(&mut self, request: Request, result: Result<&[u8], Error>) -> bool {
        let Pending::Running(active, canceled) = self.pending else {
            return false;
        };
        if active != request || result == Err(Error::Again) {
            return false;
        }
        if canceled {
            self.pending = Pending::Idle;
            return true;
        }
        let result = result.and_then(|bytes| {
            if bytes.len() != request.length {
                return Err(Error::Corrupt);
            }
            self.result[..bytes.len()].copy_from_slice(bytes);
            Ok(bytes.len())
        });
        self.pending = Pending::Ready(result);
        true
    }
    fn cancel(&mut self) {
        self.pending = match self.pending {
            Pending::Running(request, _) => Pending::Running(request, true),
            _ => Pending::Idle,
        };
        self.result.fill(0);
        self.open = None;
    }
    pub fn close(&mut self, handle: Handle) -> Result<(), Error> {
        self.validate(handle)?;
        self.cancel();
        Ok(())
    }
    /// Invalidates all application identities immediately. Runtime must still
    /// quiesce outstanding DMA and call complete before reusing the service.
    pub fn disconnect(&mut self) {
        self.cancel();
        self.online = false;
        self.entries = &[];
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn identities_never_wrap_and_invalid_catalog_is_atomic() {
        let mut name = [0; 255];
        name[0] = 65;
        let entries = [Entry {
            name,
            name_len: 1,
            bytes: 1,
        }];
        let mut f = Files::new();
        f.mount(Format::Exfat, &entries).unwrap();
        let id = f
            .list_next(&mut f.list_begin().unwrap())
            .unwrap()
            .unwrap()
            .0;
        f.serial = u64::MAX;
        assert_eq!(f.open_id(id), Err(Error::Limit));
        f.serial = u64::MAX - 1;
        let h = f.open_id(id).unwrap();
        assert_eq!(f.read(h, &mut [0; 1]), Err(Error::Limit));
        assert!(f.take_request().is_none());
        f.media = u64::MAX;
        assert_eq!(f.mount(Format::Fat32, &entries), Err(Error::Limit));
        assert_eq!(f.format(), Ok(Format::Exfat));
        let invalid = [Entry {
            name,
            name_len: 0,
            bytes: 1,
        }];
        assert_eq!(f.mount(Format::Fat32, &invalid), Err(Error::Invalid));
        assert_eq!(f.entries[0].name_len, 1);
    }
}
