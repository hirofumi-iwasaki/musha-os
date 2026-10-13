#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""Run existing viewer/cooperative cases with both new GPT boot partition types.

Creates only regular test images. External ELF cases are run separately with
smoke-external-elf.py --partition-type basic-data/esp (normal and debug).
"""
import argparse
import json
from pathlib import Path
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--firmware-dir', required=True, type=Path)
    p.add_argument('--output', required=True, type=Path)
    p.add_argument('--qemu', default='qemu-system-x86_64')
    a = p.parse_args()
    out = a.output.resolve()
    out.mkdir(parents=True, exist_ok=False)
    report = {'scope': 'QEMU only; physical boot pending', 'commands': [], 'cases': []}

    def run(name, args):
        args = list(map(str, args))
        report['commands'].append(args)
        with (out / (name + '.log')).open('w') as log:
            subprocess.run(args, cwd=ROOT, stdout=log, stderr=subprocess.STDOUT, check=True)

    def smoke(name, script, args):
        run(name, [sys.executable, ROOT / 'tools' / script, '--firmware-dir', a.firmware_dir,
                   '--qemu', a.qemu, '--output', out / name, *args])
        report['cases'].append({'name': name, 'passed': True, 'evidence': str(out / name)})
        (out / 'report.json').write_text(json.dumps(report, indent=2) + '\n')
        print('PASS', name, flush=True)

    try:
        images = {}
        for profile, features in [('viewer', 'file-viewer-probe'),
                                  ('delay', 'file-viewer-probe,live-file-probe'),
                                  ('timeout', 'file-viewer-probe,file-app-timeout'),
                                  ('cooperative', 'qemu-debug'),
                                  ('tx-timeout', 'net-tx-timeout'), ('app-error', 'app-step-error')]:
            target = out / ('target-' + profile)
            run('build-' + profile, ['cargo', 'build', '--locked', '--release', '--target',
                                     'x86_64-unknown-uefi', '-p', 'musha-boot', '--features', features,
                                     '--target-dir', target])
            efi = target / 'x86_64-unknown-uefi/release/musha-boot.efi'
            for kind in ('basic-data', 'esp'):
                image = out / (profile + '-' + kind + '.img')
                run('make-' + profile + '-' + kind, [sys.executable, ROOT / 'tools/make-usb-image.py',
                                                    image, '--efi', efi, '--partition-type', kind])
                images[profile, kind] = image
        # Existing exFAT fixtures are data-only test files, never new distribution partitions.
        exfat, corrupt = out / 'exfat.img', out / 'exfat-corrupt.img'
        for image, case in [(exfat, 'normal'), (corrupt, 'boot-checksum')]:
            run(image.stem, [sys.executable, ROOT / 'tools/make-exfat-fixture.py', image, '--case', case])
        for kind in ('basic-data', 'esp'):
            for split in (False, True):
                smoke('viewer-fat32-' + kind + ('-split' if split else ''), 'smoke-viewer.py',
                      ['--boot-image', images['viewer', kind], '--data-image', images['viewer', kind],
                       '--format', 'fat32', *(['--split-controller'] if split else [])])
                smoke('viewer-exfat-' + kind + ('-split' if split else ''), 'smoke-viewer.py',
                      ['--boot-image', images['viewer', kind], '--data-image', exfat,
                       *(['--split-controller'] if split else [])])
            smoke('viewer-no-medium-' + kind, 'smoke-viewer.py',
                  ['--boot-image', images['viewer', kind], '--data-image', corrupt, '--case', 'no-medium'])
            for case in ('cancel', 'disconnect', 'timeout'):
                profile = 'timeout' if case == 'timeout' else 'delay'
                for fmt, data in [('fat32', images[profile, kind]), ('exfat', exfat)]:
                    smoke('viewer-' + case + '-' + fmt + '-' + kind, 'smoke-viewer.py',
                          ['--boot-image', images[profile, kind], '--data-image', data, '--format', fmt,
                           '--case', case, '--split-controller'])
            for case in ('traffic', 'input-idle', 'no-keyboard', 'keyboard-disconnect', 'link-down', 'tx-timeout', 'app-error'):
                profile = case if case in ('tx-timeout', 'app-error') else 'cooperative'
                smoke('cooperative-' + case + '-' + kind, 'smoke-cooperative.py',
                      ['--usb-image', images[profile, kind], '--case', case])
            smoke('cooperative-hub5-' + kind, 'smoke-cooperative.py',
                  ['--usb-image', images['cooperative', kind], '--hub-depth', '5'])
    except Exception as e:
        report['failure'] = str(e)
        raise
    finally:
        (out / 'report.json').write_text(json.dumps(report, indent=2) + '\n')


if __name__ == '__main__':
    main()
