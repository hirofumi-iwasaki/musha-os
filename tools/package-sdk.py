#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""Deterministic standalone C SDK sources. No Rust guest or host execution."""
import argparse, hashlib, json
from pathlib import Path
import zipfile
ROOT=Path(__file__).resolve().parents[1]

def create(output, version):
    output=Path(output)
    payload={}
    for p in sorted((ROOT/'sdk').rglob('*')):
        if not p.is_file(): continue
        rel=p.relative_to(ROOT/'sdk')
        if rel.parts[0] not in ('README.md','CMakeLists.txt','abi','cmake','examples','linker','src','sysroot','tests','tools'): continue
        if any(x in ('rust','targets','hello-rust','__pycache__') for x in rel.parts): continue
        if p.suffix in ('.rs','.pyc'): continue
        payload[rel.as_posix()]=p.read_bytes()
    payload['tools/check-sdk.py']=(ROOT/'tools/check-sdk.py').read_bytes()
    for name in ('LICENSE','NOTICE'): payload[name]=(ROOT/name).read_bytes()
    # Include the source ledger for inspection, not as a parent-directory dependency.
    for name in ('application-abi.md','sdk.md','sdk-guide.md','external-app.md'):
        payload['docs/'+name]=(ROOT/'docs'/name).read_bytes()
    # The repository README lives one directory below docs; the archive is standalone.
    payload['README.md']=payload['README.md'].replace(b'(../docs/', b'(docs/')
    manifest={'version':version,'language':'C','abi':'experimental candidate r1','public_compatibility_fixed':False,
              'rust_included':False,'files':{n:{'bytes':len(b),'sha256':hashlib.sha256(b).hexdigest()} for n,b in sorted(payload.items())}}
    payload['sdk-manifest.json']=(json.dumps(manifest,indent=2)+'\n').encode()
    with zipfile.ZipFile(output,'x',zipfile.ZIP_DEFLATED) as z:
        for name,data in sorted(payload.items()):
            info=zipfile.ZipInfo(name,(1980,1,1,0,0,0));info.compress_type=zipfile.ZIP_DEFLATED
            info.create_system=3;info.external_attr=0o100644<<16
            z.writestr(info,data)
    return manifest

if __name__=='__main__':
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('output',type=Path);p.add_argument('--version',default=(ROOT/'VERSION').read_text().strip());a=p.parse_args()
    create(a.output,a.version)
