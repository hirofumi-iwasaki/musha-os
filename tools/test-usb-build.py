#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""Check bundle contents, checksums, reproducibility and output protection."""
import importlib.util
from pathlib import Path
import struct
import tempfile
import unittest
import zipfile

spec = importlib.util.spec_from_file_location('builder', Path(__file__).with_name('build-usb.py'))
builder = importlib.util.module_from_spec(spec)
spec.loader.exec_module(builder)


class BundleTests(unittest.TestCase):
    def test_bundle_and_image_match_and_existing_output_is_preserved(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            efi = root / 'input.efi'
            data = bytearray(1800)
            data[:2] = b'MZ'
            struct.pack_into('<I', data, 60, 128)
            data[128:132] = b'PE\0\0'
            struct.pack_into('<H', data, 132, 0x8664)
            struct.pack_into('<H', data, 152, 0x20b)
            struct.pack_into('<H', data, 220, 10)
            efi.write_bytes(data)
            out = root / 'one'
            result = builder.package(out, efi, size_bytes=64 * 1024 * 1024 + 512)
            self.assertEqual(result['image_bytes'], 64 * 1024 * 1024 + 512)
            with zipfile.ZipFile(out / 'musha-os-fat32-files.zip') as archive:
                self.assertEqual(set(archive.namelist()), {'EFI/BOOT/BOOTX64.EFI', 'MUSHA.TXT', 'LICENSE', 'NOTICE', 'r-efi-AUTHORS'})
                self.assertEqual(archive.read('EFI/BOOT/BOOTX64.EFI'), data)
                self.assertEqual(archive.read('MUSHA.TXT'), b'Hello Musha-OS!\n')
                self.assertEqual(archive.read('r-efi-AUTHORS'), (builder.ROOT / 'third_party/r-efi/AUTHORS').read_bytes())
                for name in ('LICENSE', 'NOTICE'):
                    self.assertEqual(archive.read(name), (builder.ROOT / name).read_bytes())
            for name, value in result['sha256'].items():
                self.assertEqual(builder.digest(out / name), value)
            with self.assertRaises(FileExistsError):
                builder.package(out, efi)
            self.assertEqual(builder.digest(out / 'musha-os.img'), result['sha256']['musha-os.img'])
            second = builder.package(root / 'two', efi, size_bytes=64 * 1024 * 1024 + 512)
            self.assertEqual(second['sha256'], result['sha256'])
            link = root / 'link'
            link.symlink_to(out, target_is_directory=True)
            with self.assertRaises(FileExistsError):
                builder.package(link, efi)


if __name__ == '__main__':
    unittest.main()
