#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""Independently render a single-console transcript and compare every pixel."""
import argparse,ast,hashlib,json,re
from pathlib import Path
p=argparse.ArgumentParser(description=__doc__)
p.add_argument('capture',type=Path);p.add_argument('--output',type=Path,required=True)
p.add_argument('--text-file',type=Path,help='Exact raw console bytes in display order; excludes debug/ring prefixes')
p.add_argument('--baseline',type=Path,help='Framebuffer before startup; untouched pixels must stay identical')
p.add_argument('--active-log',action='store_true',help='Explicit black-cleared application fixture containing 4096 Z bytes')
a=p.parse_args()
if bool(a.text_file)==bool(a.active_log):p.error('select exactly one of --text-file or --active-log')
if a.text_file and not a.baseline:p.error('--text-file requires the actual pre-console --baseline')
a.output.mkdir(parents=True,exist_ok=False)
root=Path(__file__).resolve().parents[1]
def ppm(path):
 raw=path.read_bytes();m=re.match(rb'P6\s+(\d+)\s+(\d+)\s+255\s',raw);assert m,'P6 required'
 w,h=map(int,m.groups());assert len(raw[m.end():])==w*h*3
 return raw,m,w,h
old,m,w,h=ppm(a.capture);s=max(1,min(4,(h+300)//600));cols=w//(6*s);rows=h//(10*s)
assert 3<=cols<=384 and 1<=rows<=128
if a.baseline:
 base,bm,bw,bh=ppm(a.baseline);assert (bw,bh)==(w,h);pixels=bytearray(base[bm.end():])
else:pixels=bytearray(w*h*3)
font={}
for key,values in re.findall(r"(b'(?:\\.|[^'])+'|\d+) => \[([\d, ]+)\]",(root/'framebuffer/src/lib.rs').read_text()):
 c=ast.literal_eval(key);font[c[0] if isinstance(c,bytes) else c]=tuple(map(int,values.split(',')))
font[32]=(0,)*7
cells=[[32]*cols for _ in range(rows)];painted=[0]*rows;row=col=0
text=b'Z'*4096 if a.active_log else a.text_file.read_bytes()
def paint(r,c,ch):
 glyph=font.get(ch,font[63])
 for y in range(8*s):
  for x in range(6*s):
   on=y//s<7 and x//s<5 and glyph[y//s]&(1<<(4-x//s))
   at=((r*10*s+y)*w+c*6*s+x)*3;pixels[at:at+3]=b'\xff'*3 if on else b'\0'*3
def newline():
 global row,col
 col=0
 if row+1<rows:row+=1;return
 for rr in range(rows-1):
  cells[rr]=cells[rr+1][:];painted[rr]=max(painted[rr],painted[rr+1])
 cells[-1]=[32]*cols
 for rr in range(rows):
  for cc in range(painted[rr]):paint(rr,cc,cells[rr][cc])
def put(ch):
 global col
 cells[row][col]=ch;painted[row]=max(painted[row],col+1);paint(row,col,ch);col+=1
for ch in text:
 if ch==10:newline();continue
 if col==cols:newline();put(32);put(32)
 put(ch if 32<=ch<=126 else 46)
expected=old[:m.end()]+pixels;(a.output/'expected.ppm').write_bytes(expected)
result={'passed':old==expected,'resolution':[w,h],'scale':s,'columns':cols,'rows':rows,'capture_sha256':hashlib.sha256(old).hexdigest(),'expected_sha256':hashlib.sha256(expected).hexdigest(),'transcript_sha256':hashlib.sha256(text).hexdigest(),'primary':'exact transcript; pixel comparison secondary','reason':'origin 0, white/black 6x8 glyphs, full columns/rows, 10*scale pitch; no footer or startup clear'}
(a.output/'result.json').write_text(json.dumps(result,indent=2)+'\n')
assert result['passed'],'Single-console pixels differ; retain expected/capture and inspect the transcript'
