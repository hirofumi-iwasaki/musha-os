#!/usr/bin/env python3
# Copyright 2026 Hirofumi Iwasaki
# SPDX-License-Identifier: Apache-2.0
"""Exercise candidate packaging, provenance and output preservation."""
import hashlib
import importlib.util
import json
from pathlib import Path
import struct
import tarfile
import tempfile
import unittest

spec = importlib.util.spec_from_file_location('release', Path(__file__).with_name('prepare-release.py'))
release = importlib.util.module_from_spec(spec); spec.loader.exec_module(release)

class PackageTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(); self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name); self.efi = self.root / 'test.efi'
        data = bytearray(1800); data[:2] = b'MZ'
        struct.pack_into('<I', data, 60, 128); data[128:132] = b'PE\0\0'
        struct.pack_into('<H', data, 132, 0x8664)
        struct.pack_into('<H', data, 152, 0x20b); struct.pack_into('<H', data, 220, 10)
        self.efi.write_bytes(data)  # Format fixture, not a runnable EFI.

    def test_archive_hashes_licenses_and_provenance(self):
        output = release.stage(self.root / 'candidate', self.efi, {'commit': 'fixture', 'features': []})
        bundle = output / 'musha-os-0.1.0-candidate'
        manifest = json.loads((bundle / 'manifest.json').read_text())
        self.assertEqual(manifest['hardware_validation'], 'NOT RUN')
        self.assertEqual(manifest['status'], 'unreleased-candidate')
        for name, entry in manifest['files'].items():
            self.assertEqual(release.digest(bundle / name), entry['sha256'])
        for name in ['LICENSE', 'NOTICE', 'third_party/lwip/COPYING', 'third_party/freebsd-e1000/COPYING', 'third_party/r-efi/AUTHORS']:
            self.assertTrue((bundle / name).is_file())
        for line in (bundle / 'SHA256SUMS').read_text().splitlines():
            sha, name = line.split('  ', 1); self.assertEqual(release.digest(bundle / name), sha)
        archive = output / 'musha-os-0.1.0-candidate.tar.gz'
        self.assertEqual((output / 'SHA256SUMS').read_text().split()[0], release.digest(archive))
        with tarfile.open(archive) as tar:
            entry = tar.extractfile('musha-os-0.1.0-candidate/EFI/BOOT/BOOTX64.EFI').read()
            self.assertEqual(entry, self.efi.read_bytes())
            self.assertEqual(tar.extractfile('musha-os-0.1.0-candidate/MUSHA.TXT').read(), b'Hello Musha-OS!\n')
            self.assertTrue(all(m.mtime == 0 and m.uid == 0 for m in tar.getmembers()))

    def test_release_metadata_and_readme(self):
        output = release.stage(self.root / 'release', self.efi,
                               {'commit': 'fixture', 'features': []}, release=True)
        bundle = output / 'musha-os-0.1.0'
        manifest = json.loads((bundle / 'manifest.json').read_text())
        self.assertEqual(manifest['status'], 'initial-development-release')
        self.assertEqual(manifest['features'], [])
        self.assertTrue((bundle / 'README.md').is_file())
        self.assertTrue((output / 'musha-os-0.1.0.tar.gz').is_file())

    def test_existing_output_is_untouched(self):
        out = self.root / 'existing'; out.mkdir(); (out / 'keep').write_text('important')
        with self.assertRaises(FileExistsError): release.stage(out, self.efi, {})
        self.assertEqual((out / 'keep').read_text(), 'important')
        link = self.root / 'link'; link.symlink_to(out, target_is_directory=True)
        with self.assertRaises(FileExistsError): release.stage(link, self.efi, {})

    def test_failed_input_does_not_leave_a_bundle(self):
        self.efi.write_bytes(b'invalid')
        out = self.root / 'invalid'
        with self.assertRaises(ValueError): release.stage(out, self.efi, {})
        self.assertFalse(out.exists())

    def test_device_path_is_rejected(self):
        with self.assertRaises(ValueError): release.stage('/dev/musha-test-never-create', self.efi, {})

if __name__ == '__main__': unittest.main()
