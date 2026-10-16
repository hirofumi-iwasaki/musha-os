#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""Capture a display-grid-probe stage held by MUSHA_GRID_HOLD; local QEMU only."""
import argparse,ast,json,re,shutil,socket,subprocess,time,hashlib,importlib.util
from pathlib import Path
p=argparse.ArgumentParser(description=__doc__);p.add_argument('--efi',required=True,type=Path);p.add_argument('--resolution',required=True);p.add_argument('--stage',choices=['starting','diagnostics'],required=True);p.add_argument('--output',required=True,type=Path);a=p.parse_args()
root=Path(__file__).resolve().parents[1];o=a.output.resolve();o.mkdir(parents=True,exist_ok=False)
w,h=map(int,a.resolution.split('x'));scale=max(1,min(4,(h+300)//600));fw=Path('/opt/homebrew/share/qemu')
(o/'esp/EFI/BOOT').mkdir(parents=True);shutil.copyfile(a.efi,o/'esp/EFI/BOOT/BOOTX64.EFI');shutil.copyfile(fw/'edk2-i386-vars.fd',o/'vars.fd')
spec=importlib.util.spec_from_file_location('startup_fixture',root/'tools/smoke-startup.py')
startup=importlib.util.module_from_spec(spec);spec.loader.exec_module(startup)
startup.fixture(o/'boot.img',a.efi,b'log=devices\n')
qmp=root/'out'/('grid-'+hashlib.sha256(str(o).encode()).hexdigest()[:10]+'.sock')
cmd=['qemu-system-x86_64','-machine','q35,accel=tcg','-m','256M','-drive',f'if=pflash,format=raw,readonly=on,file={fw/"edk2-x86_64-code.fd"}','-drive',f'if=pflash,format=raw,file={o/"vars.fd"}','-drive',f'if=ide,format=raw,snapshot=on,file={o/"boot.img"}','-device','qemu-xhci','-vga','std','-global',f'VGA.xres={w}','-global',f'VGA.yres={h}','-global','VGA.vgamem_mb=64','-display','none','-qmp',f'unix:{qmp},server=on,wait=off','-no-reboot']
font={}
for key,values in re.findall(r"(b'(?:\\.|[^'])+'|\d+) => \[([\d, ]+)\]",(root/'framebuffer/src/lib.rs').read_text()):
 c=ast.literal_eval(key);font[tuple(map(int,values.split(',')))]=chr(c[0] if isinstance(c,bytes) else c)
font[(0,)*7]=' '
def read_screen():
 data=(o/'screen.ppm').read_bytes();m=re.match(rb'P6\s+(\d+)\s+(\d+)\s+255\s',data)
 if not m or tuple(map(int,m.groups()))!=(w,h):return None
 data=data[m.end():];lines=[]
 for row in range(h//(10*scale)):
  line=''
  for col in range(w//(6*scale)):
   bits=[]
   for y in range(7):
    v=0
    for x in range(5):
     at=(((row*10+y)*scale)*w+(col*6+x)*scale)*3
     v=(v<<1)|(data[at:at+3] in [bytes([255]*3)])
    bits.append(v)
   line+=font.get(tuple(bits),'?')
  lines.append(line.rstrip())
 return lines
with (o/'qemu.log').open('w') as log:
 proc=subprocess.Popen(cmd,stdout=log,stderr=log)
 try:
  end=time.monotonic()+60
  while not qmp.exists():
   assert time.monotonic()<end;time.sleep(.05)
  with socket.socket(socket.AF_UNIX) as sock:
   sock.settimeout(10);sock.connect(str(qmp));f=sock.makefile('rwb');f.readline()
   def ctl(name,args={}):
    f.write((json.dumps({'execute':name,'arguments':args})+'\n').encode());f.flush()
    while True:
     r=json.loads(f.readline())
     if 'error' in r:raise RuntimeError(r)
     if 'return' in r:return
   ctl('qmp_capabilities')
   while time.monotonic()<end:
    ctl('screendump',{'filename':str(o/'screen.ppm')});rows=read_screen()
    if rows and ('boot: starting musha-os' if a.stage=='starting' else 'pci: devices=') in '\n'.join(rows):break
    time.sleep(.1)
   else:raise RuntimeError('Stage not visible')
   (o/'screen.txt').write_text('\n'.join(rows)+'\n')
   expected = 'boot: starting musha-os' if a.stage == 'starting' else 'pci: devices='
   assert expected in '\n'.join(rows), rows
   assert any(line.startswith('[') for line in rows), 'devices timestamp missing at left origin'
   (o/'result.json').write_text(json.dumps({'passed':True,'resolution':[w,h],'stage':a.stage,'scale':scale,'efi_sha256':hashlib.sha256(a.efi.read_bytes()).hexdigest(),'scope':'test-only exact GOP selection and stage hold; not distribution EFI','command':cmd},indent=2)+'\n')
 finally:
  proc.terminate();proc.wait(timeout=10);qmp.unlink(missing_ok=True)
