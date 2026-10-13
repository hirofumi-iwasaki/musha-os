#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""Manifest-driven file updates. Dry-run default; no image generation or raw I/O."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import plistlib
import shutil
import subprocess
import sys
import tempfile
from distribution import read_zip, path_check, digest, json_bytes

def safe_path(root,name):
    path_check(name)
    dest=root/name
    for p in [dest,*dest.parents]:
        if p.is_symlink(): raise ValueError('Symlink in destination path')
    if dest.exists() and not dest.is_file(): raise ValueError('Destination is not a regular file')
    # FAT is case insensitive; reject ambiguous aliases even on test hosts.
    parent=root
    for part in Path(name).parts:
        if parent.is_dir():
            for child in parent.iterdir():
                if child.name.casefold()==part.casefold() and child.name!=part:
                    raise ValueError('Case alias in destination')
        parent=parent/part
    return dest

def mac_identity(root):
    if sys.platform!='darwin': raise ValueError('Physical update adapter supports macOS only')
    def info(name): return plistlib.loads(subprocess.check_output(['diskutil','info','-plist',str(name)]))
    p=info(root); d=info(p['ParentWholeDisk'])
    if p.get('MountPoint')!=str(root) or p.get('FilesystemName')!='MS-DOS FAT32' or not p.get('WritableVolume'):
        raise ValueError('Select writable FAT32 volume root')
    if p.get('Content') not in ('EFI', 'Microsoft Basic Data'):
        raise ValueError('Expected FAT32 boot partition (ESP or Basic Data)')
    if d.get('Internal') is not False or d.get('BusProtocol')!='USB' or d.get('VirtualOrPhysical')!='Physical' or not d.get('WholeDisk'):
        raise ValueError('Refusing internal, virtual or non-USB disk')
    if d.get('DeviceBlockSize')!=512 or not p.get('VolumeUUID'): raise ValueError('Unknown device identity')
    return {k:v for k,v in {'disk':d['DeviceIdentifier'],'model':d['MediaName'],'capacity':d['TotalSize'],'volume_uuid':p['VolumeUUID'],'partition_uuid':p.get('DiskUUID'),'partition_type':p['Content'],'mount':str(root)}.items()}

def update(package,root,backup,apply=False,identity=None):
    root=Path(root).absolute(); backup=Path(backup).absolute()
    if not root.is_dir() or any(p.is_symlink() for p in [root,*root.parents]): raise ValueError('Unsafe root')
    if root==backup or root in backup.parents or backup in root.parents: raise ValueError('Backup must be outside volume')
    if any(p.is_symlink() for p in [backup,*backup.parents]): raise ValueError('Unsafe backup')
    manifest,payload=read_zip(package)
    if not safe_path(root,'EFI/BOOT/BOOTX64.EFI').is_file(): raise ValueError('Existing Musha FAT32 installation required')
    state=safe_path(root,'musha-install.json')
    if state.exists():
        old=json.loads(state.read_text())
        if old.get('layout_version')!=manifest['layout_version']: raise ValueError('Installed layout incompatible')
    targets={name:safe_path(root,name) for name in payload}
    changed=[n for n,d in targets.items() if not d.exists() or digest(d)!=manifest['files'][n]['sha256']]
    plan={'version':manifest['version'],'layout_version':1,'changed':changed,'skipped':[n for n in payload if n not in changed], 'completed':[], 'applied':False}
    before=identity(root) if identity else None
    plan['target']=before
    if identity:
        ancestor=backup.parent
        while not ancestor.exists(): ancestor=ancestor.parent
        if ancestor.stat().st_dev==root.stat().st_dev: raise ValueError('Backup must be on the host, not the USB volume')
    if not apply: return plan
    if not changed: return plan
    if backup.exists(): raise FileExistsError(backup)
    if shutil.disk_usage(root).free < sum(len(payload[n]) for n in changed)+1024**2: raise ValueError('Insufficient free space')
    backup.mkdir(parents=True)
    # Save every old file before making the first destination change.
    for name in changed+(['musha-install.json'] if state.exists() else []):
        d=safe_path(root,name)
        if d.exists():
            b=backup/name; b.parent.mkdir(parents=True,exist_ok=True); shutil.copyfile(d,b)
            if digest(b)!=digest(d): raise ValueError('Backup mismatch')
    report=backup/'update-result.json'
    def save(): report.write_bytes(json_bytes(plan))
    save()
    try:
        if identity and identity(root)!=before: raise ValueError('Device identity changed')
        for name in changed:
            d=safe_path(root,name); d.parent.mkdir(parents=True,exist_ok=True)
            fd,tmp=tempfile.mkstemp(prefix='.musha-update-',dir=d.parent)
            try:
                with os.fdopen(fd,'wb') as f: f.write(payload[name]); f.flush(); os.fsync(f.fileno())
                if digest(Path(tmp))!=manifest['files'][name]['sha256']: raise ValueError('Staging mismatch')
                safe_path(root,name)
                os.replace(tmp,d)
                if digest(d)!=manifest['files'][name]['sha256']: raise ValueError('Readback mismatch')
            finally:
                if os.path.exists(tmp): os.unlink(tmp)
            plan['completed'].append(name); save()
        state.write_bytes(json_bytes({'layout_version':1,'version':manifest['version'],'commit':manifest['commit']}))
        with state.open('rb') as f: os.fsync(f.fileno())
        if hasattr(os,'sync'): os.sync()
        plan['applied']=True; save()
    except Exception as e:
        plan['error']=str(e); save(); raise
    return plan

if __name__=='__main__':
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('package',type=Path); p.add_argument('--volume',type=Path,required=True)
    p.add_argument('--backup',type=Path,required=True); p.add_argument('--apply',action='store_true')
    p.add_argument('--confirm-volume-uuid'); p.add_argument('--test-directory',action='store_true',help='Regular-directory simulation; refuses mounted volumes')
    p.add_argument('--eject',action='store_true')
    a=p.parse_args(); root=a.volume.absolute()
    if a.test_directory:
        if any(os.path.ismount(x) for x in [root,*list(root.parents)[:-1]]) or str(root).startswith(('/Volumes/','/dev/')):
            p.error('Test mode refuses mount paths')
        identity=None
        if a.eject: p.error('Test mode cannot eject')
    else:
        identity=mac_identity; target=identity(root)
        print(json.dumps(target,indent=2))
        if a.apply and a.confirm_volume_uuid!=target['volume_uuid']: p.error('Apply requires matching --confirm-volume-uuid')
    result=update(a.package,root,a.backup,a.apply,identity)
    print(json.dumps(result,indent=2))
    if a.apply and a.eject:
        if identity(root)!=target: raise ValueError('Device changed before eject')
        subprocess.run(['diskutil','eject',target['disk']],check=True)
