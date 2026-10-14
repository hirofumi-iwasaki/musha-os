#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""Inspect a feature-free EFI and its linker map; no device access."""
import argparse, hashlib, json, struct
from pathlib import Path
p=argparse.ArgumentParser(description=__doc__)
p.add_argument('--efi',required=True,type=Path)
p.add_argument('--map',required=True,type=Path)
p.add_argument('--hello',required=True,type=Path)
p.add_argument('--output',required=True,type=Path)
a=p.parse_args();b=a.efi.read_bytes();m=a.map.read_text()
pe=struct.unpack_from('<I',b,60)[0];assert b[pe:pe+4]==b'PE\0\0'
opt=pe+24;n=struct.unpack_from('<H',b,pe+6)[0];hs=struct.unpack_from('<H',b,pe+20)[0]
sections=[]
for at in range(opt+hs,opt+hs+n*40,40):
    name=b[at:at+8].rstrip(b'\0').decode();virtual,rva,raw,offset=struct.unpack_from('<IIII',b,at+8)
    sections.append(dict(name=name,virtual_bytes=virtual,rva=rva,file_bytes=raw,file_offset=offset))
for term in ['musha_probe_start','musha_probe_end','lifecycle','C1_PROBE','user.elf']:
    assert term not in m and term.encode() not in b,term
assert a.hello.read_bytes() not in b, 'Embedded SDK ELF'
# Header comparison constants in machine code and the OS greeting are expected.
assert b'Loading AUTORUN.ELF' in b and b'AUTORUN.ELF exited:' in b
image=struct.unpack_from('<I',b,opt+56)[0]
assert len(b)<1024*1024 and 16*1024*1024<image<=32*1024*1024
assert any(s['virtual_bytes']-s['file_bytes']>=16*1024*1024 for s in sections)
r=dict(efi_sha256=hashlib.sha256(b).hexdigest(),file_bytes=len(b),image_bytes=image,sections=sections,probe_symbols_absent=True,embedded_sdk_elf_absent=True,
       note='Build without features separately; this inspects the linked result, not a signed provenance manifest',
       inputs=dict(efi=str(a.efi.resolve()),link_map=str(a.map.resolve()),hello=str(a.hello.resolve())))
a.output.write_text(json.dumps(r,indent=2)+'\n');print('PASS: external normal EFI layout and probe exclusion')
