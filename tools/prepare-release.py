#!/usr/bin/env python3
# Copyright 2026 Hirofumi Iwasaki
# SPDX-License-Identifier: Apache-2.0
"""Build a normal EFI and stage a candidate bundle; never flash or publish."""
import argparse
import gzip
import hashlib
import importlib.util
import json
import os
import re
from pathlib import Path
import shutil
import subprocess
import tempfile
import zipfile
import sys
sys.path.insert(0, str(Path(__file__).resolve().parent))
from distribution import version_check, make_zip, json_bytes, usb_readme

ROOT = Path(__file__).resolve().parent.parent

def digest(path):
    h = hashlib.sha256()
    with path.open('rb') as source:
        for chunk in iter(lambda : source.read(1024 * 1024), b''):
            h.update(chunk)
    return h.hexdigest()

def command(*args):
    return subprocess.check_output(
        args,
        cwd=ROOT,
        text=True,
    ).strip()

def documentation_zip(output, files):
    """Ignore checkout/copy mtimes and modes; match the deterministic ZIP contract."""
    with zipfile.ZipFile(
        output,
        'x',
        zipfile.ZIP_DEFLATED,
    ) as archive:
        for (name, path) in sorted(files.items()):
            info = zipfile.ZipInfo(name, (1980, 1, 1, 0, 0, 0))
            info.compress_type = zipfile.ZIP_DEFLATED
            info.create_system = 3
            info.external_attr = 33188 << 16
            archive.writestr(info, path.read_bytes())

def stage(destination, efi, provenance, release=False, version=None, previous_packages=(), docs_complete=False, sdk_samples=None):
    """Stage deterministic development assets; no device/publication operation."""
    from distribution import drive_payload, previous_catalog, digest
    version = version or (ROOT / 'VERSION').read_text().strip()
    version_check(version, provenance.get('tag'))
    if version != (ROOT / 'VERSION').read_text().strip() or provenance.get('version', version) != version:
        raise ValueError('Distribution VERSION/provenance mismatch')
    if provenance.get('features', []) or provenance.get('RUSTFLAGS', '') or provenance.get('CARGO_ENCODED_RUSTFLAGS', ''):
        raise ValueError('Only the normal, unmodified build flags are permitted')
    destination = Path(destination).absolute()
    if destination.exists() or destination.is_symlink():
        raise FileExistsError(destination)
    if str(destination).startswith('/dev/'):
        raise ValueError('Destination must not be a device path')
    destination.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix='.musha-stage-', dir=destination.parent) as tmp:
        work = Path(tmp)
        prefix = f'musha-os-{version}'
        payload = drive_payload(efi, version)
        update = make_zip(
            work / (prefix + '-x64.zip'),
            payload,
            version,
            provenance,
            previous_catalog(previous_packages),
        )
        spec = importlib.util.spec_from_file_location('usb_image', ROOT / 'tools/make-usb-image.py')
        usb = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(usb)
        image = usb.create(
            work / 'qemu.img',
            efi,
            payload=payload,
        )
        with (work / (prefix + '-x64-qemu.img.gz')).open('xb') as raw, gzip.GzipFile(
            filename='',
            mode='wb',
            fileobj=raw,
            mtime=0,
        ) as gz, image.open('rb') as source:
            shutil.copyfileobj(source, gz)
        runtime = {'BOOTX64.EFI': {'bytes': len(payload['EFI/BOOT/BOOTX64.EFI']), 'sha256': digest(efi)}, 'QEMU image': {'bytes': image.stat().st_size, 'sha256': digest(image)}}
        image.unlink()
        spec = importlib.util.spec_from_file_location('sdk_package', ROOT / 'tools/package-sdk.py')
        sdk = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(sdk)
        sdk_status = {}
        for language in ('c', 'rust'):
            sm = sdk.create(
                work / (prefix + '-' + language + '-sdk.zip'),
                version,
                language=language,
                samples=(sdk_samples or {}).get(language, {}),
            )
            sdk_status[language] = {'executable_sdk': sm['executable_sdk'], 'sample_elfs': sm['sample_elfs']}
            for (name, entry) in sm['sample_elfs'].items():
                runtime[language + ' SDK/' + name] = entry
        if release and any((not status['executable_sdk'] or not status['sample_elfs'] for status in sdk_status.values())):
            raise ValueError('Release SDK implementation/sample gate incomplete')
        if docs_complete:
            if not (ROOT / 'docs/README.md').is_file():
                raise ValueError('Documentation stage2 index missing')
            documentation_zip(work / (prefix + '-docs.zip'), {p.relative_to(ROOT / 'docs').as_posix(): p for p in sorted((ROOT / 'docs').rglob('*')) if p.is_file()})
        manifest = dict(
            provenance,
            schema=2,
            version=version,
            status='preview' if provenance.get('working_tree') else 'candidate',
            layout_version=2,
            payload=update['files'],
            update_schema=3,
            retired_files=update['retired_files'],
            runtime_payload=runtime,
            sdk_status=sdk_status,
            docs_stage2_complete=docs_complete,
            hardware_validation='NOT RUN',
            image_use='QEMU only; not a physical USB image',
            assets={p.name: {'bytes': p.stat().st_size, 'sha256': digest(p)} for p in sorted(work.iterdir()) if p.is_file()},
        )
        (work / 'build-manifest.json').write_bytes(json_bytes(manifest))
        (work / 'SHA256SUMS').write_text(''.join((f'{digest(p)}  {p.name}\n' for p in sorted(work.iterdir()) if p.is_file() and p.name != 'SHA256SUMS')))
        destination.mkdir()
        for p in work.iterdir():
            shutil.move(str(p), str(destination / p.name))
    return destination

def validate_efi(efi, original, version, tag, commit, dirty):
    """A reused EFI requires trusted, clean, matching build provenance."""
    if original.get('version') != version or original.get('tag') != tag or original.get('commit') != commit or original.get('working_tree') or dirty or (original.get('features') != []) or (original.get('status') != 'candidate') or (original.get('target') != 'x86_64-unknown-uefi') or (original.get('profile') != 'release') or original.get('RUSTFLAGS') or original.get('CARGO_ENCODED_RUSTFLAGS'):
        raise ValueError('Original normal/clean provenance/version/tag/commit mismatch')
    if digest(efi) != original['payload']['EFI/BOOT/BOOTX64.EFI']['sha256']:
        raise ValueError('EFI hash mismatch')

def main():
    # An isolated target directory cannot reuse a fault/debug build from out/esp.
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        '--output',
        type=Path,
        default=ROOT / 'dist/candidate',
    )
    parser.add_argument('--version', default=(ROOT / 'VERSION').read_text().strip())
    parser.add_argument('--tag', help='Existing vVERSION tag, must resolve to HEAD')
    parser.add_argument(
        '--efi',
        type=Path,
        help='Package a previously verified EFI without rebuilding',
    )
    parser.add_argument(
        '--provenance',
        type=Path,
        help='Original build-manifest.json for --efi',
    )
    parser.add_argument(
        '--allow-dirty',
        action='store_true',
        help='Preview only; record uncommitted changes',
    )
    parser.add_argument(
        '--previous-package',
        type=Path,
        action='append',
        default=[],
        help='Trusted checksum-verified prior drive/files ZIP for hash-protected migration',
    )
    parser.add_argument(
        '--docs-complete',
        action='store_true',
        help='Only after documentation stage2 acceptance',
    )
    parser.add_argument(
        '--release',
        action='store_true',
        help='Package an approved development release; does not publish',
    )
    args = parser.parse_args()
    version_check(args.version, args.tag)
    if args.version != (ROOT / 'VERSION').read_text().strip():
        parser.error('VERSION mismatch')
    if args.tag and command(
        'git',
        'rev-parse',
        'refs/tags/' + args.tag + '^{commit}',
    ) != command(
        'git',
        'rev-parse',
        'HEAD',
    ):
        parser.error('Tag must resolve to HEAD')
    if args.release and (not args.tag):
        parser.error('Release packaging requires an existing tag')
    if args.release and args.allow_dirty:
        parser.error('Release packaging requires a clean commit')
    if args.output.exists() or args.output.is_symlink():
        parser.error('Output must be new')
    dirty = command(
        'git',
        'status',
        '--porcelain',
    )
    if dirty and (not args.allow_dirty):
        parser.error('Commit changes first, or use --allow-dirty for a preview')
    if bool(args.efi) != bool(args.provenance):
        parser.error('--efi and --provenance must be paired')
    if args.efi:
        original = json.loads(args.provenance.read_text())
        validate_efi(
            args.efi,
            original,
            args.version,
            args.tag,
            command(
                'git',
                'rev-parse',
                'HEAD',
            ),
            dirty,
        )
        print(stage(
            args.output,
            args.efi,
            original,
            release=args.release,
            version=args.version,
            previous_packages=args.previous_package,
            docs_complete=args.docs_complete,
        ))
        return
    if os.environ.get('RUSTFLAGS') or os.environ.get('CARGO_ENCODED_RUSTFLAGS'):
        parser.error('Custom Rust flags are not allowed')
    import platform
    cc = os.environ.get('CC', 'clang')
    provenance = {
        'tag': args.tag,
        'target': 'x86_64-unknown-uefi',
        'profile': 'release',
        'toolchain_file': (ROOT / 'rust-toolchain.toml').read_text(),
        'commit': command(
            'git',
            'rev-parse',
            'HEAD',
        ),
        'working_tree': dirty,
        'rustc': command('rustc', '-vV'),
        'cargo': command('cargo', '--version'),
        'clang': command(cc, '--version'),
        'python': command('python3', '--version'),
        'Cargo.lock_sha256': digest(ROOT / 'Cargo.lock'),
        'features': [],
        'RUSTFLAGS': os.environ.get('RUSTFLAGS', ''),
        'CARGO_ENCODED_RUSTFLAGS': os.environ.get('CARGO_ENCODED_RUSTFLAGS', ''),
    }
    provenance['host'] = platform.platform()
    provenance['qemu'] = command('qemu-system-x86_64', '--version') if shutil.which('qemu-system-x86_64') else 'NOT AVAILABLE'
    firmware = Path(os.environ.get('MUSHA_FIRMWARE_DIR', '/opt/homebrew/share/qemu'))
    provenance['firmware'] = {name: digest(firmware / name) if (firmware / name).is_file() else 'NOT AVAILABLE' for name in ['edk2-x86_64-code.fd', 'edk2-i386-vars.fd']}
    with tempfile.TemporaryDirectory(prefix='musha-normal-build-') as tmp:
        subprocess.run(
            ['cargo', 'build', '--locked', '--release', '--target', 'x86_64-unknown-uefi', '-p', 'musha-boot', '--no-default-features', '--target-dir', tmp],
            cwd=ROOT,
            check=True,
        )
        efi = Path(tmp) / 'x86_64-unknown-uefi/release/musha-boot.efi'
        print('Candidate staged:', stage(
            args.output,
            efi,
            provenance,
            release=args.release,
            version=args.version,
            previous_packages=args.previous_package,
            docs_complete=args.docs_complete,
        ))
    print('No device write, tag creation or publication performed.')

if __name__ == '__main__':
    main()
