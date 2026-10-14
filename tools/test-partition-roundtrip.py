#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""macOS make/deploy/update integration on NEW virtual disks only. No physical USB."""
import argparse
import hashlib
import importlib.util
import json
import plistlib
import subprocess
import sys
from pathlib import Path
from distribution import make_zip


def module(name):
    spec = importlib.util.spec_from_file_location(name, Path(__file__).with_name(name + '.py'))
    result = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(result)
    return result


def command(*args):
    return subprocess.check_output(list(map(str, args)), stderr=subprocess.PIPE)


def metadata(image):
    with image.open('rb') as f:
        primary = f.read(34 * 512)
        f.seek(-33 * 512, 2)
        return primary + f.read(33 * 512)


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--efi', required=True, type=Path)
    p.add_argument('--output', required=True, type=Path)
    a = p.parse_args()
    if sys.platform != 'darwin':
        p.error('Requires macOS virtual-disk services')
    out = a.output.resolve()
    if str(out).startswith(('/dev/', '/Volumes/')):
        p.error('Use a host directory, never a device or mounted volume')
    out.mkdir(parents=True, exist_ok=False)
    usb, deploy, update = (module(n) for n in ('make-usb-image', 'deploy-image', 'update-usb'))
    package = out / 'update.zip'
    make_zip(package, {'EFI/BOOT/BOOTX64.EFI': a.efi.read_bytes(), 'DIAGNOSE.TXT': b'Updated file only\n'},
             '0.3.0', {'commit': '0' * 40, 'features': [], 'working_tree': 'local virtual test'})
    report = {'scope': 'Virtual images only; no physical acceptance', 'cases': []}
    try:
        for kind, content in [('basic-data', 'Microsoft Basic Data'), ('esp', 'EFI')]:
            source = usb.create(out / (kind + '-source.img'), a.efi, partition_type=kind)
            image = out / (kind + '.img')
            deploy.adjust(source, image, 128 * 1024**2, apply=True)
            before = metadata(image)
            attached = plistlib.loads(command('hdiutil', 'attach', '-nomount', '-readwrite',
                                             '-imagekey', 'diskimage-class=CRawDiskImage', '-plist', image))
            entities = attached['system-entities']
            whole = next(e['dev-entry'] for e in entities if e.get('content-hint') == 'GUID_partition_scheme')
            volume = out / (kind + '-volume')
            volume.mkdir()
            try:
                part = next(e['dev-entry'] for e in entities if e['dev-entry'] != whole)
                info = plistlib.loads(command('diskutil', 'info', '-plist', whole))
                if info.get('VirtualOrPhysical') != 'Virtual':
                    raise ValueError('Refusing anything except the attached virtual image')
                command('diskutil', 'mount', '-mountPoint', volume, part)
                info = plistlib.loads(command('diskutil', 'info', '-plist', part))
                assert info['Content'] == content and info['FilesystemName'] == 'MS-DOS FAT32'
                assert info['MountPoint'] == str(volume)
                efi = volume / 'EFI/BOOT/BOOTX64.EFI'
                stamp = efi.stat().st_mtime_ns
                # Only this test invokes the file updater on a verified virtual mount.
                # The physical CLI's identity checks are not weakened.
                (volume/'MUSHA.TXT').write_bytes(b'Hello Musha-OS!\n')
                result = update.update(package, volume, out / (kind + '-backup'), apply=True)
                assert result['changed'] == ['DIAGNOSE.TXT','update-manifest.json'] and result['applied']
                assert result['removed_completed']==['MUSHA.TXT']
                assert not (volume/'MUSHA.TXT').exists()
                assert (out/(kind+'-backup')/'MUSHA.TXT').read_bytes()==b'Hello Musha-OS!\n'
                assert efi.stat().st_mtime_ns == stamp
                assert (volume / 'DIAGNOSE.TXT').read_bytes() == b'Updated file only\n'
                assert update.update(package, volume, out / (kind + '-unused'), apply=True)['changed'] == []
                assert not (out / (kind + '-unused')).exists()
            finally:
                command('hdiutil', 'detach', whole)
            assert metadata(image) == before, 'GPT changed during file-only update'
            report['cases'].append({'partition_type': kind, 'passed': True,
                                    'changed': result['changed'], 'gpt_sha256': hashlib.sha256(before).hexdigest(),
                                    'efi_mtime_unchanged': True, 'second_update_changed': []})
            print('PASS virtual make/deploy/update:', kind, flush=True)
    except Exception as e:
        report['error'] = str(e)
        raise
    finally:
        (out / 'report.json').write_text(json.dumps(report, indent=2) + '\n')


if __name__ == '__main__':
    main()
