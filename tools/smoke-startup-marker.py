#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""Negative marker tests on disposable read-only QEMU images, never real USB."""
import argparse,json,shutil,struct,subprocess,time
from pathlib import Path
from distribution import digest
p=argparse.ArgumentParser(description=__doc__);p.add_argument('--image',required=True,type=Path);p.add_argument('--firmware-dir',required=True,type=Path);p.add_argument('--output',required=True,type=Path);p.add_argument('--qemu',default='qemu-system-x86_64');a=p.parse_args()
a.output.mkdir(parents=True,exist_ok=False);source=a.image.resolve();original=digest(source);report={'scope':'Disposable QEMU images only','source_sha256':original,'cases':[]}
for case in ('contents','length','missing'):
    out=(a.output/case).resolve();out.mkdir();image=out/'test.img';shutil.copyfile(source,image)
    with image.open('r+b') as f:
        f.seek(2048*512);bpb=f.read(512);spc=bpb[13];fat=struct.unpack_from('<I',bpb,36)[0];data=2048+32+2*fat
        f.seek(data*512);root=f.read(spc*512);slot=next(i for i in range(0,len(root),32) if root[i:i+11]==b'README  TXT');entry=root[slot:slot+32]
        if case=='contents':
            cluster=(struct.unpack_from('<H',entry,20)[0]<<16)|struct.unpack_from('<H',entry,26)[0];f.seek((data+(cluster-2)*spc)*512);b=f.read(1);f.seek(-1,1);f.write(bytes([b[0]^1]))
        elif case=='length':f.seek(data*512+slot+28);f.write(struct.pack('<I',struct.unpack_from('<I',entry,28)[0]+1))
        else:f.seek(data*512+slot);f.write(b'MISSING TXT')
    before=digest(image);shutil.copyfile(a.firmware_dir/'edk2-i386-vars.fd',out/'vars.fd');log=out/'debug.log'
    cmd=[a.qemu,'-machine','q35,accel=tcg','-m','256M','-drive',f'if=pflash,format=raw,readonly=on,file={a.firmware_dir/"edk2-x86_64-code.fd"}',
         '-drive',f'if=pflash,format=raw,file={out/"vars.fd"}','-drive',f'if=none,id=data,format=raw,readonly=on,file={image}',
         '-device','qemu-xhci,id=usbhost','-device','usb-storage,drive=data,bus=usbhost.0','-device','usb-kbd,bus=usbhost.0',
         '-vga','std','-display','none','-debugcon',f'file:{log}','-global','isa-debugcon.iobase=0xe9','-no-reboot']
    expected='device: file readme.txt missing' if case=='missing' else 'XHCI_FAILED README MARKER'
    with (out/'qemu.log').open('w') as stream:
        proc=subprocess.Popen(cmd,stdout=stream,stderr=subprocess.STDOUT)
        try:
            deadline=time.monotonic()+45
            while True:
                text=log.read_text(errors='replace') if log.exists() else ''
                if expected in text:break
                if proc.poll() is not None or time.monotonic()>deadline:raise RuntimeError('Missing rejection '+expected+'\n'+text[-2000:])
                time.sleep(.1)
            assert 'FAT32_FILE_OK' not in text,'Invalid marker accepted'
        finally:
            proc.terminate()
            try:proc.wait(timeout=5)
            except subprocess.TimeoutExpired:proc.kill();proc.wait()
    assert digest(image)==before;report['cases'].append({'case':case,'expected':expected,'passed':True,'image_unchanged':True});print('PASS marker rejection:',case,flush=True)
assert digest(source)==original
(a.output/'report.json').write_text(json.dumps(report,indent=2)+'\n')
