// Copyright 2026 Hirofumi Iwasaki
// SPDX-License-Identifier: Apache-2.0
use musha_fs::{
    Error,
    exfat::{FileSystem, Workspace},
};
use std::{path::Path, process::Command, sync::OnceLock};
fn fixture(case: &str) -> Vec<u8> {
    fixture_size(case, 512)
}
fn fixture_size(case: &str, size: usize) -> Vec<u8> {
    let script = Path::new(env!("CARGO_MANIFEST_DIR")).join("../tools/make-exfat-fixture.py");
    let out=Command::new("python3").arg("-c").arg("import importlib.util,sys; s=importlib.util.spec_from_file_location('fixture',sys.argv[1]); m=importlib.util.module_from_spec(s); s.loader.exec_module(m); sys.stdout.buffer.write(m.build(sys.argv[2],int(sys.argv[3]))[0])").arg(script).arg(case).arg(size.to_string()).output().expect("python3 fixture generator");
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    out.stdout
}
fn normal() -> Vec<u8> {
    static IMAGE: OnceLock<Vec<u8>> = OnceLock::new();
    IMAGE.get_or_init(|| fixture("normal")).clone()
}
fn reader(image: &[u8]) -> impl FnMut(u64, &mut [u8]) -> Result<(), Error> + '_ {
    move |lba, b| {
        let offset = usize::try_from(lba)
            .map_err(|_| Error::Io)?
            .checked_mul(b.len())
            .ok_or(Error::Io)?;
        let data = image.get(offset..offset + b.len()).ok_or(Error::Io)?;
        b.copy_from_slice(data);
        Ok(())
    }
}
fn mount_result(image: &[u8]) -> Result<(), Error> {
    let mut w = Box::new(Workspace::new());
    FileSystem::mount(
        (image.len() / 512) as u64,
        512,
        0,
        (image.len() / 512) as u64,
        reader(image),
        &mut w,
    )
    .map(|_| ())
}
fn put16(b: &mut [u8], i: usize, v: u16) {
    b[i..i + 2].copy_from_slice(&v.to_le_bytes());
}
fn put32(b: &mut [u8], i: usize, v: u32) {
    b[i..i + 4].copy_from_slice(&v.to_le_bytes());
}
fn put64(b: &mut [u8], i: usize, v: u64) {
    b[i..i + 8].copy_from_slice(&v.to_le_bytes());
}
fn boot_sum(b: &mut [u8]) {
    let mut sum = 0u32;
    for (i, &v) in b[..5632].iter().enumerate() {
        if !matches!(i, 106 | 107 | 112) {
            sum = sum.rotate_right(1).wrapping_add(v as u32);
        }
    }
    for off in (5632..6144).step_by(4) {
        put32(b, off, sum);
    }
}
fn set_sum(b: &mut [u8], off: usize) {
    let len = (b[off + 1] as usize + 1) * 32;
    let mut sum = 0u16;
    for (i, &v) in b[off..off + len].iter().enumerate() {
        if !matches!(i, 2 | 3) {
            sum = sum.rotate_right(1).wrapping_add(v as u16);
        }
    }
    put16(b, off + 2, sum);
}
#[test]
fn listing_and_chunked_contents_match_independent_payloads() {
    let image = normal();
    let mut w = Box::new(Workspace::new());
    let mut fs = FileSystem::mount(4160, 512, 0, 4160, reader(&image), &mut w).unwrap();
    let names: Vec<_> = fs
        .entries()
        .iter()
        .map(|e| String::from_utf16(e.name()).unwrap())
        .collect();
    assert_eq!(
        names,
        [
            "MUSHA.TXT",
            "FRAGMENT.BIN",
            "Long name for exFAT test.txt",
            "EMPTY.TXT",
            "ZEROTAIL.BIN"
        ]
    );
    let mut fragment: Vec<_> = (0..1280).map(|n| (n % 256) as u8).collect();
    fragment.extend_from_slice(b"end");
    let mut tail = b"valid".to_vec();
    tail.resize(700, 0);
    let expected = [
        b"Hello Musha-OS!\n".to_vec(),
        fragment,
        b"Long file name\n".to_vec(),
        vec![],
        tail,
    ];
    for (index, payload) in expected.iter().enumerate() {
        assert_eq!(fs.entries()[index].len(), payload.len() as u64);
        for chunk in [1, 7, 511, 512, 513, 4096] {
            let mut got = vec![];
            loop {
                let mut b = vec![0xcc; chunk];
                let n = fs.read_at(index, got.len() as u64, &mut b).unwrap();
                assert!(b[n..].iter().all(|&x| x == 0xcc));
                got.extend_from_slice(&b[..n]);
                if n == 0 {
                    break;
                }
            }
            assert_eq!(&got, payload);
        }
    }
    let mut b = [0xcc; 8];
    assert_eq!(fs.read_at(0, u64::MAX, &mut b), Ok(0));
    assert_eq!(b, [0xcc; 8]);
    assert_eq!(fs.read_at(99, 0, &mut b), Err(Error::NotFound));
    assert_eq!(fs.read_at(0, 0, &mut [0; 4097]), Err(Error::Limit));
}
#[test]
fn undefined_disk_bytes_are_zeroed() {
    let image = fixture("valid-tail");
    let mut w = Box::new(Workspace::new());
    let mut fs = FileSystem::mount(4160, 512, 0, 4160, reader(&image), &mut w).unwrap();
    let mut b = [0xcc; 701];
    assert_eq!(fs.read_at(4, 0, &mut b), Ok(700));
    assert_eq!(&b[..5], b"valid");
    assert!(b[5..700].iter().all(|&x| x == 0));
    assert_eq!(b[700], 0xcc);
    assert_eq!(fs.read_at(4, 4, &mut b), Ok(696));
    assert_eq!(b[0], b'd');
    assert!(b[1..696].iter().all(|&x| x == 0));
}
#[test]
fn fixture_corruptions_fail_without_publishing_bytes() {
    for case in ["boot-checksum", "entry-checksum"] {
        assert_eq!(mount_result(&fixture(case)), Err(Error::Corrupt), "{case}");
    }
    for (case, index) in [("chain-cycle", 1), ("bitmap-free", 0)] {
        let image = fixture(case);
        let mut w = Box::new(Workspace::new());
        let mut fs = FileSystem::mount(4160, 512, 0, 4160, reader(&image), &mut w).unwrap();
        let mut out = [0xcc; 4096];
        assert_eq!(fs.read_at(index, 0, &mut out), Err(Error::Corrupt));
        assert_eq!(out, [0xcc; 4096]);
    }
}
#[test]
fn geometry_rejected_even_with_valid_checksum() {
    for (off, value) in [(80, 0), (84, 0), (88, 25), (92, 0), (92, u32::MAX), (96, 1)] {
        let mut image = normal();
        put32(&mut image, off, value);
        boot_sum(&mut image);
        assert_eq!(mount_result(&image), Err(Error::Corrupt), "{off}");
    }
    for shift in [26, 255] {
        let mut image = normal();
        image[109] = shift;
        boot_sum(&mut image);
        assert_eq!(mount_result(&image), Err(Error::Corrupt));
    }
    let mut image = normal();
    put64(&mut image, 72, u64::MAX);
    boot_sum(&mut image);
    assert_eq!(mount_result(&image), Err(Error::Corrupt));
    let mut w = Box::new(Workspace::new());
    assert!(matches!(
        FileSystem::mount(u64::MAX, 512, u64::MAX - 10, 24, reader(&image), &mut w),
        Err(Error::Corrupt)
    ));
}
#[test]
fn unsupported_and_dirty_media() {
    for (off, value) in [(104, 1), (110, 2), (108, 10)] {
        let mut image = normal();
        image[off] = value;
        boot_sum(&mut image);
        assert_eq!(mount_result(&image), Err(Error::Unsupported));
    }
    for flag in [1, 2, 4] {
        let mut image = normal();
        image[106] = flag;
        assert_eq!(mount_result(&image), Err(Error::Corrupt));
    }
}
#[test]
fn metadata_cycles_missing_bitmap_and_unknown_critical() {
    for (n, value) in [(2, 2), (4, 4), (3, 3)] {
        let mut image = normal();
        put32(&mut image, 24 * 512 + n * 4, value);
        assert_eq!(mount_result(&image), Err(Error::Corrupt));
    }
    let mut image = normal();
    image[64 * 512] = 1;
    assert_eq!(mount_result(&image), Err(Error::Corrupt));
    let mut image = normal();
    image[64 * 512] = 0x84;
    assert_eq!(mount_result(&image), Err(Error::Unsupported));
}
#[test]
fn file_short_long_bad_and_reserved_chains() {
    for next in [0, 1, 0xfffffff7, 0xffffffff, 4098] {
        let mut image = normal();
        put32(&mut image, 24 * 512 + 270 * 4, next);
        let mut w = Box::new(Workspace::new());
        let mut fs = FileSystem::mount(4160, 512, 0, 4160, reader(&image), &mut w).unwrap();
        assert_eq!(fs.read_at(1, 0, &mut [0; 10]), Err(Error::Corrupt));
    }
    let mut image = normal();
    put32(&mut image, 24 * 512 + 271 * 4, 260);
    let mut w = Box::new(Workspace::new());
    let mut fs = FileSystem::mount(4160, 512, 0, 4160, reader(&image), &mut w).unwrap();
    assert_eq!(fs.read_at(1, 0, &mut [0; 10]), Err(Error::Corrupt));
}
#[test]
fn invalid_name_hash_and_valid_length() {
    let file = 64 * 512 + 64;
    for (off, value) in [(file + 36, 0), (file + 64 + 2, 0), (file + 64 + 2, 0xd800)] {
        let mut image = normal();
        put16(&mut image, off, value);
        set_sum(&mut image, file);
        assert_eq!(mount_result(&image), Err(Error::Corrupt));
    }
    let mut image = normal();
    put64(&mut image, file + 40, 17);
    set_sum(&mut image, file);
    assert_eq!(mount_result(&image), Err(Error::Corrupt));
}
#[test]
fn oversized_file_is_listed_but_not_read() {
    let mut image = normal();
    let file = 64 * 512 + 64;
    put64(&mut image, file + 56, 64 * 1024 * 1024 + 1);
    set_sum(&mut image, file);
    let mut w = Box::new(Workspace::new());
    let mut fs = FileSystem::mount(4160, 512, 0, 4160, reader(&image), &mut w).unwrap();
    assert_eq!(fs.entries()[0].len(), 64 * 1024 * 1024 + 1);
    assert_eq!(fs.read_at(0, 0, &mut [0; 10]), Err(Error::TooLarge));
}
#[test]
fn compressed_upcase_and_checksum() {
    let mut image = normal();
    let mut words = vec![0xffffu16, 97];
    words.extend(65..=90);
    words.extend([0xffff, 65413]);
    let bytes: Vec<u8> = words.iter().flat_map(|v| v.to_le_bytes()).collect();
    image[66 * 512..66 * 512 + bytes.len()].copy_from_slice(&bytes);
    let sum = bytes
        .iter()
        .fold(0u32, |s, &v| s.rotate_right(1).wrapping_add(v as u32));
    put32(&mut image, 64 * 512 + 36, sum);
    put64(&mut image, 64 * 512 + 56, bytes.len() as u64);
    put32(&mut image, 24 * 512 + 4 * 4, 0xffffffff);
    assert_eq!(mount_result(&image), Ok(()));
    image[66 * 512] ^= 1;
    assert_eq!(mount_result(&image), Err(Error::Corrupt));
}
#[test]
fn read_io_error_keeps_output_unchanged() {
    let image = normal();
    let mut w = Box::new(Workspace::new());
    let mut backend = reader(&image);
    let read = move |lba, b: &mut [u8]| {
        if lba == 64 + 270 - 2 {
            Err(Error::Io)
        } else {
            backend(lba, b)
        }
    };
    let mut fs = FileSystem::mount(4160, 512, 0, 4160, read, &mut w).unwrap();
    let mut out = [0xcc; 100];
    assert_eq!(fs.read_at(1, 0, &mut out), Err(Error::Io));
    assert_eq!(out, [0xcc; 100]);
}
#[test]
fn truncated_backend_returns_io() {
    let mut image = normal();
    image.truncate(70 * 512);
    let mut w = Box::new(Workspace::new());
    assert!(matches!(
        FileSystem::mount(4160, 512, 0, 4160, reader(&image), &mut w),
        Err(Error::Io)
    ));
}

#[test]
fn sectors_4096_and_explicit_partition_region() {
    let image = fixture_size("normal", 4096);
    let mut w = Box::new(Workspace::new());
    let mut fs = FileSystem::mount(4160, 4096, 0, 4160, reader(&image), &mut w).unwrap();
    assert_eq!(fs.entries().len(), 5);
    assert_eq!(fs.entries()[1].len(), 8451);
    let mut result = vec![];
    loop {
        let mut b = [0; 4096];
        let n = fs.read_at(1, result.len() as u64, &mut b).unwrap();
        result.extend_from_slice(&b[..n]);
        if n == 0 {
            break;
        }
    }
    let mut expected: Vec<_> = (0..8448).map(|i| (i % 256) as u8).collect();
    expected.extend_from_slice(b"end");
    assert_eq!(result, expected);
    let mut part = normal();
    put64(&mut part, 64, 2048);
    boot_sum(&mut part);
    let mut disk = vec![0; 2048 * 512];
    disk.extend_from_slice(&part);
    let mut fs = FileSystem::mount(6208, 512, 2048, 4160, reader(&disk), &mut w).unwrap();
    let mut b = [0; 16];
    assert_eq!(fs.read_at(0, 0, &mut b), Ok(16));
    assert_eq!(&b, b"Hello Musha-OS!\n");
}
#[test]
fn root_budget_stops_long_allocation_chain() {
    let mut image = normal();
    // The directory end marker stays intact, so mount's separate chain check
    // must catch an allocation exceeding the 1MiB root budget.
    image[65 * 512..66 * 512].fill(0xff);
    // Keep the existing bitmap and up-case chains; root uses free clusters.
    put32(&mut image, 24 * 512 + 2 * 4, 1000);
    for n in 1000..=3050 {
        put32(
            &mut image,
            24 * 512 + n * 4,
            if n == 3050 { 0xffffffff } else { n as u32 + 1 },
        );
    }
    // root's final file entry crosses to 600 in the source; end it before that
    // unused portion so system discovery itself succeeds.
    image[64 * 512 + 480] = 0;
    assert_eq!(mount_result(&image), Err(Error::Limit));
}
#[test]
fn workspace_fits_metadata_budget() {
    assert!(std::mem::size_of::<Workspace>() < 256 * 1024);
}

#[test]
fn reserved_fat_entries_are_validated() {
    for off in [24 * 512, 24 * 512 + 4] {
        let mut image = normal();
        put32(&mut image, off, 0);
        assert_eq!(mount_result(&image), Err(Error::Corrupt));
    }
}
