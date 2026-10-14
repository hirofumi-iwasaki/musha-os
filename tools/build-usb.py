#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""Build a UEFI application, GPT image and FAT32 file-copy bundle."""
import argparse
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import zipfile
import tempfile

from distribution import usb_readme

ROOT = Path(__file__).resolve().parent.parent
TARGET = 'x86_64-unknown-uefi'


def digest(path):
    value = hashlib.sha256()
    with path.open('rb') as source:
        for chunk in iter(lambda: source.read(1024 * 1024), b''):
            value.update(chunk)
    return value.hexdigest()


def package(output, efi, size_mib=64, size_bytes=None, files_only=False):
    # mkdir without exist_ok refuses existing directories and symlinks.
    output.mkdir(parents=True)
    esp = output / 'files'
    boot = esp / 'EFI' / 'BOOT'
    boot.mkdir(parents=True)
    shutil.copyfile(efi, boot / 'BOOTX64.EFI')
    (esp / 'README.TXT').write_bytes(usb_readme((ROOT/'VERSION').read_text().strip()))
    (esp / 'MUSHA.TXT').write_bytes(b'Hello Musha-OS!\n')
    for name in ('LICENSE', 'NOTICE'):
        shutil.copyfile(ROOT / name, esp / name)
    shutil.copyfile(ROOT / 'third_party/r-efi/AUTHORS', esp / 'r-efi-AUTHORS')
    image = None
    if not files_only:
        spec = importlib.util.spec_from_file_location('usb_image', ROOT / 'tools/make-usb-image.py')
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        image = module.create(output / 'musha-os.img', boot / 'BOOTX64.EFI', size_mib, size_bytes)
    with zipfile.ZipFile(output / 'musha-os-fat32-files.zip', 'x', zipfile.ZIP_DEFLATED) as archive:
        for path in sorted(esp.rglob('*')):
            if path.is_file():
                info = zipfile.ZipInfo(path.relative_to(esp).as_posix(), (1980, 1, 1, 0, 0, 0))
                info.compress_type = zipfile.ZIP_DEFLATED
                info.external_attr = 0o100644 << 16
                archive.writestr(info, path.read_bytes())
    for name in ('LICENSE', 'NOTICE'):
        shutil.copyfile(ROOT / name, output / name)
    shutil.copyfile(ROOT / 'third_party/r-efi/AUTHORS', output / 'r-efi-AUTHORS')
    paths = ([image] if image else []) + [output / 'musha-os-fat32-files.zip', boot / 'BOOTX64.EFI', esp / 'README.TXT']
    checksums = {str(path.relative_to(output)): digest(path) for path in paths}
    (output / 'SHA256SUMS').write_text(''.join(f'{value}  {name}\n' for name, value in checksums.items()), encoding='utf-8')
    return {'image_bytes': image.stat().st_size if image else 0, 'sha256': checksums, 'features': [],
            'image_use': 'Exact-capacity raw USB image, or QEMU; default 64MiB is for QEMU.',
            'file_bundle_use': 'Extract at the root of an existing FAT32 USB. No formatting performed.'}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output-dir', type=Path, required=True, help='New directory; existing paths are refused')
    parser.add_argument('--t2-diagnostics', action='store_true', help='Build read-only T2-D1 diagnostics instead of the normal image')
    sizes = parser.add_mutually_exclusive_group()
    sizes.add_argument('--size-mib', type=int, default=64, help='Image size for QEMU (default: 64)')
    sizes.add_argument('--size-bytes', type=int, help='Exact physical USB capacity, multiple of 512')
    parser.add_argument('--with-image', action='store_true', help='Opt in to a legacy QEMU image; default builds files only')
    args = parser.parse_args()
    if not args.with_image and (args.size_bytes is not None or args.size_mib != 64): parser.error('Size options require --with-image')
    if os.environ.get('RUSTFLAGS') or os.environ.get('CARGO_ENCODED_RUSTFLAGS'):
        parser.error('Custom Rust flags are not allowed in USB bundles')
    output = args.output_dir.resolve()
    if args.output_dir.exists() or args.output_dir.is_symlink():
        parser.error('output directory must not already exist')
    size = args.size_bytes if args.size_bytes is not None else args.size_mib * 1024 * 1024
    if not 64 * 1024 * 1024 <= size <= 128 * 1024**3 or size % 512:
        parser.error('image size must be 64MiB..128GiB and a multiple of 512 bytes')
    env = os.environ.copy()
    local = ROOT / '.local-tools'
    if 'CARGO_HOME' not in env and 'RUSTUP_HOME' not in env and (local / 'cargo/bin/cargo').is_file():
        env['CARGO_HOME'] = str(local / 'cargo')
        env['RUSTUP_HOME'] = str(local / 'rustup')
        env['PATH'] = str(local / 'cargo/bin') + os.pathsep + env.get('PATH', '')
    compiler = env.get('CC', 'clang')
    for tool in ('git', 'cargo', compiler):
        if shutil.which(tool, path=env.get('PATH')) is None:
            parser.error(f'{tool} is required; see docs/test-usb-build.md')
    def run(command, capture=False):
        return subprocess.run(command, cwd=ROOT, env=env, check=True,
                              text=True, stdout=subprocess.PIPE if capture else None)
    versions = {'cargo': run(['cargo', '--version'], True).stdout.strip(),
                'clang': run([compiler, '--version'], True).stdout.splitlines()[0]}
    build_temp = tempfile.TemporaryDirectory(prefix='musha-usb-build-')
    target_dir = Path(build_temp.name)
    features = ['t2-diagnostics'] if args.t2_diagnostics else []
    run(['cargo', 'build', '--locked', '--release', '--target', TARGET,
         '--target-dir', str(target_dir), '-p', 'musha-boot']
        + (['--features', ','.join(features)] if features else []))
    manifest = package(output, target_dir / TARGET / 'release/musha-boot.efi', args.size_mib, args.size_bytes, files_only=not args.with_image)
    manifest.update({'target': TARGET, 'tool_versions': versions, 'features': features,
                     'diagnostic_revision': 'T2-D1' if args.t2_diagnostics else 'H19',
                     'commit': run(['git', 'rev-parse', 'HEAD'], True).stdout.strip(),
                     'working_tree_dirty': bool(run(['git', 'status', '--porcelain'], True).stdout.strip())})
    (output / 'build-manifest.json').write_text(json.dumps(manifest, indent=2) + '\n', encoding='utf-8')
    print(f'Created {output}')
    print('Legacy developer bundle: musha-os-fat32-files.zip. Use prepare-release.py for manifest-driven distribution.')
    build_temp.cleanup()


if __name__ == '__main__':
    try:
        main()
    except (OSError, ValueError, subprocess.CalledProcessError) as error:
        print(f'Build failed: {error}', file=sys.stderr)
        sys.exit(1)
