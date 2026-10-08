#!/usr/bin/env python3
# Copyright 2026 Hirofumi Iwasaki
# SPDX-License-Identifier: Apache-2.0
"""Validate generated GPT/FAT32 structure, contents and exclusive creation."""
import hashlib
import importlib.util
import pathlib
import struct
import tempfile
import unittest
import zlib

spec = importlib.util.spec_from_file_location('usb_image', pathlib.Path(__file__).with_name('make-usb-image.py'))
usb = importlib.util.module_from_spec(spec)
spec.loader.exec_module(usb)

class ImageTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = pathlib.Path(self.temp.name)
        self.efi = self.root / 'input.efi'
        # A minimal input for the generator's PE type checks; QEMU separately
        # boots a real Rust EFI executable from the generated image.
        data = bytearray(1800)
        data[:2] = b'MZ'; struct.pack_into('<I', data, 60, 128)
        data[128:132] = b'PE\0\0'
        struct.pack_into('<H', data, 132, 0x8664)
        struct.pack_into('<H', data, 152, 0x20b)
        struct.pack_into('<H', data, 220, 10)
        self.efi.write_bytes(data)

    def test_layout_crcs_and_exact_nested_file_contents(self):
        path = usb.create(self.root / 'image.img', self.efi)
        with path.open('rb') as disk:
            def read(lba, length=512):
                disk.seek(lba * 512); return disk.read(length)
            blocks = path.stat().st_size // 512
            mbr = read(0)
            self.assertEqual(mbr[450], 0xee)
            self.assertEqual(struct.unpack_from('<II', mbr, 454), (1, blocks - 1))
            arrays = []
            for lba, alternate in [(1, blocks - 1), (blocks - 1, 1)]:
                header = bytearray(read(lba))
                self.assertEqual(header[:8], b'EFI PART')
                self.assertEqual(struct.unpack_from('<QQ', header, 24), (lba, alternate))
                crc = struct.unpack_from('<I', header, 16)[0]
                header[16:20] = bytes(4)
                self.assertEqual(zlib.crc32(header[:92]), crc)
                array_lba = struct.unpack_from('<Q', header, 72)[0]
                array = read(array_lba, 16384)
                self.assertEqual(zlib.crc32(array), struct.unpack_from('<I', header, 88)[0])
                arrays.append(array)
            self.assertEqual(*arrays)
            first, last = struct.unpack_from('<QQ', arrays[0], 32)
            self.assertEqual(arrays[0][:16], usb.ESP_GUID)
            self.assertLess(last, blocks - 33)
            bpb = read(first)
            fatsecs = struct.unpack_from('<I', bpb, 36)[0]
            self.assertEqual(read(first + 6), bpb)
            fat = read(first + 32, fatsecs * 512)
            self.assertEqual(read(first + 32 + fatsecs, fatsecs * 512), fat)
            data_start = first + 32 + fatsecs * 2
            def file_from_entry(entry):
                cluster = (struct.unpack_from('<H', entry, 20)[0] << 16) | struct.unpack_from('<H', entry, 26)[0]
                length = struct.unpack_from('<I', entry, 28)[0]
                result = bytearray(); seen = set()
                while cluster < 0x0ffffff8:
                    self.assertNotIn(cluster, seen); seen.add(cluster)
                    result.extend(read(data_start + cluster - 2))
                    cluster = struct.unpack_from('<I', fat, cluster * 4)[0] & 0x0fffffff
                return bytes(result[:length])
            root = read(data_start)
            self.assertEqual(root[:11], b'EFI        ')
            self.assertEqual(file_from_entry(root[32:64]), usb.PAYLOAD)
            efi_dir = read(data_start + 1)
            self.assertEqual(efi_dir[64:75], b'BOOT       ')
            boot_dir = read(data_start + 2)
            self.assertEqual(boot_dir[64:75], b'BOOTX64 EFI')
            self.assertEqual(file_from_entry(boot_dir[64:96]), self.efi.read_bytes())

    def test_reproducible_and_refuses_existing_files_and_symlinks(self):
        a = usb.create(self.root / 'a.img', self.efi)
        b = usb.create(self.root / 'b.img', self.efi)
        def digest(path):
            with path.open('rb') as source:
                result = hashlib.sha256()
                for chunk in iter(lambda: source.read(1024 * 1024), b''):
                    result.update(chunk)
                return result.digest()
        original = digest(a)
        self.assertEqual(original, digest(b))
        with self.assertRaises(FileExistsError): usb.create(a, self.efi)
        link = self.root / 'link.img'; link.symlink_to(a)
        with self.assertRaises(FileExistsError): usb.create(link, self.efi)
        self.assertEqual(original, digest(a))

    def test_exact_capacity_and_larger_cluster_geometry(self):
        capacity = 2 * 1024**3 + 512
        path = usb.create(self.root / 'large.img', self.efi, size_bytes=capacity)
        self.assertEqual(path.stat().st_size, capacity)
        with path.open('rb') as disk:
            disk.seek(2048 * 512); bpb = disk.read(512)
            self.assertEqual(bpb[13], 8)
            total = struct.unpack_from('<I', bpb, 32)[0]
            fatsecs = struct.unpack_from('<I', bpb, 36)[0]
            clusters = (total - 32 - 2 * fatsecs) // bpb[13]
            self.assertGreaterEqual(clusters, 65525)
            self.assertGreaterEqual(fatsecs * 512 // 4, clusters + 2)
            data = 2048 + 32 + 2 * fatsecs
            disk.seek((data + 2 * bpb[13]) * 512)
            boot_dir = disk.read(512)
            cluster = struct.unpack_from('<H', boot_dir, 64 + 26)[0]
            disk.seek((data + (cluster - 2) * bpb[13]) * 512)
            self.assertEqual(disk.read(len(self.efi.read_bytes())), self.efi.read_bytes())
        with self.assertRaises(ValueError):
            usb.create(self.root / 'invalid.img', self.efi, size_bytes=capacity + 1)

    def test_invalid_input_does_not_create_output(self):
        output = self.root / 'output.img'
        self.efi.write_bytes(b'invalid')
        with self.assertRaises(ValueError): usb.create(output, self.efi)
        self.assertFalse(output.exists())

if __name__ == '__main__':
    unittest.main()
