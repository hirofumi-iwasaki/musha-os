#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""PE size header semantics and malformed-input rejection, without executing EFI."""
import struct
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch
from efi_size import measure, measure_bytes, MAX_EFI_BYTES

PE = 128
OPTIONAL = PE + 24
TABLE = OPTIONAL + 240


def fixture(bss=False):
    data = bytearray(4096)
    data[:2] = b'MZ'
    struct.pack_into('<I', data, 60, PE)
    data[PE:PE + 4] = b'PE\0\0'
    struct.pack_into('<HH', data, PE + 4, 0x8664, 4 if bss else 3)
    struct.pack_into('<H', data, PE + 20, 240)
    struct.pack_into('<H', data, OPTIONAL, 0x20b)
    struct.pack_into('<II', data, OPTIONAL + 32, 4096, 512)
    struct.pack_into('<II', data, OPTIONAL + 56, 0x6000, 1024)
    struct.pack_into('<H', data, OPTIONAL + 68, 10)
    sections = [(b'.text', 400, 0x1000, 512, 1024),
                (b'.rdata', 200, 0x2000, 512, 1536),
                (b'.data', 0x1234, 0x3000, 512, 2048)]
    if bss:
        sections.append((b'.bss', 0x1000, 0x5000, 0, 0))
    for i, (name, virtual, rva, raw, pointer) in enumerate(sections):
        offset = TABLE + 40 * i
        data[offset:offset + 8] = name.ljust(8, b'\0')
        struct.pack_into('<IIII', data, offset + 8, virtual, rva, raw, pointer)
    return data


class SizeTests(unittest.TestCase):
    def test_raw_padding_and_virtual_data_are_distinct(self):
        result = measure_bytes(fixture())
        self.assertEqual(result['file_bytes'], 4096)
        self.assertEqual(result['sections']['.text'], {'raw_bytes': 512, 'virtual_bytes': 400})
        self.assertEqual(result['sections']['.data'], {'raw_bytes': 512, 'virtual_bytes': 0x1234})
        self.assertEqual(result['data_bss_virtual_bytes'], 0x1234)
        self.assertEqual(result['image_virtual_bytes'], 0x6000)

    def test_separate_bss_has_no_disk_bytes_and_is_counted_once(self):
        result = measure_bytes(fixture(True))
        self.assertEqual(result['sections']['.bss']['raw_bytes'], 0)
        self.assertEqual(result['data_bss_virtual_bytes'], 0x2234)

    def test_bad_dos_pe_and_truncated_optional_header(self):
        for data in (b'', b'MZ', fixture()[:PE + 23], fixture()[:TABLE - 1]):
            with self.subTest(length=len(data)), self.assertRaises(ValueError):
                measure_bytes(data)
        data = fixture()
        struct.pack_into('<I', data, 60, 0xffffffff)
        with self.assertRaises(ValueError):
            measure_bytes(data)

    def test_wrong_machine_magic_subsystem_or_section_count(self):
        for offset, value in ((PE + 4, 0x14c), (OPTIONAL, 0x10b), (OPTIONAL + 68, 3), (PE + 6, 97), (PE + 6, 0)):
            data = fixture()
            struct.pack_into('<H', data, offset, value)
            with self.subTest(offset=offset, value=value), self.assertRaises(ValueError):
                measure_bytes(data)

    def test_raw_or_virtual_extent_outside_image(self):
        for offset, value in ((TABLE + 16, 0xffffffff), (TABLE + 20, 0), (TABLE + 8, 0xffffffff), (OPTIONAL + 56, 0)):
            data = fixture()
            struct.pack_into('<I', data, offset, value)
            with self.subTest(offset=offset), self.assertRaises(ValueError):
                measure_bytes(data)

    def test_raw_and_virtual_overlap(self):
        for offset, value in ((TABLE + 40 + 20, 1024), (TABLE + 40 + 12, 0x1100)):
            data = fixture()
            struct.pack_into('<I', data, offset, value)
            with self.assertRaisesRegex(ValueError, 'Overlapping'):
                measure_bytes(data)

    def test_duplicate_missing_or_nonascii_names(self):
        for name in (b'.text', b'.other', b'\xff'):
            data = fixture()
            data[TABLE + 40:TABLE + 48] = name.ljust(8, b'\0')
            with self.assertRaises(ValueError):
                measure_bytes(data)

    def test_section_table_outside_headers(self):
        data = fixture()
        struct.pack_into('<I', data, OPTIONAL + 60, TABLE + 40)
        with self.assertRaises(ValueError):
            measure_bytes(data)

    def test_file_api_and_size_bound_precede_read(self):
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / 'test.efi'
            path.write_bytes(fixture())
            self.assertEqual(measure(path), measure_bytes(fixture()))
            with patch.object(Path, 'stat') as stat, patch.object(Path, 'read_bytes') as read:
                stat.return_value.st_size = MAX_EFI_BYTES + 1
                with self.assertRaises(ValueError):
                    measure(path)
                read.assert_not_called()


if __name__ == '__main__':
    unittest.main()
