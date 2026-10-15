#!/usr/bin/env python3
# Copyright 2026 Hirofumi Iwasaki
# SPDX-License-Identifier: Apache-2.0
"""QEMU no-DHCP-server deadline, NIC stop and continuing viewer input."""
import argparse,hashlib,json,shutil,socket,struct,subprocess,time
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
p=argparse.ArgumentParser(description=__doc__);p.add_argument('--efi',type=Path,required=True);p.add_argument('--image',type=Path,required=True);p.add_argument('--output',type=Path,required=True);p.add_argument('--firmware-dir',type=Path,required=True);p.add_argument('--qemu',default='qemu-system-x86_64');a=p.parse_args();o=a.output.resolve();o.mkdir(parents=True,exist_ok=False)
esp=o/'esp/EFI/BOOT';esp.mkdir(parents=True);shutil.copyfile(a.efi,esp/'BOOTX64.EFI');shutil.copyfile(a.firmware_dir/'edk2-i386-vars.fd',o/'vars.fd')
with socket.socket()as reservation:reservation.bind(('127.0.0.1',0));port=reservation.getsockname()[1]
sock=ROOT/'out'/('dhcp-timeout-'+hashlib.sha256(str(o).encode()).hexdigest()[:10]+'.sock');sock.unlink(missing_ok=True);debug=o/'debug.log'
cmd=[a.qemu,'-machine','q35,accel=tcg','-m','256M','-drive',f'if=pflash,format=raw,readonly=on,file={a.firmware_dir}/edk2-x86_64-code.fd','-drive',f'if=pflash,format=raw,file={o}/vars.fd','-drive',f'if=ide,format=raw,file=fat:rw:{o}/esp','-device','qemu-xhci,id=x','-device','usb-kbd,bus=x.0','-drive',f'if=none,id=data,format=raw,readonly=on,file={a.image.resolve()}','-device','usb-storage,drive=data,bus=x.0','-netdev',f'socket,id=n,listen=127.0.0.1:{port}','-device','e1000e,netdev=n','-display','none','-vga','std','-debugcon',f'file:{debug}','-qmp',f'unix:{sock},server=on,wait=off','-no-reboot']
report={'passed':False,'scope':'QEMU82574 no-server; no I218 hardware evidence','efi_sha256':hashlib.sha256(a.efi.read_bytes()).hexdigest()}
with (o/'qemu.log').open('w')as log:
 proc=subprocess.Popen(cmd,stdout=log,stderr=log);peer=None
 try:
  end=time.monotonic()+65
  while peer is None:
   try:peer=socket.create_connection(('127.0.0.1',port),timeout=.2)
   except OSError:
    assert time.monotonic()<end and proc.poll()is None;time.sleep(.05)
  peer.settimeout(.1)
  while not sock.exists():assert time.monotonic()<end;time.sleep(.05)
  with socket.socket(socket.AF_UNIX)as q:
   q.connect(str(sock));f=q.makefile('rwb');f.readline()
   def call(name,args=None):
    f.write((json.dumps({'execute':name,'arguments':args or {}})+'\n').encode());f.flush()
    while True:
     x=json.loads(f.readline())
     if 'error'in x:raise RuntimeError(x)
     if 'return'in x:return
   call('qmp_capabilities');packets=0
   while True:
    text=debug.read_text()if debug.exists()else''
    if 'NET_FAILED DHCP TIMEOUT'in text and 'NET_QUIESCED DMA_DISABLED'in text:break
    assert time.monotonic()<end and proc.poll()is None,text
    try:packets+=bool(peer.recv(65536))
    except socket.timeout:pass
   assert 'NET_READY IP='not in text and 'NET_DHCP_STARTED NO_ADDRESS'in text and packets
   start=len(text);call('send-key',{'keys':[{'type':'qcode','data':'down'}],'hold-time':100})
   # The ESP fixture has an empty list: Down is dispatched but need not repaint.
   while 'APP_KEY_DOWN=0000000000000051'not in debug.read_text()[start:]:
    assert time.monotonic()<end and proc.poll()is None,debug.read_text();time.sleep(.05)
   call('screendump',{'filename':str(o/'screen.ppm')})
   report.update(passed=True,no_static_fallback=True,viewer_input_after_nic_stop=True)
   print('PASS DHCP timeout -> NIC DMA stopped; viewer input continues')
 finally:
  if peer:peer.close()
  proc.terminate();proc.wait(timeout=10);sock.unlink(missing_ok=True)
  (o/'report.json').write_text(json.dumps(report,indent=2)+'\n');(o/'command.json').write_text(json.dumps(cmd,indent=2)+'\n')
