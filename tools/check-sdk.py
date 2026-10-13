#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""Compile-only SDK and host conformance; never execute a candidate syscall."""
import argparse,importlib.util,json,os,pathlib,shutil,struct,subprocess,tempfile
ROOT=pathlib.Path(__file__).resolve().parents[1]
p=argparse.ArgumentParser();p.add_argument('--sdk',type=pathlib.Path,default=ROOT/'sdk');p.add_argument('--output',type=pathlib.Path,default=ROOT/'out/sdk-check');a=p.parse_args()
sdk=a.sdk.resolve();out=a.output.resolve();out.mkdir(parents=True,exist_ok=True)
commands=[]
def run(args,**kw):
 args=list(map(str,args));commands.append(args);return subprocess.check_output(args,stderr=subprocess.STDOUT,**kw).decode()
clang=os.environ.get('CC','clang');rustc=os.environ.get('RUSTC','rustc')
versions={x:run([cmd,'--version']).splitlines()[0] for x,cmd in [('clang',clang),('rustc',rustc)]}
sysroot=pathlib.Path(run([rustc,'--print','sysroot']).strip());host=run([rustc,'-vV']).split('host: ')[1].splitlines()[0]
lld=os.environ.get('LD_LLD',str(sysroot/'lib/rustlib'/host/'bin/rust-lld'))
os.environ['DYLD_LIBRARY_PATH']=str(sysroot/'lib')
versions['lld']=run([lld,'-flavor','gnu','--version']).strip()
run(['python3',sdk/'tools/generate.py','--check'])
flags=['--target=x86_64-unknown-none-elf','--sysroot='+str(sdk/'sysroot'),'-isystem',str(sdk/'sysroot/include'),'-std=c11','-ffreestanding','-fno-builtin','-fno-stack-protector','-fno-pic','-mno-red-zone','-march=x86-64','-mno-avx','-Wall','-Wextra','-Werror','-O2']
objects=[]
for name,source in [('layout','tests/layout.c'),('wrapper','tests/wrapper.c'),('fp','tests/fp.c'),('crt0','src/crt0.S'),('libmusha','src/libmusha.c'),('memory','src/memory.c'),('main','examples/hello-c/main.c')]:
 obj=out/(name+'.o');run([clang,*flags,'-c',sdk/source,'-o',obj]);objects.append(obj)
 require=obj.read_bytes();assert require[:7]==b'\x7fELF\x02\x01\x01' and struct.unpack_from('<HH',require,16)==(1,62)
run([clang,*flags,'-S',sdk/'tests/wrapper.c','-o',out/'wrapper.s'])
asm=(out/'wrapper.s').read_text();assert 'syscall' in asm and '%rcx, %r10' in asm and '"+d"(c)' in (sdk/'src/syscall.h').read_text()
# Ordinary C FP compile material; caller/callee comparison is not execution evidence.
run([clang,*flags,'-S','-emit-llvm',sdk/'tests/fp.c','-o',out/'fp-c.ll'])
# Native layout test is separate from the ELF x64 assertions above.
run([clang,'-std=c11','-I',sdk/'sysroot/include',sdk/'tests/layout.c','-o',out/'layout-host']);run([out/'layout-host'])
k=json.loads((sdk/'abi/experimental.json').read_text())['constants']
desc=struct.pack('<IIHHIQQQQQQ',64,k['CANDIDATE_REVISION'],k['ABI_MAJOR'],k['ABI_MINOR'],k['CPU_PROFILE'],1,1,1048576,65536,0,0)
note=struct.pack('<III',6,64,k['NOTE_TYPE'])+b'MUSHA\0\0\0'+desc
(out/'note.S').write_text('.section .note.musha,"a",@note\n.balign 4\n.byte '+','.join(map(str,note))+'\n')
run([clang,*flags,'-c',out/'note.S','-o',out/'note.o'])
run([lld,'-flavor','gnu','-static','--no-undefined','-T',sdk/'linker/app.ld','-o',out/'hello-c.elf',*[out/(n+'.o') for n in ['crt0','libmusha','memory','main','note']]])
spec=importlib.util.spec_from_file_location('checker',sdk/'tools/app-check.py');checker=importlib.util.module_from_spec(spec);spec.loader.exec_module(checker)
b=(out/'hello-c.elf').read_bytes();elf=checker.check(b)
# Reject malformed identities, entry, PH bounds, missing/duplicate NOTE, permissions,
# alignment, page overlap, dynamic/TLS headers, reserved fields and memory limits.
mutations=[]
def mutate(off,fmt,val):x=bytearray(b);struct.pack_into(fmt,x,off,val);mutations.append(x)
mutate(16,'H',3);mutate(18,'H',183);mutate(24,'Q',0);mutate(32,'Q',len(b));mutate(56,'H',33)
ph=struct.unpack_from('<Q',b,32)[0];num=struct.unpack_from('<H',b,56)[0]
loads=[]
for i in range(num):
 q=ph+i*56;t=struct.unpack_from('<I',b,q)[0]
 if t==1:loads.append(q)
 if t==4:
  mutate(q,'I',0);off=struct.unpack_from('<Q',b,q+8)[0]
  for offset,fmt,val in [(20,'I',63),(24,'I',99),(28,'H',3),(32,'I',99),(36,'Q',8),(44,'Q',8),(52,'Q',0),(60,'Q',0),(68,'Q',1),(76,'Q',1)]:mutate(off+offset,fmt,val)
q=loads[0]
for t in [2,3,7]:mutate(q,'I',t)
mutate(q+4,'I',7);mutate(q+48,'Q',3);mutate(q+40,'Q',0);mutate(q+8,'Q',len(b));mutate(q+16,'Q',0x70000000)
if len(loads)>1:mutate(loads[1]+16,'Q',struct.unpack_from('<Q',b,q+16)[0])
# Duplicate the NOTE through the spare GNU_STACK PH; do not invent a second format.
headers=[ph+i*56 for i in range(num)]
note_header=next(q for q in headers if struct.unpack_from('<I',b,q)[0]==4)
spare=next(q for q in headers if struct.unpack_from('<I',b,q)[0]==0x6474e551)
x=bytearray(b);x[spare:spare+56]=b[note_header:note_header+56];mutations.append(x)
for x in mutations:
 try:checker.check(x)
 except ValueError:pass
 else:raise AssertionError('Malformed ELF accepted')
# Rust x64 types/wrapper compile on current installed UEFI target, explicitly not
# an application ELF target. Explicit SysV FP functions expose the separate ABI.
run(['cargo','test','--manifest-path',sdk/'rust/Cargo.toml','--target-dir',out/'rust-host'])
run(['cargo','check','--manifest-path',sdk/'rust/Cargo.toml','--target','x86_64-unknown-uefi','--target-dir',out/'rust-x64'])
run([rustc,'--edition=2024','--crate-type=lib','--target=x86_64-unknown-uefi','-C','no-redzone=yes','--emit=llvm-ir',sdk/'tests/fp.rs','-o',out/'fp-rust.ll'])
assert 'x86_64_sysvcc' in (out/'fp-rust.ll').read_text()
# The Rust Hello is compiled as an object library with explicit SysV ABI on the
# installed x64 target. This deliberately does not claim a Rust application ELF.
run([rustc,'--edition=2024','--crate-type=rlib','--crate-name=musha_sys','--target=x86_64-unknown-uefi',sdk/'rust/musha-sys/src/lib.rs','-o',out/'libmusha_sys.rlib'])
run([rustc,'--edition=2024','--crate-type=lib','--crate-name=hello_musha','--target=x86_64-unknown-uefi','--extern','musha_sys='+str(out/'libmusha_sys.rlib'),'--emit=llvm-ir',sdk/'examples/hello-rust/main.rs','-o',out/'hello-rust.ll'])
# Inspect actual pinned compiler target settings; bootstrap only enables this
# introspection. No unstable production Rust application is built here.
inspect_env=dict(os.environ,RUSTC_BOOTSTRAP='1')
for name,target in [('builtin-none','x86_64-unknown-none'),('custom-user',str(sdk/'targets/x86_64-musha-user.json'))]:
 spec=run([rustc,'-Z','unstable-options','--print','target-spec-json','--target',target],env=inspect_env)
 (out/(name+'.json')).write_text(spec)
 if name=='builtin-none': assert json.loads(spec).get('rustc-abi')=='softfloat'
 else: assert json.loads(spec).get('rustc-abi')!='softfloat'
report={'versions':versions,'commands':commands,'elf':elf,'negative_elf_cases':len(mutations),'c_x64_layout':'start and NOTE: all offsets, size 64, alignment 8','rust_x64_layout':'UEFI target compile only; not ELF SDK','guest_execution':False,'public_compatibility_fixed':False,'rust_custom_core_built':False}
(out/'report.json').write_text(json.dumps(report,indent=2)+'\n');print(json.dumps({k:v for k,v in report.items() if k!='commands'},indent=2))
