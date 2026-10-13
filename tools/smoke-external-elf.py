#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""External ELF probe: one EFI, varying read-only virtual USB media. No real USB."""
import argparse
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import shutil
import socket
import struct
import subprocess
import time

ROOT = Path(__file__).resolve().parents[1]

def module(name):
    spec = importlib.util.spec_from_file_location(name, ROOT / 'tools' / (name + '.py'))
    result = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(result)
    return result


def fixture(path, efi, payload, fmt, declared=None):
    """Use existing deterministic fixtures, append one root entry in a NEW file."""
    if fmt == 'fat32':
        module('make-usb-image').create(path, efi)
        if payload is None:
            return
        with path.open('r+b') as f:
            f.seek(2048 * 512); boot = f.read(512)
            spc = boot[13]; fat_sectors = struct.unpack_from('<I', boot, 36)[0]
            assert spc == 1
            fat_start = 2048 + 32
            data = fat_start + 2 * fat_sectors
            f.seek(fat_start * 512); fat = bytearray(f.read(fat_sectors * 512))
            start = next(n for n in range(5, len(fat)//4) if struct.unpack_from('<I', fat, n*4)[0] == 0)
            count = max(1, (len(payload)+511)//512)
            assert start + count < len(fat)//4
            for n in range(start, start+count):
                assert struct.unpack_from('<I', fat, n*4)[0] == 0
                struct.pack_into('<I', fat, n*4, n+1 if n+1<start+count else 0xfffffff)
            for at in (fat_start, fat_start+fat_sectors):
                f.seek(at*512); f.write(fat)
            entry = bytearray(32); entry[:11] = b'MUSHA   ELF'; entry[11] = 0x20
            struct.pack_into('<H', entry, 20, start>>16)
            struct.pack_into('<HI', entry, 26, start&65535, len(payload) if declared is None else declared)
            f.seek(data*512+64); f.write(entry)
            f.seek((data+start-2)*512); f.write(payload.ljust(count*512,b'\0'))
    else:
        ex = module('make-exfat-fixture')
        image, _ = ex.build()
        if payload is not None:
            count = max(1, (len(payload)+511)//512); chain = list(range(700,700+count))
            assert chain[-1] < ex.CLUSTERS+2
            root = bytearray(image[ex.HEAP*512:(ex.HEAP+1)*512] + image[(ex.HEAP+598)*512:(ex.HEAP+599)*512])
            at = 64
            while root[at] != 0:
                at += (root[at+1]+1)*32
            entry = ex.entry_set('MUSHA.ELF',payload,chain,True,len(payload))
            if declared is not None:
                struct.pack_into('<Q',entry,56,declared)
                struct.pack_into('<H',entry,2,ex.checksum(entry,16,(2,3)))
            assert at+len(entry)+32 <= len(root)
            root[at:at+len(entry)] = entry
            image[ex.HEAP*512:(ex.HEAP+1)*512] = root[:512]
            image[(ex.HEAP+598)*512:(ex.HEAP+599)*512] = root[512:]
            off=(ex.HEAP+chain[0]-2)*512
            image[off:off+count*512]=payload.ljust(count*512,b'\0')
            for n in chain:
                pos=(ex.HEAP+1)*512+(n-2)//8
                image[pos] |= 1<<((n-2)%8)
        with path.open('xb') as f:
            f.write(image)


def main():
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('--firmware-dir',required=True,type=Path)
    p.add_argument('--qemu',default='qemu-system-x86_64')
    p.add_argument('--output',type=Path,default=ROOT/'out/external-elf')
    a=p.parse_args(); out=a.output.resolve();out.mkdir(parents=True,exist_ok=False)
    report={'scope':'local external-elf-probe; default build integration and screen log remain pending','commands':[],'cases':[]}
    def run(cmd,env=None):
        cmd=list(map(str,cmd)); report['commands'].append(cmd)
        return subprocess.check_output(cmd,cwd=ROOT,env=env,stderr=subprocess.STDOUT).decode()
    try:
        (out/'sdk.log').write_text(run(['python3','tools/check-sdk.py','--output',out/'sdk']))
        env=dict(os.environ,CARGO_TARGET_DIR=str(out/'target'),MUSHA_RING3_CASE='normal')
        (out/'build.log').write_text(run(['cargo','build','--locked','--release','--target','x86_64-unknown-uefi','-p','musha-boot','--features','external-elf-probe'],env))
        efi=out/'target/x86_64-unknown-uefi/release/musha-boot.efi'
        report['efi_sha256']=hashlib.sha256(efi.read_bytes()).hexdigest()
        esp=out/'esp/EFI/BOOT';esp.mkdir(parents=True);shutil.copyfile(efi,esp/'BOOTX64.EFI')
        good=(out/'sdk/hello-c.elf').read_bytes()
        for fmt in ('fat32','exfat'):
            for name,payload,declared,expected in (
                ('hello',good,None,'RING3_EXIT_OK'),
                ('alternate',good.replace(b'Hello Musha-OS!',b'Hello from USB!'),None,'RING3_EXIT_OK'),
                ('no-keyboard',good,None,'RING3_EXIT_OK'),
                ('split-controller',good,None,'RING3_EXIT_OK'),
                ('missing',None,None,'EXTERNAL_ELF_ABSENT VIEWER'),
                ('malformed',b'X'+good[1:],None,'EXTERNAL_ELF_REJECT Invalid ELF or ABI NOTE'),
                ('oversize',good,16*1024*1024+1,'EXTERNAL_ELF_REJECT File size'),
            ):
                if fmt=='exfat' and name in ('no-keyboard','split-controller'): continue
                d=out/(fmt+'-'+name);d.mkdir();data=d/'data.img'
                fixture(data,efi,payload,fmt,declared)
                before=hashlib.sha256(data.read_bytes()).hexdigest()
                shutil.copyfile(a.firmware_dir/'edk2-i386-vars.fd',d/'vars.fd')
                debug=d/'debug.log';debug.write_text('')
                qmp=ROOT/'out'/('ee-'+hashlib.sha256(str(d).encode()).hexdigest()[:10]+'.sock')
                assert not qmp.exists()
                cmd=[a.qemu,'-machine','q35,accel=tcg','-cpu','qemu64,+syscall,+nx','-m','256M',
                     '-drive',f'if=pflash,format=raw,readonly=on,file={a.firmware_dir/"edk2-x86_64-code.fd"}',
                     '-drive',f'if=pflash,format=raw,file={d/"vars.fd"}',
                     '-drive',f'if=ide,format=raw,file=fat:rw:{out/"esp"}',
                     '-drive',f'if=none,id=data,format=raw,readonly=on,file={data}',
                     '-device','qemu-xhci,id=x','-device','usb-storage,drive=data,bus=x.0,port=1',
                     '-qmp',f'unix:{qmp},server=on,wait=off','-netdev','user,id=n','-device','e1000e,netdev=n',
                     '-vga','std','-display','none','-monitor','none','-serial','none',
                     '-debugcon',f'file:{debug}','-global','isa-debugcon.iobase=0xe9','-no-reboot']
                if name=='split-controller': cmd+=['-device','qemu-xhci,id=k']
                if name!='no-keyboard': cmd+=['-device','usb-kbd,bus='+('k.0' if name=='split-controller' else 'x.0')]
                report['commands'].append(cmd)
                with (d/'qemu.log').open('w') as log:
                    proc=subprocess.Popen(cmd,stdout=log,stderr=log)
                    try:
                        end=time.monotonic()+60
                        while time.monotonic()<end:
                            text=debug.read_text()
                            done=expected in text and (name!='missing' or 'VIEWER_FRAME LIST' in text)
                            if done and text.endswith('\n'): break
                            if proc.poll() is not None or 'EXCEPTION VECTOR=' in text or 'RING3_REJECT' in text or 'EXTERNAL_ELF_REJECT' in text:
                                raise RuntimeError(text+(d/'qemu.log').read_text())
                            time.sleep(.05)
                        else: raise RuntimeError('Timeout '+text)
                        if expected=='RING3_EXIT_OK':
                            assert ('Hello from USB' if name=='alternate' else 'Hello Musha-OS!') in text and 'RING3_ELF_RUNNING' in text,text
                            handoff=text.index('EXTERNAL_ELF_DEVICES_STOPPED')
                            assert text.index('EXTERNAL_ELF_READ_COMPLETE') < text.rindex('XHCI_QUIESCED') < handoff < text.index('RING3_ELF_ENTERING'),text
                            assert text.index('NET_QUIESCED DMA_DISABLED') < handoff,text
                        else: assert 'RING3_ELF_ENTERING' not in text,text
                        with socket.socket(socket.AF_UNIX) as qs:
                            qs.settimeout(10);qs.connect(str(qmp));stream=qs.makefile('rwb');stream.readline()
                            def control(command,args=None):
                                stream.write((json.dumps({'execute':command,'arguments':args or {}})+'\n').encode());stream.flush()
                                while True:
                                    response=json.loads(stream.readline())
                                    if 'error' in response: raise RuntimeError(response)
                                    if 'return' in response: return
                            control('qmp_capabilities')
                            control('screendump',{'filename':str(d/'screen.ppm')})
                        assert before==hashlib.sha256(data.read_bytes()).hexdigest()
                        report['cases'].append({'case':fmt+'-'+name,'passed':True,'image_sha256':before})
                        print('PASS external ELF '+fmt+' '+name,flush=True)
                    finally:
                        proc.terminate()
                        try:proc.wait(timeout=5)
                        except subprocess.TimeoutExpired:proc.kill();proc.wait()
                        qmp.unlink(missing_ok=True)
    finally:
        (out/'report.json').write_text(json.dumps(report,indent=2)+'\n')

if __name__=='__main__':main()
