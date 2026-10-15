#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""Independently render the specified Hello console and compare every captured pixel."""
import argparse,ast,hashlib,json,re
from pathlib import Path
p=argparse.ArgumentParser(description=__doc__);p.add_argument('capture',type=Path);p.add_argument('--output',type=Path,required=True);p.add_argument('--ready-footer',action='store_true',help='Include the v0.4.0 startup footer below the unchanged app console');a=p.parse_args();a.output.mkdir(parents=True,exist_ok=False)
root=Path(__file__).resolve().parents[1];old=a.capture.read_bytes();m=re.match(rb'P6\s+(\d+)\s+(\d+)\s+255\s',old);assert m
w,h=map(int,m.groups());s=max(1,min(4,(h+300)//600));pixels=bytearray(bytes([12,20,32])*w*h);font={}
for key,values in re.findall(r"(b'(?:\\.|[^'])+'|\d+) => \[([\d, ]+)\]",(root/'framebuffer/src/lib.rs').read_text()):
 c=ast.literal_eval(key);font[chr(c[0] if isinstance(c,bytes) else c)]=tuple(map(int,values.split(',')))
font[' ']=(0,)*7
for row,text in enumerate(['AUTORUN.ELF  (Ring 3)','Hello Musha-OS!','','AUTORUN.ELF exited: status 0']):
 for col,c in enumerate(text):
  for y,bits in enumerate(font[c]):
   for x in range(5):
    if bits&(1<<(4-x)):
     for dy in range(s):
      for dx in range(s):
       at=(((20+row*10+y)*s+dy)*w+(12+col*6+x)*s+dx)*3;pixels[at:at+3]=bytes([240]*3)
if a.ready_footer:
 for col,c in enumerate('Musha-OS ready.'):
  for y,bits in enumerate(font[c]):
   for x in range(5):
    if bits&(1<<(4-x)):
     for dy in range(s):
      for dx in range(s):
       at=(((h-16*s+y*s+dy))*w+8+col*6*s+x*s+dx)*3;pixels[at:at+3]=bytes([160,240,160])
expected=old[:m.end()]+pixels;(a.output/'hello.ppm').write_bytes(expected)
assert old==expected,'Hello console pixels differ'
(a.output/'result.json').write_text(json.dumps({'passed':True,'resolution':[w,h],'scale':s,'sha256':hashlib.sha256(expected).hexdigest(),'source':str(a.capture.resolve()),'reason':'Unified grid origin (2,2), scale round(height/600), 10*scale line pitch; independently rendered specified text, unchanged font'},indent=2)+'\n')
