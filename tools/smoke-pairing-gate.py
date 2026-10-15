#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""Verify that P/pair=on visibly refuse an Bluetooth adapter absence or the isolated passkey UI fixture."""
import argparse
import hashlib
import importlib.util
import json
from pathlib import Path
import shutil
import socket
import subprocess
import time
ROOT=Path(__file__).resolve().parents[1]
p=argparse.ArgumentParser(description=__doc__)
p.add_argument('--efi',type=Path,required=True)
p.add_argument('--output',type=Path,required=True)
p.add_argument('--firmware-dir',type=Path,required=True)
p.add_argument('--qemu',default='qemu-system-x86_64')
p.add_argument('--entry',choices=['p','config','submit','cancel','timeout','restart'],required=True)
a=p.parse_args();out=a.output.resolve();out.mkdir(parents=True,exist_ok=False)
def load(name,path):
 s=importlib.util.spec_from_file_location(name,ROOT/path);m=importlib.util.module_from_spec(s);s.loader.exec_module(m);return m
startup=load('startup_fixture','tools/smoke-startup.py');external=load('screen_decoder','tools/smoke-external-elf.py')
image=out/'data.img';startup.fixture(image,a.efi.resolve(),b'log=ready\npair=on\n' if a.entry=='config' else b'log=ready\n')
before=hashlib.sha256(image.read_bytes()).hexdigest();shutil.copyfile(a.firmware_dir/'edk2-i386-vars.fd',out/'vars.fd')
sock=ROOT/'out'/('pairgate-'+hashlib.sha256(str(out).encode()).hexdigest()[:10]+'.sock');assert not sock.exists()
log=out/'debug.log';cmd=[a.qemu,'-machine','q35,accel=tcg','-m','256M','-drive',f'if=pflash,format=raw,readonly=on,file={a.firmware_dir}/edk2-x86_64-code.fd','-drive',f'if=pflash,format=raw,file={out}/vars.fd','-drive',f'if=ide,format=raw,snapshot=on,file={image}','-drive',f'if=none,id=data,format=raw,readonly=on,file={image}','-device','qemu-xhci,id=x','-device','usb-storage,drive=data,bus=x.0','-device','usb-kbd,bus=x.0','-display','none','-vga','std','-debugcon',f'file:{log}','-global','isa-debugcon.iobase=0xe9','-qmp',f'unix:{sock},server=on,wait=off','-no-reboot']
with (out/'qemu.log').open('w') as errors:
 proc=subprocess.Popen(cmd,stdout=errors,stderr=errors)
 try:
  deadline=time.monotonic()+90
  while not sock.exists():
   assert proc.poll() is None and time.monotonic()<deadline;time.sleep(.05)
  with socket.socket(socket.AF_UNIX) as q:
   q.settimeout(10);q.connect(str(sock));stream=q.makefile('rwb');stream.readline()
   def call(name,args=None):
    stream.write((json.dumps({'execute':name,'arguments':args or {}})+'\n').encode());stream.flush()
    while True:
     response=json.loads(stream.readline())
     if 'error' in response:raise RuntimeError(response)
     if 'return' in response:return response
   call('qmp_capabilities')
   while not log.exists() or b'VIEWER_FRAME LIST' not in log.read_bytes():
    assert proc.poll() is None and time.monotonic()<deadline;time.sleep(.05)
   if a.entry=='restart':
    while True:
     text=log.read_text()
     if text.count('RUNTIME_POLL_READY')>=2 and 'VIEWER_FRAME LIST' in text.split('APP_ARENA_SCRUBBED VIEWER_RESTART')[-1]:break
     assert time.monotonic()<deadline and proc.poll() is None;time.sleep(.05)
    assert 'BT_RESTART_VIEWER_SOFTWARE_FIXTURE' in text and 'BT_RESTART_STOP_CONFIRMED' in text
    call('screendump',{'filename':str(out/'screen.ppm')});rows=external.screen_rows(out/'screen.ppm',viewer=True)
   elif a.entry in ('submit','cancel','timeout'):
    while b'BT_PASSKEY_FIXTURE_BEGIN' not in log.read_bytes():
     assert time.monotonic()<deadline and proc.poll() is None;time.sleep(.05)
    # The same USB keyboard/parser/viewer consumer as production; no radio,
    # RNG bypass, or synthetic pairing success is exercised by this fixture.
    while True:
     call('screendump',{'filename':str(out/'entry.ppm')})
     rows=external.screen_rows(out/'entry.ppm',viewer=True)
     if 'Bluetooth passkey: 0 digits entered.' in '\n'.join(rows):break
     assert time.monotonic()<deadline;time.sleep(.05)
    sequence=list('123456')+['ret'] if a.entry=='submit' else ['esc'] if a.entry=='cancel' else []
    for key in sequence:
     call('send-key',{'keys':[{'type':'qcode','data':key}],'hold-time':50});time.sleep(.15)
    expected={'submit':'SUBMITTED','cancel':'CANCELLED','timeout':'TIMEOUT'}[a.entry]
    while ('BT_PASSKEY_'+expected) not in log.read_text():
     assert time.monotonic()<deadline and proc.poll() is None;time.sleep(.05)
    text=log.read_text().split('BT_PASSKEY_FIXTURE_BEGIN',1)[1]
    assert not any(marker in text for marker in ('HID_KEY_DOWN=','HID_KEY_UP=','APP_KEY_DOWN=','APP_KEY_UP='))
    assert '123456' not in text
    call('screendump',{'filename':str(out/'screen.ppm')})
   else:
    if a.entry=='p':call('send-key',{'keys':[{'type':'qcode','data':'p'}],'hold-time':50})
    while True:
     assert time.monotonic()<deadline and proc.poll() is None
     call('screendump',{'filename':str(out/'screen.ppm')})
     rows=external.screen_rows(out/'screen.ppm',viewer=True)
     if 'Bluetooth pairing unavailable: no live USB HCI adapter.' in '\n'.join(rows):break
     time.sleep(.1)
    if a.entry=='config':assert 'BT_PAIR_REQUEST_BLOCKED NO_HCI_ADAPTER' in log.read_text()
   text=log.read_text();assert 'BT_HCI_RESET_TRANSPORT_OK' not in text
   assert 'BT_PAIRING_RECORD_DURABLE' not in text
   call('quit');proc.wait(timeout=5)
 finally:
  if proc.poll() is None:proc.terminate();proc.wait(timeout=5)
  sock.unlink(missing_ok=True)
assert hashlib.sha256(image.read_bytes()).hexdigest()==before
(out/'screen.txt').write_text('\n'.join(rows)+'\n')
(out/'report.json').write_text(json.dumps({'passed':True,'entry':a.entry,'image_unchanged':True,'efi_sha256':hashlib.sha256(a.efi.read_bytes()).hexdigest(),'scope':'adapter absence or keyboard UI only; no radio pairing success','command':cmd},indent=2)+'\n')
print('PASS',a.entry,'adapter/UI check; no medium changes')
