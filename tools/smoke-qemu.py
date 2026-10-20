#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""Boot the staged EFI on QEMU USB storage; verify the post-exit marker."""
import sys
import argparse, hashlib, importlib.util, json, os, pathlib, shutil, socket, subprocess, time
from smoke_qemu_checks import check_hub_enumerations
parser=argparse.ArgumentParser()
parser.add_argument('--efi',type=pathlib.Path,help='Stage this EFI in the private output ESP; avoids shared-root build races')
parser.add_argument('--output',type=pathlib.Path,help='Directory for local smoke evidence')
parser.add_argument('--qemu',default='qemu-system-x86_64')
parser.add_argument('--firmware-dir',required=True)
parser.add_argument('--keyboard-usb-version',type=int,choices=[1,2],default=2)
parser.add_argument('--case',choices=['normal','ud','gp','df','pf','ro','nx','guard','overflow','xhci-timeout','xhci-command-timeout','usb-descriptor-timeout','control-out-timeout','control-out-stall','storage-timeout','fat-corrupt'],default='normal')
parser.add_argument('--no-keyboard-input',action='store_true')
parser.add_argument('--expect-t2-diagnostics',action='store_true',help='Require current read-only T2-D1 and phase markers on QEMU (no BCE device)')
parser.add_argument('--keyboard-exit',action='store_true')
parser.add_argument('--keyboard-wrap',action='store_true')
parser.add_argument('--storage-fixture',type=int,choices=[512,4096])
parser.add_argument('--storage-high-speed',action='store_true')
parser.add_argument('--fat-fixture',choices=['mbr','superfloppy','gpt'])
parser.add_argument('--usb-image',type=pathlib.Path,help='Boot an actual GPT/FAT32 USB image read-only')
parser.add_argument('--nic', choices=['e1000e','e1000'],default='e1000e',help='e1000 exercises the unsupported Intel NIC diagnostic')
parser.add_argument('--extra-xhci',action='store_true',help='Add a second controller to verify sequential discovery')
parser.add_argument('--control-ring-probe',action='store_true',help='Require the opt-in EP0 wrap/short-response probe markers')
parser.add_argument('--hub-disconnect',action='store_true',help='Remove a hub-child keyboard during idle input and verify cleanup')
parser.add_argument('--hub-depth',type=int,choices=range(1,6),help='Put keyboard and fixture behind this many USB2 hubs; use keyboard USB version 1')
parser.add_argument('--usb-hub',action='store_true',help='Add a root-port hub to verify unsupported-hub diagnostics')
parser.add_argument('--keyboard-second',action='store_true',help='Connect keyboard to the second controller; requires --extra-xhci')
parser.add_argument('--devices-second',action='store_true',help='Leave the first controller empty; requires --extra-xhci')
args=parser.parse_args()
if args.expect_t2_diagnostics and args.case != 'normal':parser.error('T2 evidence checks require the normal case')
if (args.keyboard_second or args.devices_second) and not args.extra_xhci:parser.error('Second-controller device placement requires --extra-xhci')
if args.hub_disconnect and (not args.hub_depth or args.keyboard_exit or args.keyboard_wrap):parser.error('Hub disconnect requires a hub topology without key injection')
if args.hub_depth and (args.keyboard_usb_version!=1 or args.extra_xhci or args.usb_hub or args.storage_high_speed):parser.error('Hub topology requires USB1 keyboard and no conflicting placement options')
if args.storage_fixture and args.fat_fixture:parser.error('Choose one fixture type')
if args.usb_image and args.fat_fixture:parser.error('Choose either a boot image or a FAT fixture; the API selects the first successful file')
if args.case=='fat-corrupt' and not args.fat_fixture:parser.error('fat-corrupt requires --fat-fixture')
if args.usb_image and args.case=='fat-corrupt':parser.error('fat-corrupt uses its deliberately invalid FAT fixture')
root=pathlib.Path(__file__).resolve().parent.parent
out=args.output.resolve() if args.output else root/'out'/('qemu-'+args.case);out.mkdir(parents=True,exist_ok=True)
firmware=pathlib.Path(args.firmware_dir)
shutil.copyfile(firmware/'edk2-i386-vars.fd',out/'vars.fd')
esp=root/'out/esp'
if args.efi:
 esp=out/'esp';(esp/'EFI/BOOT').mkdir(parents=True,exist_ok=True);shutil.copyfile(args.efi,esp/'EFI/BOOT/BOOTX64.EFI')
log=out/'debug.log';log.write_text('')
qmp_path=root/'out'/('sq-'+hashlib.sha256(str(out).encode()).hexdigest()[:12]+'.sock')
if qmp_path.exists(): qmp_path.unlink()
cmd=[args.qemu,'-machine','q35,accel=tcg','-m','256M','-netdev','user,id=net0','-device',args.nic+',netdev=net0',
 '-drive',f'if=pflash,format=raw,readonly=on,file={firmware / "edk2-x86_64-code.fd"}',
 '-drive',f'if=pflash,format=raw,file={out / "vars.fd"}',
 '-drive',f'if=none,id=esp,format=raw,file=fat:rw:{esp}',
 '-device',('qemu-xhci,id=usbhost,p3=1' if args.storage_high_speed else 'qemu-xhci,id=usbhost'),'-device',('usb-storage,drive=esp,bus=usbhost.0,port=2' if args.storage_high_speed else 'usb-storage,drive=esp,bus=usbhost.0'),'-device',f'usb-kbd,id=testkbd,bus=usbhost.0,usb_version={args.keyboard_usb_version}'+(',port=3' if args.storage_high_speed else ''),'-vga','std','-display','none',
 '-debugcon',f'file:{log}','-global','isa-debugcon.iobase=0xe9',
 '-qmp',f'unix:{qmp_path},server=on,wait=off','-no-reboot']
if args.extra_xhci:
 index=next(i for i,x in enumerate(cmd) if x.startswith('qemu-xhci,'))
 cmd[index+1:index+1]=['-device','qemu-xhci,id=unused']
if args.usb_hub:cmd.extend(['-device','usb-hub,ports=4,bus=usbhost.0,port=3'])
if args.keyboard_second or args.devices_second:
 index=next(i for i,x in enumerate(cmd) if x.startswith('usb-kbd,'))
 cmd[index]=cmd[index].replace('bus=usbhost.0','bus=unused.0')
if args.devices_second:
 index=next(i for i,x in enumerate(cmd) if x.startswith('usb-storage,drive=esp'))
 cmd[index]=cmd[index].replace('bus=usbhost.0','bus=unused.0')
if args.hub_depth:
 index=next(i for i,x in enumerate(cmd) if x.startswith('usb-kbd,'))
 cmd[index]+= ',port=3'+'.1'*args.hub_depth
 index=next(i for i,x in enumerate(cmd) if x.startswith('usb-storage,drive=esp'))
 cmd[index]+= ',port=1'
 hubs=[]
 for depth in range(args.hub_depth):
  hubs.extend(['-device','usb-hub,ports=4,bus=usbhost.0,port=3'+'.1'*depth+',port-power=on'])
 index=next(i for i,x in enumerate(cmd) if x.startswith('qemu-xhci,'))
 cmd[index+1:index+1]=hubs

def sha256_file(path):
 digest=hashlib.sha256()
 with path.open('rb') as source:
  for chunk in iter(lambda:source.read(1024*1024),b''):digest.update(chunk)
 return digest.hexdigest()
# v0.4.0 requires a readable boot volume. vvfat is not a FAT32 fixture;
# use the same deterministic generator as deployment tests, never a real device.
if not args.usb_image and not args.fat_fixture:
 generated=out/'boot-media.img'
 if generated.exists(): raise RuntimeError('Refusing to replace an existing fixture')
 source_efi=args.efi.resolve() if args.efi else esp/'EFI/BOOT/BOOTX64.EFI'
 subprocess.run([sys.executable,str(root/'tools/make-usb-image.py'),str(generated),'--efi',str(source_efi)],check=True,stdout=subprocess.DEVNULL)
 args.usb_image=generated
image_hash=None
if args.usb_image:
 args.usb_image=args.usb_image.resolve()
 if not args.usb_image.is_file():parser.error('USB image must be a regular file')
 image_hash=sha256_file(args.usb_image)
 index=next(i for i,arg in enumerate(cmd) if arg.startswith('if=none,id=esp,'))
 cmd[index]=f'if=none,id=esp,format=raw,readonly=on,file={args.usb_image}'
fixture=None
fixture_hash=None
from distribution import usb_readme
boot_payload=usb_readme((root/'VERSION').read_text().strip())
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
  '-device',f'usb-storage,drive=fixture,bus=usbhost.0,logical_block_size={size},physical_block_size={size}'+(',port=4' if args.storage_high_speed else '')])
if args.fat_fixture:
 spec=importlib.util.spec_from_file_location('fat_fixture',root/'tools/make-fat32-fixture.py')
 fat=importlib.util.module_from_spec(spec);spec.loader.exec_module(fat)
 fixture=out/'fat32-fixture.raw'
 if fixture.exists():fixture.unlink()
 fat.create(fixture,args.fat_fixture,args.case=='fat-corrupt')
 fixture_hash=hashlib.sha256(fixture.read_bytes()).hexdigest()
 fat_marker=f'MUSHA: FAT32_FILE_OK BYTES={len(fat.PAYLOAD):016X} HASH={fnv(fat.PAYLOAD):016X}'
 cmd.extend(['-drive',f'if=none,id=fat_fixture,format=raw,readonly=on,file={fixture}',
  '-device','usb-storage,drive=fat_fixture,bus=usbhost.0'+(',port=4' if args.storage_high_speed else '')])
if args.hub_depth and fixture:
 index=next(i for i,x in enumerate(cmd) if x.startswith('usb-storage,drive=fixture') or x.startswith('usb-storage,drive=fat_fixture'))
 cmd[index]+= ',port=3'+'.1'*(args.hub_depth-1)+'.2'
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
  marker={'control-out-timeout':'MUSHA: XHCI_FAILED TRANSFER TIMEOUT','control-out-stall':'MUSHA: XHCI_FAILED BAD TRANSFER','fat-corrupt':'MUSHA: XHCI_FAILED FAT32 READ','storage-timeout':'MUSHA: XHCI_FAILED TRANSFER TIMEOUT','usb-descriptor-timeout':'MUSHA: XHCI_FAILED TRANSFER TIMEOUT', 'xhci-command-timeout':'MUSHA: XHCI_FAILED COMMAND TIMEOUT', 'xhci-timeout':'MUSHA: XHCI_FAILED TIMEOUT', 'normal':'MUSHA: EXIT_BOOT_SERVICES_OK STACK_OK GOP_OK CPU_TABLES_OK PAGING_OK ARENA_OK',
   'ud':'MUSHA: EXCEPTION VECTOR=0000000000000006 ERROR=0000000000000000',
   'df':'MUSHA: EXCEPTION VECTOR=0000000000000008 ERROR=0000000000000000',
   'guard':'MUSHA: EXCEPTION VECTOR=000000000000000E ERROR=0000000000000002',
   'overflow':'MUSHA: EXCEPTION VECTOR=000000000000000E ERROR=0000000000000000',
   'pf':'MUSHA: EXCEPTION VECTOR=000000000000000E ERROR=0000000000000002',
   'ro':'MUSHA: EXCEPTION VECTOR=000000000000000E ERROR=0000000000000003',
   'nx':'MUSHA: EXCEPTION VECTOR=000000000000000E ERROR=0000000000000011',
   'gp':'MUSHA: EXCEPTION VECTOR=000000000000000D ERROR=0000000000000038'}[args.case]
  while time.monotonic()<deadline:
   text=log.read_text()
   if args.hub_disconnect and not injected and 'MUSHA: HID_READY\n' in text:
    if sock is None:connect()
    command('device_del',{'id':'testkbd'})
    injected=True
   if args.hub_disconnect and marker in text:
    for expected in ['MUSHA: XHCI_FAILED KEYBOARD DISCONNECTED','MUSHA: XHCI_QUIESCED DMA_DISABLED','MUSHA: NET_QUIESCED DMA_DISABLED']:
     if expected not in text:raise RuntimeError('Hub disconnect check failed: '+text)
    break
   if args.case=='normal' and not args.hub_disconnect and not args.no_keyboard_input and not injected and 'MUSHA: HID_READY\n' in text:
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
     if args.control_ring_probe and ('MUSHA: EP0_SHORT_OK BYTES=18 STATUS_COMPLETE' not in text or 'MUSHA: EP0_RING_WRAP_OK' not in text or 'MUSHA: EP0_OUT_DATA_STATUS_OK' not in text):raise RuntimeError('EP0 ring/short probe missing: '+text)
     if args.expect_t2_diagnostics:
      for expected in ['MUSHA: T2-D1 READ ONLY / NO MMIO / NO DMA', 'MUSHA: T2 SMBIOS OK', 'MUSHA: T2 BCE COUNT 0 OMIT 0',
                       'MUSHA: T2 PHASE EARLY CAPTURED', 'MUSHA: T2 PHASE PRE EBS CAPTURED', 'MUSHA: T2 PHASE POST EBS CAPTURED',
                       'MUSHA: T2 BAR RANGES UNVALIDATED / DRIVER NOT STARTED', 'MUSHA: T2_DIAGNOSTICS_COMPLETE']:
       if expected+'\n' not in text:raise RuntimeError('T2 diagnostic missing: '+expected+'\n'+text)
     if args.keyboard_wrap and 'MUSHA: HID_RING_WRAP_OK' not in text:raise RuntimeError('HID ring wrap missing: '+text)
     if args.keyboard_exit and 'MUSHA: APP_KEY_DOWN=0000000000000029' not in text:raise RuntimeError('App Escape exit missing: '+text)
     if args.keyboard_exit and ('MUSHA: INPUT_SOURCE_DETACHED' not in text or text.count('MUSHA: APP_KEY_UP=0000000000000029')!=1):raise RuntimeError('Escape release/cleanup missing or duplicated: '+text)
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
     expected_count=(1 if args.keyboard_second and not args.devices_second else 2)+(1 if args.storage_fixture or args.fat_fixture else 0)+int(args.usb_hub)+(args.hub_depth or 0)
     if args.devices_second:
      # Boot disk + keyboard are on second; an optional fixture stays on first.
      for count in [2, int(bool(args.storage_fixture or args.fat_fixture))]:
       if f'MUSHA: USB_ENUMERATION_OK COUNT={count:016X}' not in text:raise RuntimeError('Per-controller USB enumeration failed: '+text)
     elif f'MUSHA: USB_ENUMERATION_OK COUNT={expected_count:016X}' not in text:raise RuntimeError('USB enumeration failed: '+text)
     if 'VID=00000000000046F4 PID=0000000000000001' not in text or 'VID=0000000000000627 PID=0000000000000001' not in text:raise RuntimeError('Expected USB disk and keyboard missing: '+text)
     if 'MUSHA: XHCI_NOOP_OK COUNT=0000000000000258 COMMAND_WRAP_OK EVENT_WRAP_OK' not in text or 'MUSHA: XHCI_QUIESCED DMA_DISABLED' not in text:raise RuntimeError('Command ring probe failed: '+text)
     if 'MUSHA: APP_LIFECYCLE_OK STEPS=' not in text:raise RuntimeError('App lifecycle failed: '+text)
     if 'MUSHA: XHCI_RESET_OK PORTS=' not in text or 'DMA_DISABLED' not in text:raise RuntimeError('xHCI reset failed: '+text)
     if 'MUSHA: ACPI_TIMER_OK MS=' not in text:raise RuntimeError('Timer probe failed: '+text)
     if 'CLASS=00000000000C0330' not in text or ('ID=00000000'+('10D3' if args.nic=='e1000e' else '100E')+'8086 CLASS=0000000000020000') not in text:
      raise RuntimeError('Expected xHCI and Intel 82574 missing: '+text)
     if 'MUSHA: PCI_ENUMERATION_OK' not in text:raise RuntimeError('PCI enumeration incomplete')
    if args.case not in ['normal','xhci-timeout','xhci-command-timeout','usb-descriptor-timeout','control-out-timeout','control-out-stall','storage-timeout','fat-corrupt']:
     if 'Musha-OS halted' not in text or text.rfind('MUSHA: RING_END') < text.rfind('Musha-OS halted'):
      time.sleep(.02);continue
     names={'ud':('#UD',6),'df':('#DF',8),'gp':('#GP',13),'pf':('#PF',14),'ro':('#PF',14),'nx':('#PF',14),'guard':('#PF',14),'overflow':('#PF',14)}
     name,vector=names[args.case]
     expected=f'ERROR: cpu: exception {name} (vector {vector}) error '
     assert expected in text, 'Sequential exception record missing'
     if args.case=='ud': assert expected+'n/a' in text, 'No-error-code vector was displayed as numeric zero'
     else: assert expected+'0x' in text, 'Hardware error code missing'
     import re
     match=re.search(r'RIP=([0-9A-F]{16}) CR2=([0-9A-F]{16})',text)
     if not match or (args.case!='nx' and int(match.group(1),16)==0):raise RuntimeError('Invalid exception frame: '+text)
     if args.case=='pf' and int(match.group(2),16)!=0:raise RuntimeError('Unexpected CR2')
     if args.case in ['ro','nx','guard','overflow'] and int(match.group(2),16)==0:raise RuntimeError('Missing fault address')
     if args.case!='nx' and not re.search(r'ERROR: cpu: rip image\+0x[0-9a-f]+ rsp 0x[0-9a-f]+ ',text):raise RuntimeError('Stack context line missing: '+text)
     if args.case=='overflow':
      # An overflow must stay a #PF on its IST stack: CR2 and RSP are in the
      # guard page, so the reported use exceeds the mapped stack. The probe only
      # reads, so less than one 16 KiB frame below RSP stays unwritten.
      used=re.search(r'stack used (\d+) of (\d+) bytes, never touched (\d+)',text)
      if not used or int(used.group(1))<int(used.group(2)) or int(used.group(3))>=32768:raise RuntimeError('Overflow stack report: '+text)
      if '#DF' in text:raise RuntimeError('Overflow escalated to #DF')
      if 'cpu: stack image addresses image+0x' not in text:raise RuntimeError('Backtrace candidates missing: '+text)
    if args.case in ['xhci-timeout','xhci-command-timeout','usb-descriptor-timeout','control-out-timeout','control-out-stall','storage-timeout','fat-corrupt'] and ('Musha-OS halted' not in text or text.rfind('MUSHA: RING_END') < text.rfind('Musha-OS halted')):
     time.sleep(0.1);continue
    if args.case in ['xhci-command-timeout','usb-descriptor-timeout','control-out-timeout','control-out-stall','storage-timeout','fat-corrupt'] and 'MUSHA: XHCI_QUIESCED DMA_DISABLED' not in text:raise RuntimeError('DMA cleanup missing: '+text)
    if args.case=='storage-timeout' and 'MUSHA: STORAGE_CAPACITY BLOCKS=' not in text:raise RuntimeError('Read timeout was not reached')
    if args.case in ['normal','xhci-timeout','xhci-command-timeout','usb-descriptor-timeout','control-out-timeout','control-out-stall','storage-timeout','fat-corrupt']:
     # v0.4.0 uses one chronological console, not retained panel rows.
     import re
     events = '\n'.join(re.sub(r'^MUSHA: LOG (?:\[[^]]+\] )?', '', line) for line in text.splitlines() if line.startswith('MUSHA: LOG '))
     required=['device: state session complete', '1b36:000d'] if args.case=='normal' else ['1b36:000d', 'ERROR: storage: no readable boot media found', 'Musha-OS halted']
     if args.case=='normal':
      required+=['usb: stage stopped dma disabled', 'usb: root=', '0627:0001',
       '8086:'+('10d3' if args.nic=='e1000e' else '100e'),
       'lan: stage '+('stopped dma disabled' if args.nic=='e1000e' else 'driver unsupported / absent'),
       'usb: kind boot keyboard match / check input', 'input=true']
      if not args.keyboard_second or args.devices_second: required+=['46f4:0001', 'usb: kind bot storage match / check read']
      if not args.extra_xhci: required+=['storage='+str(2 if args.fat_fixture or args.storage_fixture else 1)]
      if args.extra_xhci:
       if text.count('MUSHA: XHCI_SCAN BDF=') != 2: raise RuntimeError('Both controllers must be scanned exactly once')
       if text.count('MUSHA: RUNTIME_POLL_READY') != 1: raise RuntimeError('Ready must be emitted exactly once')
       required+=['bdf=0x0020', 'scanned']
       expected_input='0000000000000020' if args.keyboard_second or args.devices_second else '0000000000000018'
       if 'MUSHA: XHCI_INPUT BDF='+expected_input not in text: raise RuntimeError('Wrong input controller')
      if args.usb_hub or args.hub_depth:
       if 'MUSHA: USB2_HUB_READY' not in text: raise RuntimeError('Hub readiness missing')
      if args.hub_depth:
       check_hub_enumerations(text, args.hub_depth)
       if 'MUSHA: HUB_CHILD_RESET_OK' not in text: raise RuntimeError('Hub child reset missing')
      if args.usb_image: required += [f'device: file readme.txt bytes {len(boot_payload):08x}',f'device: file hash {fnv(boot_payload):016x}']
     else:
      required+=['usb: failure']
      stages={'control-out-timeout':'configuration','control-out-stall':'configuration','xhci-timeout':'halt','xhci-command-timeout':'rings / commands','usb-descriptor-timeout':'device descriptor','storage-timeout':'storage sector read','fat-corrupt':'fat32 file read'}
      required+=['usb: stage '+stages[args.case]]
     for expected in required:
      if expected.casefold() not in events.casefold(): raise RuntimeError('Sequential diagnostic missing: '+expected+'\n'+events)

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
