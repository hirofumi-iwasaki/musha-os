#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""Real QEMU error completion -> Disable Slot -> next-root storage/Hello.

usb-address-reject deliberately clears first device's Add Context flags. No
synthetic completion is supplied: QEMU must return the command error itself.
Only temporary regular images are used; no physical USB access.
"""
import argparse, hashlib, importlib.util, json, os
from pathlib import Path
import shutil, socket, subprocess, time
ROOT = Path(__file__).resolve().parents[1]
p = argparse.ArgumentParser(description=__doc__)
p.add_argument('--firmware-dir', type=Path, required=True)
p.add_argument('--output', type=Path, required=True)
p.add_argument('--qemu', default='qemu-system-x86_64')
p.add_argument('--efi', type=Path)
p.add_argument('--diagnostic', action='store_true', help='Capture rotating diagnostic pages as well as final Hello')
a = p.parse_args(); out = a.output.resolve(); out.mkdir(parents=True, exist_ok=False)
report = {'scope': 'QEMU only; not physical acceptance', 'cases': []}
def run(cmd):
    return subprocess.check_output(list(map(str,cmd)), cwd=ROOT, stderr=subprocess.STDOUT).decode()
def load(name):
    spec=importlib.util.spec_from_file_location(name, ROOT/'tools'/(name+'.py'))
    mod=importlib.util.module_from_spec(spec); spec.loader.exec_module(mod); return mod
try:
    (out/'sdk.log').write_text(run(['python3','tools/check-sdk.py','--output',out/'sdk']))
    if a.efi: efi=a.efi.resolve()
    else:
        (out/'build.log').write_text(run(['cargo','build','--locked','--release','--target','x86_64-unknown-uefi','-p','musha-boot','--features','usb-address-reject','--target-dir',out/'target']))
        efi=out/'target/x86_64-unknown-uefi/release/musha-boot.efi'
    report['efi_sha256']=hashlib.sha256(efi.read_bytes()).hexdigest()
    for first in ('keyboard','storage'):
        d=out/first; d.mkdir(); esp=d/'esp/EFI/BOOT'; esp.mkdir(parents=True)
        shutil.copyfile(efi,esp/'BOOTX64.EFI')
        data=d/'data.img'; load('smoke-external-elf').fixture(data,efi,(out/'sdk/hello-c.elf').read_bytes(),'fat32')
        before=hashlib.sha256(data.read_bytes()).hexdigest()
        shutil.copyfile(a.firmware_dir/'edk2-i386-vars.fd',d/'vars.fd')
        qmp=ROOT/'out'/('ar-'+hashlib.sha256(str(d).encode()).hexdigest()[:10]+'.sock')
        assert not qmp.exists()
        debug=d/'debug.log'
        cmd=[a.qemu,'-machine','q35,accel=tcg','-cpu','qemu64,+syscall,+nx','-m','256M',
             '-drive',f'if=pflash,format=raw,readonly=on,file={a.firmware_dir/"edk2-x86_64-code.fd"}',
             '-drive',f'if=pflash,format=raw,file={d/"vars.fd"}',
             '-drive',f'if=ide,format=raw,file=fat:rw:{d/"esp"}',
             '-device','qemu-xhci,id=x,p2=8,p3=1',
             '-drive',f'if=none,id=data,format=raw,readonly=on,file={data}',
             '-device','usb-storage,drive=data,bus=x.0,port=3',
             '-qmp',f'unix:{qmp},server=on,wait=off','-vga','std','-display','none',
             '-debugcon',f'file:{debug}','-no-reboot']
        if first=='keyboard': cmd+=['-device','usb-kbd,bus=x.0,port=2,usb_version=2']
        else: cmd+=['-drive',f'if=none,id=bad,format=raw,readonly=on,file={data}','-device','usb-storage,drive=bad,bus=x.0,port=2']
        (d/'command.json').write_text(json.dumps(cmd,indent=2))
        with (d/'qemu.log').open('w') as log:
            proc=subprocess.Popen(cmd,stdout=log,stderr=log)
            try:
                end=time.monotonic()+75
                while not qmp.exists():
                    if proc.poll() is not None or time.monotonic()>end: raise RuntimeError('QMP startup')
                    time.sleep(.05)
                with socket.socket(socket.AF_UNIX) as s:
                    s.settimeout(10); s.connect(str(qmp)); stream=s.makefile('rwb');stream.readline()
                    def call(n,args={}):
                        stream.write((json.dumps({'execute':n,'arguments':args})+'\n').encode());stream.flush()
                        while True:
                            v=json.loads(stream.readline())
                            if 'error' in v: raise RuntimeError(v)
                            if 'return' in v: return
                    call('qmp_capabilities')
                    capture_at=0; frame=0
                    while time.monotonic()<end:
                        if a.diagnostic and time.monotonic() >= capture_at:
                            call('screendump',{'filename':str(d/f'diagnostic-{frame}.ppm')})
                            frame+=1;capture_at=time.monotonic()+2
                        text=debug.read_text() if debug.exists() else ''
                        if 'EXTERNAL_APP_EXIT' in text: break
                        if proc.poll() is not None: raise RuntimeError((d/'qemu.log').read_text())
                        time.sleep(.1)
                    else: raise RuntimeError('Timeout: '+text)
                    time.sleep(.3); call('screendump',{'filename':str(d/'screen.ppm')})
                markers=['ADDRESS_REJECT_INJECTED','COMMAND_CC=0000000000000005',
                         'ADDRESS_FAILED ROOT=0000000000000003','ADDRESS_SLOT_DISABLED_CONTINUE',
                         'USB_PORT_RESET_OK PORT=0000000000000004','EXTERNAL_ELF_READ_COMPLETE','EXTERNAL_APP_EXIT']
                positions=[text.index(m) for m in markers]; assert positions==sorted(positions),text
                assert 'BAD COMPLETION' not in text and 'COMMAND TIMEOUT' not in text,text
                shown='\n'.join(load('smoke-external-elf').screen_rows(d/'screen.ppm'))
                (d/'screen.txt').write_text(shown+'\n')
                assert 'Hello Musha-OS!' in shown and 'AUTORUN.ELF exited: status 0' in shown,shown
                assert hashlib.sha256(data.read_bytes()).hexdigest()==before
                report['cases'].append({'case':first+'-fails-next-storage-runs','passed':True,'completion_code':5,'screen':str(d/'screen.ppm')})
                print('PASS',first,flush=True)
            finally:
                if proc.poll() is None:proc.terminate()
                proc.wait(timeout=10);qmp.unlink(missing_ok=True)
except Exception as e:
    report['failure']=str(e);raise
finally:
    (out/'report.json').write_text(json.dumps(report,indent=2)+'\n')
