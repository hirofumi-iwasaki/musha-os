#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""Derive a new-resolution viewer reference from old verified pixels, never a guest capture.

Only integer enlargement of the unchanged 80x28, 6x8-cell viewer is supported.
The surrounding source pixels must all equal the background. Source is preserved.
"""
import argparse, hashlib, json, re
from pathlib import Path
p=argparse.ArgumentParser(description=__doc__)
p.add_argument('--source',type=Path,required=True)
p.add_argument('--output',type=Path,required=True)
p.add_argument('--width',type=int,required=True)
p.add_argument('--height',type=int,required=True)
a=p.parse_args(); a.output.mkdir(parents=True,exist_ok=False)
def scale(w,h):return max(1,min(4,(w-32)//480,(h-32)//224))
records=[]
for name in ['list.ppm','text.ppm']:
    old=(a.source/name).read_bytes(); m=re.match(rb'P6\s+(\d+)\s+(\d+)\s+255\s',old);assert m
    w,h=map(int,m.groups());pixels=old[m.end():];assert len(pixels)==w*h*3
    s=scale(w,h);t=scale(a.width,a.height);assert t>=s and t%s==0
    factor=t//s;rw,rh=480*s,224*s
    assert 16+rw<=w and 16+rh<=h and 16+480*t<=a.width and 16+224*t<=a.height
    bg=pixels[:3]
    for y in range(h):
        row=pixels[y*w*3:(y+1)*w*3]
        assert (row[:48]==bg*16 and row[(16+rw)*3:]==bg*(w-16-rw)) if 16<=y<16+rh else row==bg*w
    dest=bytearray(bg*(a.width*a.height))
    for y in range(rh):
        row=pixels[((16+y)*w+16)*3:((16+y)*w+16+rw)*3]
        enlarged=b''.join(row[x:x+3]*factor for x in range(0,len(row),3))
        for dy in range(factor):
            start=((16+y*factor+dy)*a.width+16)*3;dest[start:start+len(enlarged)]=enlarged
    new=f'P6\n{a.width} {a.height}\n255\n'.encode()+dest;(a.output/name).write_bytes(new)
    records.append(dict(file=name,source_sha256=hashlib.sha256(old).hexdigest(),sha256=hashlib.sha256(new).hexdigest(),source_dimensions=[w,h],dimensions=[a.width,a.height],source_scale=s,scale=t))
(a.output/'reference-provenance.json').write_text(json.dumps({'reason':'GOP maximum RGB/BGR mode selection changes resolution; unchanged viewer renderer integer scale; expected pixels derived from previous reference','source':str(a.source.resolve()),'files':records},indent=2)+'\n')
