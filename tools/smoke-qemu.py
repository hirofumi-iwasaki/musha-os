#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""Boot the staged EFI on QEMU USB storage; verify the post-exit marker."""
import argparse, hashlib, importlib.util, json, os, pathlib, shutil, socket, subprocess, time
parser=argparse.ArgumentParser()
parser.add_argument('--qemu',default='qemu-system-x86_64')
parser.add_argument('--firmware-dir',required=True)
parser.add_argument('--keyboard-usb-version',type=int,choices=[1,2],default=2)
parser.add_argument('--case',choices=['normal','ud','gp','df','pf','ro','nx','guard','xhci-timeout','xhci-command-timeout','usb-descriptor-timeout','storage-timeout','fat-corrupt'],default='normal')
parser.add_argument('--no-keyboard-input',action='store_true')
parser.add_argument('--keyboard-exit',action='store_true')
parser.add_argument('--keyboard-wrap',action='store_true')
parser.add_argument('--storage-fixture',type=int,choices=[512,4096])
parser.add_argument('--storage-high-speed',action='store_true')
parser.add_argument('--fat-fixture',choices=['mbr','superfloppy','gpt'])
parser.add_argument('--usb-image',type=pathlib.Path,help='Boot an actual GPT/FAT32 USB image read-only')
parser.add_argument('--nic', choices=['e1000e','e1000'],default='e1000e',help='e1000 exercises the unsupported Intel NIC diagnostic')
args=parser.parse_args()
if args.storage_fixture and args.fat_fixture:parser.error('Choose one fixture type')
if args.usb_image and args.fat_fixture:parser.error('Choose either a boot image or a FAT fixture; the API selects the first successful file')
if args.case=='fat-corrupt' and not args.fat_fixture:parser.error('fat-corrupt requires --fat-fixture')
if args.usb_image and args.case != 'normal':parser.error('--usb-image requires a normal diagnostic build')
root=pathlib.Path(__file__).resolve().parent.parent
out=root/'out'/('qemu-'+args.case);out.mkdir(parents=True,exist_ok=True)
firmware=pathlib.Path(args.firmware_dir)
shutil.copyfile(firmware/'edk2-i386-vars.fd',out/'vars.fd')
log=out/'debug.log';log.write_text('')
qmp_path=out/'qmp.sock'
if qmp_path.exists(): qmp_path.unlink()
cmd=[args.qemu,'-machine','q35,accel=tcg','-m','256M','-netdev','user,id=net0','-device',args.nic+',netdev=net0',
 '-drive',f'if=pflash,format=raw,readonly=on,file={firmware / "edk2-x86_64-code.fd"}',
 '-drive',f'if=pflash,format=raw,file={out / "vars.fd"}',
 '-drive',f'if=none,id=esp,format=raw,file=fat:rw:{root / "out/esp"}',
 '-device',('qemu-xhci,p3=1' if args.storage_high_speed else 'qemu-xhci'),'-device',('usb-storage,drive=esp,port=2' if args.storage_high_speed else 'usb-storage,drive=esp'),'-device',f'usb-kbd,usb_version={args.keyboard_usb_version}'+(',port=3' if args.storage_high_speed else ''),'-vga','std','-display','none',
 '-debugcon',f'file:{log}','-global','isa-debugcon.iobase=0xe9',
 '-qmp',f'unix:{qmp_path},server=on,wait=off','-no-reboot']
def sha256_file(path):
 digest=hashlib.sha256()
 with path.open('rb') as source:
  for chunk in iter(lambda:source.read(1024*1024),b''):digest.update(chunk)
 return digest.hexdigest()
image_hash=None
if args.usb_image:
 args.usb_image=args.usb_image.resolve()
 if not args.usb_image.is_file():parser.error('USB image must be a regular file')
 image_hash=sha256_file(args.usb_image)
 index=next(i for i,arg in enumerate(cmd) if arg.startswith('if=none,id=esp,'))
 cmd[index]=f'if=none,id=esp,format=raw,readonly=on,file={args.usb_image}'
fixture=None
fixture_hash=None
boot_payload=b'Hello Musha-OS!\n'
def fnv(data):
 value=0xcbf29ce484222325
 for byte in data:value=((value^byte)*0x100000001b3)&0xffffffffffffffff
 return value
if args.storage_fixture:
 fixture=out/'storage-fixture.raw'
 payload=bytes(((i*17)^(i>>8)^0xa5)&255 for i in range(4*1024*1024))
 fixture.write_bytes(payload)
 fixture_hash=hashlib.sha256(payload).hexdigest()
 size=args.storage_fixture
 fixture_markers=[
  f'MUSHA: STORAGE_CAPACITY BLOCKS={len(payload)//size:016X} SECTOR={size:016X}',
  f'MUSHA: STORAGE_READ_OK LBA={0:016X} HASH={fnv(payload[:size]):016X}',
  f'MUSHA: STORAGE_READ_OK LBA={len(payload)//size-1:016X} HASH={fnv(payload[-size:]):016X}']
 cmd.extend(['-drive',f'if=none,id=fixture,format=raw,readonly=on,file={fixture}',
  '-device',f'usb-storage,drive=fixture,logical_block_size={size},physical_block_size={size}'+(',port=4' if args.storage_high_speed else '')])
if args.fat_fixture:
 spec=importlib.util.spec_from_file_location('fat_fixture',root/'tools/make-fat32-fixture.py')
 fat=importlib.util.module_from_spec(spec);spec.loader.exec_module(fat)
 fixture=out/'fat32-fixture.raw'
 if fixture.exists():fixture.unlink()
 fat.create(fixture,args.fat_fixture,args.case=='fat-corrupt')
 fixture_hash=hashlib.sha256(fixture.read_bytes()).hexdigest()
 fat_marker=f'MUSHA: FAT32_FILE_OK BYTES={len(fat.PAYLOAD):016X} HASH={fnv(fat.PAYLOAD):016X}'
 cmd.extend(['-drive',f'if=none,id=fat_fixture,format=raw,readonly=on,file={fixture}',
  '-device','usb-storage,drive=fat_fixture'+(',port=4' if args.storage_high_speed else '')])
with (out/'qemu.log').open('w') as err:
 proc=subprocess.Popen(cmd,stdout=err,stderr=err)
 try:
  sock=None
  stream=None
  injected=False
  escape_sent=False
  repeats=0
  awaiting_repeat=False
  releases=0
  def command(name,arguments=None):
   stream.write((json.dumps({'execute':name,'arguments':arguments or {}})+'\n').encode());stream.flush()
   while True:
    message=json.loads(stream.readline())
    if 'error' in message:raise RuntimeError(message)
    if 'return' in message:return
  def connect():
   global sock,stream
   sock=socket.socket(socket.AF_UNIX);sock.settimeout(5);sock.connect(str(qmp_path))
   stream=sock.makefile('rwb');stream.readline()
   command('qmp_capabilities')
  deadline=time.monotonic()+45
  marker={'fat-corrupt':'MUSHA: XHCI_FAILED FAT32 READ','storage-timeout':'MUSHA: XHCI_FAILED TRANSFER TIMEOUT','usb-descriptor-timeout':'MUSHA: XHCI_FAILED TRANSFER TIMEOUT', 'xhci-command-timeout':'MUSHA: XHCI_FAILED COMMAND TIMEOUT', 'xhci-timeout':'MUSHA: XHCI_FAILED TIMEOUT', 'normal':'MUSHA: EXIT_BOOT_SERVICES_OK STACK_OK GOP_OK CPU_TABLES_OK PAGING_OK ARENA_OK',
   'ud':'MUSHA: EXCEPTION VECTOR=0000000000000006 ERROR=0000000000000000',
   'df':'MUSHA: EXCEPTION VECTOR=0000000000000008 ERROR=0000000000000000',
   'guard':'MUSHA: EXCEPTION VECTOR=000000000000000E ERROR=0000000000000002',
   'pf':'MUSHA: EXCEPTION VECTOR=000000000000000E ERROR=0000000000000002',
   'ro':'MUSHA: EXCEPTION VECTOR=000000000000000E ERROR=0000000000000003',
   'nx':'MUSHA: EXCEPTION VECTOR=000000000000000E ERROR=0000000000000011',
   'gp':'MUSHA: EXCEPTION VECTOR=000000000000000D ERROR=0000000000000028'}[args.case]
  while time.monotonic()<deadline:
   text=log.read_text()
   if args.case=='normal' and not args.no_keyboard_input and not injected and 'MUSHA: HID_READY\n' in text:
    if sock is None:connect()
    command('send-key',{'keys':[{'type':'qcode','data':'shift'},{'type':'qcode','data':'a'}],'hold-time':200})
    injected=True
   if args.keyboard_wrap and injected:
    current=text.count('MUSHA: APP_KEY_UP=0000000000000004')
    if current > releases:
     releases=current
     awaiting_repeat=False
    if current > 0 and not awaiting_repeat and repeats < 160:
     command('send-key',{'keys':[{'type':'qcode','data':'a'}],'hold-time':2})
     repeats+=1
     awaiting_repeat=True
   if args.keyboard_exit and injected and not escape_sent and 'MUSHA: APP_KEY_UP=00000000000000E1' in text and (not args.keyboard_wrap or (repeats==160 and not awaiting_repeat)):
    command('send-key',{'keys':[{'type':'qcode','data':'esc'}],'hold-time':50})
    escape_sent=True
   if marker in text and text.endswith('\n'):
    if args.case=='normal':
     if args.keyboard_wrap and 'MUSHA: HID_RING_WRAP_OK' not in text:raise RuntimeError('HID ring wrap missing: '+text)
     if args.keyboard_exit and 'MUSHA: APP_KEY_DOWN=0000000000000029' not in text:raise RuntimeError('App Escape exit missing: '+text)
     if 'MUSHA: HID_DIAGNOSTIC_OK REPORTS=' not in text:raise RuntimeError('HID diagnostic missing: '+text)
     if not args.no_keyboard_input:
      for key in ['00000000000000E1','0000000000000004']:
       for direction in ['DOWN','UP']:
        if 'MUSHA: HID_KEY_'+direction+'='+key not in text or 'MUSHA: APP_KEY_'+direction+'='+key not in text:raise RuntimeError('HID/app key transition missing: '+text)
     if 'MUSHA: STORAGE_PROBE_OK READ_ONLY' not in text:raise RuntimeError('Storage probe missing: '+text)
     if args.usb_image:
      for prefix in ['MUSHA: FAT32_FILE_OK','MUSHA: APP_FILE_OK']:
       if f'{prefix} BYTES={len(boot_payload):016X} HASH={fnv(boot_payload):016X}' not in text:raise RuntimeError('Boot image file read missing: '+text)
      if 'CLOSED_HANDLE_REJECTED' not in text:raise RuntimeError('Stale file handle accepted')
     if args.fat_fixture:
      if fat_marker not in text or fat_marker.replace('FAT32_FILE_OK','APP_FILE_OK') not in text:raise RuntimeError('FAT32/app file differs: '+text)
     if args.storage_fixture:
      for expected in fixture_markers:
       if expected not in text:raise RuntimeError('Storage contents differ: '+expected+'\n'+text)
     expected_count=3 if (args.storage_fixture or args.fat_fixture) else 2
     if f'MUSHA: USB_ENUMERATION_OK COUNT={expected_count:016X}' not in text:raise RuntimeError('USB enumeration failed: '+text)
     if 'VID=00000000000046F4 PID=0000000000000001' not in text or 'VID=0000000000000627 PID=0000000000000001' not in text:raise RuntimeError('Expected USB disk and keyboard missing: '+text)
     if 'MUSHA: XHCI_NOOP_OK COUNT=0000000000000258 COMMAND_WRAP_OK EVENT_WRAP_OK' not in text or 'MUSHA: XHCI_QUIESCED DMA_DISABLED' not in text:raise RuntimeError('Command ring probe failed: '+text)
     if 'MUSHA: APP_LIFECYCLE_OK STEPS=' not in text:raise RuntimeError('App lifecycle failed: '+text)
     if 'MUSHA: XHCI_RESET_OK PORTS=' not in text or 'DMA_DISABLED' not in text:raise RuntimeError('xHCI reset failed: '+text)
     if 'MUSHA: ACPI_TIMER_OK MS=' not in text:raise RuntimeError('Timer probe failed: '+text)
     if 'CLASS=00000000000C0330' not in text or ('ID=00000000'+('10D3' if args.nic=='e1000e' else '100E')+'8086 CLASS=0000000000020000') not in text:
      raise RuntimeError('Expected xHCI and Intel 82574 missing: '+text)
     if 'MUSHA: PCI_ENUMERATION_OK' not in text:raise RuntimeError('PCI enumeration incomplete')
    if args.case not in ['normal','xhci-timeout','xhci-command-timeout','usb-descriptor-timeout','storage-timeout','fat-corrupt']:
     import re
     match=re.search(r'RIP=([0-9A-F]{16}) CR2=([0-9A-F]{16})',text)
     if not match or (args.case!='nx' and int(match.group(1),16)==0):raise RuntimeError('Invalid exception frame: '+text)
     if args.case=='pf' and int(match.group(2),16)!=0:raise RuntimeError('Unexpected CR2')
     if args.case in ['ro','nx','guard'] and int(match.group(2),16)==0:raise RuntimeError('Missing fault address')
    if args.case in ['xhci-timeout','xhci-command-timeout','usb-descriptor-timeout','storage-timeout','fat-corrupt'] and 'MUSHA: EXIT_BOOT_SERVICES_OK STACK_OK GOP_OK CPU_TABLES_OK PAGING_OK ARENA_OK' not in text:
     time.sleep(0.1);continue
    if args.case in ['xhci-command-timeout','usb-descriptor-timeout','storage-timeout','fat-corrupt'] and 'MUSHA: XHCI_QUIESCED DMA_DISABLED' not in text:raise RuntimeError('DMA cleanup missing: '+text)
    if args.case=='storage-timeout' and 'MUSHA: STORAGE_CAPACITY BLOCKS=' not in text:raise RuntimeError('Read timeout was not reached')
    if args.case in ['normal','xhci-timeout','xhci-command-timeout','usb-descriptor-timeout','storage-timeout','fat-corrupt']:
     required=['MUSHA: DIAG STATE SESSION COMPLETE','MUSHA: DIAG XHCI PCI 1B36:000D']
     if args.case=='normal':
      required+=['MUSHA: DIAG USB STEP STOPPED DMA DISABLED','MUSHA: DIAG USB PORT','46F4:0001','0627:0001',
       'MUSHA: DIAG LAN PCI 8086:'+('10D3' if args.nic=='e1000e' else '100E'),
       'MUSHA: DIAG NET STEP '+('STOPPED DMA DISABLED' if args.nic=='e1000e' else 'DRIVER UNSUPPORTED / ABSENT')]
      if args.usb_image:required+=['MUSHA: DIAG FILE MUSHA.TXT BYTES 00000010','MUSHA: DIAG FILE HASH 9A42A948C590F507']
     else:
      required+=['MUSHA: DIAG FAIL AT USB STEP','MUSHA: DIAG USB ERROR']
      stages={'xhci-timeout':'HALT','xhci-command-timeout':'RINGS / COMMANDS','usb-descriptor-timeout':'DEVICE DESCRIPTOR','storage-timeout':'STORAGE SECTOR READ','fat-corrupt':'FAT32 FILE READ'}
      required+=['MUSHA: DIAG FAIL AT USB STEP '+stages[args.case]]
     for expected in required:
      if expected not in text:raise RuntimeError('Hardware diagnostic missing: '+expected+'\n'+text)

    break
   if proc.poll() is not None:raise RuntimeError((out/'qemu.log').read_text())
   time.sleep(0.001 if args.keyboard_wrap else 0.1)
  else:raise RuntimeError('Runtime marker absent: '+(out/'qemu.log').read_text())
  if sock is None:connect()
  command('screendump',{'filename':str(out/'screen.ppm')})
  command('quit');sock.close()
  proc.wait(timeout=5)
  if fixture is not None and hashlib.sha256(fixture.read_bytes()).hexdigest()!=fixture_hash:raise RuntimeError('Fixture changed')
  if args.usb_image and sha256_file(args.usb_image)!=image_hash:raise RuntimeError('Boot image changed')
  print(marker);print('Screenshot:',out/'screen.ppm')
 finally:
  if proc.poll() is None:
   proc.terminate()
   try:proc.wait(timeout=5)
   except subprocess.TimeoutExpired:proc.kill();proc.wait()
