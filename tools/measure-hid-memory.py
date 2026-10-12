#!/usr/bin/env python3
# Copyright 2026 Hirofumi Iwasaki
# SPDX-License-Identifier: Apache-2.0
"""Measure fixed UEFI layouts and selected optimized prologues; NOT a stack bound."""
import ast
import json
import re
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / 'out/hid-memory'
OUT.mkdir(parents=True, exist_ok=True)

def run(args):
    return subprocess.run(args, cwd=ROOT, check=True, text=True, capture_output=True)

build = run(['cargo', 'rustc', '--locked', '--release', '--target',
             'x86_64-unknown-uefi', '-p', 'musha-input', '--lib',
             '--message-format=json', '--target-dir', str(OUT / 'build'), '--', '--emit=asm,link'])
artifacts = [json.loads(line) for line in build.stdout.splitlines() if line.startswith('{')]
libs = [Path(p) for a in artifacts if a.get('reason') == 'compiler-artifact'
        and a['target']['name'] == 'musha_input' for p in a['filenames'] if p.endswith('.rlib')]
assert len(libs) == 1, libs
lib = libs[0]
assemblies = list((lib.parent / 'deps').glob('musha_input-*.s'))
assert len(assemblies) == 1, assemblies
assembly = assemblies[0]
probe = OUT / 'probe.s'
run(['rustc', '--edition=2024', '--crate-type=lib', '--target=x86_64-unknown-uefi',
     '-C', 'opt-level=3', '-C', 'panic=abort', '-C', 'no-redzone=yes',
     '--extern', f'musha_input={lib}', '-L', f'dependency={lib.parent / "deps"}',
     '--emit=asm', '-o', str(probe), 'tools/hid-stack-probe.rs'])
text = assembly.read_text()
(OUT / 'input.s').write_text(text)
ptext = probe.read_text()

def body(source, suffix):
    matches = list(re.finditer(r'^([^\s:]*' + re.escape(suffix) + r'):\n', source, re.M))
    assert len(matches) == 1, (suffix, len(matches))
    start = matches[0].end()
    return source[start:].split('\n\t.def\t', 1)[0]

def frame(source, suffix):
    # Only the pinned compiler's straight-line prologue. Fail on unfamiliar form.
    lines = body(source, suffix).splitlines()
    pushes = 0
    eax = None
    for line in lines[:24]:
        if re.fullmatch(r'\s*pushq\s+%\w+', line):
            pushes += 8
        m = re.fullmatch(r'\s*movl\s+\$(\d+), %eax', line)
        if m:
            eax = int(m[1])
        m = re.fullmatch(r'\s*subq\s+\$(\d+), %rsp', line)
        if m:
            return pushes + int(m[1])
        if re.fullmatch(r'\s*subq\s+%rax, %rsp', line):
            assert eax is not None
            return pushes + eax
        if re.fullmatch(r'\s*retq', line):
            return pushes
    raise RuntimeError(f'Unrecognized prologue: {suffix}; inspect assembly manually')

sizes = {}
for name in ['HID_LAYOUT_SIZE', 'HID_DECODER_SIZE', 'HID_LAYOUT_ALIGN', 'HID_DECODER_ALIGN']:
    m = re.search(r'^' + name + r':\n\s*\.asciz\s+(".*")', ptext, re.M)
    assert m, name
    raw = ast.literal_eval(m[1]).encode('latin1') + b'\0'
    assert len(raw) == 8, name
    sizes[name] = int.from_bytes(raw, 'little')
frames = {name: frame(source, symbol) for name, source, symbol in [
    ('Layout::parse', text, '6Layout5parse'),
    ('Decoder::new', text, '7Decoder3new'),
    ('construct probe', ptext, 'hid_probe_construct'),
    ('update probe', ptext, 'hid_probe_update'),
    ('release probe', ptext, 'hid_probe_release'),
]}
result = {'rustc': run(['rustc','-vV']).stdout, 'target': 'x86_64-unknown-uefi',
          'sizes_bytes': sizes, 'prologue_bytes_including_saved_registers': frames,
          'limitations': 'Pre-link optimized code; excludes return addresses, callees, callbacks, interrupts and final LTO effects. Not runtime high-water or a worst-case bound.'}
(OUT / 'report.json').write_text(json.dumps(result, indent=2) + '\n')
print(json.dumps(result, indent=2))
