#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""Build no_std SSE2 guest ELFs; never execute application syscalls on the host."""
import argparse
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import subprocess

SDK = Path(__file__).resolve().parents[1]
p = argparse.ArgumentParser(description=__doc__)
p.add_argument('--output', required=True, type=Path)
p.add_argument('--mixed-fp', action='store_true', help='Link C/Rust integer/FP/struct/stack conformance guest (requires --r2)')
p.add_argument('--r2', action='store_true', help='Build experimental r2 NOTE with time/wait/input permissions')
a = p.parse_args()
if a.mixed_fp and not a.r2: p.error('--mixed-fp requires --r2')
out = a.output.resolve()
out.mkdir(parents=True, exist_ok=True)
commands = []
def run(args, **kwargs):
    args = list(map(str, args))
    commands.append(args)
    return subprocess.check_output(args, stderr=subprocess.STDOUT, **kwargs).decode()

# Stable toolchain is pinned by the repository. Bootstrap enables build-std only;
# rebuilt core and guest code share the exact SSE2/no-red-zone target specification.
env = dict(os.environ, RUSTC_BOOTSTRAP='1')
versions = {'rustc': run(['rustc', '-vV']), 'clang': run(['clang', '--version']).splitlines()[0]}
flags = ['--target=x86_64-unknown-none-elf', '-mno-red-zone', '-ffreestanding']
run(['clang', *flags, '-c', SDK/'src/crt0.S', '-o', out/'crt0.o'])
k = json.loads((SDK/'abi/experimental.json').read_text())['constants']
import struct
note = struct.pack('<III', 6, 64, k['NOTE_TYPE']) + b'MUSHA\0\0\0' + struct.pack(
    '<IIHHIQQQQQQ', 64, (k['R2_REVISION'] if a.r2 else k['CANDIDATE_REVISION']), k['ABI_MAJOR'], (k['R2_MINOR'] if a.r2 else k['ABI_MINOR']),
    k['CPU_PROFILE'], (3 if a.r2 else k['FEATURE_BASE']), (15 if a.r2 else k['PERMISSION_LOG']), 1048576, 65536, 0, 0)
(out/'note.S').write_text('.section .note.musha,"a",@note\n.balign 4\n.byte '+','.join(map(str, note))+'\n')
run(['clang', *flags, '-c', out/'note.S', '-o', out/'note.o'])
# Cargo fingerprints record the rebuilt core; per-bin link inputs never affect core.
link_args = ['-C', 'link-arg=-T'+str(SDK/'linker/app.ld'), '-C', 'link-arg='+str(out/'crt0.o'),
             '-C', 'link-arg='+str(out/'note.o'), '-C', 'link-arg=--no-undefined',
             '-C', 'link-arg=-static', '-C', 'relocation-model=static', '-C', 'no-redzone=yes']
samples = {}
names = ['hello-rust', 'start-record-rust', 'io-rust'] if a.r2 else ['hello-rust', 'start-record-rust']
if a.mixed_fp:
    names.append('fp-rust')
    run(['clang', *flags, '-O2', '-c', SDK/'tests/fp-bridge.c', '-o', out/'fp-bridge.o'])
for name in names:
    try:
        log = run(['cargo', 'rustc', '--locked', '--manifest-path', SDK/'rust/Cargo.toml',
                   '-p', 'musha-samples', '--bin', name, '--release',
                   '--target', SDK/'targets/x86_64-musha-user.json', '--target-dir', out/'target',
                   '-Zjson-target-spec', '-Zbuild-std=core,compiler_builtins', '-Zbuild-std-features=compiler-builtins-mem',
                   '--', *link_args, *(['-C', 'link-arg='+str(out/'fp-bridge.o')] if name=='fp-rust' else [])], env=env)
    except subprocess.CalledProcessError as error:
        (out/(name+'.log')).write_bytes(error.output)
        raise
    (out/(name+'.log')).write_text(log)
    data = (out/'target/x86_64-musha-user/release'/name).read_bytes()
    target = out/(name+'.elf')
    target.write_bytes(data)
    spec = importlib.util.spec_from_file_location('checker', SDK/'tools/app-check.py')
    checker = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(checker)
    samples[name] = {'sha256': hashlib.sha256(data).hexdigest(), 'bytes': len(data), 'elf': checker.check(data)}
report = {'versions': versions, 'commands': commands, 'samples': samples, 'rust_custom_core_built': True,
          'workspace_lock_sha256': hashlib.sha256((SDK/'rust/Cargo.lock').read_bytes()).hexdigest(),
          'target_sha256': hashlib.sha256((SDK/'targets/x86_64-musha-user.json').read_bytes()).hexdigest(),
          'target': json.loads((SDK/'targets/x86_64-musha-user.json').read_text()),
          'guest_execution': False, 'public_compatibility_fixed': False}
(out/'report.json').write_text(json.dumps(report, indent=2)+'\n')
print(json.dumps(samples, indent=2))
