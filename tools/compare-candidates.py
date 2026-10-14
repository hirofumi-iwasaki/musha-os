#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""Read-only comparison of candidate EFI and seven published files.

--mode efi requires EFI equality only; it does not approve changed payloads.
--mode same-source also requires identical source commits and all seven files.
"""
import argparse
import hashlib
import json
from pathlib import Path


def digest(path):
    h = hashlib.sha256()
    with path.open('rb') as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b''):
            h.update(chunk)
    return h.hexdigest()


def inventory(root):
    manifest = json.loads((root / 'build-manifest.json').read_text())
    checks = {}
    for line in (root / 'SHA256SUMS').read_text().splitlines():
        expected, name = line.split('  ', 1)
        if Path(name).name != name or name in checks:
            raise ValueError('Invalid or duplicate checksum filename: ' + name)
        actual = digest(root / name)
        if actual != expected:
            raise ValueError('Checksum mismatch: ' + str(root / name))
        checks[name] = actual
    if len(checks) != 6 or 'build-manifest.json' not in checks:
        raise ValueError('Expected five archives and build-manifest in SHA256SUMS')
    checks['SHA256SUMS'] = digest(root / 'SHA256SUMS')
    efis = list(root.glob('musha-os-*-candidate/EFI/BOOT/BOOTX64.EFI'))
    if len(efis) != 1:
        raise ValueError('Expected exactly one unpacked candidate EFI')
    if manifest.get('features') != [] or manifest.get('working_tree') != '':
        raise ValueError('Expected a clean, feature-free candidate')
    return {'source_commit': manifest['commit'], 'efi_sha256': digest(efis[0]),
            'files': checks, 'directory': str(root.resolve())}


def compare(left, right, mode):
    a, b = inventory(left), inventory(right)
    names = sorted(a['files'].keys() | b['files'].keys())
    files = {name: {'left': a['files'].get(name), 'right': b['files'].get(name),
                    'identical': name in a['files'] and name in b['files'] and
                    a['files'][name] == b['files'][name]} for name in names}
    efi_equal = a['efi_sha256'] == b['efi_sha256']
    source_equal = a['source_commit'] == b['source_commit']
    all_equal = len(files) == 7 and all(item['identical'] for item in files.values())
    return {'mode': mode, 'left': a, 'right': b, 'files': files,
            'efi_identical': efi_equal, 'same_source': source_equal,
            'seven_files_identical': all_equal,
            'passed': efi_equal and (mode == 'efi' or (source_equal and all_equal)),
            'scope': 'EFI-only mode does not establish runtime payload or archive equivalence.'}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--left', required=True, type=Path)
    parser.add_argument('--right', required=True, type=Path)
    parser.add_argument('--mode', required=True, choices=['efi', 'same-source'])
    parser.add_argument('--report', required=True, type=Path)
    args = parser.parse_args()
    result = compare(args.left, args.right, args.mode)
    args.report.parent.mkdir(parents=True, exist_ok=True)
    with args.report.open('x') as stream:
        stream.write(json.dumps(result, indent=2) + '\n')
    print(('PASS' if result['passed'] else 'FAIL') + ': ' + args.mode)
    return 0 if result['passed'] else 1


if __name__ == '__main__':
    raise SystemExit(main())
