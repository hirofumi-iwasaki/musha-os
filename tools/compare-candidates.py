#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""Read-only asset and member-by-member runtime comparison."""
import argparse,gzip,hashlib,importlib.util,json,subprocess,zipfile
from pathlib import Path
from distribution import digest,read_zip,USER_DOCS
from efi_size import measure_bytes as measure_efi_size
ROOT=Path(__file__).resolve().parents[1]

def inventory(root):
    # Independently inspect the image's EFI rather than trusting manifest fields.
    root = Path(root)
    m = json.loads((root / 'build-manifest.json').read_text())
    checks = {}
    required = set(m['assets']) | {'build-manifest.json'}
    for line in (root / 'SHA256SUMS').read_text().splitlines():
        (expected, name) = line.split('  ', 1)
        if Path(name).name != name or name in checks or name not in required:
            raise ValueError('Invalid checksum filename')
        path = root / name
        if path.is_symlink() or not path.is_file() or digest(path) != expected:
            raise ValueError('Checksum mismatch')
        if name in m['assets'] and m['assets'][name] != {'bytes': path.stat().st_size, 'sha256': expected}:
            raise ValueError('Asset manifest mismatch')
        checks[name] = expected
    if set(checks) != required or m.get('features') != [] or m.get('working_tree'):
        raise ValueError('Incomplete/unclean candidate')
    checks['SHA256SUMS'] = digest(root / 'SHA256SUMS')
    prefix = 'musha-os-' + m['version']
    (_, payload) = read_zip(root / (prefix + '-x64.zip'))

    def record(data):
        return {'bytes': len(data), 'sha256': hashlib.sha256(data).hexdigest()}
    image = gzip.decompress((root / (prefix + '-x64-qemu.img.gz')).read_bytes())
    runtime = {'BOOTX64.EFI': record(payload['EFI/BOOT/BOOTX64.EFI']), 'QEMU image': record(image)}
    for language in ('c', 'rust'):
        with zipfile.ZipFile(root / (prefix + '-' + language + '-sdk.zip')) as archive:
            sm = json.loads(archive.read('sdk-manifest.json'))
            if sm['sample_elfs'] != m['sdk_status'][language]['sample_elfs']:
                raise ValueError('SDK manifest differs')
            for (name, entry) in sm['sample_elfs'].items():
                if record(archive.read(name)) != entry:
                    raise ValueError('SDK sample differs')
                runtime[language + ' SDK/' + name] = entry
    if runtime != m['runtime_payload']:
        raise ValueError('Runtime manifest differs')
    if 'efi_size' in m and m['efi_size'] != measure_efi_size(payload['EFI/BOOT/BOOTX64.EFI']):
        raise ValueError('EFI size manifest differs')
    import io
    spec = importlib.util.spec_from_file_location('deploy', ROOT / 'tools/deploy-image.py')
    deploy = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(deploy)
    if deploy.inspect(io.BytesIO(image), len(image))['efi'] != payload['EFI/BOOT/BOOTX64.EFI']:
        raise ValueError('Image and drive EFI differ')
    spec = importlib.util.spec_from_file_location('candidate_image', ROOT / 'tools/candidate-image.py')
    candidate_image = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(candidate_image)
    if candidate_image.managed_files(image) != payload:
        raise ValueError('Image and drive managed files differ')
    return {
        'source_commit': m['commit'],
        'files': checks,
        'runtime_payload': runtime,
        'sdk_status': m['sdk_status'],
        'directory': str(root.resolve()),
    }

def compare(left, right, mode):
    (a, b) = (inventory(left), inventory(right))
    names = sorted(a['files'].keys() | b['files'].keys())
    files = {n: {
        'left': a['files'].get(n),
        'right': b['files'].get(n),
        'identical': n in a['files'] and a['files'][n] == b['files'].get(n),
    } for n in names}
    runtime = {n: {
        'left': a['runtime_payload'].get(n),
        'right': b['runtime_payload'].get(n),
        'identical': n in a['runtime_payload'] and a['runtime_payload'][n] == b['runtime_payload'].get(n),
    } for n in sorted(a['runtime_payload'].keys() | b['runtime_payload'].keys())}
    efi = runtime['BOOTX64.EFI']['identical']
    same = a['source_commit'] == b['source_commit']
    all_assets = all((v['identical'] for v in files.values()))
    all_runtime = all((v['identical'] for v in runtime.values()))
    complete = all((a['sdk_status'][l]['executable_sdk'] and b['sdk_status'][l]['executable_sdk'] and a['sdk_status'][l]['sample_elfs'] and b['sdk_status'][l]['sample_elfs'] for l in ('c', 'rust')))
    allowed = True
    changed = []
    if mode == 'docs-only':
        # A documentation successor must be downstream of the runtime commit.
        # A reversed comparison or a different documentation branch is not a
        # successor even when the changed paths and runtime bytes happen to match.
        ancestry = subprocess.run(
            ['git', 'merge-base', '--is-ancestor', a['source_commit'], b['source_commit']],
            cwd=ROOT,
            check=False,
        )
        if ancestry.returncode not in (0, 1):
            raise ValueError('Unable to verify docs-only source ancestry')
        if ancestry.returncode == 0 and a['source_commit'] != b['source_commit']:
            changed = subprocess.check_output(
                ['git', 'diff', '--name-only', a['source_commit'], b['source_commit']],
                cwd=ROOT,
                text=True,
            ).splitlines()
        allowed = bool(changed) and all(((n == 'README.md' or n.startswith('docs/')) and n not in {'docs/' + name for name in USER_DOCS} for n in changed))
    passed = efi if mode == 'efi' else same and all_assets and all_runtime if mode == 'same-source' else bool(complete and all_runtime and allowed)
    return {
        'mode': mode,
        'left': a,
        'right': b,
        'files': files,
        'runtime_members': runtime,
        'efi_identical': efi,
        'same_source': same,
        'all_assets_identical': all_assets,
        'runtime_identical': all_runtime,
        'sdk_runtime_complete': bool(complete),
        'changed_paths': changed,
        'docs_only_source': allowed,
        'passed': passed,
        'scope': 'EFI-only is limited. Same-source is full asset equality. Docs-only requires complete SDK samples and eligible source changes.',
    }

def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument(
        '--left',
        required=True,
        type=Path,
    )
    p.add_argument(
        '--right',
        required=True,
        type=Path,
    )
    p.add_argument(
        '--mode',
        required=True,
        choices=['efi', 'same-source', 'docs-only'],
    )
    p.add_argument(
        '--report',
        required=True,
        type=Path,
    )
    a = p.parse_args()
    result = compare(
        a.left,
        a.right,
        a.mode,
    )
    a.report.parent.mkdir(parents=True, exist_ok=True)
    with a.report.open('x') as stream:
        stream.write(json.dumps(result, indent=2) + '\n')
    print(('PASS' if result['passed'] else 'FAIL') + ': ' + a.mode)
    return 0 if result['passed'] else 1
if __name__=='__main__':raise SystemExit(main())
