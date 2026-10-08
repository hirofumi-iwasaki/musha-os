// Copyright 2026 Hirofumi Iwasaki
// SPDX-License-Identifier: Apache-2.0
use super::*;
fn put16(b: &mut [u8], n: usize, v: u16) {
    b[n..n + 2].copy_from_slice(&v.to_le_bytes());
}
fn put32(b: &mut [u8], n: usize, v: u32) {
    b[n..n + 4].copy_from_slice(&v.to_le_bytes());
}
struct Disk {
    start: u64,
    size: usize,
    root_cycle: bool,
    file_cycle: bool,
    short: bool,
    active: u16,
    bad_bpb: bool,
}
impl Disk {
    fn new(size: usize, start: u64) -> Self {
        Self {
            start,
            size,
            root_cycle: false,
            file_cycle: false,
            short: false,
            active: 0,
            bad_bpb: false,
        }
    }
    fn fatsecs(&self) -> u64 {
        (65527 * 4u64).div_ceil(self.size as u64)
    }
    fn data(&self) -> u64 {
        32 + 2 * self.fatsecs()
    }
    fn total(&self) -> u64 {
        self.data() + 65525
    }
    fn read(&self, lba: u64, b: &mut [u8]) -> Result<(), Error> {
        assert_eq!(b.len(), self.size);
        assert!(lba < self.start + self.total());
        b.fill(0);
        if lba == 0 && self.start != 0 {
            b[0] = 0xeb; // MBR bootstrap can also begin with a jump.
            b[510..512].copy_from_slice(&[0x55, 0xaa]);
            b[450] = 12;
            put32(b, 454, self.start as u32);
            put32(b, 458, self.total() as u32);
            return Ok(());
        }
        let Some(relative) = lba.checked_sub(self.start) else {
            return Ok(());
        };
        if relative == 0 {
            b[0] = 0xeb;
            put16(b, 11, self.size as u16);
            b[13] = if self.bad_bpb { 3 } else { 1 };
            put16(b, 14, 32);
            b[16] = 2;
            put32(b, 32, self.total() as u32);
            put32(b, 36, self.fatsecs() as u32);
            put16(b, 40, self.active);
            put32(b, 44, 2);
            b[510..512].copy_from_slice(&[0x55, 0xaa]);
        } else if relative == 32 || relative == 32 + self.fatsecs() {
            for (cluster, next) in [
                (2, if self.root_cycle { 2 } else { 4 }),
                (4, 0x0fffffff),
                (
                    3,
                    if self.file_cycle {
                        3
                    } else if self.short {
                        0x0fffffff
                    } else {
                        5
                    },
                ),
                (5, 7),
                (7, 0x0fffffff),
            ] {
                put32(b, cluster * 4, next);
            }
        } else if relative == self.data() {
            for e in b.chunks_exact_mut(32) {
                e[0] = 0xe5;
            }
        } else if relative == self.data() + 2 {
            b[..11].copy_from_slice(b"MUSHA   TXT");
            b[11] = 32;
            put16(b, 26, 3);
            put32(b, 28, if self.size == 4096 { 9000 } else { 1200 });
        } else if let Some(index) = [self.data() + 1, self.data() + 3, self.data() + 5]
            .iter()
            .position(|v| *v == relative)
        {
            for (i, v) in b.iter_mut().enumerate() {
                *v = ((index * self.size + i) * 17) as u8;
            }
        }
        Ok(())
    }
}
#[test]
fn fragmented_file_and_directory_mbr_and_superfloppy() {
    for start in [0, 2048] {
        let disk = Disk::new(512, start);
        let mut out = [0u8; 1200];
        assert_eq!(
            read_root(
                disk.start + disk.total(),
                512,
                b"MUSHA   TXT",
                &mut out,
                &mut |l, b| disk.read(l, b)
            ),
            Ok(1200)
        );
        for (i, b) in out.iter().enumerate() {
            assert_eq!(*b, (i * 17) as u8);
        }
    }
}
#[test]
fn cycles_short_chain_and_small_buffer() {
    for kind in 0..3 {
        let mut disk = Disk::new(512, 0);
        disk.root_cycle = kind == 0;
        disk.file_cycle = kind == 1;
        disk.short = kind == 2;
        let mut out = [0u8; 1200];
        assert_eq!(
            read_root(disk.total(), 512, b"MUSHA   TXT", &mut out, &mut |l, b| {
                disk.read(l, b)
            }),
            Err(Error::Corrupt)
        );
    }
    let disk = Disk::new(512, 0);
    assert_eq!(
        read_root(
            disk.total(),
            512,
            b"MUSHA   TXT",
            &mut [0u8; 10],
            &mut |l, b| disk.read(l, b)
        ),
        Err(Error::TooLarge)
    );
}
#[test]
fn geometry_partition_limits_and_active_fat() {
    let mut disk = Disk::new(512, 2048);
    let blocks = disk.start + disk.total();
    disk.bad_bpb = true;
    assert_eq!(
        read_root(
            blocks,
            512,
            b"MUSHA   TXT",
            &mut [0u8; 1200],
            &mut |l, b| disk.read(l, b)
        ),
        Err(Error::Corrupt)
    );
    disk.bad_bpb = false;
    disk.active = 0x81;
    assert_eq!(
        read_root(
            blocks,
            512,
            b"MUSHA   TXT",
            &mut [0u8; 1200],
            &mut |l, b| disk.read(l, b)
        ),
        Ok(1200)
    );
    disk.active = 0x82;
    assert_eq!(
        read_root(
            blocks,
            512,
            b"MUSHA   TXT",
            &mut [0u8; 1200],
            &mut |l, b| disk.read(l, b)
        ),
        Err(Error::Corrupt)
    );
    disk.active = 0;
    assert_eq!(
        read_root(
            blocks - 1,
            512,
            b"MUSHA   TXT",
            &mut [0u8; 1200],
            &mut |l, b| disk.read(l, b)
        ),
        Err(Error::Corrupt)
    );
}
#[test]
fn missing_file_and_io_failure() {
    let disk = Disk::new(512, 0);
    assert_eq!(
        read_root(
            disk.total(),
            512,
            b"ABSENT  TXT",
            &mut [0u8; 1200],
            &mut |l, b| disk.read(l, b)
        ),
        Err(Error::NotFound)
    );
    assert_eq!(
        read_root(
            disk.total(),
            512,
            b"MUSHA   TXT",
            &mut [0u8; 1200],
            &mut |_, _| Err(Error::Io)
        ),
        Err(Error::Io)
    );
}

#[test]
fn sectors_4096_and_large_fragmented_file() {
    let disk = Disk::new(4096, 0);
    let mut out = [0u8; 9000];
    assert_eq!(
        read_root(disk.total(), 4096, b"MUSHA   TXT", &mut out, &mut |l, b| {
            disk.read(l, b)
        }),
        Ok(9000)
    );
    for (i, b) in out.iter().enumerate() {
        assert_eq!(*b, (i * 17) as u8);
    }
}
