#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""Update legacy single-marker FAT32 viewer references from specified text rows.

This does not capture guest output. Preserve every pixel outside changed rows;
record source/result hashes. Use once for the 2026-10-14 fixture transition.
"""
import ast
import hashlib
import json
from pathlib import Path
import re
from distribution import usb_readme

ROOT = Path(__file__).resolve().parents[1]

def migrate(source, output):
    source, output = Path(source), Path(output)
    output.mkdir(parents=True, exist_ok=False)
    font = {}
    for key, values in re.findall(r"(b'(?:\\.|[^'])+'|\d+) => \[([\d, ]+)\]", (ROOT/'framebuffer/src/lib.rs').read_text()):
        key=ast.literal_eval(key)
        font[chr(key[0] if isinstance(key,bytes) else key)]=tuple(map(int,values.split(',')))
    font[' ']=(0,)*7
    size=len(usb_readme((ROOT/'VERSION').read_text().strip()))
    records=[]
    for name in ('list.ppm','text.ppm'):
        original=(source/name).read_bytes()
        header=re.match(rb'P6\s+(\d+)\s+(\d+)\s+255\s',original)
        if not header: raise ValueError('Expected P6 image')
        w,h=map(int,header.groups()); pixels=bytearray(original[header.end():])
        scale=max(1,min(4,(w-32)//480,(h-32)//224))
        rows={2:'FAT32  |  2 files  |  maximum 64 MiB per file',5:'DIAGNOSE.TXT'}
        if name=='list.ppm':
            rows.update({9:f"> 001 {'DIAGNOSE.TXT':48} {16:>10} B",10:f"  002 {'README.TXT':48} {size:>10} B"})
        for row,text in rows.items():
            fg=bytes([100,230,220] if name=='list.ppm' and row==9 else [218,228,238])
            for y0 in range(8*scale):
                for x0 in range(80*6*scale):
                    x,y=16+x0,16+row*8*scale+y0
                    if x>=w or y>=h:continue
                    c=x0//(6*scale); gx,gy=x0//scale%6,y0//scale
                    glyph=font.get(text[c],font['?']) if c<len(text) else font[' ']
                    on=gx<5 and gy<7 and glyph[gy]&(1<<(4-gx))
                    pos=(y*w+x)*3;pixels[pos:pos+3]=fg if on else bytes([12,20,32])
        result=original[:header.end()]+pixels;(output/name).write_bytes(result)
        records.append({'file':name,'source_sha256':hashlib.sha256(original).hexdigest(),'sha256':hashlib.sha256(result).hexdigest(),'changed_rows':rows})
    (output/'reference-provenance.json').write_text(json.dumps({'source':str(source.resolve()),'reason':'Owner decision 2026-10-14: rename marker to DIAGNOSE.TXT, add README.TXT. FAT32 count/name/list rows change; marker bytes and text-page data stay unchanged. Independently rendered expected rows, not guest captures.','records':records},indent=2)+'\n')

if __name__=='__main__':
    import argparse
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('source');p.add_argument('output');a=p.parse_args();migrate(a.source,a.output)
