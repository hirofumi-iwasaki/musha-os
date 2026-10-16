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


def fixture(path, efi, payload, fmt, declared=None, partition_type='basic-data'):
    """Use existing deterministic fixtures, append one root entry in a NEW file."""
    if fmt == 'fat32':
        module('make-usb-image').create(path, efi, partition_type=partition_type)
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
            entry = bytearray(32); entry[:11] = b'AUTORUN ELF'; entry[11] = 0x20
            struct.pack_into('<H', entry, 20, start>>16)
            struct.pack_into('<HI', entry, 26, start&65535, len(payload) if declared is None else declared)
            f.seek(data*512); root = f.read(512)
            slot = next(i for i in range(0,512,32) if root[i] == 0)
            f.seek(data*512+slot); f.write(entry)
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
            entry = ex.entry_set('AUTORUN.ELF',payload,chain,True,len(payload))
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


def screen_rows(path, viewer=False):
    """Decode actual pixels with the checked-in original 5x7 font, not debug text."""
    import ast, re
    data=path.read_bytes(); header=re.match(rb'P6\s+(\d+)\s+(\d+)\s+255\s',data)
    assert header, 'Expected P6 screenshot'
    w,h=map(int,header.groups()); pixels=data[header.end():]; scale=max(1,min(4,(h+300)//600))
    if viewer:
        scale=max(1,min(scale,w//(6*84),h//(10*32)))
    columns=max(1,min(80,w//(6*scale)-4)) if viewer else w//(6*scale); visible_rows=max(1,min(28,h//(10*scale)-4)) if viewer else h//(10*scale)
    foreground=bytes([218,228,238]) if viewer else bytes([255]*3)
    font={}
    for key,values in re.findall(r"(b'(?:\\.|[^'])+'|\d+) => \[([\d, ]+)\]",(ROOT/'framebuffer/src/lib.rs').read_text()):
        c=ast.literal_eval(key);c=c[0] if isinstance(c,bytes) else c
        font[tuple(map(int,values.split(',')))]=chr(c)
    font[(0,)*7]=' '
    rows=[]
    for row in range(visible_rows):
        line=''
        for col in range(columns):
            bits=[]
            for y in range(7):
                v=0
                for x in range(5):
                    at=((((20 if viewer else 0)+row*10+y)*scale)*w+((12 if viewer else 0)+col*6+x)*scale)*3
                    v=(v<<1)|(pixels[at:at+3]==foreground)
                bits.append(v)
            line+=font.get(tuple(bits),'?')
        rows.append(line)
    return rows


def main():
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('--firmware-dir',required=True,type=Path)
    p.add_argument('--qemu',default='qemu-system-x86_64')
    p.add_argument('--output',type=Path,default=ROOT/'out/external-elf')
    p.add_argument('--reference',type=Path,default=ROOT/'out/v3-gop-max-pixels',help='Verified viewer pixels for the selected GOP mode')
    p.add_argument('--boot-from-data', action='store_true', help='Boot FAT32 cases directly from the read-only GPT USB image')
    p.add_argument('--partition-type', choices=['basic-data','esp'], default='basic-data')
    p.add_argument('--case',default='all',help='Functional cases by default; timing-16384/1048576/16777216 are explicit measurements')
    p.add_argument('--extra-empty-xhci',action='store_true',help='Add an empty controller for attach-wait regression')
    p.add_argument('--diagnostic-timing',action='store_true',help='Require retained timing records; sequential diagnostics keep the normal lifecycle')
    p.add_argument('--hub-depth',type=int,choices=range(1,6),help='USB2 hub chain for the keyboard (Hello timing regression)')
    p.add_argument('--app-log-level',choices=['quiet','ready','phases','devices','trace'],default='ready',help='Display level for held real-app console checks')
    p.add_argument('--hold-app-log',action='store_true',help='Require real app start/log/exit held on the shared console; uses app-log-screen-probe')
    p.add_argument('--efi',type=Path,help='Use an already built candidate; --normal supplies screen-only checks')
    p.add_argument('--resolution', help='Explicit GOP mode WIDTHxHEIGHT (EDID disabled)')
    p.add_argument('--start-at',help='Resume the all-case sequence at this named case; earlier reports remain separate')
    p.add_argument('--defer-absent',action='store_true',help='Defer absent reference comparison to a separate --case absent run')
    p.add_argument('--normal',action='store_true',help='Feature-free EFI, screen-only observation; select a hello/exit/fault/log/reject case')
    a=p.parse_args(); out=a.output.resolve();out.mkdir(parents=True,exist_ok=False)
    report={'scope':'local external ELF; no real USB or hardware claim','commands':[],'cases':[],'partition_type':a.partition_type,'boot_from_data':a.boot_from_data}
    def run(cmd,env=None):
        cmd=list(map(str,cmd)); report['commands'].append(cmd)
        return subprocess.check_output(cmd,cwd=ROOT,env=env,stderr=subprocess.STDOUT).decode()
    try:
        (out/'sdk.log').write_text(run(['python3','tools/check-sdk.py','--output',out/'sdk']))
        built=out/'sdk';good=(built/'hello-c.elf').read_bytes()
        cases={n:(good,fmt,'exit',0) for n,fmt in [('hello-fat32','fat32'),('hello-exfat','exfat'),('hello-split','fat32'),('hello-no-keyboard','fat32')]}
        sysroot=Path(run(['rustc','--print','sysroot']).strip());host=run(['rustc','-vV']).split('host: ')[1].splitlines()[0]
        linkenv=dict(os.environ,DYLD_LIBRARY_PATH=str(sysroot/'lib'))
        flags=['--target=x86_64-unknown-none-elf','-isystem',ROOT/'sdk/sysroot/include','-ffreestanding','-fno-builtin','-fno-stack-protector','-fno-pic','-mno-red-zone','-march=x86-64','-mno-avx','-O2','-Wall','-Wextra','-Werror']
        for number,name in enumerate(['exit-42','fault-pf','fault-ud','log-long','log-utf8']):
            obj=out/(name+'.o');elf=out/(name+'.elf')
            run(['clang',*flags,*(['-DEXTERNAL_OBSERVE_LOG'] if a.normal else []),'-DEXTERNAL_CASE='+str(number),'-c',ROOT/'sdk/tests/external-c.c','-o',obj])
            run([sysroot/'lib/rustlib'/host/'bin/rust-lld','-flavor','gnu','-static','--no-undefined','-T',ROOT/'sdk/linker/app.ld','-o',elf,*[built/(n+'.o') for n in ['crt0','libmusha','memory','note']],obj],linkenv)
            cases[name]=(elf.read_bytes(),'fat32','fault' if name.startswith('fault') else 'exit',42 if name=='exit-42' else 0)
        ph=struct.unpack_from('<Q',good,32)[0];headers=[ph+i*56 for i in range(struct.unpack_from('<H',good,56)[0])]
        loads=[h for h in headers if struct.unpack_from('<I',good,h)[0]==1];note=next(h for h in headers if struct.unpack_from('<I',good,h)[0]==4);off=struct.unpack_from('<Q',good,note+8)[0]
        for name,pos,fmt,value in [('bad-magic',0,'I',0),('wx',loads[0]+4,'I',7),('dynamic',loads[0],'I',2),('entry',24,'Q',0),('file-overflow',loads[0]+8,'Q',2**64-1),('note-version',off+24,'I',99),('note-permission',off+44,'Q',2),('overlap',loads[1]+16,'Q',struct.unpack_from('<Q',good,loads[0]+16)[0]),('missing-note',note,'I',0)]:
            bad=bytearray(good);struct.pack_into('<'+fmt,bad,pos,value);cases['malformed-'+name]=(bad,'fat32','reject','Invalid ELF or ABI NOTE')
        large_arena=bytearray(good);struct.pack_into('<Q',large_arena,off+52,64*1024*1024)
        cases['elf-memory']=(large_arena,'fat32','reject','ELF_MEMORY')
        cases['cpu-reject']=(good,'fat32','reject','CPU')
        cases.update({'too-large':(good,'fat32','reject','File size outside'), 'too-small':(b'X'*63,'fat32','reject','File size outside'),'duplicate':(good,'fat32','reject','Ambiguous AUTORUN.ELF name')})
        for size in (16384,1048576,16777216):
            cases['timing-'+str(size)]=(good.ljust(size,b'\0'),'fat32','timing',0)
        cases['timeout']=(good.ljust(1048576,b'\0'),'fat32','reject','File read deadline exceeded')
        cases['disconnect-while-reading']=(good.ljust(16777216,b'\0'),'fat32','disconnect',0)
        cases['hello-high-resolution']=(good,'fat32','exit',0)
        cases['absent']=(None,'fat32','absent',0)
        for delay in [200,800,5000]:
            cases['hotplug-'+str(delay)]=(good,'fat32','no-medium' if delay==5000 else 'exit',0)
        if a.case!='all' and a.case not in cases: raise ValueError('Unknown case '+a.case)
        if a.normal and a.case != 'all' and cases[a.case][2] in ('timing','disconnect'):
            raise ValueError('This observation requires the debug candidate')
        features=[] if a.normal else ['external-elf-probe']
        if a.hold_app_log:
            if a.case not in ('hello-fat32','exit-42','fault-ud','malformed-bad-magic','too-small'): raise ValueError('unsupported held-app case')
            features.append('app-log-screen-probe')
        if not a.normal and (a.diagnostic_timing or a.case=='hotplug-5000'):
            features.append('diagnostics-detail')
        env=dict(os.environ,CARGO_TARGET_DIR=str(out/'target'),MUSHA_RING3_CASE='normal')
        if a.efi:
            efi=a.efi.resolve()
        else:
            (out/'build.log').write_text(run(['cargo','build','--locked','--release','--target','x86_64-unknown-uefi','-p','musha-boot']+(['--features',','.join(features)] if features else []),env))
            efi=out/'target/x86_64-unknown-uefi/release/musha-boot.efi'
        report['features']=features
        report['efi_sha256']=hashlib.sha256(efi.read_bytes()).hexdigest()
        esp=out/'esp/EFI/BOOT';esp.mkdir(parents=True);shutil.copyfile(efi,esp/'BOOTX64.EFI')
        boot_drive=f'if=ide,format=raw,file=fat:rw:{out/"esp"}'
        if not a.normal:
            module('smoke-startup').fixture(out/'boot.img',efi,('log='+a.app_log_level+'\n').encode() if a.hold_app_log else b'log=devices\n')
            boot_drive=f'if=ide,format=raw,snapshot=on,file={out/"boot.img"}'
        if a.start_at and (a.case!='all' or a.start_at not in cases): raise ValueError('start-at requires an all-case member')
        started=a.start_at is None
        for name,(payload,fmt,kind,detail) in cases.items():
            if not started:
                if name!=a.start_at:continue
                started=True
            if a.case not in ('all',name): continue
            if a.defer_absent and name=='absent':
                report['deferred_cases']=['absent'];continue
            if a.case == 'all' and (kind == 'timing' or name.startswith('hotplug-')): continue
            hold_terminal=('app: AUTORUN.ELF faulted, vector=6' if kind=='fault' else 'app: AUTORUN.ELF rejected,' if kind=='reject' else 'app: AUTORUN.ELF exited, status '+str(detail))
            hotplug=int(name.split('-')[1]) if name.startswith('hotplug-') else None
            if hotplug is not None and (a.normal or a.boot_from_data): raise ValueError('Hotplug requires debug EFI and IDE boot')
            if a.normal and kind in ('timing','disconnect'): continue
            d=out/name;d.mkdir();data=d/'data.img'
            if kind=='absent':
                if a.normal:
                    normal=efi
                else:
                    normalenv=dict(os.environ,CARGO_TARGET_DIR=str(out/'normal-target'))
                    (d/'build.log').write_text(run(['cargo','build','--locked','--release','--target','x86_64-unknown-uefi','-p','musha-boot'],normalenv))
                    normal=out/'normal-target/x86_64-unknown-uefi/release/musha-boot.efi'
                fixture(data,normal,None,'fat32', partition_type=a.partition_type)
                reference=a.reference.resolve()
                (d/'pixels.log').write_text(run(['python3','tools/smoke-normal-viewer.py','--firmware-dir',a.firmware_dir,'--qemu',a.qemu,'--boot-image',data,'--data-image',data,'--bootfile-absent','--reference',reference,'--output',d/'pixels'] + (['--resolution',a.resolution] if a.resolution else [])))
                report['cases'].append({'case':name,'passed':True,'normal_efi_sha256':hashlib.sha256(normal.read_bytes()).hexdigest(),'reference':str(reference),'pixels':str(d/'pixels/result.json')});print('PASS external ELF '+name,flush=True);continue
            fixture(data,efi,payload,fmt,16777217 if name=='too-large' else None, partition_type=a.partition_type)
            if not a.normal and fmt=='fat32':
                module('smoke-startup').configure_fixture(data,('log='+a.app_log_level+'\n').encode() if a.hold_app_log else b'log=devices\n')
            if name=='duplicate':
                with data.open('r+b') as f:
                    f.seek(2048*512);boot=f.read(512);root=(2048+32+2*struct.unpack_from('<I',boot,36)[0])*512
                    f.seek(root); directory=f.read(512)
                    entry=next(directory[i:i+32] for i in range(0,512,32) if directory[i:i+11]==b'AUTORUN ELF')
                    slot=next(i for i in range(0,512,32) if directory[i]==0)
                    f.seek(root+slot);f.write(entry)
            before=hashlib.sha256(data.read_bytes()).hexdigest()
            shutil.copyfile(a.firmware_dir/'edk2-i386-vars.fd',d/'vars.fd')
            debug=d/'debug.log';debug.write_text('')
            qmp=ROOT/'out'/('ee-'+hashlib.sha256(str(d).encode()).hexdigest()[:10]+'.sock');assert not qmp.exists()
            cmd=[a.qemu,'-machine','q35,accel=tcg','-cpu',('qemu64,-syscall,+nx' if name=='cpu-reject' else 'qemu64,+syscall,+nx'),'-m','256M',
                 '-drive',f'if=pflash,format=raw,readonly=on,file={a.firmware_dir/"edk2-x86_64-code.fd"}',
                 '-drive',f'if=pflash,format=raw,file={d/"vars.fd"}',
                 '-drive',boot_drive,
                 '-drive',f'if=none,id=data,format=raw,readonly=on,file={data}',
                 '-device','qemu-xhci,id=x','-device','usb-storage,id=storage,drive=data,bus=x.0,port=1',
                 '-qmp',f'unix:{qmp},server=on,wait=off','-netdev','user,id=n','-device','e1000e,netdev=n',
                 '-vga','std','-display','none','-monitor','none','-serial','none',
                 '-debugcon',f'file:{debug}','-global','isa-debugcon.iobase=0xe9','-no-reboot']
            if a.boot_from_data and fmt == 'fat32':
                at=cmd.index(boot_drive)
                del cmd[at-1:at+1]
            if hotplug is not None:
                at=cmd.index('usb-storage,id=storage,drive=data,bus=x.0,port=1');del cmd[at-1:at+1]
            if a.resolution:
                w,h=map(int,a.resolution.lower().split('x'));cmd+=['-global',f'VGA.xres={w}','-global',f'VGA.yres={h}','-global','VGA.edid=off','-global','VGA.vgamem_mb=64']
            if name=='hello-high-resolution' and not a.resolution: cmd+=['-global','VGA.xres=3840','-global','VGA.yres=2160','-global','VGA.vgamem_mb=64']
            if name=='hello-split': cmd+=['-device','qemu-xhci,id=k']
            if a.extra_empty_xhci: cmd+=['-device','qemu-xhci,id=empty']
            if a.hub_depth:
                if name != 'hello-fat32': raise RuntimeError('--hub-depth requires --case hello-fat32')
                for level in range(a.hub_depth):
                    cmd+=['-device','usb-hub,ports=4,port-power=on,bus=x.0,port=3'+'.1'*level]
                cmd+=['-device','usb-kbd,bus=x.0,usb_version=1,port=3'+'.1'*a.hub_depth]
            elif name!='hello-no-keyboard' and hotplug is None: cmd+=['-device','usb-kbd,bus='+('k.0' if name=='hello-split' else 'x.0')]
            report['commands'].append(cmd)
            with (d/'qemu.log').open('w') as log:
                proc=subprocess.Popen(cmd,stdout=log,stderr=log)
                try:
                    end=time.monotonic()+120
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
                        control('qmp_capabilities');removed=False;loading=False;attach_seen=None;added=False;hold_seen=None;observed_log=False
                        while time.monotonic()<end:
                            text=debug.read_text()
                            if hotplug is not None:
                                now=time.monotonic()
                                if attach_seen is None and 'MUSHA: ATTACH WAIT START' in text: attach_seen=now
                                if hold_seen is None and 'ERROR: storage: no readable boot media found' in text: hold_seen=now
                                if attach_seen is not None and not added and now-attach_seen>=hotplug/1000:
                                    control('device_add',{'driver':'usb-storage','id':'storage','drive':'data','bus':'x.0','port':'1'});added=True
                                    (d/'hotplug.json').write_text(json.dumps({'delay_ms':hotplug,'actual_host_delay_ms':(now-attach_seen)*1000,'hold_before_add':hold_seen is not None},indent=2)+'\n')
                                if kind=='no-medium' and added and hold_seen is not None: break
                            if 'EXTERNAL_ELF_READING' in text and not loading:
                                control('screendump',{'filename':str(d/'loading.ppm')});loading=True
                                if kind=='disconnect': control('device_del',{'id':'storage'});removed=True
                            if a.normal:
                                control('screendump',{'filename':str(d/'screen.ppm')})
                                shown='\n'.join(screen_rows(d/'screen.ppm', viewer=not a.hold_app_log))
                                if name in ('log-long','log-utf8') and not observed_log:
                                    log_rows=screen_rows(d/'screen.ppm')
                                    log_screen='\n'.join(log_rows)
                                    if ('Z'*20 in log_screen if name=='log-long' else 'END' in log_screen):
                                        shutil.copyfile(d/'screen.ppm',d/'app-log.ppm')
                                        (d/'app-log.txt').write_text(log_screen+'\n')
                                        observed_log=True
                                if a.hold_app_log and hold_terminal in shown: break
                                if ('ELF exited: status' in shown or 'ELF faulted: vector' in shown or 'ELF refused.' in shown): break
                            elif a.hold_app_log and 'APP_LOG_SCREEN_HOLD' in text and text.endswith('MUSHA: RING_END\n'): break
                            elif text.endswith('\n') and 'APP_ARENA_SCRUBBED VIEWER_RESTART' in text and 'VIEWER_FRAME ' in text.split('APP_ARENA_SCRUBBED VIEWER_RESTART')[-1]:
                                break
                            if proc.poll() is not None: raise RuntimeError((d/'qemu.log').read_text())
                            time.sleep(.02)
                        else: raise RuntimeError('Timeout '+text)
                        # Debug rejection marker precedes its draw; poll pixels until final screen settles.
                        time.sleep(.3);control('screendump',{'filename':str(d/'screen.ppm')})
                    if not a.normal and len(payload)>=1048576 and name!='too-large':
                        loading_text='\n'.join(screen_rows(d/'loading.ppm'));(d/'loading.txt').write_text(loading_text+'\n')
                        assert 'app: loading AUTORUN.ELF' in text,text
                    if name=='hello-high-resolution':
                        import re
                        dimensions=re.match(rb'P6\s+(\d+)\s+(\d+)',(d/'screen.ppm').read_bytes())
                        assert dimensions and tuple(map(int,dimensions.groups()))==(4096,2160)
                    rows=screen_rows(d/'screen.ppm', viewer=not a.hold_app_log);screen='\n'.join(rows);(d/'screen.txt').write_text(screen+'\n')
                    if a.hold_app_log:
                        sequence=[] if a.app_log_level=='quiet' else ['Musha-OS 0.4.0 ready']
                        if name!='too-small': sequence.append('app: starting AUTORUN.ELF')
                        if name=='hello-fat32': sequence.append('Hello Musha-OS!')
                        sequence.append(hold_terminal)
                        positions=[screen.index(line) for line in sequence]
                        assert positions==sorted(positions),screen
                        if a.app_log_level=='quiet': assert 'Musha-OS 0.4.0 ready' not in screen,screen
                        if kind=='fault': assert 'error=n/a' in screen,screen
                        if a.normal:
                            assert text=='','Normal held EFI emitted debug output'
                            if name=='hello-fat32' and a.app_log_level=='ready':
                                (d/'console-transcript.txt').write_text('\n'.join(sequence)+'\n')
                        else:
                            primary=[line.split('MUSHA: LOG ',1)[1] for line in text.splitlines() if 'MUSHA: LOG ' in line]
                            joined='\n'.join(primary)
                            ordered=['Musha-OS 0.4.0 ready']+[x for x in sequence if x!='Musha-OS 0.4.0 ready']
                            locations=[joined.index(x) for x in ordered]
                            assert locations==sorted(locations),joined
                            assert text.index('XHCI_QUIESCED DMA_DISABLED')<text.index('MUSHA: LOG Musha-OS 0.4.0 ready'),text
                            retained=text.rsplit('MUSHA: RING_BEGIN\n',1)[1].split('MUSHA: RING_END',1)[0]
                            assert all(x in retained for x in ordered),retained
                            if kind=='reject':
                                assert 'RING3_ELF_ENTERING' not in text, 'rejected app entered Ring3'
                            if name=='too-small':
                                assert 'app: starting AUTORUN.ELF' not in joined, 'reader rejection started app'
                            assert 'app: loading AUTORUN.ELF' in retained, 'filtered trace lost from retained ring'
                        assert before==hashlib.sha256(data.read_bytes()).hexdigest()
                        report['cases'].append({'case':name,'passed':True,'application_console_sequence':sequence,'normal_debug_empty':a.normal,'screen':str(d/'screen.ppm')})
                        print('PASS real shared application console '+name,flush=True)
                        continue
                    if not a.normal and kind in ('exit','fault'):
                        handoff=text.index('EXTERNAL_ELF_DEVICES_STOPPED')
                        assert text.index('EXTERNAL_ELF_READ_COMPLETE') < text.index('XHCI_QUIESCED',text.index('EXTERNAL_ELF_READ_COMPLETE')) < handoff < text.index('RING3_ELF_ENTERING'),text
                        assert handoff < text.index('APP_RECOVERED') < text.index('APP_ARENA_SCRUBBED VIEWER_RESTART'),text
                        if a.diagnostic_timing:
                            assert 'time: bdf=0x' in text,text
                        assert text.index('NET_QUIESCED DMA_DISABLED') < handoff,text
                        assert text.index('OS_READY EXTERNAL_APP') < text.index('APP_ENTERING EXTERNAL_APP') < text.index('APP_RUNNING EXTERNAL_APP'),text
                        assert 'OS_READY FILE_VIEWER' not in text,text
                    if hotplug is not None:
                        assert added and attach_seen is not None
                        rescans=text.count('MUSHA: RESCAN 1/1 START')
                        assert rescans in (0,1),text
                        assert text.count('XHCI_RESET_OK')==(1 if kind=='no-medium' else 4),text
                        if rescans:
                            rescan_text=text.split('MUSHA: RESCAN 1/1 START',1)[1].split('MUSHA: RESCAN 1/1 DONE',1)[0]
                            assert 'XHCI_RESET_OK' not in rescan_text,rescan_text
                            assert text.count('MUSHA: RESCAN 1/1 DONE')==1,text
                        else:
                            # Arrival during the first bounded attach wait needs
                            # no rescan. Require actual readable data before its
                            # first host stop, not merely a plugged device.
                            assert kind=='exit',text
                            assert text.index('STORAGE_PROBE_OK READ_ONLY')<text.index('XHCI_QUIESCED'),text
                            assert text.index('FAT32_FILE_OK')<text.index('EXTERNAL_ELF_READING'),text
                        (d/'attach-path.json').write_text(json.dumps({'rescans':rescans,'path':'rescan' if rescans else 'first bounded attach wait'},indent=2)+'\n')
                        if kind=='exit': assert 'Hello Musha-OS!' in text and 'ELF exited: status 0' in screen,screen+text
                    if kind=='no-medium':
                        assert hold_seen is not None and hold_seen-attach_seen<5,text
                        assert 'ERROR: storage: no readable boot media found' in text and 'Musha-OS halted' in text and 'RING3_ELF_ENTERING' not in text and 'MUSHA: OS_READY ' not in text,text
                        assert 'MUSHA: RESCAN 1/1 DONE' in text,text
                    if kind=='exit':
                        assert f'ELF exited: status {detail}.' in screen,screen
                        if not a.normal:
                            assert 'RING3_ELF_EXIT_STATUS='+format(detail,'016X') in text,text
                            if name.startswith('hello'): assert 'Hello Musha-OS!' in text,text
                        if name in ('log-long','log-utf8'):
                            if a.normal:
                                assert observed_log, 'Application log was not observed before return'
                                log_rows=(d/'app-log.txt').read_text().splitlines()
                                log_screen='\n'.join(log_rows)
                            else:
                                log_screen=text
                                log_rows=[]
                            if name=='log-long':
                                if a.normal:
                                    columns=len(log_rows[0]); rows=len(log_rows)
                                    expected=[[' ']*columns for _ in range(rows)]
                                    row=column=0
                                    def newline():
                                        nonlocal row,column
                                        column=0
                                        if row+1<rows: row+=1
                                        else: expected.pop(0);expected.append([' ']*columns)
                                    for ch in 'Musha-OS 0.4.0 ready\napp: starting AUTORUN.ELF\n'+'Z'*4096:
                                        if ch=='\n': newline();continue
                                        if column==columns: newline();column=2
                                        expected[row][column]=ch;column+=1
                                    assert log_rows==[''.join(line) for line in expected],log_screen
                                else: assert 'Z'*4096 in text,text
                            else:
                                if a.normal:
                                    assert 'ASCII..... ......' in log_screen and 'END' in log_screen,log_screen
                                    assert '\x1b' not in log_screen and '\x01' not in log_screen,log_screen
                                else: assert b'ASCII\t\r\x1b\x01\x7f \xe6\x97\xa5\xe6\x9c\xac\nEND\n' in debug.read_bytes(),text
                    elif kind=='fault':
                        vector=14 if name=='fault-pf' else 6
                        assert f'ELF faulted: vector {vector}.' in screen,screen
                        if not a.normal:
                            assert 'RING3_FAULT_KERNEL_STACK_OK' in text and 'C3_APP_FAULT VECTOR='+format(vector,'016X') in text,text
                            assert 'APP_RECOVERED KIND=0000000000000001 DETAIL='+format(vector,'016X') in text,text
                    elif kind in ('reject','disconnect'):
                        assert 'RING3_ELF_ENTERING' not in text,screen+text
                        if kind=='reject':
                            assert 'ELF refused.' in screen,screen
                            if not a.normal:
                                if name.startswith('malformed-'):
                                    assert 'RING3_REJECT ELF_FORMAT' in text,text
                                elif name in ('elf-memory','cpu-reject'):
                                    assert 'RING3_REJECT '+detail in text,text
                                elif name in ('too-large','too-small','duplicate'):
                                    assert 'EXTERNAL_ELF_READ_REJECTED' in text and 'LIVE_FILE_REQUEST' not in text,text
                                elif name=='timeout':
                                    import re
                                    elapsed=int(re.search(r'EXTERNAL_ELF_ELAPSED_MS=([0-9A-F]+)',text).group(1),16)
                                    assert elapsed>=5000 and 'LIVE_FILE_REQUEST' in text and 'EXTERNAL_ELF_READ_COMPLETE' not in text,text
                                else: raise AssertionError('Missing refusal cause check '+name)
                        else:
                            assert removed and 'XHCI_QUIESCED' in text and 'APP_ARENA_SCRUBBED VIEWER_RESTART' in text,text
                    elif kind=='timing': assert 'EXTERNAL_ELF_ELAPSED_MS=' in text,text
                    if a.normal: assert text=='', 'Normal EFI emitted debug output'
                    assert before==hashlib.sha256(data.read_bytes()).hexdigest()
                    result={'case':name,'passed':True,'image_sha256':before,'screen':str(d/'screen.txt')}
                    if not a.normal and 'EXTERNAL_MEMORY IMAGE' in text:
                        import re
                        ranges={k:(int(lo,16),int(hi,16)) for k,lo,hi in re.findall(r'EXTERNAL_MEMORY (\w+) ([0-9A-F]+) ([0-9A-F]+)',text)}
                        image,staging,arena=(ranges[k] for k in ('IMAGE','STAGING','ARENA'))
                        assert image[0]<=staging[0]<staging[1]<=image[1]
                        assert 0x2000000<=arena[0]<arena[1]<=0x10000000
                        for k in ('ARENA','USB_DMA','NET_DMA'):
                            lo,hi=ranges[k]; assert hi<=staging[0] or lo>=staging[1]
                        result['memory_ranges']=ranges
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
