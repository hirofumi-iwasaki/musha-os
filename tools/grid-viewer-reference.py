#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""Migrate verified pre-grid viewer glyph pixels; never accept guest captures."""
import argparse, hashlib, json, re
from pathlib import Path
p=argparse.ArgumentParser(description=__doc__)
p.add_argument('--source',required=True,type=Path);p.add_argument('--output',required=True,type=Path)
p.add_argument('--width',required=True,type=int);p.add_argument('--height',required=True,type=int)
a=p.parse_args();a.output.mkdir(parents=True,exist_ok=False);records=[]
for name in ['list.ppm','text.ppm']:
 old=(a.source/name).read_bytes();m=re.match(rb'P6\s+(\d+)\s+(\d+)\s+255\s',old);assert m
 w,h=map(int,m.groups());src=old[m.end():];s=max(1,min(4,(w-32)//480,(h-32)//224))
 t=max(1,min(4,(a.height+300)//600,a.width//504,a.height//320))
 bg=bytes([12,20,32]);dest=bytearray(bg*a.width*a.height)
 for row in range(28):
  for col in range(80):
   for y in range(8):
    for x in range(6):
     off=((16+(row*8+y)*s)*w+16+(col*6+x)*s)*3;c=src[off:off+3]
     for dy in range(t):
      for dx in range(t):
       px=(12+col*6+x)*t+dx;py=(20+row*10+y)*t+dy
       if px<a.width and py<a.height:at=(py*a.width+px)*3;dest[at:at+3]=c
 new=f'P6\n{a.width} {a.height}\n255\n'.encode()+dest;(a.output/name).write_bytes(new)
 records.append({'file':name,'source_sha256':hashlib.sha256(old).hexdigest(),'sha256':hashlib.sha256(new).hexdigest(),'scale':t})
(a.output/'reference-provenance.json').write_text(json.dumps({'reason':'Owner 2026-10-14: unified text grid; 2-cell origin and 10*scale line pitch; glyph pixels sampled from verified pre-grid reference','source':str(a.source.resolve()),'dimensions':[a.width,a.height],'files':records},indent=2)+'\n')
