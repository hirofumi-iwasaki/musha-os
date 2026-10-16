#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""Echo fixture isolation and real ESP/config generation, no QEMU/device access."""
import hashlib
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest
import uuid
from echo_fixture import boot, load


class FixtureTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.efi = self.root / 'test.efi'
        self.efi.write_bytes(load('test-efi-size').fixture())

    def test_opt_in_is_on_esp_and_original_image_is_unchanged(self):
        source = self.root / 'original.img'
        load('make-usb-image').create(source, self.efi)
        before = hashlib.sha256(source.read_bytes()).hexdigest()
        cmd = ['qemu', '-drive', 'if=none,id=data,file=' + str(source)]
        output = boot(cmd, self.root, image=source)
        with output.open('rb') as file:
            file.seek(2 * 512)
            self.assertEqual(file.read(16), uuid.UUID('c12a7328-f81f-11d2-ba4b-00a0c93ec93b').bytes_le)
        self.assertEqual(hashlib.sha256(source.read_bytes()).hexdigest(), before)
        self.assertIn('ide-hd,drive=echo_boot,bootindex=1', cmd)
        self.assertTrue(any(x.startswith('if=none,id=echo_boot,') and str(output) in x for x in cmd))
        self.assertIn(b'log=devices\nlan.echo=on\n', output.read_bytes())
        with output.open('rb') as stream:
            self.assertEqual(load('deploy-image').inspect(stream, output.stat().st_size)['efi'], self.efi.read_bytes())

    def test_off_omits_key_and_replaces_existing_sata_boot(self):
        cmd = ['qemu', '-drive', 'if=ide,file=old', '-boot', 'order=d']
        output = boot(cmd, self.root, efi=self.efi, enabled=False)
        metadata = json.loads((output.parent / 'config.json').read_text())
        self.assertEqual(metadata['config'], 'log=phases\n')
        self.assertFalse(metadata['echo_enabled'])
        self.assertEqual(sum(x.startswith('if=none,id=echo_boot,') for x in cmd), 1)
        self.assertNotIn('-boot', cmd)
        self.assertEqual(cmd[-1], 'ide-hd,drive=echo_boot,bootindex=1')

    def test_supplied_symlink_is_rejected_without_modification(self):
        alias = self.root / 'alias.img'
        alias.symlink_to(self.efi)
        before = self.efi.read_bytes()
        with self.assertRaises(ValueError):
            boot([], self.root, image=alias)
        self.assertEqual(self.efi.read_bytes(), before)


if __name__ == '__main__':
    unittest.main()
