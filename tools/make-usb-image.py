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

SECTOR = 512
ENTRIES = 128
ENTRY_SIZE = 128
ARRAY_SECTORS = ENTRIES * ENTRY_SIZE // SECTOR
ESP_GUID = uuid.UUID('c12a7328-f81f-11d2-ba4b-00a0c93ec93b').bytes_le
PAYLOAD = b'Hello Musha-OS!\n'

def gpt_tables(blocks, first, last, seed):
    """Return MBR, primary/backup headers and the identical entry arrays."""
    namespace = uuid.UUID('5fce2fd0-1e96-4cbe-8eae-b0a986b4fa18')
    disk_guid = uuid.uuid5(namespace, 'disk:' + seed).bytes_le
    part_guid = uuid.uuid5(namespace, 'esp:' + seed).bytes_le
    array = bytearray(ENTRIES * ENTRY_SIZE)
    array[:16] = ESP_GUID
    array[16:32] = part_guid
    struct.pack_into('<QQQ', array, 32, first, last, 0)
    name = 'Musha-OS ESP'.encode('utf-16le')
    array[56:56 + len(name)] = name
    array_crc = zlib.crc32(array)
    def header(current, other, array_lba):
        data = bytearray(SECTOR)
        struct.pack_into('<8sIIIIQQQQ16sQIII', data, 0, b'EFI PART', 0x10000,
                         92, 0, 0, current, other, 2 + ARRAY_SECTORS,
                         blocks - ARRAY_SECTORS - 2, disk_guid, array_lba,
                         ENTRIES, ENTRY_SIZE, array_crc)
        struct.pack_into('<I', data, 16, zlib.crc32(data[:92]))
        return data
    mbr = bytearray(SECTOR)
    mbr[447:450] = b'\x00\x02\x00'
    mbr[450] = 0xee
    mbr[451:454] = b'\xff\xff\xff'
    struct.pack_into('<II', mbr, 454, 1, min(blocks - 1, 0xffffffff))
    mbr[510:512] = b'\x55\xaa'
    return mbr, header(1, blocks - 1, 2), header(blocks - 1, 1, blocks - 1 - ARRAY_SECTORS), array

def create(output, efi, size_mib=64, size_bytes=None):
    source = pathlib.Path(efi)
    # Read and validate input before creating any output. Limit allocation size.
    length = source.stat().st_size
    if not source.is_file() or not 64 <= length <= 16 * 1024 * 1024:
        raise ValueError('EFI input must be a regular file of 64 bytes to 16MiB')
    executable = source.read_bytes()
    if executable[:2] != b'MZ':
        raise ValueError('EFI input is not PE32+')
    pe = struct.unpack_from('<I', executable, 60)[0]
    if pe + 94 > len(executable) or executable[pe:pe + 4] != b'PE\0\0':
        raise ValueError('Invalid PE header')
    if (struct.unpack_from('<H', executable, pe + 4)[0] != 0x8664
            or struct.unpack_from('<H', executable, pe + 24)[0] != 0x20b
            or struct.unpack_from('<H', executable, pe + 92)[0] != 10):
        raise ValueError('Expected an x86-64 PE32+ EFI application')
    image_bytes = size_mib * 1024 * 1024 if size_bytes is None else size_bytes
    if not 64 * 1024 * 1024 <= image_bytes <= 128 * 1024**3 or image_bytes % SECTOR:
        raise ValueError('Image size must be 64MiB–128GiB and a multiple of 512 bytes')
    blocks = image_bytes // SECTOR
    first, last = 2048, blocks - ARRAY_SECTORS - 2
    total = last - first + 1
    # Larger media use bounded cluster sizes and two FATs.
    spc = 1 if image_bytes <= 1024**3 else 8 if image_bytes <= 8 * 1024**3 else 32 if image_bytes <= 32 * 1024**3 else 64
    cluster_bytes = spc * SECTOR
    # Solve geometry conservatively: the FAT must fit every data cluster.
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
    efi_count = (len(executable) + cluster_bytes - 1) // cluster_bytes
    file_start, text_cluster = 5, 5 + efi_count
    if text_cluster >= clusters + 2:
        raise ValueError('EFI input does not fit')
    fat = bytearray(fat_sectors * SECTOR)
    def set_fat(cluster, value):
        struct.pack_into('<I', fat, cluster * 4, value)
    for cluster, value in [(0, 0x0ffffff8), (1, 0xffffffff), (2, 0x0fffffff),
                           (3, 0x0fffffff), (4, 0x0fffffff), (text_cluster, 0x0fffffff)]:
        set_fat(cluster, value)
    for cluster in range(file_start, text_cluster):
        set_fat(cluster, cluster + 1 if cluster + 1 < text_cluster else 0x0fffffff)
    def entry(name, attr, cluster, length=0):
        result = bytearray(32)
        result[:11] = name
        result[11] = attr
        struct.pack_into('<H', result, 20, cluster >> 16)
        struct.pack_into('<HI', result, 26, cluster & 0xffff, length)
        return result
    root = bytearray(SECTOR)
    root[:32] = entry(b'EFI        ', 0x10, 3)
    root[32:64] = entry(b'MUSHA   TXT', 0x20, text_cluster, len(PAYLOAD))
    def directory(cluster, parent, child):
        result = bytearray(SECTOR)
        result[:32] = entry(b'.          ', 0x10, cluster)
        result[32:64] = entry(b'..         ', 0x10, parent)
        result[64:96] = child
        return result
    boot = bytearray(SECTOR)
    boot[:3] = b'\xeb\x58\x90'; boot[3:11] = b'MUSHA-OS'
    struct.pack_into('<H', boot, 11, SECTOR); boot[13] = spc
    struct.pack_into('<H', boot, 14, 32); boot[16] = 2; boot[21] = 0xf8
    struct.pack_into('<HHII', boot, 24, 63, 255, first, total)
    struct.pack_into('<I', boot, 36, fat_sectors)
    struct.pack_into('<IHH', boot, 44, 2, 1, 6)
    boot[64] = 0x80; boot[66] = 0x29
    struct.pack_into('<I', boot, 67, 0x4d555348)
    boot[71:82] = b'MUSHA-OS   '; boot[82:90] = b'FAT32   '
    boot[510:512] = b'\x55\xaa'
    info = bytearray(SECTOR)
    struct.pack_into('<I', info, 0, 0x41615252)
    struct.pack_into('<III', info, 484, 0x61417272, 0xffffffff, 0xffffffff)
    struct.pack_into('<I', info, 508, 0xaa550000)
    seed = hashlib.sha256(executable).hexdigest() + ':' + str(image_bytes)
    mbr, primary, backup, array = gpt_tables(blocks, first, last, seed)
    # Exclusive creation refuses existing files, symlinks and device paths.
    with pathlib.Path(output).open('xb') as image:
        image.truncate(blocks * SECTOR)
        def write(lba, contents):
            image.seek(lba * SECTOR); image.write(contents)
        write(0, mbr); write(1, primary); write(2, array)
        write(blocks - 1 - ARRAY_SECTORS, array); write(blocks - 1, backup)
        write(first, boot); write(first + 6, boot)
        write(first + 1, info); write(first + 7, info)
        write(first + 32, fat); write(first + 32 + fat_sectors, fat)
        write(data_start, root)
        write(data_start + spc, directory(3, 0, entry(b'BOOT       ', 0x10, 4)))
        write(data_start + 2 * spc, directory(4, 3, entry(b'BOOTX64 EFI', 0x20, file_start, len(executable))))
        write(data_start + (file_start - 2) * spc, executable.ljust(efi_count * cluster_bytes, b'\0'))
        write(data_start + (text_cluster - 2) * spc, PAYLOAD.ljust(SECTOR, b'\0'))
    return pathlib.Path(output)

if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('output')
    parser.add_argument('--efi', default='out/esp/EFI/BOOT/BOOTX64.EFI')
    sizes = parser.add_mutually_exclusive_group()
    sizes.add_argument('--size-mib', type=int, default=64)
    sizes.add_argument('--size-bytes', type=int, help='Exact physical medium capacity; multiple of 512')
    args = parser.parse_args()
    path = create(args.output, args.efi, args.size_mib, args.size_bytes)
    digest = hashlib.sha256()
    with path.open('rb') as source:
        for chunk in iter(lambda: source.read(1024 * 1024), b''):
            digest.update(chunk)
    digest = digest.hexdigest()
    print('Created:', path)
    print('SHA-256:', digest)
