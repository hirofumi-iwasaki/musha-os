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


def screen_rows(path):
    """Decode actual pixels with the checked-in original 5x7 font, not debug text."""
    import ast, re
    data=path.read_bytes(); header=re.match(rb'P6\s+(\d+)\s+(\d+)\s+255\s',data)
    assert header, 'Expected P6 screenshot'
    w,h=map(int,header.groups()); pixels=data[header.end():]; scale=2 if w>=992 and h>=416 else 1
    font={}
    for key,values in re.findall(r"(b'(?:\\.|[^'])+'|\d+) => \[([\d, ]+)\]",(ROOT/'framebuffer/src/lib.rs').read_text()):
        c=ast.literal_eval(key);c=c[0] if isinstance(c,bytes) else c
        font[tuple(map(int,values.split(',')))]=chr(c)
    font[(0,)*7]=' '
    rows=[]
    for row in range(24):
        line=''
        for col in range(80):
            bits=[]
            for y in range(7):
                v=0
                for x in range(5):
                    at=((16+(row*8+y)*scale)*w+16+(col*6+x)*scale)*3
                    v=(v<<1)|(pixels[at:at+3]==bytes([240]*3))
                bits.append(v)
            line+=font.get(tuple(bits),'?')
        rows.append(line)
    return rows


def main():
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('--firmware-dir',required=True,type=Path)
    p.add_argument('--qemu',default='qemu-system-x86_64')
    p.add_argument('--output',type=Path,default=ROOT/'out/external-elf')
    p.add_argument('--case',default='all')
    a=p.parse_args(); out=a.output.resolve();out.mkdir(parents=True,exist_ok=False)
    report={'scope':'local external ELF; no real USB or hardware claim','commands':[],'cases':[]}
    def run(cmd,env=None):
        cmd=list(map(str,cmd)); report['commands'].append(cmd)
        return subprocess.check_output(cmd,cwd=ROOT,env=env,stderr=subprocess.STDOUT).decode()
    try:
        (out/'sdk.log').write_text(run(['python3','tools/check-sdk.py','--output',out/'sdk']))
        built=out/'sdk';good=(built/'hello-c.elf').read_bytes()
        cases={n:(good,fmt,'exit',0) for n,fmt in [('hello-fat32','fat32'),('hello-exfat','exfat'),('hello-split','fat32')]}
        sysroot=Path(run(['rustc','--print','sysroot']).strip());host=run(['rustc','-vV']).split('host: ')[1].splitlines()[0]
        linkenv=dict(os.environ,DYLD_LIBRARY_PATH=str(sysroot/'lib'))
        flags=['--target=x86_64-unknown-none-elf','-isystem',ROOT/'sdk/sysroot/include','-ffreestanding','-fno-builtin','-fno-stack-protector','-fno-pic','-mno-red-zone','-march=x86-64','-mno-avx','-O2','-Wall','-Wextra','-Werror']
        for number,name in enumerate(['exit-42','fault-pf','fault-ud','log-long','log-utf8']):
            obj=out/(name+'.o');elf=out/(name+'.elf')
            run(['clang',*flags,'-DEXTERNAL_CASE='+str(number),'-c',ROOT/'sdk/tests/external-c.c','-o',obj])
            run([sysroot/'lib/rustlib'/host/'bin/rust-lld','-flavor','gnu','-static','--no-undefined','-T',ROOT/'sdk/linker/app.ld','-o',elf,*[built/(n+'.o') for n in ['crt0','libmusha','memory','note']],obj],linkenv)
            cases[name]=(elf.read_bytes(),'fat32','fault' if name.startswith('fault') else 'exit',42 if name=='exit-42' else 0)
        ph=struct.unpack_from('<Q',good,32)[0];headers=[ph+i*56 for i in range(struct.unpack_from('<H',good,56)[0])]
        loads=[h for h in headers if struct.unpack_from('<I',good,h)[0]==1];note=next(h for h in headers if struct.unpack_from('<I',good,h)[0]==4);off=struct.unpack_from('<Q',good,note+8)[0]
        for name,pos,fmt,value in [('bad-magic',0,'I',0),('wx',loads[0]+4,'I',7),('dynamic',loads[0],'I',2),('entry',24,'Q',0),('file-overflow',loads[0]+8,'Q',2**64-1),('note-version',off+24,'I',99),('note-permission',off+44,'Q',2),('overlap',loads[1]+16,'Q',struct.unpack_from('<Q',good,loads[0]+16)[0]),('missing-note',note,'I',0)]:
            bad=bytearray(good);struct.pack_into('<'+fmt,bad,pos,value);cases['malformed-'+name]=(bad,'fat32','reject','Invalid ELF or ABI NOTE')
        cases.update({'too-large':(good,'fat32','reject','File size outside'), 'too-small':(b'X'*63,'fat32','reject','File size outside'),'duplicate':(good,'fat32','reject','Ambiguous MUSHA.ELF name')})
        for size in (16384,1048576,16777216):
            cases['timing-'+str(size)]=(good.ljust(size,b'\0'),'fat32','timing',0)
        cases['timeout']=(good.ljust(16777216,b'\0'),'fat32','reject','File read deadline exceeded')
        cases['disconnect-while-reading']=(good.ljust(16777216,b'\0'),'fat32','disconnect',0)
        cases['absent']=(None,'fat32','absent',0)
        if a.case!='all' and a.case not in cases: raise ValueError('Unknown case '+a.case)
        env=dict(os.environ,CARGO_TARGET_DIR=str(out/'target'),MUSHA_RING3_CASE='normal')
        (out/'build.log').write_text(run(['cargo','build','--locked','--release','--target','x86_64-unknown-uefi','-p','musha-boot','--features','external-elf-probe'],env))
        efi=out/'target/x86_64-unknown-uefi/release/musha-boot.efi'
        report['efi_sha256']=hashlib.sha256(efi.read_bytes()).hexdigest()
        esp=out/'esp/EFI/BOOT';esp.mkdir(parents=True);shutil.copyfile(efi,esp/'BOOTX64.EFI')
        for name,(payload,fmt,kind,detail) in cases.items():
            if a.case not in ('all',name): continue
            d=out/name;d.mkdir();data=d/'data.img'
            if kind=='absent':
                normalenv=dict(os.environ,CARGO_TARGET_DIR=str(out/'normal-target'))
                (d/'build.log').write_text(run(['cargo','build','--locked','--release','--target','x86_64-unknown-uefi','-p','musha-boot'],normalenv))
                normal=out/'normal-target/x86_64-unknown-uefi/release/musha-boot.efi'
                fixture(data,normal,None,'fat32')
                reference=ROOT/'out/v3-normal-final-pixels'
                (d/'pixels.log').write_text(run(['python3','tools/smoke-normal-viewer.py','--firmware-dir',a.firmware_dir,'--qemu',a.qemu,'--boot-image',data,'--data-image',data,'--reference',reference,'--output',d/'pixels']))
                report['cases'].append({'case':name,'passed':True,'normal_efi_sha256':hashlib.sha256(normal.read_bytes()).hexdigest(),'reference':str(reference),'pixels':str(d/'pixels/result.json')});print('PASS external ELF '+name,flush=True);continue
            fixture(data,efi,payload,fmt,16777217 if name=='too-large' else None)
            if name=='duplicate':
                with data.open('r+b') as f:
                    f.seek(2048*512);boot=f.read(512);root=(2048+32+2*struct.unpack_from('<I',boot,36)[0])*512
                    f.seek(root+64);entry=f.read(32);f.seek(root+96);f.write(entry)
            before=hashlib.sha256(data.read_bytes()).hexdigest()
            shutil.copyfile(a.firmware_dir/'edk2-i386-vars.fd',d/'vars.fd')
            debug=d/'debug.log';debug.write_text('')
            qmp=ROOT/'out'/('ee-'+hashlib.sha256(str(d).encode()).hexdigest()[:10]+'.sock');assert not qmp.exists()
            cmd=[a.qemu,'-machine','q35,accel=tcg','-cpu','qemu64,+syscall,+nx','-m','256M',
                 '-drive',f'if=pflash,format=raw,readonly=on,file={a.firmware_dir/"edk2-x86_64-code.fd"}',
                 '-drive',f'if=pflash,format=raw,file={d/"vars.fd"}',
                 '-drive',f'if=ide,format=raw,file=fat:rw:{out/"esp"}',
                 '-drive',f'if=none,id=data,format=raw,readonly=on,file={data}',
                 '-device','qemu-xhci,id=x','-device','usb-storage,id=storage,drive=data,bus=x.0,port=1',
                 '-qmp',f'unix:{qmp},server=on,wait=off','-netdev','user,id=n','-device','e1000e,netdev=n',
                 '-vga','std','-display','none','-monitor','none','-serial','none',
                 '-debugcon',f'file:{debug}','-global','isa-debugcon.iobase=0xe9','-no-reboot']
            if name=='hello-split': cmd+=['-device','qemu-xhci,id=k']
            cmd+=['-device','usb-kbd,bus='+('k.0' if name=='hello-split' else 'x.0')]
            report['commands'].append(cmd)
            with (d/'qemu.log').open('w') as log:
                proc=subprocess.Popen(cmd,stdout=log,stderr=log)
                try:
                    end=time.monotonic()+60
                    while not qmp.exists():
                        if proc.poll() is not None or time.monotonic()>end: raise RuntimeError('QMP startup')
                        time.sleep(.05)
                    with socket.socket(socket.AF_UNIX) as qs:
                        qs.settimeout(10);qs.connect(str(qmp));stream=qs.makefile('rwb');stream.readline()
                        def control(command,args=None):
                            stream.write((json.dumps({'execute':command,'arguments':args or {}})+'\n').encode());stream.flush()
                            while True:
                                response=json.loads(stream.readline())
                                if 'error' in response: raise RuntimeError(response)
                                if 'return' in response: return
                        control('qmp_capabilities');removed=False;loading=False
                        while time.monotonic()<end:
                            text=debug.read_text()
                            if 'EXTERNAL_ELF_READING' in text and not loading:
                                control('screendump',{'filename':str(d/'loading.ppm')});loading=True
                                if kind=='disconnect': control('device_del',{'id':'storage'});removed=True
                            if text.endswith('\n') and any(x in text for x in ['EXTERNAL_APP_EXIT','EXCEPTION VECTOR=','EXTERNAL_ELF_REJECT','RING3_REJECT']): break
                            if proc.poll() is not None: raise RuntimeError((d/'qemu.log').read_text())
                            time.sleep(.02)
                        else: raise RuntimeError('Timeout '+text)
                        # Debug rejection marker precedes its draw; poll pixels until final screen settles.
                        time.sleep(.3);control('screendump',{'filename':str(d/'screen.ppm')})
                    rows=screen_rows(d/'screen.ppm');screen='\n'.join(rows);(d/'screen.txt').write_text(screen+'\n')
                    if kind in ('exit','fault'):
                        handoff=text.index('EXTERNAL_ELF_DEVICES_STOPPED')
                        assert text.index('EXTERNAL_ELF_READ_COMPLETE') < text.rindex('XHCI_QUIESCED') < handoff < text.index('RING3_ELF_ENTERING'),text
                        assert text.index('NET_QUIESCED DMA_DISABLED') < handoff,text
                        assert text.index('OS_READY EXTERNAL_APP') < text.index('APP_ENTERING EXTERNAL_APP') < text.index('APP_RUNNING EXTERNAL_APP'),text
                        assert 'OS_READY FILE_VIEWER' not in text,text
                    if kind=='exit':
                        assert f'MUSHA.ELF exited: status {detail}' in screen,screen
                        if name.startswith('hello'): assert 'Hello Musha-OS!' in screen,screen
                        if name=='log-long': assert rows[0]=='Z'*80 and rows[20]=='Z'*80 and rows[21].rstrip()=='Z'*16,screen
                        if name=='log-utf8': assert 'ASCII..... ......' in screen and 'END' in screen,screen
                    elif kind=='fault':
                        vector=14 if name=='fault-pf' else 6
                        assert 'MUSHA.ELF faulted' in screen and f'VECTOR {vector:016X}' in screen and 'ERROR ' in screen and 'RIP ' in screen and 'RSP ' in screen,screen
                        assert 'RING3_FAULT_KERNEL_STACK_OK' in text,text
                    elif kind in ('reject','disconnect'):
                        assert 'MUSHA.ELF rejected:' in screen and 'RING3_ELF_ENTERING' not in text,screen+text
                        if kind=='reject': assert detail in screen,screen
                        else: assert removed and 'XHCI_QUIESCED' in text and ('File read failed' in screen or 'Device session did not complete' in screen),screen+text
                    elif kind=='timing': assert 'EXTERNAL_ELF_ELAPSED_MS=' in text,text
                    assert before==hashlib.sha256(data.read_bytes()).hexdigest()
                    result={'case':name,'passed':True,'image_sha256':before,'screen':str(d/'screen.txt')}
                    if 'EXTERNAL_ELF_ELAPSED_MS=' in text:
                        result['read_elapsed_ms']=int(text.split('EXTERNAL_ELF_ELAPSED_MS=')[1].splitlines()[0],16)
                        result['read_completed']='EXTERNAL_ELF_READ_COMPLETE' in text
                        result['file_bytes']=len(payload)
                    report['cases'].append(result);print('PASS external ELF '+name,flush=True)
                finally:
                    proc.terminate()
                    try:proc.wait(timeout=5)
                    except subprocess.TimeoutExpired:proc.kill();proc.wait()
                    qmp.unlink(missing_ok=True)
    except Exception as error:
        report['failure']=str(error);raise
    finally:
        (out/'report.json').write_text(json.dumps(report,indent=2)+'\n')

if __name__=='__main__':main()
