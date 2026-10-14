#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""Candidate comparison must reject source, payload and checksum mismatches."""
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest

spec = importlib.util.spec_from_file_location('comparison', Path(__file__).with_name('compare-candidates.py'))
comparison = importlib.util.module_from_spec(spec)
spec.loader.exec_module(comparison)


class CandidateComparisonTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.left, self.right = [Path(self.temp.name) / name for name in ('left', 'right')]
        for root in (self.left, self.right):
            root.mkdir()
            efi = root / 'musha-os-0.3.0-candidate/EFI/BOOT/BOOTX64.EFI'
            efi.parent.mkdir(parents=True)
            efi.write_bytes(b'fixture EFI')
            (root / 'build-manifest.json').write_text(json.dumps({'commit': 'source-a', 'features': [], 'working_tree': ''}))
            for i in range(5):
                (root / ('archive-%d.zip' % i)).write_bytes(b'fixture archive')
            self.checksums(root)

    def checksums(self, root):
        (root / 'SHA256SUMS').write_text(''.join(comparison.digest(p) + '  ' + p.name + '\n'
            for p in sorted(root.iterdir()) if p.is_file() and p.name != 'SHA256SUMS'))

    def test_same_source_requires_seven_equal_files(self):
        self.assertTrue(comparison.compare(self.left, self.right, 'same-source')['passed'])
        (self.right / 'archive-0.zip').write_bytes(b'changed')
        self.checksums(self.right)
        self.assertFalse(comparison.compare(self.left, self.right, 'same-source')['passed'])

    def test_different_source_only_allowed_for_efi_comparison(self):
        p = self.right / 'build-manifest.json'
        p.write_text(p.read_text().replace('source-a', 'source-b'))
        self.checksums(self.right)
        self.assertFalse(comparison.compare(self.left, self.right, 'same-source')['passed'])
        self.assertTrue(comparison.compare(self.left, self.right, 'efi')['passed'])

    def test_efi_mismatch_rejected_in_both_modes(self):
        (self.right / 'musha-os-0.3.0-candidate/EFI/BOOT/BOOTX64.EFI').write_bytes(b'changed EFI')
        for mode in ('efi', 'same-source'):
            self.assertFalse(comparison.compare(self.left, self.right, mode)['passed'])

    def test_corrupt_archive_rejected_before_comparison(self):
        (self.right / 'archive-0.zip').write_bytes(b'corrupt')
        with self.assertRaisesRegex(ValueError, 'Checksum mismatch'):
            comparison.compare(self.left, self.right, 'efi')

    def test_missing_checksum_entry_rejected(self):
        p = self.right / 'SHA256SUMS'
        p.write_text('\n'.join(p.read_text().splitlines()[1:]) + '\n')
        with self.assertRaisesRegex(ValueError, 'Expected five archives'):
            comparison.compare(self.left, self.right, 'efi')


if __name__ == '__main__':
    unittest.main()
