#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""QEMU sequential diagnostic log survives session stop without rotating pages."""
import argparse, hashlib, importlib.util, json, shutil, socket, subprocess, time
from pathlib import Path
ROOT = Path(__file__).resolve().parents[1]
p = argparse.ArgumentParser(description=__doc__)
p.add_argument('--firmware-dir', type=Path, required=True)
p.add_argument('--output', type=Path, required=True)
p.add_argument('--qemu', default='qemu-system-x86_64')
p.add_argument('--debug-efi', type=Path, help='Existing qemu-debug diagnostics EFI; require primary log/ring and stop proof')
p.add_argument('--efi', type=Path, help='Existing production diagnostics EFI; secondary screen-only observation (no ring proof)')
a=p.parse_args(); out=a.output.resolve(); out.mkdir(parents=True, exist_ok=False)
if a.efi and a.debug_efi:p.error('--efi and --debug-efi are mutually exclusive')
report={'scope':'QEMU only; physical link recovery remains unverified','cases':[]}
try:
    report['source_head']=subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT,text=True).strip()
    report['source_dirty']=subprocess.check_output(['git','status','--short'],cwd=ROOT,text=True).splitlines()
    if a.efi or a.debug_efi:
        efi=(a.efi or a.debug_efi).resolve()
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
        external.module('smoke-startup').fixture(d/'boot.img',efi,b'log=devices\n')
        shutil.copyfile(a.firmware_dir/'edk2-i386-vars.fd',d/'vars.fd')
        qmp=ROOT/'out'/('dh-'+hashlib.sha256(str(d).encode()).hexdigest()[:10]+'.sock');assert not qmp.exists()
        debug=d/'debug.log'
        cmd=[a.qemu,'-machine','q35,accel=tcg','-cpu','qemu64,+syscall,+nx','-m','256M','-drive',f'if=pflash,format=raw,readonly=on,file={a.firmware_dir/"edk2-x86_64-code.fd"}','-drive',f'if=pflash,format=raw,file={d/"vars.fd"}','-drive',f'if=ide,format=raw,snapshot=on,file={d/"boot.img"}','-device','qemu-xhci,id=x','-qmp',f'unix:{qmp},server=on,wait=off','-vga','std','-display','none','-debugcon',f'file:{debug}','-no-reboot']
        if keyboard:cmd+=['-device','usb-kbd,bus=x.0']
        if storage:
            data=d/'data.img';external.fixture(data,efi,None,'fat32')
            external.module('smoke-startup').configure_fixture(data,b'log=devices\n')
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
                        # A readable medium enters the unchanged viewer; end that
                        # session with Escape before requiring confirmed DMA stop.
                        escaped=False
                        while time.monotonic()<end:
                            text=debug.read_text() if debug.exists() else ''
                            if storage and not escaped and 'OS_READY STATIC_COOPERATIVE FILE_VIEWER' in text:
                                call('send-key',{'keys':[{'type':'qcode','data':'esc'}],'hold-time':80});escaped=True
                            last_ring=text.rfind('MUSHA: RING_BEGIN\n')
                            last_end=text.find('MUSHA: RING_END',last_ring)
                            terminal='diagnostic session complete / no external app' if storage else 'Musha-OS halted'
                            if last_ring>=0 and last_end>last_ring and terminal in text[last_ring:last_end] and (not storage or 'DIAGNOSTIC_CAPTURE_STOP' in text[:last_ring]):break
                            if proc.poll() is not None:raise RuntimeError('QEMU exited')
                            time.sleep(.1)
                        else:raise RuntimeError('No stopped retained log: '+text)
                        assert 'XHCI_QUIESCED DMA_DISABLED' in text,text
                        assert 'DIAGNOSTIC_CAPTURE_LOOP' not in text,text
                        primary=[x.removeprefix('MUSHA: LOG ') for x in text.splitlines() if x.startswith('MUSHA: LOG ')]
                        begin=text.rfind('MUSHA: RING_BEGIN\n');finish=text.find('MUSHA: RING_END',begin)
                        assert begin>=0 and finish>begin,'missing final ring'
                        retained=[x.removeprefix('MUSHA: RING ') for x in text[begin:finish].splitlines() if x.startswith('MUSHA: RING ')]
                        assert primary and retained,'missing primary/ring text'
                        if not storage:assert any(x.endswith('Musha-OS halted') for x in primary) and any(x.endswith('Musha-OS halted') for x in retained)
                        else:assert not any('ERROR: ' in x for x in retained),'unexpected healthy-viewer error'
                        assert any('usb:' in x.lower() for x in retained),'missing USB diagnosis'
                        assert not any('app: starting external ELF' in x for x in primary),'absent ELF launched'
                        if not storage:
                            assert any('ERROR: storage: no readable boot media found' in x for x in retained),'no required media error'
                            assert not any('Musha-OS 0.4.0 ready' in x for x in primary),'unreadable medium declared ready'
                        if storage:assert escaped and 'OS_READY STATIC_COOPERATIVE FILE_VIEWER' in text,'readable medium never entered viewer'
                        (d/'primary.txt').write_text('\n'.join(primary)+'\n')
                        (d/'ring.txt').write_text('\n'.join(retained)+'\n')
                    hashes=[]
                    for n in range(2):
                        call('screendump',{'filename':str(d/f'stopped-{n}.ppm')})
                        hashes.append(hashlib.sha256((d/f'stopped-{n}.ppm').read_bytes()).hexdigest());time.sleep(.3)
                    assert len(set(hashes))==1,'Stopped console changed without new events'
                report['cases'].append({'case':case,'passed':True,'stopped_frames_sha256':hashes,'primary_ring_verified':not bool(a.efi),'production_screen_only':bool(a.efi)})
                print('PASS',case,flush=True)
            finally:
                if proc.poll() is None:proc.terminate()
                proc.wait(timeout=10);qmp.unlink(missing_ok=True)
except Exception as e:
    report['failure']=str(e);raise
finally:
    (out/'report.json').write_text(json.dumps(report,indent=2)+'\n')
