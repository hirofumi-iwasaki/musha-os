# SPDX-License-Identifier: Apache-2.0
"""Read bounded PE32+ section headers; no execution or external PE tools."""
import struct
from pathlib import Path

MAX_EFI_BYTES = 64 * 1024 * 1024


def measure_bytes(data):
    """Raw sizes include file alignment; virtual sizes are header values, not RSS."""
    if not 64 <= len(data) <= MAX_EFI_BYTES or data[:2] != b'MZ':
        raise ValueError('Invalid EFI DOS header/size')
    pe = struct.unpack_from('<I', data, 60)[0]
    if pe < 64 or pe > len(data) - 24 or data[pe:pe + 4] != b'PE\0\0':
        raise ValueError('Invalid EFI PE header')
    machine, count = struct.unpack_from('<HH', data, pe + 4)
    optional_size = struct.unpack_from('<H', data, pe + 20)[0]
    optional = pe + 24
    table = optional + optional_size
    if machine != 0x8664 or not 1 <= count <= 96 or optional_size < 112 or table > len(data):
        raise ValueError('Invalid EFI x64 section table')
    if struct.unpack_from('<H', data, optional)[0] != 0x20b or struct.unpack_from('<H', data, optional + 68)[0] != 10:
        raise ValueError('Expected PE32+ EFI application')
    image_size, headers_size = struct.unpack_from('<II', data, optional + 56)
    if table + count * 40 > min(headers_size, len(data)) or not image_size:
        raise ValueError('Truncated EFI sections/headers')
    sections = {}
    raw_ranges = []
    virtual_ranges = []
    for i in range(count):
        offset = table + i * 40
        raw_name = data[offset:offset + 8].split(b'\0', 1)[0]
        try:
            name = raw_name.decode('ascii')
        except UnicodeDecodeError as error:
            raise ValueError('Invalid EFI section name') from error
        if not name or name in sections or any(ord(c) < 32 for c in name):
            raise ValueError('Duplicate/invalid EFI section name')
        virtual, rva, raw, pointer = struct.unpack_from('<IIII', data, offset + 8)
        end = rva + max(virtual, raw)
        if not rva or end > image_size:
            raise ValueError('EFI virtual section outside image')
        if max(virtual, raw):
            virtual_ranges.append((rva, end))
        if raw:
            if pointer < headers_size or pointer + raw > len(data):
                raise ValueError('EFI raw section outside file')
            raw_ranges.append((pointer, pointer + raw))
        sections[name] = {'raw_bytes': raw, 'virtual_bytes': virtual}
    for ranges in (raw_ranges, virtual_ranges):
        ranges.sort()
        if any(left[1] > right[0] for left, right in zip(ranges, ranges[1:])):
            raise ValueError('Overlapping EFI sections')
    if not {'.text', '.rdata', '.data'} <= sections.keys():
        raise ValueError('Required EFI size sections missing')
    selected = {name: sections[name] for name in ('.text', '.rdata', '.data', '.bss') if name in sections}
    return {
        'schema': 1,
        'file_bytes': len(data),
        'sections': selected,
        'data_bss_virtual_bytes': sum(sections.get(name, {}).get('virtual_bytes', 0) for name in ('.data', '.bss')),
        'image_virtual_bytes': image_size,
        'measurement': 'PE32+ headers: raw includes file padding; virtual is VirtualSize, not page-rounded allocation or runtime usage',
    }


def measure(path):
    path = Path(path)
    if path.stat().st_size > MAX_EFI_BYTES:
        raise ValueError('EFI exceeds bounded measurement size')
    return measure_bytes(path.read_bytes())


def minimal_unavailable():
    """Do not relabel the existing no-default build as optional-driver-free."""
    return {
        'status': 'unavailable',
        'reason': 'optional-driver-feature-separation-missing',
        'blocking_drivers': ['lan', 'bluetooth'],
        'details': [
            'boot depends unconditionally on musha-net and its LAN runtime; no LAN-disable feature exists',
            'boot enables musha-platform/bluetooth-security and the Bluetooth runtime unconditionally',
            'T2 diagnostics and I218 opt-in features are already absent from normal builds; no-default-features does not remove LAN/Bluetooth',
        ],
    }
