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

const BASIC: [u8; 16] = [
    0xa2, 0xa0, 0xd0, 0xeb, 0xe5, 0xb9, 0x33, 0x44, 0x87, 0xc0, 0x68, 0xb6, 0xb7, 0x26, 0x99, 0xc7,
];
const ESP: [u8; 16] = [
    0x28, 0x73, 0x2a, 0xc1, 0x1f, 0xf8, 0xd2, 0x11, 0xba, 0x4b, 0, 0xa0, 0xc9, 0x3e, 0xc9, 0x3b,
];
fn crc(bytes: &[u8]) -> u32 {
    let mut c = !0u32;
    for &b in bytes {
        c ^= b as u32;
        for _ in 0..8 {
            c = if c & 1 != 0 {
                (c >> 1) ^ 0xedb88320
            } else {
                c >> 1
            };
        }
    }
    !c
}
fn relocate(image: &mut [u8], size: usize, start: u64) {
    for base in [0, 12 * size] {
        put64(image, base + 64, start);
        let mut sum = 0u32;
        for (i, &b) in image[base..base + 11 * size].iter().enumerate() {
            if !matches!(i, 106 | 107 | 112) {
                sum = sum.rotate_right(1).wrapping_add(b as u32);
            }
        }
        for off in (base + 11 * size..base + 12 * size).step_by(4) {
            put32(image, off, sum);
        }
    }
}
fn gpt_resign(d: &mut [u8], size: usize) {
    let blocks = d.len() / size;
    let backup = (blocks - 1 - 16384 / size) * size;
    let table = d[2 * size..2 * size + 16384].to_vec();
    d[backup..backup + 16384].copy_from_slice(&table);
    let checksum = crc(&table);
    for off in [size, (blocks - 1) * size] {
        put32(d, off + 88, checksum);
        put32(d, off + 16, 0);
        let c = crc(&d[off..off + 92]);
        put32(d, off + 16, c);
    }
}
fn partition_disk(size: usize, gpt: bool, multiple: bool) -> Vec<u8> {
    let mut volume = if size == 512 {
        normal()
    } else {
        fixture_size("normal", size)
    };
    let end = 2048 + 4160 * if multiple { 2 } else { 1 };
    let blocks = end + if gpt { 1 + 16384 / size } else { 0 };
    let mut d = vec![0; blocks * size];
    d[510..512].copy_from_slice(&[0x55, 0xaa]);
    relocate(&mut volume, size, 2048);
    d[2048 * size..6208 * size].copy_from_slice(&volume);
    if multiple {
        relocate(&mut volume, size, 6208);
        d[6208 * size..end * size].copy_from_slice(&volume);
    }
    if !gpt {
        // A FAT32-typed partition and a non-exFAT type-07 partition coexist.
        for (i, (kind, first, len)) in [(0x0c, 64, 64), (7, 128, 64), (7, 2048, 4160)]
            .into_iter()
            .enumerate()
        {
            let off = 446 + i * 16;
            d[off + 4] = kind;
            put32(&mut d, off + 8, first);
            put32(&mut d, off + 12, len);
        }
        if multiple {
            d[498] = 7;
            put32(&mut d, 502, 6208);
            put32(&mut d, 506, 4160);
        }
    } else {
        d[450] = 0xee;
        put32(&mut d, 454, 1);
        put32(&mut d, 458, (blocks - 1) as u32);
        for (i, (kind, first, last)) in [(ESP, 64, 127), (BASIC, 128, 191), (BASIC, 2048, 6207)]
            .into_iter()
            .enumerate()
        {
            let off = 2 * size + i * 128;
            d[off..off + 16].copy_from_slice(&kind);
            d[off + 16] = i as u8 + 1;
            put64(&mut d, off + 32, first);
            put64(&mut d, off + 40, last);
        }
        if multiple {
            let off = 2 * size + 384;
            d[off..off + 16].copy_from_slice(&BASIC);
            d[off + 16] = 4;
            put64(&mut d, off + 32, 6208);
            put64(&mut d, off + 40, end as u64 - 1);
        }
        for (current, other, array) in [
            (1, blocks - 1, 2),
            (blocks - 1, 1, blocks - 1 - 16384 / size),
        ] {
            let off = current * size;
            d[off..off + 8].copy_from_slice(b"EFI PART");
            put32(&mut d, off + 8, 0x10000);
            put32(&mut d, off + 12, 92);
            put64(&mut d, off + 24, current as u64);
            put64(&mut d, off + 32, other as u64);
            put64(&mut d, off + 40, (2 + 16384 / size) as u64);
            put64(&mut d, off + 48, (end - 1) as u64);
            d[off + 56] = 42;
            put64(&mut d, off + 72, array as u64);
            put32(&mut d, off + 80, 128);
            put32(&mut d, off + 84, 128);
        }
        gpt_resign(&mut d, size);
    }
    d
}
fn discover_result(d: &[u8], size: usize) -> Result<(), Error> {
    let mut w = Box::new(Workspace::new());
    let mut fs = FileSystem::discover((d.len() / size) as u64, size, reader(d), &mut w)?;
    let mut bytes = [0; 16];
    assert_eq!(fs.read_at(0, 0, &mut bytes), Ok(16));
    assert_eq!(&bytes, b"Hello Musha-OS!\n");
    Ok(())
}
#[test]
fn autodiscovery_superfloppy_mbr_gpt_and_non_exfat_candidates() {
    assert_eq!(discover_result(&normal(), 512), Ok(()));
    for size in [512, 4096] {
        for gpt in [false, true] {
            assert_eq!(
                discover_result(&partition_disk(size, gpt, false), size),
                Ok(())
            );
        }
    }
}
#[test]
fn multiple_exfat_volumes_are_ambiguous() {
    for gpt in [false, true] {
        assert_eq!(
            discover_result(&partition_disk(512, gpt, true), 512),
            Err(Error::Unsupported)
        );
    }
}
#[test]
fn gpt_damage_overlap_duplicate_and_hybrid_are_rejected() {
    let original = partition_disk(512, true, false);
    for off in [
        512 + 16,
        1024 + 56,
        original.len() - 512 + 16,
        original.len() - 33 * 512 + 56,
    ] {
        let mut d = original.clone();
        d[off] ^= 1;
        assert_eq!(discover_result(&d, 512), Err(Error::Corrupt));
    }
    let mut d = original.clone();
    put64(&mut d, 1024 + 128 + 32, 100);
    gpt_resign(&mut d, 512);
    assert_eq!(discover_result(&d, 512), Err(Error::Corrupt));
    let mut d = original.clone();
    d[1024 + 128 + 16] = 1;
    gpt_resign(&mut d, 512);
    assert_eq!(discover_result(&d, 512), Err(Error::Corrupt));
    let mut d = original;
    d[466] = 7;
    put32(&mut d, 470, 2048);
    put32(&mut d, 474, 4160);
    assert_eq!(discover_result(&d, 512), Err(Error::Corrupt));
}
#[test]
fn mbr_overlap_bounds_bad_status_and_extended_are_rejected() {
    for (off, value) in [(470, 64), (486, 0), (490, u32::MAX)] {
        let mut d = partition_disk(512, false, false);
        put32(&mut d, off, value);
        assert_eq!(discover_result(&d, 512), Err(Error::Corrupt));
    }
    let mut d = partition_disk(512, false, false);
    d[478] = 1;
    assert_eq!(discover_result(&d, 512), Err(Error::Corrupt));
    let mut d = partition_disk(512, false, false);
    d[450] = 0x0f;
    assert_eq!(discover_result(&d, 512), Err(Error::Unsupported));
}
#[test]
fn no_exfat_and_damaged_selected_volume_do_not_fall_back() {
    for gpt in [false, true] {
        let mut d = partition_disk(512, gpt, false);
        d[2048 * 512 + 3] = b'N';
        assert_eq!(discover_result(&d, 512), Err(Error::Unsupported));
        let mut d = partition_disk(512, gpt, false);
        d[2048 * 512 + 100] ^= 1;
        assert_eq!(discover_result(&d, 512), Err(Error::Corrupt));
    }
}

fn empty_entry(name: &[u16]) -> Vec<u8> {
    assert!(!name.is_empty() && name.len() <= 255);
    let mut entry = vec![0; 32 * (2 + name.len().div_ceil(15))];
    entry[0] = 0x85;
    entry[1] = (entry.len() / 32 - 1) as u8;
    entry[32] = 0xc0;
    entry[33] = 1;
    entry[35] = name.len() as u8;
    let mut hash = 0u16;
    for (i, &unit) in name.iter().enumerate() {
        let off = 64 + (i / 15) * 32;
        entry[off] = 0xc1;
        put16(&mut entry, off + 2 + (i % 15) * 2, unit);
        let upper = if (97..=122).contains(&unit) {
            unit - 32
        } else {
            unit
        };
        for b in upper.to_le_bytes() {
            hash = hash.rotate_right(1).wrapping_add(b as u16);
        }
    }
    put16(&mut entry, 36, hash);
    set_sum(&mut entry, 0);
    entry
}
fn root_with(entries: &[Vec<u8>]) -> Vec<u8> {
    let mut image = normal();
    let mut root = image[64 * 512..64 * 512 + 64].to_vec();
    for entry in entries {
        root.extend_from_slice(entry);
    }
    root.resize(root.len() + 32, 0);
    let mut chain = vec![2];
    chain.extend((600..).take(root.len().div_ceil(512) - 1));
    for (i, &n) in chain.iter().enumerate() {
        put32(
            &mut image,
            24 * 512 + n * 4,
            chain
                .get(i + 1)
                .copied()
                .map(|x| x as u32)
                .unwrap_or(0xffffffff),
        );
        image[65 * 512 + (n - 2) / 8] |= 1 << ((n - 2) % 8);
        let off = (64 + n - 2) * 512;
        image[off..off + 512].fill(0);
        let chunk = &root[i * 512..root.len().min((i + 1) * 512)];
        image[off..off + chunk.len()].copy_from_slice(chunk);
    }
    image
}
#[test]
fn maximum_utf16_name_crosses_sectors_and_preserves_unicode() {
    let mut name = vec![0x65e5];
    name.extend(std::iter::repeat_n(b'a' as u16, 252));
    name.extend([0xd83d, 0xde80]);
    assert_eq!(name.len(), 255);
    let image = root_with(&[empty_entry(&name)]);
    let mut w = Box::new(Workspace::new());
    let mut fs = FileSystem::discover(4160, 512, reader(&image), &mut w).unwrap();
    assert_eq!(fs.entries()[0].name(), name);
    assert_eq!(fs.read_at(0, 0, &mut [0xcc; 8]), Ok(0));
    let mut broken = name;
    broken[254] = b'a' as u16;
    assert_eq!(
        mount_result(&root_with(&[empty_entry(&broken)])),
        Err(Error::Corrupt)
    );
}
#[test]
fn catalogue_limit_accepts_128_and_rejects_129() {
    let entries: Vec<_> = (0..129)
        .map(|i| empty_entry(&format!("FILE{i:03}.TXT").encode_utf16().collect::<Vec<_>>()))
        .collect();
    let image = root_with(&entries[..128]);
    let mut w = Box::new(Workspace::new());
    let fs = FileSystem::discover(4160, 512, reader(&image), &mut w).unwrap();
    assert_eq!(fs.entries().len(), 128);
    assert_eq!(mount_result(&root_with(&entries)), Err(Error::Limit));
}
#[test]
fn case_folded_duplicate_names_are_rejected() {
    let entries: Vec<_> = ["alpha.txt", "ALPHA.TXT"]
        .iter()
        .map(|n| empty_entry(&n.encode_utf16().collect::<Vec<_>>()))
        .collect();
    assert_eq!(
        mount_result(&root_with(&entries)),
        Err(Error::DuplicateName([
            65, 76, 80, 72, 65, 46, 84, 88, 84, 0, 0, 0
        ]))
    );
}
#[test]
fn benign_secondary_is_skipped_critical_secondary_is_rejected() {
    let base = empty_entry(&[b'A' as u16]);
    for (kind, expected) in [(0xe0, Ok(())), (0xc2, Err(Error::Unsupported))] {
        let mut entry = base.clone();
        let off = entry.len();
        entry.resize(off + 32, 0);
        entry[off] = kind;
        entry[1] += 1;
        set_sum(&mut entry, 0);
        assert_eq!(mount_result(&root_with(&[entry])), expected);
    }
}
#[test]
fn malformed_secondary_counts_and_missing_names_are_rejected() {
    for count in [0, 1, 3, 255] {
        let mut entry = empty_entry(&[65]);
        entry[1] = count;
        assert_eq!(mount_result(&root_with(&[entry])), Err(Error::Corrupt));
    }
    let mut entry = empty_entry(&[65]);
    entry[64] = 0xc2;
    set_sum(&mut entry, 0);
    assert_eq!(mount_result(&root_with(&[entry])), Err(Error::Corrupt));
}

#[test]
fn mixed_esp_and_exfat_prefers_exfat_and_rejects_fat32_discovery() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../tools/make-fat32-fixture.py");
    let out=Command::new("python3").arg("-c").arg("import importlib.util,sys,tempfile,pathlib; s=importlib.util.spec_from_file_location('fat',sys.argv[1]); m=importlib.util.module_from_spec(s); s.loader.exec_module(m); t=tempfile.TemporaryDirectory(); p=pathlib.Path(t.name)/'fat.img'; m.create(p,'superfloppy'); sys.stdout.buffer.write(p.read_bytes())").arg(path).output().unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let mut fat = out.stdout;
    let fat_blocks = fat.len() / 512;
    put32(&mut fat, 28, 2048);
    put32(&mut fat, 6 * 512 + 28, 2048);
    let start = (2048 + fat_blocks).div_ceil(2048) * 2048;
    let end = start + 4160;
    let blocks = end + 33;
    let template = partition_disk(512, true, false);
    let mut disk = vec![0; blocks * 512];
    disk[..34 * 512].copy_from_slice(&template[..34 * 512]);
    put32(&mut disk, 458, (blocks - 1) as u32);
    put64(&mut disk, 1024 + 32, 2048);
    put64(&mut disk, 1024 + 40, (2048 + fat_blocks - 1) as u64);
    put64(&mut disk, 1024 + 128 + 32, start as u64);
    put64(&mut disk, 1024 + 128 + 40, (end - 1) as u64);
    disk[1024 + 256..1024 + 384].fill(0);
    put64(&mut disk, 512 + 32, (blocks - 1) as u64);
    put64(&mut disk, 512 + 48, (end - 1) as u64);
    let header = disk[512..1024].to_vec();
    let backup = (blocks - 1) * 512;
    disk[backup..backup + 512].copy_from_slice(&header);
    put64(&mut disk, backup + 24, (blocks - 1) as u64);
    put64(&mut disk, backup + 32, 1);
    put64(&mut disk, backup + 72, (blocks - 33) as u64);
    disk[2048 * 512..(2048 + fat_blocks) * 512].copy_from_slice(&fat);
    let mut exfat = normal();
    relocate(&mut exfat, 512, start as u64);
    disk[start * 512..end * 512].copy_from_slice(&exfat);
    gpt_resign(&mut disk, 512);
    assert_eq!(discover_result(&disk, 512), Ok(()));
    let mut output = [0xcc; 4096];
    let mut read = reader(&disk);
    assert_eq!(
        musha_fs::read_root(blocks as u64, 512, b"MUSHA   TXT", &mut output, &mut read),
        Err(Error::Unsupported)
    );
    // Explicit mounting still verifies that the FAT32 fixture itself is valid.
    let mut fs = musha_fs::fat32::FileSystem::mount(
        blocks as u64,
        512,
        2048,
        fat_blocks as u64,
        reader(&disk),
    )
    .unwrap();
    let n = fs.read_at(0, 0, &mut output).unwrap();
    let mut expected = b"Hello Musha-OS!\n".to_vec();
    for _ in 0..45 {
        expected.extend_from_slice(b"FAT32 file read over USB.\n");
    }
    assert_eq!(&output[..n], expected);
}

#[test]
fn exfat_catalog_through_common_request_api() {
    use musha_api::stream::{Entry, Error as ApiError, Files, Format};
    let image = normal();
    let mut workspace = Box::new(Workspace::new());
    let mut fs = FileSystem::discover(
        (image.len() / 512) as u64,
        512,
        reader(&image),
        &mut workspace,
    )
    .unwrap();
    let catalog: Vec<Entry> = fs
        .entries()
        .iter()
        .map(|e| {
            let mut name = [0; 255];
            name[..e.name().len()].copy_from_slice(e.name());
            Entry {
                name,
                name_len: e.name().len() as u8,
                bytes: e.len(),
            }
        })
        .collect();
    let sources: Vec<_> = (0..catalog.len()).map(|i| fs.source(i).unwrap()).collect();
    let expected_files: Vec<_> = (0..catalog.len())
        .map(|i| {
            let mut bytes = vec![0; catalog[i].bytes as usize];
            let n = fs.read_at(i, 0, &mut bytes).unwrap();
            assert_eq!(n, bytes.len());
            bytes
        })
        .collect();
    drop(fs);
    drop(workspace);
    let mut service = Files::new();
    service.mount(Format::Exfat, &catalog).unwrap();
    let mut cursor = service.list_begin().unwrap();
    let mut index = 0;
    while let Some((id, entry)) = service.list_next(&mut cursor).unwrap() {
        let length = entry.bytes as usize;
        let expected = &expected_files[index];
        assert_eq!(expected.len(), length);
        let handle = service.open_id(id).unwrap();
        let mut actual = Vec::new();
        loop {
            let mut out = [0xa5; 7];
            match service.read(handle, &mut out) {
                Err(ApiError::Again) => {
                    let request = service.take_request().unwrap();
                    let mut owned = [0; 4096];
                    let mut operation = musha_fs::operation::Read::new(
                        sources[request.index()]
                            .begin_plan(request.offset(), request.length())
                            .unwrap(),
                    );
                    loop {
                        match operation.poll().unwrap() {
                            musha_fs::operation::Progress::Yield => {}
                            musha_fs::operation::Progress::Read(r) => {
                                assert_eq!(
                                    operation.poll(),
                                    Ok(musha_fs::operation::Progress::Pending)
                                );
                                assert_eq!(operation.finish(&mut owned), Ok(None));
                                let mut sector = [0; 4096];
                                reader(&image)(r.lba(), &mut sector[..r.bytes()]).unwrap();
                                assert!(operation.complete(r, Ok(&sector[..r.bytes()])));
                            }
                            musha_fs::operation::Progress::Complete => break,
                            _ => panic!(),
                        }
                    }
                    let n = operation.finish(&mut owned).unwrap().unwrap();
                    assert!(service.complete(request, Ok(&owned[..n])));
                }
                Ok(0) => break,
                Ok(n) => actual.extend_from_slice(&out[..n]),
                other => panic!("{other:?}"),
            }
        }
        assert_eq!(&actual, expected);
        service.close(handle).unwrap();
        index += 1;
    }
    assert_eq!(index, 5);
}

#[test]
fn mixed_basic_data_keeps_exfat_first_rule_and_fat32_ambiguity() {
    // Unsupported product layout: a valid FAT32 and a valid exFAT on one GPT disk.
    // Fix the existing discovery behavior, without introducing a partition picker.
    let size = 512;
    let fat_start = 8192usize;
    let fat_sectors = 512usize;
    let fat_total = 32 + 2 * fat_sectors + 65525;
    let end = fat_start + fat_total;
    let blocks = end + 33;
    let mut d = partition_disk(size, true, false);
    d.resize(blocks * size, 0);
    // Keep just the exFAT partition and one FAT32 Basic Data partition.
    let ex = d[1024 + 256..1024 + 384].to_vec();
    d[1024..1024 + 16384].fill(0);
    d[1024..1152].copy_from_slice(&ex);
    let off = 1152;
    d[off..off + 16].copy_from_slice(&BASIC);
    d[off + 16] = 9;
    put64(&mut d, off + 32, fat_start as u64);
    put64(&mut d, off + 40, (end - 1) as u64);
    put32(&mut d, 458, (blocks - 1) as u32);
    let primary = d[512..1024].to_vec();
    d[(blocks - 1) * size..blocks * size].copy_from_slice(&primary);
    for (current, other, array) in [(1, blocks - 1, 2), (blocks - 1, 1, blocks - 33)] {
        let off = current * size;
        put64(&mut d, off + 24, current as u64);
        put64(&mut d, off + 32, other as u64);
        put64(&mut d, off + 48, (end - 1) as u64);
        put64(&mut d, off + 72, array as u64);
    }
    let off = fat_start * size;
    d[off] = 0xeb;
    put16(&mut d, off + 11, 512);
    d[off + 13] = 1;
    put16(&mut d, off + 14, 32);
    d[off + 16] = 2;
    put32(&mut d, off + 32, fat_total as u32);
    put32(&mut d, off + 36, fat_sectors as u32);
    put32(&mut d, off + 44, 2);
    d[off + 510..off + 512].copy_from_slice(&[0x55, 0xaa]);
    for base in [fat_start + 32, fat_start + 32 + fat_sectors] {
        put32(&mut d, base * size, 0x0ffffff8);
        put32(&mut d, base * size + 4, 0xffffffff);
        put32(&mut d, base * size + 8, 0x0fffffff);
    }
    gpt_resign(&mut d, size);
    assert!(
        musha_fs::fat32::FileSystem::mount(
            blocks as u64,
            size,
            fat_start as u64,
            fat_total as u64,
            reader(&d)
        )
        .is_ok()
    );
    assert_eq!(discover_result(&d, size), Ok(()));
    assert!(matches!(
        musha_fs::fat32::FileSystem::discover(blocks as u64, size, reader(&d)),
        Err(Error::Unsupported)
    ));
    // Entry order cannot turn this into FAT32 selection.
    let first = d[1024..1152].to_vec();
    d.copy_within(1152..1280, 1024);
    d[1152..1280].copy_from_slice(&first);
    gpt_resign(&mut d, size);
    assert_eq!(discover_result(&d, size), Ok(()));
}
