# SPDX-License-Identifier: Apache-2.0
"""Shared distribution contract. No device or publication operations."""
import hashlib
import json
import re
import stat
import zipfile
from pathlib import Path, PurePosixPath

ROOT = Path(__file__).resolve().parent.parent
MAX_BYTES = 64 * 1024 * 1024

def digest(path):
    h = hashlib.sha256()
    with Path(path).open('rb') as f:
        for b in iter(lambda: f.read(1024 * 1024), b''): h.update(b)
    return h.hexdigest()

def version_check(version, tag=None):
    if not re.fullmatch(r'(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)(?:-[a-zA-Z0-9]+(?:[.-][a-zA-Z0-9]+)*)?', version):
        raise ValueError('Invalid distribution version')
    if tag is not None and tag != 'v' + version: raise ValueError('Tag/version mismatch')

def path_check(name):
    p = PurePosixPath(name)
    if not name or p.is_absolute() or '\\' in name or (':' in name or any(ord(c)<32 for c in name)):
        raise ValueError('Unsafe path')
    if any(x in ('', '.', '..') or x.endswith((' ', '.')) for x in name.split('/')):
        raise ValueError('Unsafe path component')
    if any(x.split('.')[0].upper() in {'CON','PRN','AUX','NUL',*[f'COM{i}' for i in range(10)],*[f'LPT{i}' for i in range(10)]} for x in p.parts):
        raise ValueError('Reserved filename')
    return p

def json_bytes(value):
    return (json.dumps(value, ensure_ascii=True, indent=2, sort_keys=True)+'\n').encode()

def make_zip(output, payload, version, provenance):
    version_check(version, provenance.get('tag'))
    retiring = 'DIAGNOSE.TXT' in payload
    manifest = {'schema':2 if retiring else 1, **({'retired_files':['MUSHA.TXT']} if retiring else {}), 'layout_version':1, 'version':version,
                'commit':provenance['commit'], 'tag':provenance.get('tag'),
                'files':{k:{'bytes':len(v),'sha256':hashlib.sha256(v).hexdigest()} for k,v in payload.items()}}
    with zipfile.ZipFile(output, 'x', zipfile.ZIP_DEFLATED) as z:
        for name,data in sorted(dict(payload, **{'update-manifest.json':json_bytes(manifest)}).items()):
            path_check(name)
            i=zipfile.ZipInfo(name,(1980,1,1,0,0,0)); i.compress_type=zipfile.ZIP_DEFLATED; i.external_attr=0o100644 << 16
            z.writestr(i,data)
    return manifest

def read_zip(path):
    with zipfile.ZipFile(path) as z:
        seen=set(); total=0; payload={}
        if len(z.infolist()) > 512: raise ValueError('Too many ZIP entries')
        for i in z.infolist():
            path_check(i.filename)
            key=i.filename.casefold()
            if key in seen or stat.S_IFMT(i.external_attr >> 16) not in (0,stat.S_IFREG):
                raise ValueError('Duplicate, directory or symlink ZIP entry')
            seen.add(key); total+=i.file_size
            if total > MAX_BYTES: raise ValueError('ZIP payload too large')
            payload[i.filename]=z.read(i)
    m=json.loads(payload.pop('update-manifest.json'))
    version_check(m['version'], m.get('tag'))
    if m['schema'] not in (1,2) or m['layout_version']!=1: raise ValueError('Incompatible layout')
    retired = m.get('retired_files', [])
    if (m['schema']==1 and retired) or (m['schema']==2 and retired != ['MUSHA.TXT']):
        raise ValueError('Invalid retired managed files')
    if any(n.casefold() in {p.casefold() for p in payload} for n in retired):
        raise ValueError('Retired file is also payload')
    if retired and 'DIAGNOSE.TXT' not in payload:
        raise ValueError('Replacement marker required')
    if set(payload)!=set(m['files']) or 'EFI/BOOT/BOOTX64.EFI' not in payload or any(n.casefold() in {'musha-install.json','update-result.json'} for n in payload): raise ValueError('Manifest contents mismatch')
    for name,b in payload.items():
        entry=m['files'][name]
        if entry!={'bytes':len(b),'sha256':hashlib.sha256(b).hexdigest()}: raise ValueError('Payload mismatch')
    # A file must not also be a parent directory, including FAT case aliases.
    for name in seen:
        if any('/'.join(name.split('/')[:i]) in seen for i in range(1,len(name.split('/')))):
            raise ValueError('Conflicting paths')
    return m,payload


def usb_readme(version):
    """Owner-approved removable-media help: ASCII, CRLF, bounded lines."""
    version_check(version)
    text = f"""Musha-OS {version} - developer preview
=======================================

This USB drive boots Musha-OS, a small bare-metal runtime for x86-64
UEFI PCs. It runs one external program at Ring 3, or a read-only file
viewer when no program is present.

Quick start
-----------
1. Build the C sample with the SDK (musha-os-v{version}-x64-c-sdk.zip).
2. Copy the resulting ELF to the root of this drive as AUTORUN.ELF.
3. Boot this drive through UEFI (Secure Boot off). The program's
   output appears on screen, followed by its exit status.
4. Without AUTORUN.ELF, the file viewer starts. An external USB
   keyboard is required for the viewer.

Files on this drive
-------------------
EFI/BOOT/BOOTX64.EFI  Musha-OS itself (do not rename)
AUTORUN.ELF           Your program (optional, you add it)
DIAGNOSE.TXT          Marker file used by startup checks (keep it)
LICENSE, NOTICE       Apache-2.0 license and third-party notices
third_party/          Third-party license texts
README.TXT            This file

Documentation, source and updates:
https://github.com/hirofumi-iwasaki/musha-os

The application ABI is experimental and may change between versions.
Musha-OS never writes to this drive.
"""
    data = text.replace('\n', '\r\n').encode('ascii')
    if len(data) >= 4096 or any(len(line) > 72 for line in data.splitlines()):
        raise ValueError('USB README exceeds display limits')
    return data
