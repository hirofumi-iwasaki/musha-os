#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""Host-only experimental-r1/r2 ELF validator. Kernel must repeat all checks in stage C."""
import json,struct,pathlib,sys
K=json.loads((pathlib.Path(__file__).resolve().parents[1]/'abi/experimental.json').read_text())['constants']
def check(b):
 def require(ok,why):
  if not ok: raise ValueError(why)
 require(len(b)>=64 and b[:7]==b'\x7fELF\x02\x01\x01','ELF identity')
 h=struct.unpack_from('<HHIQQQIHHHHHH',b,16)
 typ,machine,version,entry,phoff,shoff,flags,ehsize,phsize,phnum,shsize,shnum,shstr=h
 require(typ==2 and machine==62 and version==1 and flags==0 and ehsize==64,'static x64 ET_EXEC')
 require(0<phnum<=K['MAX_SEGMENTS'] and phsize==56 and phoff>=64 and phoff+phnum*56<=len(b),'program headers')
 require(len(b)<=K['IMAGE_LIMIT'],'file size')
 ranges=[];notes=[];executable=[]
 for i in range(phnum):
  t,f,off,va,pa,fs,ms,al=struct.unpack_from('<IIQQQQQQ',b,phoff+i*56)
  require(off+fs<=len(b),'file range')
  require(t in (0,1,4,0x6474e551),'unsupported program header')
  if t==0x6474e551:require(f==6,'executable stack')
  if t==1:
   require(fs<=ms and f in (4,5,6),'load size or W^X')
   require(al>=4096 and al&(al-1)==0 and off%al==va%al,'alignment')
   require(K['IMAGE_BASE']<=va and va+ms<=K['IMAGE_BASE']+K['IMAGE_LIMIT'],'image virtual range')
   if ms:
    pages=(va//4096,(va+ms+4095)//4096)
    require(all(pages[1]<=a or pages[0]>=z for a,z in ranges),'page overlap')
    ranges.append(pages)
    if f==5:executable.append((va,va+fs))
  if t==4:
   require(off%4==0 and fs==84,'one aligned 84-byte NOTE per PH')
   pos=off
   while pos<off+fs:
    require(pos+12<=off+fs,'note header')
    ns,ds,nt=struct.unpack_from('<III',b,pos);pos+=12
    endname=pos+ns;desc=(endname+3)&~3;end=(desc+ds+3)&~3
    require(end<=off+fs,'note bounds')
    require(b[pos:endname]==b'MUSHA\0' and nt==K['NOTE_TYPE'],'note identity')
    require(ns==6 and ds==64,'note size')
    require(not any(b[endname:desc]) and not any(b[desc+ds:end]),'note padding')
    notes.append(struct.unpack_from('<IIHHIQQQQQQ',b,desc));pos=end
 require(len(notes)==1,'exactly one Musha note')
 size,rev,major,minor,cpu,features,permissions,arena,stack,flags,reserved=notes[0]
 require(size==64 and (rev,minor) in [(K['CANDIDATE_REVISION'],K['ABI_MINOR']),(K['R2_REVISION'],K['R2_MINOR'])] and major==K['ABI_MAJOR'] and cpu==K['CPU_PROFILE'],'note version/cpu')
 require(features==(K['FEATURE_BASE'] | (K['FEATURE_INPUT'] if rev==2 else 0)) and permissions&~(15 if rev==2 else K['PERMISSION_LOG'])==0 and flags==0 and reserved==0,'features/permissions/reserved')
 require(4096<=arena<=K['ARENA_LIMIT'] and arena%4096==0 and 4096<=stack<=K['STACK_LIMIT'] and stack%4096==0,'requested RAM/stack')
 require(any(a<=entry<z for a,z in executable),'entry not file-backed executable')
 return {'profile':'experimental-r'+str(rev),'entry':entry,'loads':len(ranges),'arena':arena,'stack':stack,'guest_execution':False}
if __name__=='__main__':print(json.dumps(check(pathlib.Path(sys.argv[1]).read_bytes()),indent=2))
