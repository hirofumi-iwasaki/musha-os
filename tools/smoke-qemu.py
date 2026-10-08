#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""Boot the staged EFI on QEMU USB storage; verify the post-exit marker."""
import argparse, json, os, pathlib, shutil, socket, subprocess, time
parser=argparse.ArgumentParser()
parser.add_argument('--qemu',default='qemu-system-x86_64')
parser.add_argument('--firmware-dir',required=True)
parser.add_argument('--keyboard-usb-version',type=int,choices=[1,2],default=2)
parser.add_argument('--case',choices=['normal','ud','gp','df','pf','ro','nx','guard','xhci-timeout','xhci-command-timeout','usb-descriptor-timeout'],default='normal')
parser.add_argument('--no-keyboard-input',action='store_true')
args=parser.parse_args()
root=pathlib.Path(__file__).resolve().parent.parent
out=root/'out'/('qemu-'+args.case);out.mkdir(parents=True,exist_ok=True)
firmware=pathlib.Path(args.firmware_dir)
shutil.copyfile(firmware/'edk2-i386-vars.fd',out/'vars.fd')
log=out/'debug.log';log.write_text('')
qmp_path=out/'qmp.sock'
if qmp_path.exists(): qmp_path.unlink()
cmd=[args.qemu,'-machine','q35,accel=tcg','-m','256M','-netdev','user,id=net0','-device','e1000e,netdev=net0',
 '-drive',f'if=pflash,format=raw,readonly=on,file={firmware / "edk2-x86_64-code.fd"}',
 '-drive',f'if=pflash,format=raw,file={out / "vars.fd"}',
 '-drive',f'if=none,id=esp,format=raw,file=fat:rw:{root / "out/esp"}',
 '-device','qemu-xhci','-device','usb-storage,drive=esp','-device',f'usb-kbd,usb_version={args.keyboard_usb_version}','-vga','std','-display','none',
 '-debugcon',f'file:{log}','-global','isa-debugcon.iobase=0xe9',
 '-qmp',f'unix:{qmp_path},server=on,wait=off','-no-reboot']
with (out/'qemu.log').open('w') as err:
 proc=subprocess.Popen(cmd,stdout=err,stderr=err)
 try:
  sock=None
  stream=None
  injected=False
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
  marker={'usb-descriptor-timeout':'MUSHA: XHCI_FAILED TRANSFER TIMEOUT', 'xhci-command-timeout':'MUSHA: XHCI_FAILED COMMAND TIMEOUT', 'xhci-timeout':'MUSHA: XHCI_FAILED TIMEOUT', 'normal':'MUSHA: EXIT_BOOT_SERVICES_OK STACK_OK GOP_OK CPU_TABLES_OK PAGING_OK ARENA_OK',
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
   if marker in text and text.endswith('\n'):
    if args.case=='normal':
     if 'MUSHA: HID_DIAGNOSTIC_OK REPORTS=' not in text:raise RuntimeError('HID diagnostic missing: '+text)
     if not args.no_keyboard_input:
      for key in ['00000000000000E1','0000000000000004']:
       for direction in ['DOWN','UP']:
        if 'MUSHA: HID_KEY_'+direction+'='+key not in text:raise RuntimeError('HID key transition missing: '+text)
     if 'MUSHA: USB_ENUMERATION_OK COUNT=0000000000000002' not in text:raise RuntimeError('USB enumeration failed: '+text)
     if 'VID=00000000000046F4 PID=0000000000000001' not in text or 'VID=0000000000000627 PID=0000000000000001' not in text:raise RuntimeError('Expected USB disk and keyboard missing: '+text)
     if 'MUSHA: XHCI_NOOP_OK COUNT=0000000000000258 COMMAND_WRAP_OK EVENT_WRAP_OK' not in text or 'MUSHA: XHCI_QUIESCED DMA_DISABLED' not in text:raise RuntimeError('Command ring probe failed: '+text)
     if 'MUSHA: APP_LIFECYCLE_OK STEPS=' not in text:raise RuntimeError('App lifecycle failed: '+text)
     if 'MUSHA: XHCI_RESET_OK PORTS=' not in text or 'DMA_DISABLED' not in text:raise RuntimeError('xHCI reset failed: '+text)
     if 'MUSHA: ACPI_TIMER_OK MS=' not in text:raise RuntimeError('Timer probe failed: '+text)
     if 'CLASS=00000000000C0330' not in text or 'ID=0000000010D38086 CLASS=0000000000020000' not in text:
      raise RuntimeError('Expected xHCI and Intel 82574 missing: '+text)
     if 'MUSHA: PCI_ENUMERATION_OK' not in text:raise RuntimeError('PCI enumeration incomplete')
    if args.case not in ['normal','xhci-timeout','xhci-command-timeout','usb-descriptor-timeout']:
     import re
     match=re.search(r'RIP=([0-9A-F]{16}) CR2=([0-9A-F]{16})',text)
     if not match or (args.case!='nx' and int(match.group(1),16)==0):raise RuntimeError('Invalid exception frame: '+text)
     if args.case=='pf' and int(match.group(2),16)!=0:raise RuntimeError('Unexpected CR2')
     if args.case in ['ro','nx','guard'] and int(match.group(2),16)==0:raise RuntimeError('Missing fault address')
    if args.case in ['xhci-timeout','xhci-command-timeout','usb-descriptor-timeout'] and 'MUSHA: EXIT_BOOT_SERVICES_OK STACK_OK GOP_OK CPU_TABLES_OK PAGING_OK ARENA_OK' not in text:
     time.sleep(0.1);continue
    if args.case in ['xhci-command-timeout','usb-descriptor-timeout'] and 'MUSHA: XHCI_QUIESCED DMA_DISABLED' not in text:raise RuntimeError('DMA cleanup missing: '+text)
    break
   if proc.poll() is not None:raise RuntimeError((out/'qemu.log').read_text())
   time.sleep(0.1)
  else:raise RuntimeError('Runtime marker absent: '+(out/'qemu.log').read_text())
  if sock is None:connect()
  command('screendump',{'filename':str(out/'screen.ppm')})
  command('quit');sock.close()
  print(marker);print('Screenshot:',out/'screen.ppm')
 finally:
  if proc.poll() is None:
   proc.terminate()
   try:proc.wait(timeout=5)
   except subprocess.TimeoutExpired:proc.kill();proc.wait()
