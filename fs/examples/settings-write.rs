// Copyright 2026 Hirofumi Iwasaki
// SPDX-License-Identifier: Apache-2.0
//! Development-only regular image harness. Never accepts devices or symlinks.
use musha_fs::{
    Error,
    settings::{self, Disk, Workspace},
};
use std::{
    fs::{File, OpenOptions},
    io::{Read, Seek, SeekFrom, Write},
    path::Path,
};
struct Image {
    file: File,
    protected: bool,
}
impl Disk for Image {
    fn read(&mut self, lba: u64, b: &mut [u8]) -> Result<(), Error> {
        self.file
            .seek(SeekFrom::Start(lba * 512))
            .and_then(|_| self.file.read_exact(b))
            .map_err(|_| Error::Io)
    }
    fn write(&mut self, lba: u64, b: &[u8]) -> Result<(), Error> {
        if self.protected {
            return Err(Error::Io);
        }
        self.file
            .seek(SeekFrom::Start(lba * 512))
            .and_then(|_| self.file.write_all(b))
            .map_err(|_| Error::Io)
    }
    fn sync(&mut self) -> Result<(), Error> {
        self.file.sync_all().map_err(|_| Error::Io)
    }
    fn protected(&self) -> bool {
        self.protected
    }
}
fn main() {
    let args: Vec<_> = std::env::args().collect();
    assert!(
        args.len() >= 3,
        "settings-write IMAGE SEED [NAME] [protected]"
    );
    let path = Path::new(&args[1]);
    let meta = std::fs::symlink_metadata(path).unwrap();
    assert!(
        meta.file_type().is_file() && !meta.file_type().is_symlink(),
        "regular image only"
    );
    let mut image = Image {
        file: OpenOptions::new()
            .read(true)
            .write(true)
            .open(path)
            .unwrap(),
        protected: args.get(4).is_some_and(|s| s == "protected"),
    };
    let mut record = [0; 4096];
    let hash = musha_platform::config_hash::sha256(args[2].as_bytes());
    musha_platform::pairing::encode_state(&[], Some(hash), &mut record).unwrap();
    let mut workspace = Box::new(Workspace::new());
    let result = settings::write_owned(
        &mut image,
        meta.len() / 512,
        512,
        args.get(3)
            .map(|s| s.as_bytes())
            .unwrap_or(settings::OWNED_NAME),
        &record,
        musha_platform::pairing::recognized,
        &mut workspace,
    );
    println!("{result:?}");
    if result.is_err() {
        std::process::exit(2);
    }
}
