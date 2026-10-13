#!/usr/bin/env python3
# Copyright 2026 Hirofumi Iwasaki
# SPDX-License-Identifier: Apache-2.0
"""Check a normal (no debug features) candidate against verified QEMU frames.

Reference list.ppm/text.ppm must come from smoke-viewer.py with the same fixture,
GOP mode and source revision. This checks displayed pixels, not internal DMA state.
"""
import argparse
import hashlib
import importlib.util
import json
from pathlib import Path
import shutil
import socket
import subprocess
import time

p = argparse.ArgumentParser(description=__doc__)
p.add_argument('--firmware-dir', required=True, type=Path)
p.add_argument('--boot-image', required=True, type=Path)
p.add_argument('--data-image', required=True, type=Path)
p.add_argument('--reference', required=True, type=Path)
p.add_argument('--output', required=True, type=Path)
p.add_argument('--qemu', default='qemu-system-x86_64')
p.add_argument('--bootfile-absent', action='store_true', help='Derive the normal State::Absent list status from probe reference pixels')
a = p.parse_args()
for path in (a.boot_image, a.data_image, a.reference/'list.ppm', a.reference/'text.ppm'):
    if not path.is_file(): p.error(f'Expected regular file: {path}')
a.output.mkdir(parents=True, exist_ok=False)
out = a.output.resolve()
if a.bootfile_absent:
    spec = importlib.util.spec_from_file_location('viewer_reference', Path(__file__).with_name('viewer-reference.py'))
    reference = importlib.util.module_from_spec(spec); spec.loader.exec_module(reference)
    reference.derive(a.reference, out/'expected')
    a.reference = out/'expected'
f = a.firmware_dir.resolve()
shutil.copyfile(f/'edk2-i386-vars.fd', out/'vars.fd')
qmp = Path(__file__).resolve().parent.parent/'out'/('nv-'+hashlib.sha256(str(out).encode()).hexdigest()[:10]+'.sock')
if qmp.exists(): raise RuntimeError('QMP socket already exists')
log = out/'debug.log'
def sha(path): return hashlib.sha256(path.read_bytes()).hexdigest()
before = [sha(a.boot_image), sha(a.data_image)]
cmd = [a.qemu, '-machine', 'q35,accel=tcg', '-m', '256M', '-netdev', 'user,id=n', '-device', 'e1000e,netdev=n',
       '-drive', f'if=pflash,format=raw,readonly=on,file={f/"edk2-x86_64-code.fd"}',
       '-drive', f'if=pflash,format=raw,file={out/"vars.fd"}',
       '-drive', f'if=ide,format=raw,snapshot=on,file={a.boot_image.resolve()}',
       '-drive', f'if=none,id=data,format=raw,readonly=on,file={a.data_image.resolve()}',
       '-device', 'qemu-xhci,id=x', '-device', 'usb-storage,drive=data,bus=x.0,port=1',
       '-device', 'usb-kbd,bus=x.0', '-vga', 'std', '-display', 'none',
       '-debugcon', f'file:{log}', '-global', 'isa-debugcon.iobase=0xe9',
       '-qmp', f'unix:{qmp},server=on,wait=off', '-no-reboot']
with (out/'qemu.log').open('w') as err:
    proc = subprocess.Popen(cmd, stdout=err, stderr=err)
    try:
        end = time.monotonic()+60
        while not qmp.exists():
            if proc.poll() is not None or time.monotonic()>end: raise RuntimeError('QMP startup failed')
            time.sleep(.1)
        with socket.socket(socket.AF_UNIX) as qs:
            qs.settimeout(10); qs.connect(str(qmp)); stream = qs.makefile('rwb'); stream.readline()
            def control(name, args=None):
                stream.write((json.dumps({'execute':name, 'arguments':args or {}})+'\n').encode()); stream.flush()
                while True:
                    response = json.loads(stream.readline())
                    if 'error' in response: raise RuntimeError(response)
                    if 'return' in response: return
            control('qmp_capabilities')
            def match(name):
                expected = (a.reference/(name+'.ppm')).read_bytes()
                end = time.monotonic()+60
                while True:
                    target = out/(name+'.ppm')
                    control('screendump', {'filename':str(target)})
                    if target.read_bytes()==expected: return
                    if proc.poll() is not None or time.monotonic()>end: raise RuntimeError('Screen mismatch: '+name)
                    time.sleep(.2)
            def key(code):
                control('send-key', {'keys':[{'type':'qcode','data':code}], 'hold-time':30})
                time.sleep(.1)
            match('list'); key('ret'); match('text'); key('esc'); match('list')
            key('q'); time.sleep(2)
            control('screendump', {'filename':str(out/'ended.ppm')})
            assert log.read_bytes()==b'', 'Candidate unexpectedly emits debug output'
            assert before==[sha(a.boot_image), sha(a.data_image)], 'Image changed'
            control('quit'); proc.wait(timeout=5)
        (out/'result.json').write_text(json.dumps({'result':'PASS', 'images_sha256':before,
            'frames':['list','text','list'], 'debug_output_bytes':0}, indent=2)+'\n')
        print('PASS: normal candidate list/open/back pixel match; no debug log; images unchanged')
    finally:
        if proc.poll() is None: proc.terminate(); proc.wait(timeout=5)
        qmp.unlink(missing_ok=True)
