#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""Deterministic standalone C SDK sources. No Rust guest or host execution."""
import argparse, hashlib, json
from pathlib import Path
import zipfile
ROOT=Path(__file__).resolve().parents[1]

def create(output, version, language='c', samples=None):
    # Include the source ledger for inspection, not as a parent-directory dependency.
    # The repository README lives one directory below docs; the archive is standalone.
    if language not in ('c', 'rust'):
        raise ValueError('Unknown SDK language')
    output = Path(output)
    payload = {}
    for p in sorted((ROOT / 'sdk').rglob('*')):
        if not p.is_file():
            continue
        rel = p.relative_to(ROOT / 'sdk')
        if rel.parts[0] not in ('README.md', 'CMakeLists.txt', 'abi', 'cmake', 'examples', 'linker', 'src', 'sysroot', 'tests', 'tools', 'rust', 'targets'):
            continue
        if '__pycache__' in rel.parts:
            continue
        if language == 'c' and any((x in ('rust', 'targets', 'hello-rust') for x in rel.parts)):
            continue
        if p.suffix == '.pyc' or (language == 'c' and p.suffix == '.rs'):
            continue
        payload[rel.as_posix()] = p.read_bytes()
    payload['tools/check-sdk.py'] = (ROOT / 'tools/check-sdk.py').read_bytes()
    for name in ('LICENSE', 'NOTICE'):
        payload[name] = (ROOT / name).read_bytes()
    for name in ('application-abi.md', 'sdk.md', 'sdk-guide.md', 'external-app.md', 'file-viewer.md', 'distribution-usage.md', 'formatted-media-boot-verification.md', 'distribution-usage-through-0.3.0.md'):
        payload['docs/' + name] = (ROOT / 'docs' / name).read_bytes()
    payload['README.md'] = payload['README.md'].replace(b'(../docs/', b'(docs/')
    sample_elfs = {}
    for (name, path) in sorted((samples or {}).items()):
        if name not in ('hello.elf', 'start-record.elf'):
            raise ValueError('Unknown SDK sample')
        data = Path(path).read_bytes()
        import importlib.util
        spec = importlib.util.spec_from_file_location('app_checker', ROOT / 'sdk/tools/app-check.py')
        checker = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(checker)
        checker.check(data)
        payload['samples/' + name] = data
        sample_elfs['samples/' + name] = {'bytes': len(data), 'sha256': hashlib.sha256(data).hexdigest()}
    if language == 'rust':
        payload['DEVELOPMENT-STATUS.TXT'] = b'Rust SDK source scaffold only. Runnable Rust guest/core build awaits stage 4.\n'
    manifest = {
        'version': version,
        'language': language.upper(),
        'executable_sdk': language == 'c',
        'sample_elfs': sample_elfs,
        'abi': 'experimental candidate r1',
        'public_compatibility_fixed': False,
        'rust_included': language == 'rust',
        'files': {n: {'bytes': len(b), 'sha256': hashlib.sha256(b).hexdigest()} for (n, b) in sorted(payload.items())},
    }
    payload['sdk-manifest.json'] = (json.dumps(manifest, indent=2) + '\n').encode()
    with zipfile.ZipFile(
        output,
        'x',
        zipfile.ZIP_DEFLATED,
    ) as z:
        for (name, data) in sorted(payload.items()):
            info = zipfile.ZipInfo(name, (1980, 1, 1, 0, 0, 0))
            info.compress_type = zipfile.ZIP_DEFLATED
            info.create_system = 3
            info.external_attr = 33188 << 16
            z.writestr(info, data)
    return manifest

if __name__=='__main__':
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('output',type=Path);p.add_argument('--version',default=(ROOT/'VERSION').read_text().strip());a=p.parse_args()
    create(a.output,a.version)
