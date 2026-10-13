#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""C2 SDK C execution and kernel rejection tests. Local QEMU, never host syscalls."""
import argparse, hashlib, json, os, pathlib, shutil, struct, subprocess, time
ROOT=pathlib.Path(__file__).resolve().parents[1]
p=argparse.ArgumentParser();p.add_argument('--firmware-dir',required=True,type=pathlib.Path);p.add_argument('--qemu',default='qemu-system-x86_64');p.add_argument('--case',default='all');p.add_argument('--lifecycle',action='store_true');a=p.parse_args()
out=ROOT/('out/ring3-lifecycle' if a.lifecycle else 'out/ring3-elf');out.mkdir(parents=True,exist_ok=True)
report={'scope':'C3 repeated launch and OS return' if a.lifecycle else 'C2 embedded SDK C ELF; no Rust guest or production loader','commands':[],'cases':[]}
def run(cmd,**kw):
 cmd=list(map(str,cmd));report['commands'].append(cmd);return subprocess.check_output(cmd,stderr=subprocess.STDOUT,cwd=ROOT,**kw).decode()
try:
 (out/'sdk.log').write_text(run(['python3','tools/check-sdk.py','--output',out/'sdk']))
 sdk=ROOT/'sdk';built=out/'sdk'
 sysroot=pathlib.Path(run(['rustc','--print','sysroot']).strip());host=run(['rustc','-vV']).split('host: ')[1].splitlines()[0]
 env=dict(os.environ,DYLD_LIBRARY_PATH=str(sysroot/'lib'))
 lld=sysroot/'lib/rustlib'/host/'bin/rust-lld'
 cases={'hello':(built/'hello-c.elf','exit')}
 flags=['--target=x86_64-unknown-none-elf','-isystem',sdk/'sysroot/include','-I',sdk/'src','-ffreestanding','-fno-builtin','-fno-stack-protector','-fno-pic','-mno-red-zone','-march=x86-64','-mno-avx','-O2','-Wall','-Wextra','-Werror']
 for n,name in enumerate((['abi','start-ro','stack-guard','log-denied','nonzero','ud','gp','divide'] if a.lifecycle else ['abi','start-ro','stack-guard','log-denied'])):
  obj=out/(name+'.o');elf=out/(name+'.elf')
  run(['clang',*flags,'-DC2_CASE='+str(n),'-c',sdk/'tests/guest-c.c','-o',obj])
  run([lld,'-flavor','gnu','-static','--no-undefined','-T',sdk/'linker/app.ld','-o',elf,*[built/(x+'.o') for x in ['crt0','libmusha','memory','note']],obj],env=env)
  cases[name]=(elf,('fault'+str({5:6,6:13,7:0}[n])) if n>=5 else 'exit' if n in (0,3,4) else ('pf7' if n==1 else 'pf4'))
 b=(built/'hello-c.elf').read_bytes();ph=struct.unpack_from('<Q',b,32)[0];count=struct.unpack_from('<H',b,56)[0]
 headers=[ph+i*56 for i in range(count)];loads=[h for h in headers if struct.unpack_from('<I',b,h)[0]==1]
 note=next(h for h in headers if struct.unpack_from('<I',b,h)[0]==4);off=struct.unpack_from('<Q',b,note+8)[0]
 denied=bytearray((out/'log-denied.elf').read_bytes());dn=next(h for h in headers if struct.unpack_from('<I',denied,h)[0]==4);do=struct.unpack_from('<Q',denied,dn+8)[0];struct.pack_into('<Q',denied,do+44,0);(out/'log-denied.elf').write_bytes(denied)
 mutations=[('bad-magic',0,'I',0),('wx',loads[0]+4,'I',7),('dynamic',loads[0],'I',2),('entry',24,'Q',0),('file-overflow',loads[0]+8,'Q',2**64-1),('note-version',off+24,'I',99),('note-permission',off+44,'Q',2),('overlap',loads[1]+16,'Q',struct.unpack_from('<Q',b,loads[0]+16)[0]),('missing-note',note,'I',0)]
 for name,pos,fmt,value in mutations:
  bad=bytearray(b);struct.pack_into('<'+fmt,bad,pos,value);path=out/(name+'.elf');path.write_bytes(bad);cases[name]=(path,'reject')
 if a.lifecycle:
  cases['map-failure']=(out/'abi.elf','reject-map')
  cases['kernel-fault']=(out/'abi.elf','fatal')
 if a.case!='all' and a.case not in cases:raise ValueError('Unknown case '+a.case)
 for name,(elf,expected) in cases.items():
  if a.case not in ('all',name):continue
  d=out/name;d.mkdir(exist_ok=True)
  buildenv=dict(os.environ,MUSHA_RING3_CASE='normal',MUSHA_USER_ELF=str(elf),CARGO_TARGET_DIR=str(out/'target'))
  report['commands'].append({'case':name,'MUSHA_USER_ELF':str(elf),'CARGO_TARGET_DIR':buildenv['CARGO_TARGET_DIR']})
  (d/'build.log').write_text(run(['cargo','build','--locked','--release','--target','x86_64-unknown-uefi','-p','musha-boot','--features',(('ring3-lifecycle-kernel-fault' if name=='kernel-fault' else ('ring3-lifecycle-map-failure' if name=='map-failure' else 'ring3-lifecycle-probe'))+',file-viewer-probe') if a.lifecycle else 'ring3-elf-probe'],env=buildenv))
  esp=d/'esp/EFI/BOOT';esp.mkdir(parents=True,exist_ok=True);shutil.copyfile(out/'target/x86_64-unknown-uefi/release/musha-boot.efi',esp/'BOOTX64.EFI')
  shutil.copyfile(a.firmware_dir/'edk2-i386-vars.fd',d/'vars.fd');debug=d/'debug.log';debug.write_text('')
  cmd=[a.qemu,'-machine','q35,accel=tcg','-cpu','qemu64,+syscall,+nx','-m','256M','-drive',f'if=pflash,format=raw,readonly=on,file={a.firmware_dir/"edk2-x86_64-code.fd"}','-drive',f'if=pflash,format=raw,file={d/"vars.fd"}','-drive',f'if=ide,format=raw,file=fat:rw:{d/"esp"}','-device','qemu-xhci','-vga','std','-display','none','-monitor','none','-serial','none','-debugcon',f'file:{debug}','-global','isa-debugcon.iobase=0xe9','-no-reboot']
  report['commands'].append(cmd)
  with (d/'qemu.log').open('w') as log:
   proc=subprocess.Popen(cmd,stdout=log,stderr=log)
   try:
    deadline=time.monotonic()+60
    while time.monotonic()<deadline:
     text=debug.read_text()
     if a.lifecycle:
      if 'C3_OS_RESUMED\n' in text and 'MUSHA: APP_ENTERING\n' in text:break
      if 'EXCEPTION VECTOR=' in text and text.endswith('\n'):
       if expected=='fatal':break
       raise RuntimeError(text)
     elif text.endswith('\n') and any(x in text for x in ['RING3_EXIT_OK','RING3_REJECT','EXCEPTION VECTOR=']):break
     if proc.poll() is not None:raise RuntimeError((d/'qemu.log').read_text())
     time.sleep(.05)
    else:raise RuntimeError('Timeout '+text)
    if a.lifecycle and expected=='fatal':
     assert 'EXCEPTION VECTOR=0000000000000006' in text and 'RING3_FAULT CS=0000000000000008' in text,text
     assert 'C3_RECLAIMED' not in text and 'C3_OS_RESUMED' not in text,text
    elif a.lifecycle:
     outcome=2 if expected.startswith('reject') else (1 if expected.startswith(('pf','fault')) else 0)
     detail=14 if expected.startswith('pf') else (int(expected[5:]) if expected.startswith('fault') else (42 if name=='nonzero' else 0))
     for attempt in range(3):
      assert f'C3_RECLAIMED ATTEMPT={attempt:016X} OUTCOME={outcome:016X} DETAIL={detail:016X}\n' in text,text
     if expected.startswith('pf'):
      error=7 if expected=='pf7' else 4
      assert text.count(f'C3_APP_FAULT VECTOR=000000000000000E ERROR={error:016X}')==3,text
     elif expected.startswith('fault'):
      assert text.count(f'C3_APP_FAULT VECTOR={detail:016X} ERROR=0000000000000000')==3,text
     elif expected.startswith('reject'):
      reason='ELF_FORMAT' if expected=='reject' else 'INJECTED_MAP_FAILURE'
      assert text.count('RING3_REJECT '+reason)==3 and 'RING3_ELF_ENTERING' not in text,text
     elif name=='abi':assert text.count('C2 ABI BSS ARENA OK\n')==3,text
     elif name=='hello':assert text.count('Hello Musha-OS!')==3,text
     elif name=='log-denied':assert 'denied' not in text,text
    elif expected=='reject':
     assert 'RING3_REJECT ELF_FORMAT' in text and 'RING3_ELF_ENTERING' not in text,text
    else:
     assert 'RING3_ELF_RUNNING' in text and 'RING3_QUERY KERNEL_STACK_OK' in text,text
     if expected=='exit':
      assert 'RING3_EXIT_OK' in text and 'EXCEPTION' not in text and 'RING3_REJECT' not in text,text
      if name=='hello':assert 'Hello Musha-OS!' in text,text
      if name=='abi':assert 'C2 COPY\n' in text and 'C2 ABI BSS ARENA OK\n' in text,text
      if name=='log-denied':assert 'denied' not in text,text
     else:
      error=7 if expected=='pf7' else 4
      assert f'EXCEPTION VECTOR=000000000000000E ERROR={error:016X}' in text,text
      assert 'RING3_FAULT_KERNEL_STACK_OK' in text and 'RING3_EXIT_OK' not in text,text
    report['cases'].append({'case':name,'passed':True,'elf_sha256':hashlib.sha256(elf.read_bytes()).hexdigest()});print(('PASS C3 ' if a.lifecycle else 'PASS C2 ')+name,flush=True)
   finally:
    proc.terminate()
    try:proc.wait(timeout=5)
    except subprocess.TimeoutExpired:proc.kill();proc.wait()
finally:
 (out/'report.json').write_text(json.dumps(report,indent=2)+'\n')
