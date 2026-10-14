#!/usr/bin/env python3
# Copyright 2026 Hirofumi Iwasaki
# SPDX-License-Identifier: Apache-2.0
"""Create a reproducible GPT/FAT32 UEFI image in a NEW regular file only."""
import argparse
import hashlib
import pathlib
import struct
import uuid
import zlib
from distribution import usb_readme

SECTOR = 512
ENTRIES = 128
ENTRY_SIZE = 128
ARRAY_SECTORS = ENTRIES * ENTRY_SIZE // SECTOR
ESP_GUID = uuid.UUID('c12a7328-f81f-11d2-ba4b-00a0c93ec93b').bytes_le
BASIC_DATA_GUID = uuid.UUID('ebd0a0a2-b9e5-4433-87c0-68b6b72699c7').bytes_le
PARTITION_TYPES = {'basic-data': BASIC_DATA_GUID, 'esp': ESP_GUID}


def gpt_tables(blocks, first, last, seed, partition_type='basic-data'):
    """Return MBR, primary/backup headers and the identical entry arrays."""
    if partition_type not in PARTITION_TYPES:
        raise ValueError('Unsupported partition type')
    namespace = uuid.UUID('5fce2fd0-1e96-4cbe-8eae-b0a986b4fa18')
    disk_guid = uuid.uuid5(namespace, 'disk:' + seed).bytes_le
    part_guid = uuid.uuid5(namespace, partition_type + ':' + seed).bytes_le
    array = bytearray(ENTRIES * ENTRY_SIZE)
    array[:16] = PARTITION_TYPES[partition_type]
    array[16:32] = part_guid
    struct.pack_into(
        '<QQQ',
        array,
        32,
        first,
        last,
        0,
    )
    name = 'Musha-OS'.encode('utf-16le')
    array[56:56 + len(name)] = name
    array_crc = zlib.crc32(array)

    def header(current, other, array_lba):
        data = bytearray(SECTOR)
        struct.pack_into(
            '<8sIIIIQQQQ16sQIII',
            data,
            0,
            b'EFI PART',
            65536,
            92,
            0,
            0,
            current,
            other,
            2 + ARRAY_SECTORS,
            blocks - ARRAY_SECTORS - 2,
            disk_guid,
            array_lba,
            ENTRIES,
            ENTRY_SIZE,
            array_crc,
        )
        struct.pack_into(
            '<I',
            data,
            16,
            zlib.crc32(data[:92]),
        )
        return data
    mbr = bytearray(SECTOR)
    mbr[447:450] = b'\x00\x02\x00'
    mbr[450] = 238
    mbr[451:454] = b'\xff\xff\xff'
    struct.pack_into(
        '<II',
        mbr,
        454,
        1,
        min(blocks - 1, 4294967295),
    )
    mbr[510:512] = b'U\xaa'
    return (mbr, header(
        1,
        blocks - 1,
        2,
    ), header(
        blocks - 1,
        1,
        blocks - 1 - ARRAY_SECTORS,
    ), array)

def create(output, efi, size_mib=64, size_bytes=None, partition_type='basic-data', payload=None):
    # Read and validate input before creating any output. Limit allocation size.
    # Larger media use bounded cluster sizes and two FATs.
    # Solve geometry conservatively: the FAT must fit every data cluster.
    # A small deterministic FAT32 serializer. Directory entries include standard
    # VFAT LFN records for documents; the boot/root marker keeps its 8.3 name.
    # Replace the reserved directory bytes after allocating descendants.
    # Exclusive creation refuses existing files, symlinks and device paths.
    source = pathlib.Path(efi)
    length = source.stat().st_size
    if not source.is_file() or not 64 <= length <= 16 * 1024 * 1024:
        raise ValueError('EFI input must be a regular file of 64 bytes to 16MiB')
    executable = source.read_bytes()
    if executable[:2] != b'MZ':
        raise ValueError('EFI input is not PE32+')
    pe = struct.unpack_from(
        '<I',
        executable,
        60,
    )[0]
    if pe + 94 > len(executable) or executable[pe:pe + 4] != b'PE\x00\x00':
        raise ValueError('Invalid PE header')
    if struct.unpack_from(
        '<H',
        executable,
        pe + 4,
    )[0] != 34404 or struct.unpack_from(
        '<H',
        executable,
        pe + 24,
    )[0] != 523 or struct.unpack_from(
        '<H',
        executable,
        pe + 92,
    )[0] != 10:
        raise ValueError('Expected an x86-64 PE32+ EFI application')
    image_bytes = size_mib * 1024 * 1024 if size_bytes is None else size_bytes
    if not 64 * 1024 * 1024 <= image_bytes <= 128 * 1024 ** 3 or image_bytes % SECTOR:
        raise ValueError('Image size must be 64MiB–128GiB and a multiple of 512 bytes')
    readme = usb_readme((pathlib.Path(__file__).resolve().parent.parent / 'VERSION').read_text().strip())
    blocks = image_bytes // SECTOR
    (first, last) = (2048, blocks - ARRAY_SECTORS - 2)
    total = last - first + 1
    spc = 1 if image_bytes <= 1024 ** 3 else 8 if image_bytes <= 8 * 1024 ** 3 else 32 if image_bytes <= 32 * 1024 ** 3 else 64
    cluster_bytes = spc * SECTOR
    fat_sectors = 1
    while True:
        clusters = (total - 32 - 2 * fat_sectors) // spc
        required = (clusters + 2) * 4
        required = (required + SECTOR - 1) // SECTOR
        if required <= fat_sectors:
            break
        fat_sectors = required
    if clusters < 65525:
        raise ValueError('Volume too small for FAT32')
    data_start = first + 32 + 2 * fat_sectors
    from distribution import drive_payload, managed_path
    payload = drive_payload(source, (pathlib.Path(__file__).resolve().parent.parent / 'VERSION').read_text().strip()) if payload is None else dict(payload)
    if payload.get('EFI/BOOT/BOOTX64.EFI') != executable:
        raise ValueError('Image EFI differs from payload')
    for name in payload:
        managed_path(name)
    tree = {
        'EFI': {'BOOT': {}},
        'DOCS': {},
        'SETTINGS': {},
    }
    for (name, data) in sorted(payload.items()):
        parts = name.split('/')
        node = tree
        for part in parts[:-1]:
            node = node.setdefault(part, {})
        if parts[-1] in node:
            raise ValueError('Conflicting image paths')
        node[parts[-1]] = data
    fat = bytearray(fat_sectors * SECTOR)
    allocated = []
    next_cluster = 2

    def allocate(data):
        nonlocal next_cluster
        count = max(1, (len(data) + cluster_bytes - 1) // cluster_bytes)
        start = next_cluster
        next_cluster += count
        if next_cluster > clusters + 2:
            raise ValueError('Drive payload does not fit image')
        for c in range(start, next_cluster):
            struct.pack_into(
                '<I',
                fat,
                c * 4,
                c + 1 if c + 1 < next_cluster else 268435455,
            )
        allocated.append((start, data.ljust(count * cluster_bytes, b'\x00')))
        return start
    struct.pack_into(
        '<II',
        fat,
        0,
        268435448,
        4294967295,
    )

    def short_name(name, used):
        (base, _, ext) = name.rpartition('.')
        if not base:
            base = name
            ext = ''
        valid = lambda value: all((c in 'ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789_-~' for c in value))
        candidate = base.upper().ljust(8) + ext.upper().ljust(3)
        if len(base) <= 8 and len(ext) <= 3 and valid(base.upper()) and valid(ext.upper()) and (candidate not in used):
            used.add(candidate)
            return (candidate.encode('ascii'), name != name.upper())
        base = ''.join((c for c in base.upper() if c.isascii() and c.isalnum())) or 'FILE'
        ext = ''.join((c for c in ext.upper() if c.isascii() and c.isalnum()))[:3]
        for i in range(1, 10000):
            suffix = '~' + str(i)
            candidate = (base[:8 - len(suffix)] + suffix).ljust(8) + ext.ljust(3)
            if candidate not in used:
                used.add(candidate)
                return (candidate.encode('ascii'), True)
        raise ValueError('Image short-name aliases exhausted')

    def entry(name, attr, cluster, length=0):
        result = bytearray(32)
        result[:11] = name
        result[11] = attr
        struct.pack_into(
            '<H',
            result,
            20,
            cluster >> 16,
        )
        struct.pack_into(
            '<HI',
            result,
            26,
            cluster & 65535,
            length,
        )
        return result

    def lfn(name, short):
        checksum = 0
        for b in short:
            checksum = ((checksum & 1) << 7) + (checksum >> 1) + b & 255
        units = list(struct.unpack('<' + 'H' * (len(name.encode('utf-16le')) // 2), name.encode('utf-16le')))
        if len(units) > 255:
            raise ValueError('Image filename too long')
        units.append(0)
        units += [65535] * (-len(units) % 13)
        result = bytearray()
        for ordinal in range(
            len(units) // 13,
            0,
            -1,
        ):
            rec = bytearray(32)
            rec[0] = ordinal | (64 if ordinal == len(units) // 13 else 0)
            rec[11] = 15
            rec[13] = checksum
            for (offset, unit) in zip((1, 3, 5, 7, 9, 14, 16, 18, 20, 22, 24, 28, 30), units[(ordinal - 1) * 13:ordinal * 13]):
                struct.pack_into(
                    '<H',
                    rec,
                    offset,
                    unit,
                )
            result += rec
        return result

    def directory(node, parent=0, root=False):
        used = set()
        names = []
        for (name, value) in sorted(node.items()):
            (short, long) = short_name(name, used)
            names.append((name, value, short, lfn(name, short) if long else b''))
        size = (0 if root else 64) + sum((32 + len(long) for (_, _, _, long) in names)) + 32
        start = allocate(bytes(size))
        records = bytearray()
        if not root:
            records += entry(
                b'.          ',
                16,
                start,
            ) + entry(
                b'..         ',
                16,
                parent,
            )
        for (name, value, short, long) in names:
            isdir = isinstance(value, dict)
            cluster = directory(value, 0 if root else start) if isdir else allocate(value) if value else 0
            records += long + entry(
                short,
                16 if isdir else 32,
                cluster,
                0 if isdir else len(value),
            )
        for (index, (cluster, data)) in enumerate(allocated):
            if cluster == start:
                allocated[index] = (start, records.ljust(len(data), b'\x00'))
                break
        return start
    assert directory(tree, root=True) == 2
    boot = bytearray(SECTOR)
    boot[:3] = b'\xebX\x90'
    boot[3:11] = b'MUSHA-OS'
    struct.pack_into(
        '<H',
        boot,
        11,
        SECTOR,
    )
    boot[13] = spc
    struct.pack_into(
        '<H',
        boot,
        14,
        32,
    )
    boot[16] = 2
    boot[21] = 248
    struct.pack_into(
        '<HHII',
        boot,
        24,
        63,
        255,
        first,
        total,
    )
    struct.pack_into(
        '<I',
        boot,
        36,
        fat_sectors,
    )
    struct.pack_into(
        '<IHH',
        boot,
        44,
        2,
        1,
        6,
    )
    boot[64] = 128
    boot[66] = 41
    struct.pack_into(
        '<I',
        boot,
        67,
        1297437512,
    )
    boot[71:82] = b'MUSHA-OS   '
    boot[82:90] = b'FAT32   '
    boot[510:512] = b'U\xaa'
    info = bytearray(SECTOR)
    struct.pack_into(
        '<I',
        info,
        0,
        1096897106,
    )
    struct.pack_into(
        '<III',
        info,
        484,
        1631679090,
        clusters - (next_cluster - 2),
        next_cluster,
    )
    struct.pack_into(
        '<I',
        info,
        508,
        2857697280,
    )
    seed = hashlib.sha256(executable).hexdigest() + ':' + str(image_bytes)
    (mbr, primary, backup, array) = gpt_tables(
        blocks,
        first,
        last,
        seed,
        partition_type,
    )
    with pathlib.Path(output).open('xb') as image:
        image.truncate(blocks * SECTOR)

        def write(lba, contents):
            image.seek(lba * SECTOR)
            image.write(contents)
        write(0, mbr)
        write(1, primary)
        write(2, array)
        write(blocks - 1 - ARRAY_SECTORS, array)
        write(blocks - 1, backup)
        write(first, boot)
        write(first + 6, boot)
        write(first + 1, info)
        write(first + 7, info)
        write(first + 32, fat)
        write(first + 32 + fat_sectors, fat)
        for (cluster, contents) in allocated:
            write(data_start + (cluster - 2) * spc, contents)
    return pathlib.Path(output)

if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('output')
    parser.add_argument('--efi', default='out/esp/EFI/BOOT/BOOTX64.EFI')
    parser.add_argument('--partition-type', choices=PARTITION_TYPES, default='basic-data')
    sizes = parser.add_mutually_exclusive_group()
    sizes.add_argument('--size-mib', type=int, default=64)
    sizes.add_argument('--size-bytes', type=int, help='Exact physical medium capacity; multiple of 512')
    args = parser.parse_args()
    path = create(args.output, args.efi, args.size_mib, args.size_bytes, args.partition_type)
    digest = hashlib.sha256()
    with path.open('rb') as source:
        for chunk in iter(lambda: source.read(1024 * 1024), b''):
            digest.update(chunk)
    digest = digest.hexdigest()
    print('Created:', path)
    print('SHA-256:', digest)
