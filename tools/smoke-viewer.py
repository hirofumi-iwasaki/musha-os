#!/usr/bin/env python3
# Copyright 2026 Hirofumi Iwasaki
# SPDX-License-Identifier: Apache-2.0
"""Drive the production viewer with QMP keys; verify rendered-page hashes and cleanup."""
import argparse, hashlib, json, pathlib, shutil, socket, subprocess, time
p=argparse.ArgumentParser()
p.add_argument('--firmware-dir',required=True)
p.add_argument('--qemu',default='qemu-system-x86_64')
p.add_argument('--boot-image',required=True,type=pathlib.Path)
p.add_argument('--data-image',required=True,type=pathlib.Path)
p.add_argument('--format',choices=['exfat','fat32'],default='exfat')
p.add_argument('--split-controller',action='store_true')
p.add_argument('--apple-double-files',action='store_true',help='macOS fixture has one ._ metadata file after each test file')
p.add_argument('--case',choices=['normal','cancel','disconnect','timeout','no-medium'],default='normal')
a=p.parse_args();root=pathlib.Path(__file__).resolve().parent.parent
out=root/'out'/('qemu-viewer-'+a.format+'-'+a.case+('-split' if a.split_controller else ''));out.mkdir(parents=True,exist_ok=True)
f=pathlib.Path(a.firmware_dir);shutil.copyfile(f/'edk2-i386-vars.fd',out/'vars.fd')
log=out/'debug.log';log.write_text('');qmp=root/'out'/'viewer.sock';qmp.unlink(missing_ok=True)
boot=a.boot_image.resolve();data=a.data_image.resolve()
def sha(path):return hashlib.sha256(path.read_bytes()).hexdigest()
hashes=(sha(boot),sha(data))
cmd=[a.qemu,'-machine','q35,accel=tcg','-m','256M','-netdev','user,id=n','-device','e1000e,netdev=n',
'-drive',f'if=pflash,format=raw,readonly=on,file={f/"edk2-x86_64-code.fd"}',
'-drive',f'if=pflash,format=raw,file={out/"vars.fd"}',
'-drive',f'if=ide,format=raw,snapshot=on,file={boot}',
'-drive',f'if=none,id=data,format=raw,readonly=on,file={data}',
'-device','qemu-xhci,id=usbhost','-device','usb-storage,id=storage,drive=data,bus=usbhost.0,port=1']
if a.split_controller:cmd+=['-device','qemu-xhci,id=inputhost']
cmd+=['-device','usb-kbd,id=keyboard,bus='+('inputhost.0' if a.split_controller else 'usbhost.0'),'-vga','std','-display','none','-debugcon',f'file:{log}','-global','isa-debugcon.iobase=0xe9','-qmp',f'unix:{qmp},server=on,wait=off','-no-reboot']
with (out/'qemu.log').open('w') as err:
 proc=subprocess.Popen(cmd,stdout=err,stderr=err)
 try:
  def wait(marker,start=0,limit=60):
   deadline=time.monotonic()+limit
   while True:
    text=log.read_text()
    if marker in text[start:]:return text.index(marker,start)+len(marker)
    if time.monotonic()>deadline or proc.poll() is not None:raise RuntimeError('Missing '+marker+'\n'+text[-6000:])
    time.sleep(.02)
  wait('RUNTIME_POLL_READY')
  qs=socket.socket(socket.AF_UNIX);qs.settimeout(10);qs.connect(str(qmp));stream=qs.makefile('rwb');stream.readline()
  def control(name,args=None):
   stream.write((json.dumps({'execute':name,'arguments':args or {}})+'\n').encode());stream.flush()
   while True:
    response=json.loads(stream.readline())
    if 'error' in response:raise RuntimeError(response)
    if 'return' in response:return
  control('qmp_capabilities')
  def key(code):
   start=len(log.read_text())
   for _ in range(2 if a.apple_double_files and code=='down' else 1):
    control('send-key',{'keys':[{'type':'qcode','data':code}],'hold-time':30});time.sleep(.08)
   return start
  def screen(name):control('screendump',{'filename':str(out/(name+'.ppm'))})
  def fnv(b):
   h=0xcbf29ce484222325
   for x in b:h=((h^x)*0x100000001b3)&0xffffffffffffffff
   return h
  def frame(view,index=0,offset=0,payload=b'',start=0):
   if a.apple_double_files:index*=2
   wait(f'VIEWER_FRAME {view} {index:016X} {offset:016X} {len(payload):016X} {fnv(payload):016X}',start)
  if a.case=='no-medium':
   frame('UNAVAILABLE');screen('no-medium')
  else:
   frame('LIST');screen('list')
  if a.case in ('cancel','disconnect','timeout'):
   start=key('ret');wait('LIVE_STORAGE_WAIT',start)
   if a.case=='cancel':
    start=key('esc');frame('LIST',start=start);start=key('ret');frame('PAGE',payload=b'Hello Musha-OS!\n',start=start)
   else:
    if a.case=='disconnect':control('device_del',{'id':'storage'})
    wait('INPUT_SOURCE_DETACHED',start);frame('UNAVAILABLE',start=start);screen(a.case)
    assert 'VIEWER_FRAME PAGE' not in log.read_text()[start:]
  elif a.case=='normal':
   start=key('ret');frame('PAGE',payload=b'Hello Musha-OS!\n',start=start);screen('text')
   start=key('esc');frame('LIST',start=start)
   if a.format=='exfat':
    start=key('down');frame('LIST',index=1,start=start);start=key('ret');chunk=bytes(range(256));frame('PAGE',1,payload=chunk,start=start);screen('binary')
    for code,offset in [('pgdn',256),('pgdn',512),('pgdn',768),('pgdn',1024),('pgdn',1280),('pgup',1024),('home',0)]:
     start=key(code);frame('PAGE',1,offset,b'end' if offset==1280 else chunk,start)
    start=key('esc');frame('LIST',1,start=start)
    for index,payload in [(2,b'Long file name\n'),(3,b''),(4,b'valid'+bytes(251))]:
     start=key('down');frame('LIST',index,start=start);start=key('ret');frame('PAGE',index,payload=payload,start=start);screen('file-'+str(index))
     if index==4:
      start=key('pgdn');frame('PAGE',index,256,bytes(256),start)
      start=key('pgdn');frame('PAGE',index,512,bytes(188),start)
     start=key('esc');frame('LIST',index,512 if index==4 else 0,start=start)
  if a.case=='no-medium' and 'INPUT_SOURCE_DETACHED' in log.read_text():
   wait('RUNTIME_COOPERATIVE_OK')
  elif a.case not in ('disconnect','timeout'):
   start=key('q');wait('INPUT_SOURCE_DETACHED',start);wait('RUNTIME_COOPERATIVE_OK',start)
  wait('BOOT_TIMES CORE READY ENTRY RUNNING FRAME')
  text=log.read_text();assert 'XHCI_QUIESCED DMA_DISABLED' in text
  milestones=['CORE_READY','OS_READY STATIC_COOPERATIVE','APP_ENTERING','APP_RUNNING STATIC_LINKED']
  positions=[text.index('MUSHA: '+m) for m in milestones]
  assert positions==sorted(positions) and text.count('MUSHA: OS_READY ')==1
  if a.case!='no-medium':
   assert text.index('LIVE_FILE_API_READY')<positions[1]
   assert 'OS_READY STATIC_COOPERATIVE FILE_VIEWER' in text
  import re
  sample=re.search(r'BOOT_TIMES CORE READY ENTRY RUNNING FRAME MS UNKNOWN=FFFFFFFFFFFFFFFF ([0-9A-F ]+)',text)
  assert sample, 'Missing boot timestamps'
  times=[int(x,16) for x in sample.group(1).split()]
  assert len(times)==5 and times==sorted(times) and times[-1]!=2**64-1

  assert (sha(boot),sha(data))==hashes
  control('quit');proc.wait(timeout=5)
  print('PASS: viewer',a.format,a.case,'split' if a.split_controller else 'same','keys, page hashes, read-only images, DMA cleanup')
 finally:
  if proc.poll() is None:proc.terminate();proc.wait(timeout=5)
  qmp.unlink(missing_ok=True)
