#!/usr/bin/env python3
# Copyright 2026 Hirofumi Iwasaki
# SPDX-License-Identifier: Apache-2.0
"""Deterministic synthetic exFAT fixtures. Creates new regular files only.

Test data, not a general formatter or a boot image. A deliberately minimal custom
up-case table maps ASCII lowercase to uppercase and all other code units to self.
"""
import argparse
import hashlib
import json
from pathlib import Path
import struct

SECTOR = 512
CLUSTERS = 4096
FAT = 24
FAT_LENGTH = 33
HEAP = 64
BLOCKS = HEAP + CLUSTERS
CASES = ('normal', 'valid-tail', 'boot-checksum', 'entry-checksum', 'chain-cycle', 'bitmap-free')


def checksum(data, bits, skip=()):
    value = 0
    mask = (1 << bits) - 1
    for index, byte in enumerate(data):
        if index not in skip:
            value = (((value >> 1) | ((value & 1) << (bits - 1))) + byte) & mask
    return value


def upper(unit):
    return unit - 32 if 97 <= unit <= 122 else unit


def entry_set(name, payload, chain, contiguous, valid):
    encoded = name.encode('utf-16le')
    units = list(struct.unpack('<' + 'H' * (len(encoded) // 2), encoded))
    names = (len(units) + 14) // 15
    result = bytearray(32 * (2 + names))
    result[0:2] = bytes((0x85, 1 + names))
    struct.pack_into('<H', result, 4, 0x20)
    # 2026-10-11 00:00:00, unknown timezone.
    stamp = (46 << 25) | (10 << 21) | (11 << 16)
    struct.pack_into('<III', result, 8, stamp, stamp, stamp)
    result[32] = 0xc0
    result[33] = 3 if contiguous and chain else 1
    result[35] = len(units)
    folded = struct.pack('<' + 'H' * len(units), *(upper(u) for u in units))
    struct.pack_into('<H', result, 36, checksum(folded, 16))
    struct.pack_into('<Q', result, 40, valid)
    struct.pack_into('<I', result, 52, chain[0] if chain else 0)
    struct.pack_into('<Q', result, 56, len(payload))
    for index in range(names):
        off = (2 + index) * 32
        result[off] = 0xc1
        chunk = encoded[index * 30:(index + 1) * 30]
        result[off + 2:off + 2 + len(chunk)] = chunk
    struct.pack_into('<H', result, 2, checksum(result, 16, (2, 3)))
    return result


def build(case='normal', sector_size=512):
    if sector_size not in (512, 4096):
        raise ValueError('unsupported fixture sector size')
    SECTOR = sector_size
    if case not in CASES:
        raise ValueError('unknown fixture case')
    image = bytearray(BLOCKS * SECTOR)

    def put(lba, data):
        off = lba * SECTOR
        if off + len(data) > len(image):
            raise ValueError('fixture allocation exceeds image')
        image[off:off + len(data)] = data

    def cluster(n, data):
        put(HEAP + n - 2, data)

    boot = bytearray(12 * SECTOR)
    boot[:11] = b'\xeb\x76\x90EXFAT   '
    struct.pack_into('<QQIIIIIIHH', boot, 64, 0, BLOCKS, FAT, FAT_LENGTH,
                     HEAP, CLUSTERS, 2, 0x4d555348, 0x100, 0)
    boot[108:113] = bytes((SECTOR.bit_length() - 1, 0, 1, 0x80, 0xff))
    boot[510:512] = b'\x55\xaa'
    for index in range(1, 9):
        struct.pack_into('<I', boot, (index + 1) * SECTOR - 4, 0xaa550000)
    check = checksum(boot[:11 * SECTOR], 32, (106, 107, 112))
    boot[11 * SECTOR:] = struct.pack('<I', check) * (SECTOR // 4)
    put(0, boot)
    put(12, boot)

    fat = bytearray(FAT_LENGTH * SECTOR)
    struct.pack_into('<II', fat, 0, 0xfffffff8, 0xffffffff)
    allocated = set()

    def allocate(chain, linked=True):
        if allocated.intersection(chain):
            raise ValueError('fixture cluster overlap')
        allocated.update(chain)
        if linked:
            for i, n in enumerate(chain):
                value = chain[i + 1] if i + 1 < len(chain) else 0xffffffff
                struct.pack_into('<I', fat, n * 4, value)

    roots = [2, 600]
    allocate(roots)
    allocate([3])  # bitmap, 4096 bits
    upcase_chain = list(range(4, 4 + 131072 // SECTOR))
    allocate(upcase_chain)
    table = struct.pack('<65536H', *(upper(u) for u in range(65536)))
    for i, n in enumerate(upcase_chain):
        cluster(n, table[i * SECTOR:(i + 1) * SECTOR])
    root = bytearray(64)
    root[0] = 0x81
    struct.pack_into('<IQ', root, 20, 3, CLUSTERS // 8)
    root[32] = 0x82
    struct.pack_into('<I', root, 36, checksum(table, 32))
    struct.pack_into('<IQ', root, 52, 4, len(table))

    # Expected data is defined before image serialization, not recovered from it.
    files = [
        ('MUSHA.TXT', b'Hello Musha-OS!\n', [260], True, 16),
        ('FRAGMENT.BIN', bytes(range(256)) * (2 * SECTOR // 256 + 1) + b'end', [270, 272, 271], False, 2 * SECTOR + 259),
        ('Long name for exFAT test.txt', b'Long file name\n', [280], True, 15),
        ('EMPTY.TXT', b'', [], False, 0),
        ('ZEROTAIL.BIN', b'valid' + b'\0' * 695, list(range(290, 290 + (700 + SECTOR - 1) // SECTOR)), True, 5),
    ]
    expected = []
    offsets = []
    for name, payload, chain, contiguous, valid in files:
        allocate(chain, not contiguous)
        offsets.append(len(root))
        root += entry_set(name, payload, chain, contiguous, valid)
        # Deliberately leave nonzero disk bytes beyond ValidDataLength.
        fill = b'\xa5' if case == 'valid-tail' else b'\0'
        disk_data = payload[:valid] + fill * (len(chain) * SECTOR - valid)
        for i, n in enumerate(chain):
            cluster(n, disk_data[i * SECTOR:(i + 1) * SECTOR])
        expected.append({'name': name, 'length': len(payload), 'valid_length': valid,
                         'clusters': chain, 'no_fat_chain': contiguous,
                         'sha256': hashlib.sha256(payload).hexdigest()})
    root += bytes(32)  # end marker; last entry set crosses the root cluster boundary at 512 bytes
    if len(root) > len(roots) * SECTOR:
        raise ValueError('root overflow')
    if case == 'entry-checksum':
        root[offsets[0] + 2] ^= 1
    if case == 'chain-cycle':
        struct.pack_into('<I', fat, 272 * 4, 270)
    for i, n in enumerate(roots):
        cluster(n, root[i * SECTOR:(i + 1) * SECTOR].ljust(SECTOR, b'\0'))
    bitmap = bytearray(CLUSTERS // 8)
    for n in allocated:
        bitmap[(n - 2) // 8] |= 1 << ((n - 2) % 8)
    if case == 'bitmap-free':
        bitmap[(260 - 2) // 8] &= ~(1 << ((260 - 2) % 8))
    cluster(3, bitmap)
    put(FAT, fat)
    if case == 'boot-checksum':
        image[100] ^= 1  # backup stays valid; no automatic recovery in initial reader
    manifest = {'schema': 1, 'case': case, 'sector_size': SECTOR, 'blocks': BLOCKS,
                'expected_result': 'ok' if case in ('normal', 'valid-tail') else 'Corrupt',
                'files': expected, 'image_sha256': hashlib.sha256(image).hexdigest()}
    return image, manifest


def create(path, case='normal', sector_size=512):
    image, manifest = build(case, sector_size)
    # Exclusive creation rejects existing images, symlinks and device nodes.
    with Path(path).open('xb') as output:
        output.write(image)
    return manifest


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('output')
    parser.add_argument('--case', choices=CASES, default='normal')
    parser.add_argument('--sector-size', type=int, choices=(512, 4096), default=512)
    args = parser.parse_args()
    print(json.dumps(create(args.output, args.case, args.sector_size), ensure_ascii=False, indent=2))
