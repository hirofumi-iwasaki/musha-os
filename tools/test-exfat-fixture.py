#!/usr/bin/env python3
# Copyright 2026 Hirofumi Iwasaki
# SPDX-License-Identifier: Apache-2.0
"""Fixture contract tests, independent of the future Musha-OS exFAT reader."""
import hashlib
import importlib.util
import struct
import tempfile
from pathlib import Path
import unittest

spec = importlib.util.spec_from_file_location('fixture', Path(__file__).with_name('make-exfat-fixture.py'))
fixture = importlib.util.module_from_spec(spec)
spec.loader.exec_module(fixture)


class Fixtures(unittest.TestCase):
    def test_deterministic_image_and_manifest(self):
        a, ma = fixture.build()
        b, mb = fixture.build()
        self.assertEqual(a, b)
        self.assertEqual(ma, mb)
        self.assertEqual(len(a), ma['blocks'] * ma['sector_size'])
        self.assertEqual(hashlib.sha256(a).hexdigest(), ma['image_sha256'])
        # DIAGNOSE.TXT replaced MUSHA.TXT; the marker payload is unchanged.
        self.assertEqual(ma['image_sha256'],
                         '9b57b27780ad01ca202aa0b87f6fe807df1b618b4601834e23fce6d46e9a54d9')
        self.assertEqual(a[:6144], a[6144:12288])
        self.assertEqual(a[3:11], b'EXFAT   ')

    def test_known_payloads_and_allocation(self):
        image, m = fixture.build()
        self.assertEqual([f['length'] for f in m['files']], [16, 1283, 15, 0, 700])
        expected = [b'Hello Musha-OS!\n', bytes(range(256)) * 5 + b'end',
                    b'Long file name\n', b'', b'valid' + bytes(695)]
        heap = struct.unpack_from('<I', image, 88)[0] * 512
        bitmap = image[heap + 512:heap + 1024]
        fat = struct.unpack_from('<I', image, 80)[0] * 512
        for entry, payload in zip(m['files'], expected):
            raw = b''
            for i, n in enumerate(entry['clusters']):
                off = heap + (n - 2) * 512
                raw += image[off:off + 512]
                self.assertTrue(bitmap[(n - 2) // 8] & (1 << ((n - 2) % 8)))
                if not entry['no_fat_chain']:
                    following = entry['clusters'][i + 1] if i + 1 < len(entry['clusters']) else 0xffffffff
                    self.assertEqual(struct.unpack_from('<I', image, fat + n * 4)[0], following)
            self.assertEqual(raw[:len(payload)], payload)
            self.assertEqual(entry['sha256'], hashlib.sha256(payload).hexdigest())

    def test_uninitialized_tail_is_not_expected_file_content(self):
        image, m = fixture.build('valid-tail')
        entry = m['files'][-1]
        off = (64 + entry['clusters'][0] - 2) * 512
        self.assertEqual(image[off:off + 8], b'valid\xa5\xa5\xa5')
        self.assertEqual(entry['valid_length'], 5)
        self.assertEqual(entry['sha256'], hashlib.sha256(b'valid' + bytes(695)).hexdigest())
        self.assertEqual(m['expected_result'], 'ok')

    def test_corruptions_are_local_and_have_explicit_expectations(self):
        original, _ = fixture.build()
        locations = {'boot-checksum': 100, 'entry-checksum': 64 * 512 + 64 + 2,
                     'chain-cycle': 24 * 512 + 272 * 4,
                     'bitmap-free': 65 * 512 + (260 - 2) // 8}
        for case, off in locations.items():
            with self.subTest(case=case):
                mutated, m = fixture.build(case)
                changed = [i for i, (a, b) in enumerate(zip(original, mutated)) if a != b]
                self.assertTrue(changed)
                self.assertTrue(all(off <= i < off + (4 if case == 'chain-cycle' else 1) for i in changed))
                self.assertEqual(m['expected_result'], 'Corrupt')
                self.assertEqual(mutated[6144:12288], original[6144:12288])

    def test_root_entry_set_crosses_cluster_boundary(self):
        image, _ = fixture.build()
        # Last file primary at root byte 480; its stream starts in cluster 600.
        self.assertEqual(image[64 * 512 + 480], 0x85)
        self.assertEqual(image[(64 + 600 - 2) * 512], 0xc0)
        self.assertEqual(struct.unpack_from('<I', image, 24 * 512 + 2 * 4)[0], 600)

    def test_refuses_overwrite_and_symlink(self):
        with tempfile.TemporaryDirectory() as tmp:
            p = Path(tmp) / 'existing.img'
            p.write_bytes(b'keep')
            with self.assertRaises(FileExistsError):
                fixture.create(p)
            link = Path(tmp) / 'link.img'
            link.symlink_to(p)
            with self.assertRaises(FileExistsError):
                fixture.create(link)
            self.assertEqual(p.read_bytes(), b'keep')

    def test_bad_case_does_not_create_output(self):
        with tempfile.TemporaryDirectory() as tmp:
            p = Path(tmp) / 'bad.img'
            with self.assertRaises(ValueError):
                fixture.create(p, 'typo')
            self.assertFalse(p.exists())


if __name__ == '__main__':
    unittest.main()
