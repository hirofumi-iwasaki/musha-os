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
from pathlib import Path
import shutil
import subprocess
import tarfile
import tempfile

ROOT = Path(__file__).resolve().parent.parent

def digest(path):
    h = hashlib.sha256()
    with path.open('rb') as source:
        for chunk in iter(lambda: source.read(1024 * 1024), b''):
            h.update(chunk)
    return h.hexdigest()

def command(*args):
    return subprocess.check_output(args, cwd=ROOT, text=True).strip()

def stage(destination, efi, provenance, release=False):
    """Package an already-built candidate into a new directory only."""
    destination = Path(destination).absolute()
    if destination.exists() or destination.is_symlink():
        raise FileExistsError(destination)
    if '/dev' == str(destination) or str(destination).startswith('/dev/'):
        raise ValueError('Destination must not be a device path')
    destination.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix='.musha-stage-', dir=destination.parent) as tmp:
        work = Path(tmp)
        name = 'musha-os-0.1.0' if release else 'musha-os-0.1.0-candidate'
        bundle = work / name
        (bundle / 'EFI/BOOT').mkdir(parents=True)
        shutil.copyfile(efi, bundle / 'EFI/BOOT/BOOTX64.EFI')
        (bundle / 'MUSHA.TXT').write_bytes(b'Hello Musha-OS!\n')
        for name in ['LICENSE', 'NOTICE', 'README.md']:
            shutil.copyfile(ROOT / name, bundle / name)
        for name in sorted(p.name for p in (ROOT / 'docs').glob('*.md')):
            (bundle / 'docs').mkdir(exist_ok=True)
            shutil.copyfile(ROOT / 'docs' / name, bundle / 'docs' / name)
        (bundle / 'tools').mkdir()
        shutil.copyfile(ROOT / 'tools/make-usb-image.py', bundle / 'tools/make-usb-image.py')
        for dependency in ['lwip', 'freebsd-e1000']:
            target = bundle / 'third_party' / dependency
            target.mkdir(parents=True)
            for name in ['COPYING', 'UPSTREAM']:
                shutil.copyfile(ROOT / 'third_party' / dependency / name, target / name)
        target = bundle / 'third_party/r-efi'
        target.mkdir()
        for name in ['AUTHORS', 'UPSTREAM']:
            shutil.copyfile(ROOT / 'third_party/r-efi' / name, target / name)
        spec = importlib.util.spec_from_file_location('usb_image', ROOT / 'tools/make-usb-image.py')
        usb = importlib.util.module_from_spec(spec); spec.loader.exec_module(usb)
        usb.create(bundle / 'musha-qemu-64MiB.img', bundle / 'EFI/BOOT/BOOTX64.EFI')
        provenance = dict(provenance, status='initial-development-release' if release else 'unreleased-candidate', version='0.1.0',
                          image_scope='64MiB QEMU image; physical USB needs exact-capacity regeneration',
                          hardware_validation='See docs/release-validation-0.1.0.md and docs/pci-h19-hardware-results.md' if release else 'NOT RUN',
                          release_acceptance='Limited scope: docs/release-scope-0.1.0.md' if release else 'NOT DETERMINED')
        provenance['files'] = {p.relative_to(bundle).as_posix():
                               {'bytes': p.stat().st_size, 'sha256': digest(p)}
                               for p in sorted(bundle.rglob('*')) if p.is_file()}
        (bundle / 'manifest.json').write_text(json.dumps(provenance, ensure_ascii=False, indent=2) + '\n')
        files = sorted(p for p in bundle.rglob('*') if p.is_file())
        (bundle / 'SHA256SUMS').write_text(''.join(
            f'{digest(p)}  {p.relative_to(bundle).as_posix()}\n' for p in files))
        archive = work / (bundle.name + '.tar.gz')
        with archive.open('wb') as raw, gzip.GzipFile(filename='', mode='wb', fileobj=raw, mtime=0) as gz:
            with tarfile.open(fileobj=gz, mode='w') as tar:
                for p in sorted(bundle.rglob('*')):
                    info = tar.gettarinfo(str(p), arcname=f'{bundle.name}/{p.relative_to(bundle).as_posix()}')
                    info.uid = info.gid = info.mtime = 0
                    info.uname = info.gname = ''
                    info.mode = 0o755 if p.is_dir() else 0o644
                    if p.is_file():
                        with p.open('rb') as source: tar.addfile(info, source)
                    else: tar.addfile(info)
        (work / 'SHA256SUMS').write_text(f'{digest(archive)}  {archive.name}\n')
        # Exclusive mkdir means another process's output is never replaced.
        destination.mkdir()
        for p in list(work.iterdir()): shutil.move(str(p), str(destination / p.name))
    return destination

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', type=Path, default=ROOT / 'dist/mi68-candidate')
    parser.add_argument('--allow-dirty', action='store_true', help='Preview only; record uncommitted changes')
    parser.add_argument('--release', action='store_true', help='Package the approved initial development release; does not publish')
    args = parser.parse_args()
    if args.release and args.allow_dirty: parser.error('Release packaging requires a clean commit')
    if args.output.exists() or args.output.is_symlink(): parser.error('Output must be new')
    dirty = command('git', 'status', '--porcelain')
    if dirty and not args.allow_dirty: parser.error('Commit changes first, or use --allow-dirty for a preview')
    cc = os.environ.get('CC', 'clang')
    provenance = {'commit': command('git', 'rev-parse', 'HEAD'), 'working_tree': dirty,
                  'rustc': command('rustc', '-vV'), 'cargo': command('cargo', '--version'),
                  'clang': command(cc, '--version'), 'python': command('python3', '--version'),
                  'Cargo.lock_sha256': digest(ROOT / 'Cargo.lock'),
                  'features': [], 'RUSTFLAGS': os.environ.get('RUSTFLAGS', ''),
                  'CARGO_ENCODED_RUSTFLAGS': os.environ.get('CARGO_ENCODED_RUSTFLAGS', '')}
    # An isolated target directory cannot reuse a fault/debug build from out/esp.
    with tempfile.TemporaryDirectory(prefix='musha-normal-build-') as tmp:
        subprocess.run(['cargo', 'build', '--locked', '--release', '--target',
                        'x86_64-unknown-uefi', '-p', 'musha-boot', '--no-default-features',
                        '--target-dir', tmp], cwd=ROOT, check=True)
        efi = Path(tmp) / 'x86_64-unknown-uefi/release/musha-boot.efi'
        print('Candidate staged:', stage(args.output, efi, provenance, release=args.release))
    print('No device write, tag creation or publication performed.')

if __name__ == '__main__':
    main()
