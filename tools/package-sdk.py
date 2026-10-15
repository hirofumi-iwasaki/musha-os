#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""Deterministic separate C/Rust development SDK sources; no guest execution."""
import argparse, hashlib, json
from pathlib import Path
import zipfile
import re
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
        if rel.parts[0] not in ('README.md', 'CMakeLists.txt', 'rust-toolchain.toml', 'abi', 'cmake', 'examples', 'linker', 'src', 'sysroot', 'tests', 'tools', 'rust', 'targets'):
            continue
        if '__pycache__' in rel.parts or 'target' in rel.parts:
            continue
        if language == 'c' and (rel.name in ('build-rust.py', 'rust-toolchain.toml') or any((x in ('rust', 'targets', 'hello-rust') for x in rel.parts))):
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
    # Development execution logs stay in the source checkout, not SDK archives.
    for name, data in list(payload.items()):
        if name.endswith('.md'):
            text = data.decode('utf-8')
            text = re.sub(r'\[r2 evidence\]\((?:\.\./docs/|docs/)?stage4-r2-0\.4\.0-progress\.md\)', 'development verification in the source checkout', text)
            text = re.sub(r'\[r2証拠\]\((?:\.\./docs/|docs/)?stage4-r2-0\.4\.0-progress\.md\)', 'source checkoutの開発検証', text)
            payload[name] = text.encode('utf-8')
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
        payload['DEVELOPMENT-STATUS.TXT'] = b'Rust r1/r2 custom-core samples and r2 TIME/WAIT/INPUT are locally verified. This is a development SDK; physical stage4 acceptance and formal release qualification remain incomplete.\n'
    manifest = {
        'version': version,
        'language': language.upper(),
        'executable_sdk': language == 'c',
        'sample_elfs': sample_elfs,
        'abi': 'experimental candidates r1/r2; r1 default',
        'development_guest_build': True,
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
