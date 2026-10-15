#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
from pathlib import Path
import os, subprocess, struct, json
import argparse
r = Path(__file__).resolve().parents[1]
p = argparse.ArgumentParser()
p.add_argument('--output', required=True, type=Path)
a = p.parse_args()
o = a.output.resolve()
o.mkdir(parents=True, exist_ok=True)
subprocess.run(['python3', 'tools/check-sdk.py', '--c-only', '--output', str(o)], cwd=r, check=True, stdout=(o / 'sdk.log').open('w'), stderr=subprocess.STDOUT)
sysroot = Path(subprocess.check_output(['rustc', '--print', 'sysroot'], text=True).strip())
host = subprocess.check_output(['rustc', '-vV'], text=True).split('host: ')[1].splitlines()[0]
lld = sysroot / 'lib/rustlib' / host / 'bin/rust-lld'
env = dict(os.environ, DYLD_LIBRARY_PATH=str(sysroot / 'lib'))
flags = ['--target=x86_64-unknown-none-elf', '-isystem', str(r / 'sdk/sysroot/include'), '-I', str(r / 'sdk/src'), '-ffreestanding', '-fno-builtin', '-fno-stack-protector', '-fno-pic', '-mno-red-zone', '-march=x86-64', '-mno-avx', '-O2', '-Wall', '-Wextra', '-Werror']
k = json.loads((r / 'sdk/abi/experimental.json').read_text())['constants']
note = struct.pack('<III', 6, k['NOTE_SIZE'], k['NOTE_TYPE']) + b'MUSHA\x00\x00\x00' + struct.pack('<IIHHIQQQQQQ', k['NOTE_SIZE'], k['R2_REVISION'], k['ABI_MAJOR'], k['R2_MINOR'], k['CPU_PROFILE'], k['FEATURE_BASE'] | k['FEATURE_INPUT'], 15, 1048576, 65536, 0, 0)
(o / 'note2.S').write_text('.section .note.musha,"a",@note\n.balign 4\n.byte ' + ','.join(map(str, note)) + '\n')
for (name, src) in [('note2', o / 'note2.S')]:
    subprocess.run(['clang', *flags, '-c', str(src), '-o', str(o / (name + '.o'))], check=True)
for (case, name) in [(0, 'io-c'), (1, 'denied'), (2, 'fault'), (3, 'overflow'), (4, 'removed'), (5, 'time-only'), (6, 'wait-only'), (7, 'mouse-only')]:
    subprocess.run(['clang', *flags, '-DIO_CASE=' + str(case), '-c', str(r / 'sdk/tests/io-c.c'), '-o', str(o / (name + '.o'))], check=True)
    note_object = o / 'note2.o'
    if case in (1, 5, 6):
        data = bytearray(note)
        struct.pack_into('<Q', data, 44, {1: 1, 5: 3, 6: 5}[case])
        (o / 'denied-note.S').write_text('.section .note.musha,"a",@note\n.balign 4\n.byte ' + ','.join(map(str, data)) + '\n')
        subprocess.run(['clang', *flags, '-c', str(o / 'denied-note.S'), '-o', str(o / 'denied-note.o')], check=True)
        note_object = o / 'denied-note.o'
    subprocess.run([str(lld), '-flavor', 'gnu', '-static', '--no-undefined', '-T', str(r / 'sdk/linker/app.ld'), '-o', str(o / (name + '.elf')), *[str(o / (x + '.o')) for x in ['crt0', 'libmusha', 'memory']], str(note_object), str(o / (name + '.o'))], env=env, check=True)
    subprocess.run(['python3', str(r / 'sdk/tools/app-check.py'), str(o / (name + '.elf'))], check=True)
