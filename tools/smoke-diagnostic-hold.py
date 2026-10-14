#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""QEMU diagnostic capture stays visible after no medium or absent ELF."""
import argparse, hashlib, importlib.util, json, shutil, socket, subprocess, time
from pathlib import Path
ROOT = Path(__file__).resolve().parents[1]
p = argparse.ArgumentParser(description=__doc__)
p.add_argument('--firmware-dir', type=Path, required=True)
p.add_argument('--output', type=Path, required=True)
p.add_argument('--qemu', default='qemu-system-x86_64')
p.add_argument('--efi', type=Path, help='Existing diagnostics-detail EFI without qemu-debug; screen-only observation')
a=p.parse_args(); out=a.output.resolve(); out.mkdir(parents=True, exist_ok=False)
report={'scope':'QEMU only; physical link recovery remains unverified','cases':[]}
try:
    if a.efi:
        efi=a.efi.resolve()
    else:
        target=out/'target'
        with (out/'build.log').open('w') as log:
            subprocess.run(['cargo','build','--locked','--release','--target','x86_64-unknown-uefi','-p','musha-boot','--features','diagnostics-detail,qemu-debug','--target-dir',str(target)],cwd=ROOT,stdout=log,stderr=subprocess.STDOUT,check=True)
        efi=target/'x86_64-unknown-uefi/release/musha-boot.efi'
    report['efi_sha256']=hashlib.sha256(efi.read_bytes()).hexdigest()
    spec=importlib.util.spec_from_file_location('external',ROOT/'tools/smoke-external-elf.py')
    external=importlib.util.module_from_spec(spec);spec.loader.exec_module(external)
    for case,storage,keyboard in [('no-medium',False,True),('elf-absent',True,True),('no-devices',False,False)]:
        d=out/case;d.mkdir();esp=d/'esp/EFI/BOOT';esp.mkdir(parents=True);shutil.copyfile(efi,esp/'BOOTX64.EFI')
        shutil.copyfile(a.firmware_dir/'edk2-i386-vars.fd',d/'vars.fd')
        qmp=ROOT/'out'/('dh-'+hashlib.sha256(str(d).encode()).hexdigest()[:10]+'.sock');assert not qmp.exists()
        debug=d/'debug.log'
        cmd=[a.qemu,'-machine','q35,accel=tcg','-cpu','qemu64,+syscall,+nx','-m','256M','-drive',f'if=pflash,format=raw,readonly=on,file={a.firmware_dir/"edk2-x86_64-code.fd"}','-drive',f'if=pflash,format=raw,file={d/"vars.fd"}','-drive',f'if=ide,format=raw,file=fat:rw:{d/"esp"}','-device','qemu-xhci,id=x','-qmp',f'unix:{qmp},server=on,wait=off','-vga','std','-display','none','-debugcon',f'file:{debug}','-no-reboot']
        if keyboard:cmd+=['-device','usb-kbd,bus=x.0']
        if storage:
            data=d/'data.img';external.fixture(data,efi,None,'fat32')
            cmd+=['-drive',f'if=none,id=data,format=raw,readonly=on,file={data}','-device','usb-storage,drive=data,bus=x.0']
        (d/'command.json').write_text(json.dumps(cmd,indent=2))
        with (d/'qemu.log').open('w') as log:
            proc=subprocess.Popen(cmd,stdout=log,stderr=log)
            try:
                end=time.monotonic()+75
                while not qmp.exists():
                    if proc.poll() is not None or time.monotonic()>end:raise RuntimeError('QMP start failed')
                    time.sleep(.05)
                with socket.socket(socket.AF_UNIX) as s:
                    s.settimeout(10);s.connect(str(qmp));stream=s.makefile('rwb');stream.readline()
                    def call(name,args):
                        stream.write((json.dumps({'execute':name,'arguments':args})+'\n').encode());stream.flush()
                        while True:
                            r=json.loads(stream.readline())
                            if 'error' in r:raise RuntimeError(r)
                            if 'return' in r:return
                    call('qmp_capabilities',{})
                    if a.efi:
                        time.sleep(30)
                        assert debug.read_bytes()==b'', 'Production diagnostic emitted debug'
                    else:
                        while time.monotonic()<end:
                            text=debug.read_text() if debug.exists() else ''
                            if 'DIAGNOSTIC_CAPTURE_LOOP' in text:break
                            if proc.poll() is not None:raise RuntimeError('QEMU exited')
                            time.sleep(.1)
                        else:raise RuntimeError('No capture loop: '+text)
                        assert 'XHCI_QUIESCED DMA_DISABLED' in text,text
                        assert text.count('MUSHA: RESCAN 1/1 START') == (1 if case=='no-devices' else 0),text
                        if case=='no-devices': assert text.count('MUSHA: RESCAN 1/1 DONE')==1,text
                        if storage:assert 'DIAGNOSTIC_HOLD ELF_ABSENT' in text,text
                        assert 'OS_READY STATIC_COOPERATIVE' not in text and 'EXTERNAL_APP_EXIT' not in text,text
                    hashes=[]
                    for n in range(3):
                        call('screendump',{'filename':str(d/f'page-{n}.ppm')})
                        hashes.append(hashlib.sha256((d/f'page-{n}.ppm').read_bytes()).hexdigest());time.sleep(2.2)
                    assert len(set(hashes))>1,'Diagnostic pages did not rotate'
                report['cases'].append({'case':case,'passed':True,'pages_sha256':hashes})
                print('PASS',case,flush=True)
            finally:
                if proc.poll() is None:proc.terminate()
                proc.wait(timeout=10);qmp.unlink(missing_ok=True)
except Exception as e:
    report['failure']=str(e);raise
finally:
    (out/'report.json').write_text(json.dumps(report,indent=2)+'\n')
