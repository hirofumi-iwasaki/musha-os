#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""Adjust the 64MiB distribution image into a NEW sparse regular file. No devices."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import stat
import struct
import zlib

S=512
MIN=64*1024**2
MAX=128*1024**3

def read_at(f, offset, size):
    f.seek(offset); b=f.read(size)
    if len(b)!=size: raise ValueError('Truncated image')
    return b

def inspect(f, size):
    if size < MIN or size > MAX or size % S: raise ValueError('Unsupported capacity')
    n=size//S
    m=read_at(f,0,S)
    if m[510:]!=b'\x55\xaa' or m[450]!=0xee or any(m[462:510]) or struct.unpack_from('<II',m,454)!=(1,min(n-1,0xffffffff)):
        raise ValueError('Invalid protective MBR')
    arrays=[]; headers=[]
    for lba,other,alba in [(1,n-1,2),(n-1,1,n-33)]:
        h=bytearray(read_at(f,lba*S,S)); crc=struct.unpack_from('<I',h,16)[0]; h[16:20]=bytes(4)
        if h[:8]!=b'EFI PART' or struct.unpack_from('<II',h,8)!=(0x10000,92) or zlib.crc32(h[:92])!=crc:
            raise ValueError('Bad GPT header CRC/type')
        if struct.unpack_from('<I',h,20)[0] or struct.unpack_from('<QQQQ',h,24)!=(lba,other,34,n-34):
            raise ValueError('GPT bounds mismatch')
        if struct.unpack_from('<QII',h,72)!=(alba,128,128): raise ValueError('GPT array geometry')
        a=read_at(f,alba*S,16384)
        if zlib.crc32(a)!=struct.unpack_from('<I',h,88)[0]: raise ValueError('Array CRC')
        arrays.append(a); headers.append(h)
    if arrays[0]!=arrays[1] or headers[0][56:72]!=headers[1][56:72]: raise ValueError('GPT copies disagree')
    a=arrays[0]; first,last=struct.unpack_from('<QQ',a,32)
    import uuid
    if a[:16] not in [uuid.UUID(g).bytes_le for g in ('c12a7328-f81f-11d2-ba4b-00a0c93ec93b', 'ebd0a0a2-b9e5-4433-87c0-68b6b72699c7')] or not any(a[16:32]) or any(a[128:]):
        raise ValueError('Expected exactly one FAT32 boot partition (ESP or Basic Data)')
    if first!=2048 or last!=MIN//S-34 or last>n-34: raise ValueError('Unsupported fixed FAT32 boot partition layout')
    boot=read_at(f,first*S,S)
    if boot[510:]!=b'\x55\xaa' or struct.unpack_from('<H',boot,11)[0]!=S or boot[13]!=1 or boot[16]!=2 or struct.unpack_from('<H',boot,14)[0]!=32:
        raise ValueError('Unsupported FAT32 geometry')
    if struct.unpack_from('<I',boot,32)[0]!=last-first+1 or read_at(f,(first+6)*S,S)!=boot:
        raise ValueError('FAT boot mismatch')
    fatsecs=struct.unpack_from('<I',boot,36)[0]
    if not 1<=fatsecs<=2048: raise ValueError('FAT bounds')
    fat=read_at(f,(first+32)*S,fatsecs*S)
    if fat!=read_at(f,(first+32+fatsecs)*S,fatsecs*S): raise ValueError('FAT copies mismatch')
    data=first+32+2*fatsecs
    def cluster(c):
        if not 2<=c<(last-data+1)+2: raise ValueError('Cluster bounds')
        return read_at(f,(data+c-2)*S,S)
    def find(c,name):
        b=cluster(c)
        for i in range(0,S,32):
            e=b[i:i+32]
            if e[:11]==name:
                return ((struct.unpack_from('<H',e,20)[0]<<16)|struct.unpack_from('<H',e,26)[0],struct.unpack_from('<I',e,28)[0])
        raise ValueError('Missing FAT payload')
    def file(c,length):
        if length>16*1024**2: raise ValueError('Payload too large')
        b=bytearray(); seen=set()
        while c<0x0ffffff8:
            if c in seen or len(b)>length+S: raise ValueError('Invalid FAT chain')
            seen.add(c); b+=cluster(c)
            if c*4+4>len(fat): raise ValueError('FAT index')
            c=struct.unpack_from('<I',fat,c*4)[0]&0x0fffffff
        if len(b)<length: raise ValueError('Short FAT chain')
        return bytes(b[:length])
    root=struct.unpack_from('<I',boot,44)[0]
    ec,_=find(root,b'EFI        '); bc,_=find(ec,b'BOOT       ')
    efi=file(*find(bc,b'BOOTX64 EFI')); text=file(*find(root,b'MUSHA   TXT'))
    if text!=b'Hello Musha-OS!\n': raise ValueError('MUSHA.TXT mismatch')
    return {'first':first,'last':last,'efi_sha256':hashlib.sha256(efi).hexdigest(),'efi':efi,'headers':headers,'array':a,'mbr':m}

def adjust(source, output, capacity, sector_size=512, apply=False):
    source=Path(source); output=Path(output)
    if sector_size!=512 or not MIN<=capacity<=MAX or capacity%512: raise ValueError('Capacity/sector mismatch (512 only)')
    if source.is_symlink() or not stat.S_ISREG(source.stat().st_mode): raise ValueError('Source must be a regular image')
    if source.stat().st_size!=MIN: raise ValueError('Expected 64MiB uncompressed source')
    if output.exists() or output.is_symlink() or str(output.absolute()).startswith('/dev/'):
        raise ValueError('Output must be a NEW regular file, never a device')
    with source.open('rb') as src:
        layout=inspect(src,MIN)
        plan={'capacity':capacity,'sector_bytes':512,'layout_version':1,'write_ranges':[[0,MIN-33*S],[capacity-33*S,33*S]],'efi_sha256':layout['efi_sha256'],'applied':apply}
        if not apply: return plan
        n=capacity//S
        m=bytearray(layout['mbr']); struct.pack_into('<I',m,458,min(n-1,0xffffffff))
        def header(index,current,other,array_lba):
            h=layout['headers'][index][:]
            struct.pack_into('<QQ',h,24,current,other); struct.pack_into('<Q',h,48,n-34); struct.pack_into('<Q',h,72,array_lba)
            h[16:20]=bytes(4); struct.pack_into('<I',h,16,zlib.crc32(h[:92])); return h
        with output.open('xb+') as dst:
            dst.truncate(capacity); src.seek(0); remaining=MIN-33*S
            while remaining:
                b=src.read(min(1024**2,remaining))
                if not b: raise ValueError('Source changed during copy')
                dst.write(b); remaining-=len(b)
            dst.seek(0); dst.write(m); dst.write(header(0,1,n-1,2))
            dst.seek((n-33)*S); dst.write(layout['array']); dst.write(header(1,n-1,1,n-33)); dst.flush(); os.fsync(dst.fileno())
            result=inspect(dst,capacity)
            if result['efi_sha256']!=layout['efi_sha256']: raise ValueError('Readback EFI mismatch')
            # Compare only the fixed partition, never the unused capacity.
            src.seek(2048*S); dst.seek(2048*S); remaining=(layout['last']-2048+1)*S
            while remaining:
                count=min(1024**2,remaining)
                if src.read(count)!=dst.read(count): raise ValueError('Partition readback mismatch')
                remaining-=count
        return plan

if __name__=='__main__':
    p=argparse.ArgumentParser(description=__doc__); p.add_argument('image',type=Path); p.add_argument('output',type=Path)
    p.add_argument('--capacity-bytes',type=int,required=True); p.add_argument('--sector-size',type=int,default=512)
    p.add_argument('--apply',action='store_true',help='Create regular file; default is dry-run')
    a=p.parse_args(); print(json.dumps(adjust(a.image,a.output,a.capacity_bytes,a.sector_size,a.apply),indent=2))
