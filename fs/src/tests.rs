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

fn put64(b: &mut [u8], n: usize, v: u64) {
    b[n..n + 8].copy_from_slice(&v.to_le_bytes());
}
// Independent test CRC implementation, also checked against a published vector.
fn crc32(b: &[u8]) -> u32 {
    let mut crc = 0xffffffff;
    for &v in b {
        crc ^= v as u32;
        for _ in 0..8 {
            if crc & 1 != 0 {
                crc = (crc >> 1) ^ 0xedb88320;
            } else {
                crc >>= 1;
            }
        }
    }
    !crc
}
struct GptDisk {
    disk: Disk,
    blocks: u64,
    primary: [u8; 4096],
    backup: [u8; 4096],
    array: [u8; 16384],
    hybrid: bool,
    bad_backup_array: bool,
}
impl GptDisk {
    fn new(size: usize) -> Self {
        let disk = Disk::new(size, 2048);
        let blocks = disk.start + disk.total() + 16384 / size as u64 + 1;
        let mut result = Self {
            disk,
            blocks,
            primary: [0; 4096],
            backup: [0; 4096],
            array: [0; 16384],
            hybrid: false,
            bad_backup_array: false,
        };
        result.array[..16].copy_from_slice(&[
            0x28, 0x73, 0x2a, 0xc1, 0x1f, 0xf8, 0xd2, 0x11, 0xba, 0x4b, 0, 0xa0, 0xc9, 0x3e, 0xc9,
            0x3b,
        ]);
        result.array[16] = 2;
        put64(&mut result.array, 32, 2048);
        put64(&mut result.array, 40, 2048 + result.disk.total() - 1);
        let array_sectors = 16384 / size as u64;
        for (b, mine, other, array) in [
            (&mut result.primary, 1, blocks - 1, 2),
            (
                &mut result.backup,
                blocks - 1,
                1,
                blocks - 1 - array_sectors,
            ),
        ] {
            b[..8].copy_from_slice(b"EFI PART");
            put32(b, 8, 0x10000);
            put32(b, 12, 92);
            put64(b, 24, mine);
            put64(b, 32, other);
            put64(b, 40, 2 + array_sectors);
            put64(b, 48, blocks - array_sectors - 2);
            b[56] = 1;
            put64(b, 72, array);
            put32(b, 80, 128);
            put32(b, 84, 128);
        }
        result.checksums();
        result
    }
    fn checksums(&mut self) {
        let crc = crc32(&self.array);
        for b in [&mut self.primary, &mut self.backup] {
            put32(b, 88, crc);
            put32(b, 16, 0);
            let value = crc32(&b[..92]);
            put32(b, 16, value);
        }
    }
    fn read(&self, lba: u64, b: &mut [u8]) -> Result<(), Error> {
        assert!(lba < self.blocks);
        let n = b.len();
        if lba == 0 {
            b.fill(0);
            b[510..512].copy_from_slice(&[0x55, 0xaa]);
            b[450] = 0xee;
            put32(b, 454, 1);
            put32(b, 458, (self.blocks - 1) as u32);
            if self.hybrid {
                b[466] = 12;
            }
        } else if lba == 1 {
            b.copy_from_slice(&self.primary[..n]);
        } else if lba == self.blocks - 1 {
            b.copy_from_slice(&self.backup[..n]);
        } else if (2..2 + 16384 / n as u64).contains(&lba) {
            let offset = (lba - 2) as usize * n;
            b.copy_from_slice(&self.array[offset..offset + n]);
        } else if (self.blocks - 1 - 16384 / n as u64..self.blocks - 1).contains(&lba) {
            let offset = (lba - (self.blocks - 1 - 16384 / n as u64)) as usize * n;
            b.copy_from_slice(&self.array[offset..offset + n]);
            if self.bad_backup_array {
                b[0] ^= 1;
            }
        } else {
            self.disk.read(lba, b)?;
        }
        Ok(())
    }
    fn file(&self) -> Result<usize, Error> {
        let mut out = [0; 9000];
        read_root(
            self.blocks,
            self.disk.size,
            b"MUSHA   TXT",
            &mut out,
            &mut |l, b| self.read(l, b),
        )
    }
}
#[test]
fn gpt_primary_backup_512_and_4096() {
    assert_eq!(crc32(b"123456789"), 0xcbf43926);
    for size in [512, 4096] {
        let disk = GptDisk::new(size);
        assert_eq!(disk.file(), Ok(if size == 512 { 1200 } else { 9000 }));
    }
}
#[test]
fn gpt_rejects_crc_corruption_and_hybrid_mbr() {
    for kind in 0..5 {
        let mut disk = GptDisk::new(512);
        match kind {
            0 => disk.primary[56] ^= 1,
            1 => disk.backup[56] ^= 1,
            2 => disk.array[60] ^= 1,
            3 => disk.bad_backup_array = true,
            _ => disk.hybrid = true,
        }
        assert_eq!(disk.file(), Err(Error::Corrupt), "case {kind}");
    }
}
#[test]
fn gpt_rejects_ranges_duplicates_overlap_and_header_mismatch() {
    for kind in 0..10 {
        let mut disk = GptDisk::new(512);
        match kind {
            0 => put64(&mut disk.array, 32, 1),
            1 => put64(&mut disk.array, 40, u64::MAX),
            2 => put64(&mut disk.array, 32, 2048 + disk.disk.total()),
            3 => disk.array[16..32].fill(0),
            4 | 5 => {
                disk.array[128] = 1;
                disk.array[144] = if kind == 4 { 2 } else { 3 };
                put64(&mut disk.array, 160, 2048);
                put64(&mut disk.array, 168, 2049);
            }
            6 => put64(&mut disk.primary, 72, u64::MAX),
            7 => put64(&mut disk.backup, 32, 2),
            8 => disk.backup[56] = 3,
            _ => put64(&mut disk.primary, 40, u64::MAX),
        }
        disk.checksums();
        assert_eq!(disk.file(), Err(Error::Corrupt), "case {kind}");
    }
}
#[test]
fn gpt_bounded_format_and_missing_esp() {
    let mut disk = GptDisk::new(512);
    put32(&mut disk.primary, 80, u32::MAX);
    disk.checksums();
    assert_eq!(disk.file(), Err(Error::Unsupported));
    let mut disk = GptDisk::new(512);
    disk.array[0] = 1;
    disk.checksums();
    assert_eq!(disk.file(), Err(Error::Unsupported));
}
