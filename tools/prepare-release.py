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
import tarfile
import tempfile
import zipfile
import sys
sys.path.insert(0, str(Path(__file__).resolve().parent))
from distribution import version_check, make_zip, json_bytes, usb_readme

ROOT = Path(__file__).resolve().parent.parent

def digest(path):
    h = hashlib.sha256()
    with path.open('rb') as source:
        for chunk in iter(lambda: source.read(1024 * 1024), b''):
            h.update(chunk)
    return h.hexdigest()

def command(*args):
    return subprocess.check_output(args, cwd=ROOT, text=True).strip()

def deployment_zip(output, files):
    """Ignore checkout/copy mtimes and modes; match the deterministic ZIP contract."""
    with zipfile.ZipFile(output, 'x', zipfile.ZIP_DEFLATED) as archive:
        for name, path in sorted(files.items()):
            info = zipfile.ZipInfo(name, (1980, 1, 1, 0, 0, 0))
            info.compress_type = zipfile.ZIP_DEFLATED
            info.create_system = 3
            info.external_attr = 0o100644 << 16
            archive.writestr(info, path.read_bytes())

def stage(destination, efi, provenance, release=False, version=None):
    """Package an already-built candidate into a new directory only."""
    version = version or (ROOT/"VERSION").read_text().strip()
    version_check(version, provenance.get("tag"))
    if version != (ROOT/"VERSION").read_text().strip() or provenance.get("version",version)!=version:
        raise ValueError("Distribution VERSION/provenance mismatch")
    if provenance.get("features", []) or provenance.get("RUSTFLAGS", "") or provenance.get("CARGO_ENCODED_RUSTFLAGS", ""):
        raise ValueError("Only the normal, unmodified build flags are permitted")
    destination = Path(destination).absolute()
    if destination.exists() or destination.is_symlink():
        raise FileExistsError(destination)
    if '/dev' == str(destination) or str(destination).startswith('/dev/'):
        raise ValueError('Destination must not be a device path')
    destination.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix='.musha-stage-', dir=destination.parent) as tmp:
        work = Path(tmp)
        name = f'musha-os-{version}' + ('' if release else '-candidate')
        bundle = work / name
        (bundle / 'EFI/BOOT').mkdir(parents=True)
        shutil.copyfile(efi, bundle / 'EFI/BOOT/BOOTX64.EFI')
        (bundle / 'README.TXT').write_bytes(usb_readme(version))
        (bundle / 'DIAGNOSE.TXT').write_bytes(b'Hello Musha-OS!\n')
        for name in ['LICENSE', 'NOTICE', 'README.md', 'VERSION']:
            shutil.copyfile(ROOT / name, bundle / name)
        for name in sorted(p.name for p in (ROOT / 'docs').glob('*.md')):
            (bundle / 'docs').mkdir(exist_ok=True)
            shutil.copyfile(ROOT / 'docs' / name, bundle / 'docs' / name)
        (bundle / 'tools').mkdir()
        shutil.copyfile(ROOT / 'tools/make-usb-image.py', bundle / 'tools/make-usb-image.py')
        shutil.copyfile(ROOT / 'tools/distribution.py', bundle / 'tools/distribution.py')
        for dependency in ['lwip', 'freebsd-e1000', 'freebsd-apple-bce']:
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
        shutil.copyfile(ROOT / 'tools/make-exfat-fixture.py', bundle / 'tools/make-exfat-fixture.py')
        fixture = subprocess.check_output(['python3', str(ROOT / 'tools/make-exfat-fixture.py'), str(bundle / 'exfat-test-data.img')], text=True)
        (bundle / 'exfat-test-data.json').write_text(fixture)
        provenance = dict(provenance, status='initial-development-release' if release else 'unreleased-candidate', version=version,
                          image_scope='64MiB fixed FAT32 Basic Data; deploy-image.py preserves partition type; physical Basic Data boot pending',
                          hardware_validation=f'See docs/release-validation-{version}.md' if release else 'NOT RUN',
                          release_acceptance=f'Limited scope: docs/release-scope-{version}.md' if release else 'NOT DETERMINED')
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
        # Distribution assets use the exact same EFI as the retained QEMU bundle.
        prefix = f'musha-os-v{version}-x64'
        payload = {name:(bundle / name).read_bytes() for name in ['EFI/BOOT/BOOTX64.EFI','DIAGNOSE.TXT','README.TXT','LICENSE','NOTICE']}
        for path in sorted((bundle / 'third_party').rglob('*')):
            if path.is_file(): payload[path.relative_to(bundle).as_posix()] = path.read_bytes()
        update = make_zip(work / (prefix+'-files.zip'), payload, version, provenance)
        image = bundle / 'musha-qemu-64MiB.img'
        with (work / (prefix+'-usb.img.gz')).open('xb') as raw, gzip.GzipFile(filename='',mode='wb',fileobj=raw,mtime=0) as gz, image.open('rb') as source:
            shutil.copyfileobj(source,gz)
        toolzip = work / (prefix+'-deployment.zip')
        deployment = {}
        for name in ['distribution.py', 'deploy-image.py', 'update-usb.py']:
            deployment['tools/'+name] = ROOT/'tools'/name
        for name in ['release-distribution-policy.md', 'distribution-usage.md']:
            if (ROOT/'docs'/name).exists():
                deployment['docs/'+name] = ROOT/'docs'/name
        for name in ['LICENSE', 'NOTICE']:
            deployment[name] = ROOT/name
        for path in sorted((bundle/'third_party').rglob('*')):
            if path.is_file():
                deployment[path.relative_to(bundle).as_posix()] = path
        deployment_zip(toolzip, deployment)
        sdk_spec = importlib.util.spec_from_file_location('package_sdk', ROOT/'tools/package-sdk.py')
        sdk_package = importlib.util.module_from_spec(sdk_spec); sdk_spec.loader.exec_module(sdk_package)
        sdk_package.create(work / (prefix+'-c-sdk.zip'), version)
        manifest = dict(provenance, schema=1, tag=provenance.get('tag'),
            status='preview' if provenance.get('working_tree') else 'candidate',
            layout_version=1, minimum_media_bytes=64*1024**2, logical_sector_bytes=512,
            image_uncompressed={'bytes':image.stat().st_size,'sha256':digest(image)},
            payload=update['files'], update_schema=update['schema'], retired_files=update.get('retired_files',[]), assets={path.name:{'bytes':path.stat().st_size,'sha256':digest(path)}
                for path in sorted(work.iterdir()) if path.is_file()})
        (work/'build-manifest.json').write_bytes(json_bytes(manifest))
        (work / 'SHA256SUMS').write_text(''.join(f'{digest(path)}  {path.name}\n' for path in sorted(work.iterdir()) if path.is_file() and path.name!='SHA256SUMS'))
        # Exclusive mkdir means another process's output is never replaced.
        destination.mkdir()
        for p in list(work.iterdir()): shutil.move(str(p), str(destination / p.name))
    return destination

def validate_efi(efi, original, version, tag, commit, dirty):
    """A reused EFI requires trusted, clean, matching build provenance."""
    if (original.get('version') != version or original.get('tag') != tag
            or original.get('commit') != commit or original.get('working_tree') or dirty
            or original.get('features') != [] or original.get('status') != 'candidate'
            or original.get('target') != 'x86_64-unknown-uefi' or original.get('profile') != 'release'
            or original.get('RUSTFLAGS') or original.get('CARGO_ENCODED_RUSTFLAGS')):
        raise ValueError('Original normal/clean provenance/version/tag/commit mismatch')
    if digest(efi) != original['payload']['EFI/BOOT/BOOTX64.EFI']['sha256']:
        raise ValueError('EFI hash mismatch')

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', type=Path, default=ROOT / 'dist/candidate')
    parser.add_argument('--version', default=(ROOT/'VERSION').read_text().strip())
    parser.add_argument('--tag', help='Existing vVERSION tag, must resolve to HEAD')
    parser.add_argument('--efi', type=Path, help='Package a previously verified EFI without rebuilding')
    parser.add_argument('--provenance', type=Path, help='Original build-manifest.json for --efi')
    parser.add_argument('--allow-dirty', action='store_true', help='Preview only; record uncommitted changes')
    parser.add_argument('--release', action='store_true', help='Package an approved development release; does not publish')
    args = parser.parse_args()
    version_check(args.version, args.tag)
    if args.version != (ROOT/'VERSION').read_text().strip(): parser.error('VERSION mismatch')
    if args.tag and command('git','rev-parse','refs/tags/'+args.tag+'^{commit}') != command('git','rev-parse','HEAD'): parser.error('Tag must resolve to HEAD')
    if args.release and not args.tag: parser.error('Release packaging requires an existing tag')
    if args.release and args.allow_dirty: parser.error('Release packaging requires a clean commit')
    if args.output.exists() or args.output.is_symlink(): parser.error('Output must be new')
    dirty = command('git', 'status', '--porcelain')
    if dirty and not args.allow_dirty: parser.error('Commit changes first, or use --allow-dirty for a preview')
    if bool(args.efi) != bool(args.provenance): parser.error('--efi and --provenance must be paired')
    if args.efi:
        original=json.loads(args.provenance.read_text())
        validate_efi(args.efi,original,args.version,args.tag,command('git','rev-parse','HEAD'),dirty)
        print(stage(args.output,args.efi,original,release=args.release,version=args.version)); return
    if os.environ.get('RUSTFLAGS') or os.environ.get('CARGO_ENCODED_RUSTFLAGS'): parser.error('Custom Rust flags are not allowed')
    import platform
    cc = os.environ.get('CC', 'clang')
    provenance = {'tag':args.tag, 'target':'x86_64-unknown-uefi', 'profile':'release', 'toolchain_file':(ROOT/'rust-toolchain.toml').read_text(), 'commit': command('git', 'rev-parse', 'HEAD'), 'working_tree': dirty,
                  'rustc': command('rustc', '-vV'), 'cargo': command('cargo', '--version'),
                  'clang': command(cc, '--version'), 'python': command('python3', '--version'),
                  'Cargo.lock_sha256': digest(ROOT / 'Cargo.lock'),
                  'features': [], 'RUSTFLAGS': os.environ.get('RUSTFLAGS', ''),
                  'CARGO_ENCODED_RUSTFLAGS': os.environ.get('CARGO_ENCODED_RUSTFLAGS', '')}
    provenance['host'] = platform.platform()
    provenance['qemu'] = command('qemu-system-x86_64','--version') if shutil.which('qemu-system-x86_64') else 'NOT AVAILABLE'
    firmware = Path(os.environ.get('MUSHA_FIRMWARE_DIR','/opt/homebrew/share/qemu'))
    provenance['firmware'] = {name:digest(firmware/name) if (firmware/name).is_file() else 'NOT AVAILABLE' for name in ['edk2-x86_64-code.fd','edk2-i386-vars.fd']}
    # An isolated target directory cannot reuse a fault/debug build from out/esp.
    with tempfile.TemporaryDirectory(prefix='musha-normal-build-') as tmp:
        subprocess.run(['cargo', 'build', '--locked', '--release', '--target',
                        'x86_64-unknown-uefi', '-p', 'musha-boot', '--no-default-features',
                        '--target-dir', tmp], cwd=ROOT, check=True)
        efi = Path(tmp) / 'x86_64-unknown-uefi/release/musha-boot.efi'
        print('Candidate staged:', stage(args.output, efi, provenance, release=args.release, version=args.version))
    print('No device write, tag creation or publication performed.')

if __name__ == '__main__':
    main()
