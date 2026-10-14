#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""Trusted-host draft attachment only. Never builds, publishes, overwrites or creates tags."""
import argparse
import hashlib
import json
from pathlib import Path
import re
import subprocess
import tempfile
import zipfile

REPO='hirofumi-iwasaki/musha-os'

def gh(*args):
    return subprocess.check_output(['gh', *args], stderr=subprocess.PIPE)

def sha(p):
    h = hashlib.sha256()
    with p.open('rb') as f:
        for b in iter(lambda : f.read(1024 ** 2), b''):
            h.update(b)
    return h.hexdigest()

def validate(directory, tag, commit, require_complete=True):
    if not re.fullmatch('v[0-9]+\\.[0-9]+\\.[0-9]+(?:-[a-zA-Z0-9]+(?:[.-][a-zA-Z0-9]+)*)?', tag) or not re.fullmatch('[a-f0-9]{40}', commit):
        raise ValueError('Invalid tag/SHA')
    directory = Path(directory)
    m = json.loads((directory / 'build-manifest.json').read_text())
    if m.get('tag') != tag or m.get('version') != tag[1:] or m.get('commit') != commit or m.get('working_tree') or (m.get('status') != 'candidate') or (m.get('features') != []) or m.get('RUSTFLAGS') or m.get('CARGO_ENCODED_RUSTFLAGS'):
        raise ValueError('Candidate provenance mismatch')
    prefix = 'musha-os-' + tag[1:]
    required = {prefix + '-x64.zip', prefix + '-x64-qemu.img.gz', prefix + '-c-sdk.zip', prefix + '-rust-sdk.zip'}
    if m.get('docs_stage2_complete'):
        required.add(prefix + '-docs.zip')
    if m.get('schema') != 2 or set(m['assets']) != required:
        raise ValueError('Unexpected asset set')
    if require_complete and any((not m.get('sdk_status', {}).get(language, {}).get('executable_sdk') or not m['sdk_status'][language].get('sample_elfs') for language in ('c', 'rust'))):
        raise ValueError('SDK implementation/sample gate incomplete; no release draft')
    expected = {}
    for line in (directory / 'SHA256SUMS').read_text().splitlines():
        (h, name) = line.split('  ', 1)
        if name not in required | {'build-manifest.json'} or name in expected or (not re.fullmatch('[a-f0-9]{64}', h)):
            raise ValueError('Invalid checksum entry')
        expected[name] = h
    if set(expected) != required | {'build-manifest.json'}:
        raise ValueError('Incomplete checksums')
    for (name, h) in expected.items():
        p = directory / name
        if p.is_symlink() or not p.is_file() or sha(p) != h:
            raise ValueError('Asset hash mismatch')
        if name in m['assets'] and m['assets'][name] != {'bytes': p.stat().st_size, 'sha256': h}:
            raise ValueError('Manifest asset mismatch')
    if require_complete:
        for language in ('c', 'rust'):
            with zipfile.ZipFile(directory / (prefix + '-' + language + '-sdk.zip')) as archive:
                sdk = json.loads(archive.read('sdk-manifest.json'))
                if not sdk.get('executable_sdk') or sdk.get('sample_elfs') != m['sdk_status'][language]['sample_elfs']:
                    raise ValueError('SDK archive gate differs from build manifest')
                for (name, entry) in sdk['sample_elfs'].items():
                    data = archive.read(name)
                    if entry != {'bytes': len(data), 'sha256': hashlib.sha256(data).hexdigest()}:
                        raise ValueError('SDK sample hash mismatch')
    return (m, sorted(expected) + ['SHA256SUMS'])

def attach(directory, tag, commit, apply=False, api=gh):
    # Assets are data; never execute or extract their contents.
    (m, names) = validate(
        directory,
        tag,
        commit,
    )
    directory = Path(directory)

    def check_tag():
        ref = json.loads(api('api', f'repos/{REPO}/git/ref/tags/{tag}'))['object']
        for _ in range(5):
            if ref['type'] == 'commit':
                break
            if ref['type'] != 'tag':
                raise ValueError('Unsupported tag object')
            ref = json.loads(api('api', f"repos/{REPO}/git/tags/{ref['sha']}"))['object']
        if ref['type'] != 'commit' or ref['sha'] != commit:
            raise ValueError('Remote tag mismatch')
    check_tag()
    try:
        existing = json.loads(api('api', f'repos/{REPO}/releases/tags/{tag}'))
    except subprocess.CalledProcessError as e:
        if b'(HTTP 404)' not in (e.stderr or b''):
            raise
        existing = None
    if existing:
        if not existing['draft']:
            raise ValueError('Published release is immutable')
        if f'Musha source: {commit}' not in existing.get('body', ''):
            raise ValueError('Draft source mismatch')
        assets = existing['assets']
        if {x['name'] for x in assets} != set(names) or len(assets) != len(names):
            raise ValueError('Existing draft asset set differs; manual recovery required')
        for a in assets:
            content = api(
                'api',
                '-H',
                'Accept: application/octet-stream',
                f"repos/{REPO}/releases/assets/{a['id']}",
            )
            if hashlib.sha256(content).hexdigest() != sha(directory / a['name']):
                raise ValueError('Existing draft differs')
        check_tag()
        return 'Identical draft retained'
    if not apply:
        return 'Dry-run: validated; draft would be created'
    check_tag()
    api(
        'release',
        'create',
        tag,
        '--repo',
        REPO,
        '--draft',
        '--verify-tag',
        '--title',
        tag,
        '--notes',
        f'Musha source: {commit}\nHardware acceptance and owner publication approval pending.',
        *[str(directory / n) for n in names],
    )
    return 'Draft created; no publication performed'

if __name__=='__main__':
    p=argparse.ArgumentParser(description=__doc__); p.add_argument('directory',type=Path); p.add_argument('--tag',required=True); p.add_argument('--commit',required=True); p.add_argument('--apply',action='store_true')
    a=p.parse_args(); print(attach(a.directory,a.tag,a.commit,a.apply))
