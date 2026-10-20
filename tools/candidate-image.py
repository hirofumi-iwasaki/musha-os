#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""Read the managed files of a generated 64 MiB candidate image, without devices."""
import struct

SECTOR = 512
MAX_DIRECTORY = 64 * 1024
MAX_FILE = 16 * 1024 * 1024


def managed_files(image):
    """Return image files and enforce the exact initial distribution layout."""
    if len(image) != 64 * 1024 * 1024:
        raise ValueError('Candidate image must be 64 MiB')
    boot = image[2048 * SECTOR:2049 * SECTOR]
    if boot[13] != 1 or struct.unpack_from('<H', boot, 14)[0] != 32 or boot[16] != 2:
        raise ValueError('Unexpected candidate FAT geometry')
    sectors = struct.unpack_from('<I', boot, 36)[0]
    start = 2048 + 32 + 2 * sectors
    end = 2048 + struct.unpack_from('<I', boot, 32)[0]
    fat = image[(2048 + 32) * SECTOR:(2048 + 32 + sectors) * SECTOR]
    claimed = set()
    entry_count = 0

    def chain(first, limit):
        if first < 2:
            raise ValueError('Missing FAT cluster')
        current, seen, chunks = first, set(), []
        while True:
            if current in seen or current in claimed or current < 2 or start + current - 2 >= end:
                raise ValueError('Invalid or shared FAT chain')
            if current * 4 + 4 > len(fat) or len(chunks) * SECTOR >= limit:
                raise ValueError('FAT chain exceeds bound')
            seen.add(current)
            chunks.append(image[(start + current - 2) * SECTOR:(start + current - 1) * SECTOR])
            following = struct.unpack_from('<I', fat, current * 4)[0] & 0x0fffffff
            if following >= 0x0ffffff8:
                claimed.update(seen)
                return b''.join(chunks)
            if following == 0 or following == 0x0ffffff7:
                raise ValueError('Broken FAT chain')
            current = following

    def first_cluster(entry):
        return (struct.unpack_from('<H', entry, 20)[0] << 16) | struct.unpack_from('<H', entry, 26)[0]

    def lfn_name(records, short):
        if not records:
            return short[:8].decode('ascii').rstrip() + ('.' + short[8:].decode('ascii').rstrip() if short[8:].strip() else '')
        checksum = 0
        for byte in short:
            checksum = (((checksum & 1) << 7) + (checksum >> 1) + byte) & 255
        count = len(records)
        if count > 20 or records[0][0] != count | 64:
            raise ValueError('Invalid FAT long name order')
        units = []
        for index, record in enumerate(records):
            if record[0] != count - index | (64 if index == 0 else 0) or record[13] != checksum or record[12] or record[26:28] != b'\0\0':
                raise ValueError('Invalid FAT long name record')
        for record in reversed(records):
            for offset in (1, 3, 5, 7, 9, 14, 16, 18, 20, 22, 24, 28, 30):
                units.append(struct.unpack_from('<H', record, offset)[0])
        if 0 in units:
            units = units[:units.index(0)]
        try:
            name = struct.pack('<' + 'H' * len(units), *units).decode('utf-16le')
        except UnicodeError as exc:
            raise ValueError('Invalid FAT long name text') from exc
        if not name or '/' in name or '\\' in name:
            raise ValueError('Invalid FAT long name')
        return name

    def entries(directory, root=False, parent=0):
        nonlocal entry_count
        data = chain(directory, MAX_DIRECTORY)
        result, long, ended, dots = {}, [], False, []
        for offset in range(0, len(data), 32):
            entry = data[offset:offset + 32]
            if entry[0] == 0:
                ended = True
                break
            if entry[0] == 0xe5:
                raise ValueError('Deleted candidate directory entry')
            if entry[11] == 15:
                long.append(entry)
                continue
            if entry[11] == 8 and root and entry[:11] == b'MUSHA-OS   ' and not long:
                continue
            if entry[:11] in (b'.          ', b'..         ') and not root and not long and entry[11] == 16:
                dots.append((entry[:11], first_cluster(entry), struct.unpack_from('<I', entry, 28)[0]))
                continue
            name = lfn_name(long, entry[:11])
            long = []
            if name.casefold() in {n.casefold() for n in result} or entry[11] not in (16, 32):
                raise ValueError('Duplicate or invalid candidate entry')
            entry_count += 1
            if entry_count > 512:
                raise ValueError('Candidate directory entries exceed bound')
            result[name] = entry
        expected_dots = [(b'.          ', directory, 0), (b'..         ', parent, 0)]
        if not ended or long or (not root and dots != expected_dots):
            raise ValueError('Unterminated candidate directory')
        return result

    def directory(entry, parent=0):
        if entry[11] != 16 or struct.unpack_from('<I', entry, 28)[0]:
            raise ValueError('Expected candidate directory')
        return entries(first_cluster(entry), parent=parent)

    def file(entry):
        if entry[11] != 32:
            raise ValueError('Expected candidate file')
        length = struct.unpack_from('<I', entry, 28)[0]
        if length > MAX_FILE:
            raise ValueError('Candidate file exceeds bound')
        if length == 0 and first_cluster(entry) == 0:
            return b''
        data = chain(first_cluster(entry), MAX_FILE + SECTOR)
        if len(data) != max(1, (length + SECTOR - 1) // SECTOR) * SECTOR:
            raise ValueError('Candidate file chain length differs')
        return data[:length]

    root = entries(struct.unpack_from('<I', boot, 44)[0], True)
    if set(root) != {'EFI', 'DOCS', 'SETTINGS', 'README.TXT'}:
        raise ValueError('Candidate root layout differs')
    if directory(root['SETTINGS']):
        raise ValueError('Candidate SETTINGS must initially be empty')
    efi = directory(root['EFI'])
    if set(efi) != {'BOOT'}:
        raise ValueError('Candidate EFI layout differs')
    boot_dir = directory(efi['BOOT'], first_cluster(root['EFI']))
    if set(boot_dir) != {'BOOTX64.EFI'}:
        raise ValueError('Candidate BOOT layout differs')
    result = {'EFI/BOOT/BOOTX64.EFI': file(boot_dir['BOOTX64.EFI']),
              'README.TXT': file(root['README.TXT'])}
    def walk_docs(prefix, items, depth, current):
        if depth > 4 or not items:
            raise ValueError('Candidate DOCS tree exceeds bound')
        for name, entry in items.items():
            path = prefix + name
            if entry[11] == 16:
                walk_docs(path + '/', directory(entry, current), depth + 1, first_cluster(entry))
            else:
                result[path] = file(entry)
    walk_docs('DOCS/', directory(root['DOCS']), 0, first_cluster(root['DOCS']))
    return result
