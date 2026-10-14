#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""Derive the explicit boot-file-absent status row from verified viewer pixels."""
import ast
import hashlib
import json
from pathlib import Path
import re

ROOT = Path(__file__).resolve().parents[1]
NOTICE = 'AUTORUN.ELF not found in root. Starting file viewer.'


def derive(source, output):
    source, output = Path(source), Path(output)
    output.mkdir(parents=True, exist_ok=False)
    font = {}
    for key, values in re.findall(r"(b'(?:\\.|[^'])+'|\d+) => \[([\d, ]+)\]", (ROOT/'framebuffer/src/lib.rs').read_text()):
        key = ast.literal_eval(key)
        font[chr(key[0] if isinstance(key, bytes) else key)] = tuple(map(int, values.split(',')))
    font[' '] = (0,)*7
    records = []
    for name in ('list.ppm', 'text.ppm'):
        original = (source/name).read_bytes()
        header = re.match(rb'P6\s+(\d+)\s+(\d+)\s+255\s', original)
        if not header:
            raise ValueError('Expected P6 reference')
        width, height = map(int, header.groups())
        pixels = bytearray(original[header.end():])
        assert len(pixels) == width*height*3
        scale = max(1, min(4, (width-32)//480, (height-32)//224))
        if name == 'list.ppm':
            bg, fg = bytes([12,20,32]), bytes([218,228,238])
            for row in range(8*scale):
                for column in range(80*6*scale):
                    x,y = 16+column,16+7*8*scale+row
                    if x>=width or y>=height: continue
                    cell = column//(6*scale)
                    glyph = font.get(NOTICE[cell],font['?']) if cell<len(NOTICE) else font[' ']
                    gx,gy=(column//scale)%6,row//scale
                    on=gx<5 and gy<7 and glyph[gy] & (1<<(4-gx))
                    offset=(y*width+x)*3
                    pixels[offset:offset+3]=fg if on else bg
        result=original[:header.end()]+pixels
        (output/name).write_bytes(result)
        records.append({'file':name,'before':hashlib.sha256(original).hexdigest(),'after':hashlib.sha256(result).hexdigest()})
    (output/'reference-provenance.json').write_text(json.dumps({'source':str(source.resolve()),'reason':'Owner decision 2026-10-14: AUTORUN.ELF replaces the previous automatic boot filename; explicit State::Absent status; only list row 7 changes, page unchanged; expected pixels derived from source font, not guest captures','notice':NOTICE,'records':records},indent=2)+'\n')

if __name__=='__main__':
    import argparse
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('source',type=Path);p.add_argument('output',type=Path);a=p.parse_args();derive(a.source,a.output)
