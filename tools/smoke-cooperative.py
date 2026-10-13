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
parser.add_argument('--exfat-boot-image',type=pathlib.Path,help='Boot EFI from this FAT SATA disk; --usb-image is exFAT data')
parser.add_argument('--live-storage',action='store_true',help='Require live-storage-probe image and real READ(10) during traffic')
parser.add_argument('--live-file',action='store_true',help='Require common API -> filesystem -> USB file read')
parser.add_argument('--file-no-delay',action='store_true',help='App probe over production transport without artificial waits')
parser.add_argument('--file-close',action='store_true',help='App closes a pending request and reopens after acknowledgement')
parser.add_argument('--file-fault',choices=['disconnect','timeout'])
parser.add_argument('--live-file-cancel',action='store_true',help='Exit while a file sector request is pending')
parser.add_argument('--live-fault',choices=['timeout','disconnect'],help='Exercise a pending live read failure')
parser.add_argument('--hub-depth',type=int,choices=range(1,6),help='Place a USB1 keyboard under USB2 hubs')
parser.add_argument('--split-controller',action='store_true',help='Place the keyboard on a second xHCI controller')
parser.add_argument('--extra-xhci',action='store_true',help='Verify no-keyboard behavior with two controllers')
parser.add_argument('--output',type=pathlib.Path)
a=parser.parse_args();root=pathlib.Path(__file__).resolve().parent.parent
out=a.output.resolve() if a.output else root/('out/qemu-cooperative-'+a.case+('-split' if a.split_controller else '')+('-exfat' if a.exfat_boot_image else '')+('-'+a.file_fault if a.file_fault else '')+('-normal' if a.file_no_delay else '')+('-close' if a.file_close else '')+('-file-cancel' if a.live_file_cancel else '-file' if a.live_file else '')+('-live-'+(a.live_fault or 'read') if a.live_storage else ''));out.mkdir(parents=True,exist_ok=True)
f=pathlib.Path(a.firmware_dir);shutil.copyfile(f/'edk2-i386-vars.fd',out/'vars.fd')
log=out/'debug.log';log.write_text('');qmp=root/'out'/('qmp-'+hashlib.sha256(str(out).encode()).hexdigest()[:12]+'.sock')
if qmp.exists():qmp.unlink()
probe=socket.socket();probe.bind(('127.0.0.1',0));port=probe.getsockname()[1];probe.close()
cmd=[a.qemu,'-machine','q35,accel=tcg','-m','256M','-netdev',f'socket,id=net0,listen=127.0.0.1:{port}','-device','e1000e,netdev=net0',
 '-drive',f'if=pflash,format=raw,readonly=on,file={f / "edk2-x86_64-code.fd"}',
 '-drive',f'if=pflash,format=raw,file={out / "vars.fd"}',
 '-drive',f'if=none,id=esp,format=raw,file=fat:rw:{root / "out/esp"}',
 '-device','qemu-xhci,id=usbhost','-device','usb-storage,id=storage,drive=esp,bus=usbhost.0,port=1','-device','usb-kbd,id=keyboard,bus=usbhost.0','-vga','std','-display','none',
 '-debugcon',f'file:{log}','-global','isa-debugcon.iobase=0xe9','-qmp',f'unix:{qmp},server=on,wait=off','-no-reboot']
if a.case=='no-keyboard':
 index=cmd.index('usb-kbd,id=keyboard,bus=usbhost.0');del cmd[index-1:index+1]
if a.hub_depth:
 if a.case=='no-keyboard':parser.error('Hub traffic test requires keyboard')
 index=cmd.index('usb-kbd,id=keyboard,bus=usbhost.0')
 cmd[index]+= ',usb_version=1,port=3'+'.1'*a.hub_depth
 hubs=[]
 for depth in range(a.hub_depth):hubs.extend(['-device','usb-hub,ports=4,port-power=on,bus=usbhost.0,port=3'+'.1'*depth])
 index=cmd.index('qemu-xhci,id=usbhost')
 cmd[index+1:index+1]=hubs
if a.split_controller:
 if a.hub_depth or a.case=='no-keyboard':parser.error('Split-controller case requires a direct keyboard')
 index=cmd.index('usb-kbd,id=keyboard,bus=usbhost.0')
 cmd[index]='usb-kbd,id=keyboard,bus=inputhost.0'
 index=cmd.index('-device',cmd.index('qemu-xhci,id=usbhost')+1)
 cmd[index:index]=['-device','qemu-xhci,id=inputhost']
if a.extra_xhci:cmd.extend(['-device','qemu-xhci,id=extra'])
if a.live_storage and (a.case!='traffic' or not a.usb_image):parser.error('--live-storage requires traffic and --usb-image')
if a.live_fault and not a.live_storage:parser.error('--live-fault requires --live-storage')
if a.live_file and (a.case!='traffic' or not a.usb_image or a.live_storage):parser.error('--live-file requires traffic, --usb-image, and no --live-storage')
if (a.file_no_delay or a.file_fault or a.file_close) and not a.live_file:parser.error('file options require --live-file')
if a.file_no_delay and (a.live_file_cancel or a.file_fault or a.file_close):parser.error('file fault/cancel tests require delayed transport')
if a.live_file_cancel and not a.live_file:parser.error('--live-file-cancel requires --live-file')
if a.exfat_boot_image and not a.live_file:parser.error('--exfat-boot-image requires --live-file')
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
if a.exfat_boot_image:
 a.exfat_boot_image=a.exfat_boot_image.resolve()
 if not a.exfat_boot_image.is_file():parser.error('Boot image must be a regular file')
 boot_hash=hashlib.sha256(a.exfat_boot_image.read_bytes()).hexdigest()
 cmd.extend(['-drive',f'if=ide,format=raw,snapshot=on,file={a.exfat_boot_image}','-boot','order=c'])
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
  if a.live_file:
   wait_marker('LIVE_FILE_API_READY')
   if not a.file_no_delay:wait_marker('LIVE_STORAGE_WAIT')
   if a.exfat_boot_image:wait_marker('STREAM_MOUNT_EXFAT')
   if a.split_controller:wait_marker('CROSS_CONTROLLER_LIVE')
   control('send-key',{'keys':[{'type':'qcode','data':'a'}],'hold-time':30})
   wait_marker('APP_KEY_DOWN=0000000000000004')
   if a.file_close:
    wait_marker('APP_FILE_CLOSE_QUARANTINED');wait_marker('APP_FILE_REOPENED')
   if a.file_fault:
    if a.file_fault=='disconnect':control('device_del',{'id':'storage'})
    wait_marker('XHCI_FAILED',15);wait_marker('INPUT_SOURCE_DETACHED',15)
    text=log.read_text()
    assert 'LIVE_FILE_ENTRY_OK' not in text and 'LIVE_FILE_API_OK' not in text
    assert 'STORAGE DISCONNECTED' in text if a.file_fault=='disconnect' else 'TIMEOUT' in text
    assert text.count('XHCI_QUIESCED DMA_DISABLED') >= (4 if a.split_controller else 1)
    assert text.index('APP_KEY_DOWN=0000000000000004') < text.index('INPUT_SOURCE_DETACHED')
    assert hash_image()==image_hash
    if a.exfat_boot_image:assert hashlib.sha256(a.exfat_boot_image.read_bytes()).hexdigest()==boot_hash
    control('quit');proc.wait(timeout=5)
    print('PASS: pending file '+a.file_fault+', no result publication, all participating controllers quiesced')
    raise SystemExit(0)
   if a.live_file_cancel:
    control('send-key',{'keys':[{'type':'qcode','data':'esc'}],'hold-time':30})
    wait_marker('EXIT_BOOT_SERVICES_OK')
    text=log.read_text()
    assert 'LIVE_FILE_API_OK' not in text and 'XHCI_FAILED' not in text
    # CI scheduling may advance more than one sector before Esc arrives.
    # Require an actual in-flight request at cancellation, not a sector count.
    assert text.index('APP_KEY_DOWN=0000000000000029') < text.index('LIVE_FILE_CANCEL_PENDING_SECTOR') < text.index('STORAGE_RELEASED')
    assert text.index('LIVE_STORAGE_WAIT') < text.index('APP_KEY_DOWN=0000000000000029') < text.index('STORAGE_RELEASED') < text.rindex('XHCI_QUIESCED')
    assert hash_image()==image_hash
    if a.exfat_boot_image:assert hashlib.sha256(a.exfat_boot_image.read_bytes()).hexdigest()==boot_hash
    control('quit');proc.wait(timeout=5)
    print('PASS: common file API canceled during a pending sector read; slot released, DMA stopped, no file result published')
    raise SystemExit(0)
  if a.live_storage:
   wait_marker('LIVE_STORAGE_WAIT')
   control('send-key',{'keys':[{'type':'qcode','data':'a'}],'hold-time':30})
  if a.extra_xhci:
   text=log.read_text();assert text.count('MUSHA: XHCI_SCAN BDF=')==2
   assert 'MUSHA: XHCI_SCAN_COMPLETE' in text
   assert text.count('MUSHA: RUNTIME_POLL_READY')==1
  if a.case=='app-error':
   control('input-send-event',{'events':[{'type':'key','data':{'down':True,'key':{'type':'qcode','data':'a'}}}]})
   wait_marker('APP_KEY_DOWN=0000000000000004')
  if a.case=='keyboard-disconnect':
   # Hold Shift+A with no scheduled key-up before physically removing the device.
   control('input-send-event',{'events':[
    {'type':'key','data':{'down':True,'key':{'type':'qcode','data':'shift'}}},
    {'type':'key','data':{'down':True,'key':{'type':'qcode','data':'a'}}}]})
   wait_marker('APP_KEY_DOWN=00000000000000E1')
   wait_marker('APP_KEY_DOWN=0000000000000004')
   control('device_del',{'id':'keyboard'});wait_marker('XHCI_FAILED KEYBOARD DISCONNECTED')
   wait_marker('INPUT_SOURCE_DETACHED')
   for usage in ['00000000000000E1','0000000000000004']:
    wait_marker('APP_KEY_UP='+usage)
    text=log.read_text()
    assert text.count('MUSHA: INPUT_DETACH_KEY_UP='+usage)==1
    assert text.count('MUSHA: APP_KEY_UP='+usage)==1
  if a.case in ['link-down','tx-timeout']:
   if a.case=='link-down':control('set_link',{'name':'net0','up':False})
   reason='LINK DOWN' if a.case=='link-down' else 'TX TIMEOUT'
   wait_marker('NET_FAILED '+reason);wait_marker('NET_QUIESCED DMA_DISABLED')
   if image_hash:wait_marker('APP_FILE_RECHECK_OK')
   control('send-key',{'keys':[{'type':'qcode','data':'a'}],'hold-time':50})
   wait_marker('APP_KEY_DOWN=0000000000000004');wait_marker('APP_KEY_UP=0000000000000004')
   control('send-key',{'keys':[{'type':'qcode','data':'esc'}],'hold-time':50})
   wait_marker('EXIT_BOOT_SERVICES_OK')
   text=log.read_text();assert 'XHCI_QUIESCED DMA_DISABLED' in text and 'RUNTIME_COOPERATIVE_OK' in text
   assert text.index('NET_QUIESCED')<text.index('APP_KEY_DOWN=0000000000000004')
   if image_hash:
    assert hash_image()==image_hash and 'APP_FILE_OK' in text
    assert text.index('NET_QUIESCED')<text.index('APP_FILE_RECHECK_OK')
   control('quit');proc.wait(timeout=5)
   assert 'MUSHA: DIAG FAIL AT NET STEP POLLING' in text
   assert 'MUSHA: DIAG NET ERROR '+reason in text
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
   if a.case=='traffic' and not a.live_fault and i in [16,64,128]:
    key={16:'a',64:'b',128:'c'}[i]
    control('send-key',{'keys':[{'type':'qcode','data':key}],'hold-time':30})
   request=udp(payload);send(ip(17,request,i+2))
   if a.case=='app-error':
    wait_marker('APP_FAILED');text=log.read_text()
    assert 'XHCI_QUIESCED DMA_DISABLED' in text and 'NET_QUIESCED DMA_DISABLED' in text
    assert text.index('XHCI_QUIESCED')<text.index('APP_FAILED') and text.index('NET_QUIESCED')<text.index('APP_FAILED')
    assert 'APP_LIFECYCLE_OK' not in text
    assert text.count('MUSHA: INPUT_DETACH_KEY_UP=0000000000000004')==1
    assert text.index('INPUT_SOURCE_DETACHED')<text.index('APP_FAILED')
    control('quit');proc.wait(timeout=5)
    print('PASS: application error stops both DMA controllers before halt')
    raise SystemExit(0)
   answer=parsed(receive(lambda b:parsed(b,17) is not None),17)
   assert struct.unpack('!HHH',answer[:6])==(12345,23456,len(payload)+8) and answer[8:]==payload
   assert checksum(gip+hip+struct.pack('!BBH',0,17,len(answer))+answer)==0 and answer[6:8]!=b'\0\0'
   if a.live_file and not a.file_no_delay and i==0:
    assert 'LIVE_FILE_API_OK' not in log.read_text(), 'file finished before UDP response'
   if a.live_storage and i==0:
    # This is a real UDP reply received by the test process before the held
    # READ(10) completes or fails, independently of application snapshot logs.
    pending_log=log.read_text()
    assert 'LIVE_STORAGE_READ_OK' not in pending_log and 'XHCI_FAILED' not in pending_log
   if a.live_fault=='disconnect' and i==0:
    wait_marker('APP_KEY_UP=0000000000000004')
    control('device_del',{'id':'storage'})
    wait_marker('XHCI_FAILED STORAGE DISCONNECTED')

  # Corrupt software-validated checksums, then prove valid traffic still progresses.
  bad=bytearray(ip(17,udp(b'bad-ip'),200));bad[24]^=1;send(bad)
  bad=bytearray(udp(b'bad-udp'));bad[6]^=1;send(ip(17,bad,201))
  try:receive(lambda b:parsed(b,17) is not None,.3);raise AssertionError('invalid checksum echoed')
  except (socket.timeout,TimeoutError):pass
  payload=b'valid-after-errors';send(ip(17,udp(payload),202))
  answer=parsed(receive(lambda b:parsed(b,17) is not None),17);assert answer[8:]==payload
  if a.case=='traffic' and not a.live_fault:
   wait_marker('APP_KEY_UP=0000000000000006')
   wait_marker('APP_FILE_RECHECK_OK')
   if a.live_file:wait_marker('LIVE_FILE_API_OK')
   if a.live_storage:
    wait_marker('LIVE_STORAGE_READ_OK')
    # Exit during a subsequent outstanding DATA transfer, not just between reads.
    seen=log.read_text().count('LIVE_STORAGE_WAIT')
    until=time.monotonic()+3
    while log.read_text().count('LIVE_STORAGE_WAIT')==seen:
     if time.monotonic()>until:raise RuntimeError('next live read did not start')
     time.sleep(.01)
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
  if a.case=='traffic' and not a.live_fault:
   for usage in [4,5,6,0x29]:assert f'APP_KEY_DOWN={usage:016X}' in text
   assert text.index('NET_READY')<text.index('APP_KEY_DOWN=0000000000000004')<text.index('NET_QUIESCED')
  if a.case in ['input-idle','no-keyboard']:assert 'APP_KEY_DOWN' not in text
  if a.live_storage:
   begin=text.index('LIVE_STORAGE_WAIT')
   marker=('XHCI_FAILED '+('TRANSFER TIMEOUT' if a.live_fault=='timeout' else 'STORAGE DISCONNECTED')) if a.live_fault else 'LIVE_STORAGE_READ_OK'
   done=text.index(marker,begin)
   window=text[begin:done]
   assert 'APP_KEY_DOWN=0000000000000004' in window
   assert 'APP_FILE_RECHECK_OK' in window, 'UDP-triggered app progress missing during real storage wait'
   if a.live_fault:
    assert 'LIVE_STORAGE_READ_OK' not in text
    assert 'STORAGE_RELEASED' not in text
    assert text.index('XHCI_QUIESCED') < text.index('INPUT_SOURCE_DETACHED') < text.index('NET_QUIESCED')
    print('PASS: live read',a.live_fault,'input/UDP progress before failure, no result publication, controller DMA stopped')
   else:
    assert 'STORAGE_RELEASED' in text and 'XHCI_FAILED' not in text
    assert text.rindex('LIVE_STORAGE_WAIT') < text.index('APP_KEY_DOWN=0000000000000029') < text.index('STORAGE_RELEASED') < text.rindex('XHCI_QUIESCED')
    print('PASS: live READ(10) data hash, key/UDP progress while pending, Esc during DATA, slot release and DMA stop')
  if a.live_file:
   expected=b'Hello Musha-OS!\n';h=0xcbf29ce484222325
   for byte in expected:h=((h^byte)*0x100000001b3)&0xffffffffffffffff
   assert f'LIVE_FILE_API_OK BYTES={len(expected):016X} HASH={h:016X}' in text
   begin=text.index('LIVE_FILE_REQUEST');done=text.index('LIVE_FILE_API_OK')
   payloads=[('MUSHA.TXT',expected)]
   if a.exfat_boot_image:
    payloads += [('FRAGMENT.BIN',bytes(range(256))*5+b'end'),('Long name for exFAT test.txt',b'Long file name\n'),('EMPTY.TXT',b''),('ZEROTAIL.BIN',b'valid'+b'\0'*695)]
   requests=0
   for index,(name,payload) in enumerate(payloads):
    digest=0xcbf29ce484222325
    for byte in payload:digest=((digest^byte)*0x100000001b3)&0xffffffffffffffff
    assert f'LIVE_FILE_ENTRY_OK INDEX={index:016X} BYTES={len(payload):016X} HASH={digest:016X}' in text
    assert 'LIVE_FILE_NAME '+''.join(f'{ord(c):016X}' for c in name) in text
    chunk=7 if index==0 else 512
    requests+=(len(payload)+chunk-1)//chunk
   assert text.count('LIVE_FILE_ENTRY_OK')==len(payloads)
   assert text[begin:done].count('LIVE_FILE_REQUEST') == requests + int(a.file_close), 'missing split requests'
   if a.file_close:
    assert text.index('LIVE_STORAGE_WAIT') < text.index('APP_FILE_CLOSE_QUARANTINED') < text.index('APP_FILE_REOPENED') < done
    assert text[text.index('APP_FILE_CLOSE_QUARANTINED'):text.index('APP_FILE_REOPENED')].count('LIVE_STORAGE_WAIT') == 0
   if not a.file_no_delay:
    assert 'APP_KEY_DOWN=0000000000000004' in text[begin:done]
    assert 'APP_FILE_RECHECK_OK' in text[begin:done]
   assert 'XHCI_FAILED' not in text and 'STORAGE_RELEASED' in text
   if a.split_controller:assert text.count('XHCI_QUIESCED DMA_DISABLED') >= 4
   print('PASS: common file API, metadata/data USB reads, exact content/EOF, concurrent input and UDP')
  control('screendump',{'filename':str(out/'screen.ppm')});control('quit')
  proc.wait(timeout=5)
  if image_hash:assert hash_image()==image_hash and 'APP_FILE_OK' in text
  if a.exfat_boot_image:assert hashlib.sha256(a.exfat_boot_image.read_bytes()).hexdigest()==boot_hash
  print('PASS:',a.case,'ARP/ICMP, 257 UDP echoes, file rechecks, ring wraps, checksums, DMA stop')
 finally:
  if peer:peer.close()
  if qs:qs.close()
  if proc.poll() is None:proc.terminate();proc.wait(timeout=5)
