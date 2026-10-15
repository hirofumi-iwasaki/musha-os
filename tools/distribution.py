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
        for b in iter(lambda : f.read(1024 * 1024), b''):
            h.update(b)
    return h.hexdigest()

def version_check(version, tag=None):
    if not re.fullmatch('(0|[1-9][0-9]*)\\.(0|[1-9][0-9]*)\\.(0|[1-9][0-9]*)(?:-[a-zA-Z0-9]+(?:[.-][a-zA-Z0-9]+)*)?', version):
        raise ValueError('Invalid distribution version')
    if tag is not None and tag != 'v' + version:
        raise ValueError('Tag/version mismatch')

def path_check(name):
    p = PurePosixPath(name)
    if not name or p.is_absolute() or '\\' in name or (':' in name or any((ord(c) < 32 for c in name))):
        raise ValueError('Unsafe path')
    if any((x in ('', '.', '..') or x.endswith((' ', '.')) for x in name.split('/'))):
        raise ValueError('Unsafe path component')
    if any((x.split('.')[0].upper() in {'CON', 'PRN', 'AUX', 'NUL', *[f'COM{i}' for i in range(10)], *[f'LPT{i}' for i in range(10)]} for x in p.parts)):
        raise ValueError('Reserved filename')
    return p

def json_bytes(value):
    return (json.dumps(
        value,
        ensure_ascii=True,
        indent=2,
        sort_keys=True,
    ) + '\n').encode()

USER_DOCS = ('sdk-guide.md','external-app.md','file-viewer.md','distribution-usage.md')
LICENSE_FILES = tuple('third_party/'+d+'/'+n for d,names in (
    ('lwip',('COPYING','COPYING-DHCP','UPSTREAM')),('freebsd-e1000',('COPYING','UPSTREAM')),
    ('freebsd-apple-bce',('COPYING','UPSTREAM')),('r-efi',('AUTHORS','UPSTREAM')),
    ('bluetooth-crypto-licenses',('COPYING','UPSTREAM.json','SHA256SUMS'))) for n in names)
OBSOLETE_FILES = ('DIAGNOSE.TXT','MUSHA.TXT','update-manifest.json','musha-install.json',
                  'musha-installation.json','LICENSE','NOTICE') + LICENSE_FILES
COMMENT_PREFIX = b'MUSHA-UPDATE-3\n'

def fnv(data):
    value = 14695981039346656037
    for byte in data:
        value = (value ^ byte) * 1099511628211 & 18446744073709551615
    return value

def drive_payload(efi, version):
    # Only these four pages are offline. Other source-doc links stay usable
    # through the immutable release tag rather than dangling on the drive.
    payload = {'EFI/BOOT/BOOTX64.EFI': Path(efi).read_bytes(), 'README.TXT': usb_readme(version)}
    for name in ('LICENSE', 'NOTICE', *LICENSE_FILES):
        payload['DOCS/' + name] = (ROOT / name).read_bytes()
    for name in USER_DOCS:
        import posixpath

        def link(match):
            target = match.group(2)
            (path, sep, fragment) = target.partition('#')
            if not path or ':' in path or path.startswith('/'):
                return match.group(0)
            normalized = posixpath.normpath('docs/' + path)
            if normalized in {'docs/' + n for n in USER_DOCS}:
                return match.group(0)
            return '[' + match.group(1) + '](https://github.com/hirofumi-iwasaki/musha-os/blob/v' + version + '/' + normalized + (sep + fragment if sep else '') + ')'
        text = (ROOT / 'docs' / name).read_text()
        payload['DOCS/' + name] = re.sub(
            '\\[([^]\\n]+)\\]\\(([^)\\s]+)\\)',
            link,
            text,
        ).encode('utf-8')
    return payload

def managed_path(name):
    path_check(name)
    if name not in ('EFI/BOOT/BOOTX64.EFI', 'README.TXT') and (not name.startswith('DOCS/')):
        raise ValueError('Not a managed drive path: ' + name)

def zip_entry(name, directory=False):
    info = zipfile.ZipInfo(name, (1980, 1, 1, 0, 0, 0))
    info.create_system = 3
    info.compress_type = zipfile.ZIP_DEFLATED
    info.external_attr = (16877 if directory else 33188) << 16 | (16 if directory else 0)
    return info

def no_duplicates(pairs):
    result = {}
    for (key, value) in pairs:
        if key in result:
            raise ValueError('Duplicate metadata key')
        result[key] = value
    return result

def validate_manifest(m, payload):
    if set(m) != {'schema', 'layout_version', 'version', 'commit', 'tag', 'empty_directories', 'previous_hashes', 'retired_files', 'files'}:
        raise ValueError('Unexpected metadata fields')
    if not isinstance(m['version'], str) or (m['tag'] is not None and (not isinstance(m['tag'], str))):
        raise ValueError('Invalid version metadata')
    version_check(m['version'], m['tag'])
    if m['schema'] != 3 or m['layout_version'] != 2 or m['empty_directories'] != ['SETTINGS']:
        raise ValueError('Incompatible drive contract')
    if not isinstance(m['commit'], str) or not m['commit']:
        raise ValueError('Missing source commit')
    if m['retired_files'] != list(OBSOLETE_FILES):
        raise ValueError('Invalid obsolete managed paths')
    if set(payload) != set(m['files']) or not {'EFI/BOOT/BOOTX64.EFI', 'README.TXT'} <= payload.keys():
        raise ValueError('Manifest contents mismatch')
    if not isinstance(m['files'], dict):
        raise ValueError('Invalid file metadata')
    seen = set()
    for (name, data) in payload.items():
        managed_path(name)
        if name.casefold() in seen:
            raise ValueError('Case alias in payload')
        seen.add(name.casefold())
        if not isinstance(m['files'][name], dict) or type(m['files'][name].get('bytes')) is not int:
            raise ValueError('Invalid file metadata')
        if m['files'][name] != {'bytes': len(data), 'sha256': hashlib.sha256(data).hexdigest()}:
            raise ValueError('Payload mismatch')
    components = {}
    for name in payload:
        parts = name.split('/')
        for i in range(1, len(parts) + 1):
            component = '/'.join(parts[:i])
            key = component.casefold()
            if key in components and components[key] != component:
                raise ValueError('Case alias in path component')
            components[key] = component
        if any(('/'.join(parts[:i]) in payload for i in range(1, len(parts)))):
            raise ValueError('Conflicting payload paths')
    if not isinstance(m['previous_hashes'], dict):
        raise ValueError('Invalid previous hashes')
    for (name, values) in m['previous_hashes'].items():
        if name not in OBSOLETE_FILES:
            managed_path(name)
        if not isinstance(values, list) or not values or len(values) != len(set(values)) or any((not isinstance(h, str) or not re.fullmatch('[a-f0-9]{64}', h) for h in values)):
            raise ValueError('Invalid previous hashes')

def make_zip(output, payload, version, provenance, previous_hashes=None):
    m = {
        'schema': 3,
        'layout_version': 2,
        'version': version,
        'commit': provenance['commit'],
        'tag': provenance.get('tag'),
        'empty_directories': ['SETTINGS'],
        'previous_hashes': previous_hashes or {},
        'retired_files': list(OBSOLETE_FILES),
        'files': {n: {'bytes': len(b), 'sha256': hashlib.sha256(b).hexdigest()} for (n, b) in sorted(payload.items())},
    }
    validate_manifest(m, payload)
    comment = COMMENT_PREFIX + json_bytes(m)
    if len(comment) > 65535:
        raise ValueError('Update metadata exceeds ZIP comment limit')
    with zipfile.ZipFile(
        output,
        'x',
        zipfile.ZIP_DEFLATED,
    ) as archive:
        archive.comment = comment
        archive.writestr(zip_entry('SETTINGS/', True), b'')
        for (name, data) in sorted(payload.items()):
            archive.writestr(zip_entry(name), data)
    return m

def read_zip(path, allow_legacy=False):
    with zipfile.ZipFile(path) as archive:
        seen = set()
        payload = {}
        directories = []
        total = 0
        if len(archive.infolist()) > 512:
            raise ValueError('Too many ZIP entries')
        for info in archive.infolist():
            name = info.filename
            if info.is_dir():
                if name != 'SETTINGS/' or info.file_size or stat.S_IFMT(info.external_attr >> 16) not in (0, stat.S_IFDIR):
                    raise ValueError('Unexpected ZIP directory')
                directories.append('SETTINGS')
                key = 'settings'
            else:
                path_check(name)
                key = name.casefold()
                if stat.S_IFMT(info.external_attr >> 16) not in (0, stat.S_IFREG):
                    raise ValueError('ZIP symlink or special entry')
                total += info.file_size
                if total > MAX_BYTES:
                    raise ValueError('ZIP payload too large')
                payload[name] = archive.read(info)
            if key in seen:
                raise ValueError('Duplicate ZIP entry')
            seen.add(key)
        for name in seen:
            if any(('/'.join(name.split('/')[:i]) in seen for i in range(1, len(name.split('/'))))):
                raise ValueError('Conflicting ZIP paths')
        if archive.comment.startswith(COMMENT_PREFIX):
            m = json.loads(archive.comment[len(COMMENT_PREFIX):], object_pairs_hook=no_duplicates)
            if directories != ['SETTINGS']:
                raise ValueError('Missing empty SETTINGS entry')
            validate_manifest(m, payload)
        elif allow_legacy:
            m = json.loads(payload.pop('update-manifest.json'), object_pairs_hook=no_duplicates)
            version_check(m['version'], m.get('tag'))
            if directories or m.get('schema') not in (1, 2) or m.get('layout_version') != 1:
                raise ValueError('Invalid legacy package')
            if set(payload) != set(m['files']) or 'EFI/BOOT/BOOTX64.EFI' not in payload:
                raise ValueError('Legacy manifest mismatch')
            for (name, data) in payload.items():
                if m['files'][name] != {'bytes': len(data), 'sha256': hashlib.sha256(data).hexdigest()}:
                    raise ValueError('Legacy hash mismatch')
        else:
            raise ValueError('Missing package-only update metadata')
    return (m, payload)

def previous_catalog(packages):
    hashes = {}
    for package in packages:
        (m, payload) = read_zip(package, allow_legacy=True)
        with zipfile.ZipFile(package) as archive:
            if 'update-manifest.json' in archive.namelist():
                payload['update-manifest.json'] = archive.read('update-manifest.json')
        if m['schema'] in (1, 2):
            payload['musha-install.json'] = json_bytes({
                'layout_version': 1,
                'version': m['version'],
                'commit': m['commit'],
            })
        for (name, data) in payload.items():
            if name in OBSOLETE_FILES or name in ('EFI/BOOT/BOOTX64.EFI', 'README.TXT') or name.startswith('DOCS/'):
                hashes.setdefault(name, set()).add(hashlib.sha256(data).hexdigest())
        for (name, values) in m.get('previous_hashes', {}).items():
            hashes.setdefault(name, set()).update(values)
    return {n: sorted(values) for (n, values) in sorted(hashes.items())}

def usb_readme(version):
    """Owner-approved removable-media help: ASCII, CRLF, bounded lines."""
    version_check(version)
    text = f"Musha-OS {version} - developer preview\n=======================================\n\nThis USB drive boots Musha-OS, a small bare-metal runtime for x86-64\nUEFI PCs. It runs one external program at Ring 3, or a read-only file\nviewer when no program is present.\n\nQuick start\n-----------\n1. Build the C sample with the SDK (musha-os-{version}-c-sdk.zip).\n2. Copy the resulting ELF to the root of this drive as AUTORUN.ELF.\n3. Boot this drive through UEFI (Secure Boot off). The program's\n   output appears on screen, followed by its exit status.\n4. Without AUTORUN.ELF, the file viewer starts. An external USB\n   keyboard is required for the viewer.\n\nFiles on this drive\n-------------------\nEFI/BOOT/BOOTX64.EFI  Musha-OS itself (do not rename)\nAUTORUN.ELF           Your program (optional, you add it)\nDOCS/                 User guides, licenses and third-party notices\nSETTINGS/             Configuration and pairing data (initially empty)\nREADME.TXT            This file and startup read marker (keep it)\n\nDocumentation:\nhttps://github.com/hirofumi-iwasaki/musha-os/blob/main/docs/sdk-guide.md\nSource and updates:\nhttps://github.com/hirofumi-iwasaki/musha-os\n\nThe application ABI is experimental and may change between versions.\nMusha-OS writes only inside SETTINGS/.\nSee DOCS/distribution-usage.md for this development stage and limits.\n"
    data = text.replace('\n', '\r\n').encode('ascii')
    if len(data) >= 4096 or any((len(line) > 72 for line in data.splitlines())):
        raise ValueError('USB README exceeds display limits')
    return data
