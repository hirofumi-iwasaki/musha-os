#!/usr/bin/env python3
# Copyright 2026 Hirofumi Iwasaki
# SPDX-License-Identifier: Apache-2.0
"""Exercise candidate packaging, provenance and output preservation."""
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import struct
import subprocess
import tarfile
import tempfile
import unittest
import zipfile

spec = importlib.util.spec_from_file_location('release', Path(__file__).with_name('prepare-release.py'))
release = importlib.util.module_from_spec(spec); spec.loader.exec_module(release)

VERSION=(release.ROOT/'VERSION').read_text().strip()

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
        output = release.stage(self.root / 'candidate', self.efi, {'commit': 'fixture', 'features': []}, version=f'{VERSION}')
        bundle = output / f'musha-os-{VERSION}-candidate'
        manifest = json.loads((bundle / 'manifest.json').read_text())
        self.assertEqual(manifest['hardware_validation'], 'NOT RUN')
        self.assertEqual(manifest['status'], 'unreleased-candidate')
        for name, entry in manifest['files'].items():
            self.assertEqual(release.digest(bundle / name), entry['sha256'])
        for name in ['LICENSE', 'NOTICE', 'third_party/lwip/COPYING', 'third_party/freebsd-e1000/COPYING', 'third_party/freebsd-apple-bce/COPYING', 'third_party/r-efi/AUTHORS']:
            self.assertTrue((bundle / name).is_file())
        for line in (bundle / 'SHA256SUMS').read_text().splitlines():
            sha, name = line.split('  ', 1); self.assertEqual(release.digest(bundle / name), sha)
        rebuilt = self.root/'standalone image.img'
        subprocess.run(['python3',str(bundle/'tools/make-usb-image.py'),str(rebuilt),'--efi',str(bundle/'EFI/BOOT/BOOTX64.EFI')],check=True,stdout=subprocess.PIPE)
        self.assertEqual(release.digest(rebuilt),release.digest(bundle/'musha-qemu-64MiB.img'))
        archive = output / f'musha-os-{VERSION}-candidate.tar.gz'
        self.assertIn(f'{release.digest(archive)}  {archive.name}', (output / 'SHA256SUMS').read_text())
        with tarfile.open(archive) as tar:
            entry = tar.extractfile(f'musha-os-{VERSION}-candidate/EFI/BOOT/BOOTX64.EFI').read()
            self.assertEqual(entry, self.efi.read_bytes())
            self.assertEqual(tar.extractfile(f'musha-os-{VERSION}-candidate/README.TXT').read(), release.usb_readme(VERSION))
            self.assertIn('README.TXT', manifest['files'])
            self.assertEqual(tar.extractfile(f'musha-os-{VERSION}-candidate/DIAGNOSE.TXT').read(), b'Hello Musha-OS!\n')
            self.assertTrue(all(m.mtime == 0 and m.uid == 0 for m in tar.getmembers()))

    def test_c_sdk_is_standalone_deterministic_and_manifested(self):
        a = release.stage(self.root/'sdk-one', self.efi, {'commit':'fixture', 'features': []})
        b = release.stage(self.root/'sdk-two', self.efi, {'commit':'fixture', 'features': []})
        # Fixture packaging, not candidate generation: compare every top-level
        # archive and its derived manifest/checksum, not just the C SDK.
        names = sorted(p.name for p in a.iterdir() if p.is_file())
        self.assertEqual(names, sorted(p.name for p in b.iterdir() if p.is_file()))
        for asset in names:
            self.assertEqual(release.digest(a/asset), release.digest(b/asset), asset)
        version = (release.ROOT/'VERSION').read_text().strip()
        name = f'musha-os-v{version}-x64-c-sdk.zip'
        self.assertEqual((a/name).read_bytes(), (b/name).read_bytes())
        manifest = json.loads((a/'build-manifest.json').read_text())
        self.assertEqual(manifest['assets'][name]['sha256'], release.digest(a/name))
        self.assertIn(f'{release.digest(a/name)}  {name}', (a/'SHA256SUMS').read_text())
        with zipfile.ZipFile(a/name) as z:
            names=z.namelist()
            for needed in ['src/crt0.S','src/libmusha.c','linker/app.ld','sysroot/include/musha/api.h','tools/check-sdk.py','tools/app-check.py','examples/hello-c/main.c','CMakeLists.txt','LICENSE','NOTICE','docs/sdk-guide.md']:
                self.assertIn(needed,names)
            self.assertFalse(any(n.startswith(('rust/','targets/','examples/hello-rust/')) or n.endswith('.rs') for n in names))
            self.assertEqual(z.read('docs/sdk-guide.md'), (release.ROOT/'docs/sdk-guide.md').read_bytes())
            self.assertIn(b'(docs/sdk-guide.md)', z.read('README.md'))
            self.assertIn(b'(sdk-guide.md)', z.read('docs/sdk.md'))
            sm=json.loads(z.read('sdk-manifest.json'))
            for n,v in sm['files'].items(): self.assertEqual(hashlib.sha256(z.read(n)).hexdigest(),v['sha256'])

    def test_deployment_zip_ignores_input_metadata_and_order(self):
        inputs = {}
        for name in ['tools/update-usb.py', 'LICENSE', 'third_party/r-efi/AUTHORS']:
            path = self.root/'inputs'/name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(('fixture for '+name).encode())
            os.utime(path, (946684800, 946684800))
            path.chmod(0o600)
            inputs[name] = path
        first, second = self.root/'first.zip', self.root/'second.zip'
        release.deployment_zip(first, inputs)
        for path in inputs.values():
            os.utime(path, (1735689600, 1735689600))
            path.chmod(0o755)
        release.deployment_zip(second, dict(reversed(list(inputs.items()))))
        self.assertEqual(first.read_bytes(), second.read_bytes())
        with zipfile.ZipFile(second) as z:
            self.assertEqual(z.namelist(), sorted(inputs))
            for entry in z.infolist():
                self.assertEqual(z.read(entry.filename), inputs[entry.filename].read_bytes())
                self.assertEqual(entry.date_time, (1980, 1, 1, 0, 0, 0))
                self.assertEqual(entry.create_system, 3)
                self.assertEqual(entry.external_attr, 0o100644 << 16)
        original = first.read_bytes()
        with self.assertRaises(FileExistsError):
            release.deployment_zip(first, inputs)
        self.assertEqual(first.read_bytes(), original)
        inputs['LICENSE'].write_bytes(b'changed content')
        third = self.root/'third.zip'
        release.deployment_zip(third, inputs)
        self.assertNotEqual(first.read_bytes(), third.read_bytes())

    def test_release_metadata_and_readme(self):
        output = release.stage(self.root / 'release', self.efi,
                               {'commit': 'fixture', 'features': []}, release=True, version=f"{VERSION}")
        bundle = output / f'musha-os-{VERSION}'
        manifest = json.loads((bundle / 'manifest.json').read_text())
        self.assertEqual(manifest['status'], 'initial-development-release')
        self.assertEqual(manifest['features'], [])
        self.assertTrue((bundle / 'README.md').is_file())
        self.assertTrue((output / f'musha-os-{VERSION}.tar.gz').is_file())

    def test_candidate_contains_exfat_and_pending_acceptance(self):
        output = release.stage(self.root / 'v020', self.efi, {'commit': 'fixture', 'features': []})
        bundle = output / f'musha-os-{VERSION}-candidate'
        manifest = json.loads((bundle / 'manifest.json').read_text())
        self.assertEqual(manifest['version'], f'{VERSION}')
        self.assertEqual(manifest['hardware_validation'], 'NOT RUN')
        self.assertEqual(manifest['release_acceptance'], 'NOT DETERMINED')
        self.assertEqual(manifest['features'], [])
        fixture = json.loads((bundle / 'exfat-test-data.json').read_text())
        self.assertTrue((bundle / 'exfat-test-data.img').is_file())
        for name, entry in manifest['files'].items():
            self.assertEqual(release.digest(bundle / name), entry['sha256'])
        with self.assertRaises(ValueError):
            release.stage(self.root / 'bad-version', self.efi, {}, version='../unsafe')

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
