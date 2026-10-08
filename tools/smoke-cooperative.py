#!/usr/bin/env python3
# Copyright 2026 Hirofumi Iwasaki
# SPDX-License-Identifier: Apache-2.0
"""Verify concurrent keyboard, cached file reads and actual NIC traffic."""
import argparse, pathlib, socket, struct, subprocess, time, shutil, json, hashlib
parser=argparse.ArgumentParser()
parser.add_argument('--qemu',default='qemu-system-x86_64')
parser.add_argument('--firmware-dir',required=True)
parser.add_argument('--case',choices=['traffic','input-idle','no-keyboard','keyboard-disconnect','link-down','tx-timeout','app-error'],default='traffic')
parser.add_argument('--usb-image',type=pathlib.Path)
a=parser.parse_args();root=pathlib.Path(__file__).resolve().parent.parent
out=root/('out/qemu-cooperative-'+a.case);out.mkdir(parents=True,exist_ok=True)
f=pathlib.Path(a.firmware_dir);shutil.copyfile(f/'edk2-i386-vars.fd',out/'vars.fd')
log=out/'debug.log';log.write_text('');qmp=out/'qmp.sock'
if qmp.exists():qmp.unlink()
probe=socket.socket();probe.bind(('127.0.0.1',0));port=probe.getsockname()[1];probe.close()
cmd=[a.qemu,'-machine','q35,accel=tcg','-m','256M','-netdev',f'socket,id=net0,listen=127.0.0.1:{port}','-device','e1000e,netdev=net0',
 '-drive',f'if=pflash,format=raw,readonly=on,file={f / "edk2-x86_64-code.fd"}',
 '-drive',f'if=pflash,format=raw,file={out / "vars.fd"}',
 '-drive',f'if=none,id=esp,format=raw,file=fat:rw:{root / "out/esp"}',
 '-device','qemu-xhci','-device','usb-storage,drive=esp','-device','usb-kbd,id=keyboard','-vga','std','-display','none',
 '-debugcon',f'file:{log}','-global','isa-debugcon.iobase=0xe9','-qmp',f'unix:{qmp},server=on,wait=off','-no-reboot']
if a.case=='no-keyboard':
 index=cmd.index('usb-kbd,id=keyboard');del cmd[index-1:index+1]
image_hash=None
if a.usb_image:
 a.usb_image=a.usb_image.resolve()
 if not a.usb_image.is_file():parser.error('USB image must be a regular file')
 def hash_image():
  h=hashlib.sha256()
  with a.usb_image.open('rb') as source:
   for block in iter(lambda:source.read(1024*1024),b''):h.update(block)
  return h.hexdigest()
 image_hash=hash_image()
 index=next(i for i,arg in enumerate(cmd) if arg.startswith('if=none,id=esp,'))
 cmd[index]=f'if=none,id=esp,format=raw,readonly=on,file={a.usb_image}'
def checksum(b):
 if len(b)%2:b+=b'\0'
 n=sum(struct.unpack('!'+str(len(b)//2)+'H',b))
 while n>>16:n=(n&65535)+(n>>16)
 return (~n)&65535
host=bytes.fromhex('02aabbccddee');hip=socket.inet_aton('10.0.2.2');gip=socket.inet_aton('10.0.2.15')
with (out/'qemu.log').open('w') as err:
 proc=subprocess.Popen(cmd,stdout=err,stderr=err);peer=None;qs=None
 try:
  deadline=time.monotonic()+60
  while time.monotonic()<deadline:
   try:peer=socket.create_connection(('127.0.0.1',port),timeout=1);break
   except OSError:time.sleep(.05)
  if peer is None:raise RuntimeError('network socket unavailable')
  peer.settimeout(2)
  while 'NET_READY' not in log.read_text():
   if time.monotonic()>deadline or proc.poll() is not None:raise RuntimeError(log.read_text())
   time.sleep(.05)
  qs=socket.socket(socket.AF_UNIX);qs.connect(str(qmp));stream=qs.makefile('rwb');stream.readline()
  def control(name,args=None):
   stream.write((json.dumps({'execute':name,'arguments':args or {}})+'\n').encode());stream.flush()
   while True:
    response=json.loads(stream.readline())
    if 'error' in response:raise RuntimeError(response)
    if 'return' in response:return
  control('qmp_capabilities')
  def wait_marker(marker,limit=10):
   end=time.monotonic()+limit
   while marker not in log.read_text():
    if time.monotonic()>end or proc.poll() is not None:raise RuntimeError('Missing '+marker+'\n'+log.read_text())
    time.sleep(.02)
  wait_marker('RUNTIME_POLL_READY')
  if a.case=='keyboard-disconnect':
   control('device_del',{'id':'keyboard'});wait_marker('XHCI_FAILED KEYBOARD DISCONNECTED')
  if a.case in ['link-down','tx-timeout']:
   if a.case=='link-down':control('set_link',{'name':'net0','up':False})
   reason='LINK DOWN' if a.case=='link-down' else 'TX TIMEOUT'
   wait_marker('NET_FAILED '+reason);wait_marker('NET_QUIESCED DMA_DISABLED')
   control('send-key',{'keys':[{'type':'qcode','data':'a'}],'hold-time':50})
   wait_marker('APP_KEY_DOWN=0000000000000004');wait_marker('APP_KEY_UP=0000000000000004')
   control('send-key',{'keys':[{'type':'qcode','data':'esc'}],'hold-time':50})
   wait_marker('EXIT_BOOT_SERVICES_OK')
   text=log.read_text();assert 'XHCI_QUIESCED DMA_DISABLED' in text and 'RUNTIME_COOPERATIVE_OK' in text
   assert text.index('NET_QUIESCED')<text.index('APP_KEY_DOWN=0000000000000004')
   if image_hash:assert hash_image()==image_hash and 'APP_FILE_OK' in text
   control('quit');proc.wait(timeout=5)
   print('PASS:',a.case,'isolated; input/file progress and both DMA stop')
   raise SystemExit(0)
  def send(frame):
   frame=frame.ljust(60,b'\0');peer.sendall(struct.pack('!I',len(frame))+frame)
  def exact(n):
   b=b''
   while len(b)<n:
    part=peer.recv(n-len(b))
    if not part:raise RuntimeError('network disconnected')
    b+=part
   return b
  def receive(match,timeout=2):
   end=time.monotonic()+timeout
   while time.monotonic()<end:
    peer.settimeout(max(.01,end-time.monotonic()))
    size=struct.unpack('!I',exact(4))[0]
    if not 14<=size<=65536:raise RuntimeError('invalid QEMU frame length')
    frame=exact(size)
    if match(frame):return frame
   raise TimeoutError('expected frame missing')
  arp=struct.pack('!HHBBH',1,0x800,6,4,1)+host+hip+bytes(6)+gip
  send(bytes.fromhex('ffffffffffff')+host+b'\x08\x06'+arp)
  reply=receive(lambda b:b[12:14]==b'\x08\x06' and b[20:22]==b'\0\2')
  guest=reply[6:12]
  assert reply[:6]==host and reply[22:28]==guest and reply[28:32]==gip and reply[32:38]==host and reply[38:42]==hip
  def ip(proto,payload,ident):
   header=struct.pack('!BBHHHBBH4s4s',0x45,0,len(payload)+20,ident,0,64,proto,0,hip,gip)
   header=header[:10]+struct.pack('!H',checksum(header))+header[12:]
   return guest+host+b'\x08\x00'+header+payload
  def parsed(frame,proto):
   if frame[:6]!=host or frame[6:12]!=guest or frame[12:14]!=b'\x08\x00' or len(frame)<34:return None
   h=frame[14:];hl=(h[0]&15)*4;total=struct.unpack('!H',h[2:4])[0]
   if h[9]!=proto:return None
   assert hl>=20 and total>=hl and total<=len(h) and checksum(h[:hl])==0
   assert h[12:16]==gip and h[16:20]==hip
   return h[hl:total]
  payload=b'Musha-OS ICMP test'
  icmp=struct.pack('!BBHHH',8,0,0,0x1234,1)+payload
  icmp=icmp[:2]+struct.pack('!H',checksum(icmp))+icmp[4:]
  send(ip(1,icmp,1));frame=receive(lambda b:parsed(b,1) is not None);answer=parsed(frame,1)
  assert answer[0:2]==b'\0\0' and answer[4:]==icmp[4:] and checksum(answer)==0
  def udp(payload):
   b=struct.pack('!HHHH',23456,12345,len(payload)+8,0)+payload
   pseudo=hip+gip+struct.pack('!BBH',0,17,len(b))
   return b[:6]+struct.pack('!H',checksum(pseudo+b) or 65535)+b[8:]
  for i in range(256 if a.case != 'app-error' else 1):
   payload=struct.pack('!I',i)+b'Musha-OS UDP ring wrap'
   if a.case=='traffic' and i in [16,64,128]:
    key={16:'a',64:'b',128:'c'}[i]
    control('send-key',{'keys':[{'type':'qcode','data':key}],'hold-time':30})
   request=udp(payload);send(ip(17,request,i+2))
   if a.case=='app-error':
    wait_marker('APP_FAILED');text=log.read_text()
    assert 'XHCI_QUIESCED DMA_DISABLED' in text and 'NET_QUIESCED DMA_DISABLED' in text
    assert text.index('XHCI_QUIESCED')<text.index('APP_FAILED') and text.index('NET_QUIESCED')<text.index('APP_FAILED')
    assert 'APP_LIFECYCLE_OK' not in text
    control('quit');proc.wait(timeout=5)
    print('PASS: application error stops both DMA controllers before halt')
    raise SystemExit(0)
   answer=parsed(receive(lambda b:parsed(b,17) is not None),17)
   assert struct.unpack('!HHH',answer[:6])==(12345,23456,len(payload)+8) and answer[8:]==payload
   assert checksum(gip+hip+struct.pack('!BBH',0,17,len(answer))+answer)==0 and answer[6:8]!=b'\0\0'
  # Corrupt software-validated checksums, then prove valid traffic still progresses.
  bad=bytearray(ip(17,udp(b'bad-ip'),200));bad[24]^=1;send(bad)
  bad=bytearray(udp(b'bad-udp'));bad[6]^=1;send(ip(17,bad,201))
  try:receive(lambda b:parsed(b,17) is not None,.3);raise AssertionError('invalid checksum echoed')
  except (socket.timeout,TimeoutError):pass
  payload=b'valid-after-errors';send(ip(17,udp(payload),202))
  answer=parsed(receive(lambda b:parsed(b,17) is not None),17);assert answer[8:]==payload
  if a.case=='traffic':
   wait_marker('APP_KEY_UP=0000000000000006')
   wait_marker('APP_FILE_RECHECK_OK')
   control('send-key',{'keys':[{'type':'qcode','data':'esc'}],'hold-time':50})
  while 'EXIT_BOOT_SERVICES_OK' not in log.read_text():
   if time.monotonic()>deadline:raise RuntimeError('network termination timeout')
   time.sleep(.05)
  text=log.read_text();assert 'NET_FAILED' not in text and 'UDP=0000000000000101' in text
  import re
  assert int(re.search(r'RX_WRAP=([0-9A-F]+)',text)[1],16)>=16
  assert int(re.search(r'TX_WRAP=([0-9A-F]+)',text)[1],16)>=32
  assert 'RUNTIME_COOPERATIVE_OK' in text and 'XHCI_QUIESCED DMA_DISABLED' in text
  if image_hash:assert hash_image()==image_hash and 'APP_FILE_RECHECK_OK' in text
  if a.case=='traffic':
   for usage in [4,5,6,0x29]:assert f'APP_KEY_DOWN={usage:016X}' in text
   assert text.index('NET_READY')<text.index('APP_KEY_DOWN=0000000000000004')<text.index('NET_QUIESCED')
  if a.case in ['input-idle','no-keyboard']:assert 'APP_KEY_DOWN' not in text
  control('screendump',{'filename':str(out/'screen.ppm')});control('quit')
  proc.wait(timeout=5)
  if image_hash:assert hash_image()==image_hash and 'APP_FILE_OK' in text
  print('PASS:',a.case,'ARP/ICMP, 257 UDP echoes, file rechecks, ring wraps, checksums, DMA stop')
 finally:
  if peer:peer.close()
  if qs:qs.close()
  if proc.poll() is None:proc.terminate();proc.wait(timeout=5)
