#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""Boot the staged EFI on QEMU USB storage; verify the post-exit marker."""
import argparse, json, os, pathlib, shutil, socket, subprocess, time
parser=argparse.ArgumentParser()
parser.add_argument('--qemu',default='qemu-system-x86_64')
parser.add_argument('--firmware-dir',required=True)
parser.add_argument('--case',choices=['normal','ud','gp','df','pf','ro','nx','guard'],default='normal')
args=parser.parse_args()
root=pathlib.Path(__file__).resolve().parent.parent
out=root/'out'/('qemu-'+args.case);out.mkdir(parents=True,exist_ok=True)
firmware=pathlib.Path(args.firmware_dir)
shutil.copyfile(firmware/'edk2-i386-vars.fd',out/'vars.fd')
log=out/'debug.log';log.write_text('')
qmp_path=out/'qmp.sock'
if qmp_path.exists(): qmp_path.unlink()
cmd=[args.qemu,'-machine','q35,accel=tcg','-m','256M','-net','none',
 '-drive',f'if=pflash,format=raw,readonly=on,file={firmware / "edk2-x86_64-code.fd"}',
 '-drive',f'if=pflash,format=raw,file={out / "vars.fd"}',
 '-drive',f'if=none,id=esp,format=raw,file=fat:rw:{root / "out/esp"}',
 '-device','qemu-xhci','-device','usb-storage,drive=esp','-vga','std','-display','none',
 '-debugcon',f'file:{log}','-global','isa-debugcon.iobase=0xe9',
 '-qmp',f'unix:{qmp_path},server=on,wait=off','-no-reboot']
with (out/'qemu.log').open('w') as err:
 proc=subprocess.Popen(cmd,stdout=err,stderr=err)
 try:
  deadline=time.monotonic()+45
  marker={'normal':'MUSHA: EXIT_BOOT_SERVICES_OK STACK_OK GOP_OK CPU_TABLES_OK PAGING_OK ARENA_OK',
   'ud':'MUSHA: EXCEPTION VECTOR=0000000000000006 ERROR=0000000000000000',
   'df':'MUSHA: EXCEPTION VECTOR=0000000000000008 ERROR=0000000000000000',
   'guard':'MUSHA: EXCEPTION VECTOR=000000000000000E ERROR=0000000000000002',
   'pf':'MUSHA: EXCEPTION VECTOR=000000000000000E ERROR=0000000000000002',
   'ro':'MUSHA: EXCEPTION VECTOR=000000000000000E ERROR=0000000000000003',
   'nx':'MUSHA: EXCEPTION VECTOR=000000000000000E ERROR=0000000000000011',
   'gp':'MUSHA: EXCEPTION VECTOR=000000000000000D ERROR=0000000000000028'}[args.case]
  while time.monotonic()<deadline:
   text=log.read_text()
   if marker in text and text.endswith('\n'):
    if args.case!='normal':
     import re
     match=re.search(r'RIP=([0-9A-F]{16}) CR2=([0-9A-F]{16})',text)
     if not match or (args.case!='nx' and int(match.group(1),16)==0):raise RuntimeError('Invalid exception frame: '+text)
     if args.case=='pf' and int(match.group(2),16)!=0:raise RuntimeError('Unexpected CR2')
     if args.case in ['ro','nx','guard'] and int(match.group(2),16)==0:raise RuntimeError('Missing fault address')
    break
   if proc.poll() is not None:raise RuntimeError((out/'qemu.log').read_text())
   time.sleep(0.1)
  else:raise RuntimeError('Runtime marker absent: '+(out/'qemu.log').read_text())
  sock=socket.socket(socket.AF_UNIX);sock.settimeout(5);sock.connect(str(qmp_path))
  stream=sock.makefile('rwb');stream.readline()
  def command(name,arguments=None):
   stream.write((json.dumps({'execute':name,'arguments':arguments or {}})+'\n').encode());stream.flush()
   while True:
    message=json.loads(stream.readline())
    if 'error' in message:raise RuntimeError(message)
    if 'return' in message:return
  command('qmp_capabilities')
  command('screendump',{'filename':str(out/'screen.ppm')})
  command('quit');sock.close()
  print(marker);print('Screenshot:',out/'screen.ppm')
 finally:
  if proc.poll() is None:
   proc.terminate()
   try:proc.wait(timeout=5)
   except subprocess.TimeoutExpired:proc.kill();proc.wait()
