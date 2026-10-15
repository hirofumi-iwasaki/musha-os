#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""Execute rebuilt-core Rust samples in the existing CPL3 loader, never on host."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import time
ROOT = Path(__file__).resolve().parents[1]
p = argparse.ArgumentParser(description=__doc__)
p.add_argument('--case', choices=['all','fp'], default='all')
p.add_argument('--sdk-build', required=True, type=Path)
p.add_argument('--output', required=True, type=Path)
p.add_argument('--firmware-dir', required=True, type=Path)
p.add_argument('--qemu', default='qemu-system-x86_64')
a = p.parse_args()
out = a.output.resolve()
out.mkdir(parents=True, exist_ok=False)
report = {
    'scope': ('local QEMU mixed C/Rust r2 FP and TIME/WAIT boundary; no physical acceptance'
              if a.case == 'fp' else
              'local QEMU embedded Rust samples; no physical acceptance or live r2 INPUT'),
    'cases': [],
}
try:
    for name, marker in ([('fp-rust','R2 MIXED FP OK')] if a.case=='fp' else [('hello-rust', 'Hello Musha-OS from Rust!'), ('start-record-rust', 'Rust start record')]):
        d = out/name
        d.mkdir()
        elf = (a.sdk_build/(name+'.elf')).resolve()
        env = dict(os.environ, MUSHA_USER_ELF=str(elf), CARGO_TARGET_DIR=str(out/'target'))
        with (d/'build.log').open('w') as log:
            subprocess.run(['cargo', 'build', '--locked', '--release', '--target', 'x86_64-unknown-uefi',
                            '-p', 'musha-boot', '--features', 'ring3-elf-probe'],
                           env=env, cwd=ROOT, stdout=log, stderr=subprocess.STDOUT, check=True)
        esp = d/'esp/EFI/BOOT'
        esp.mkdir(parents=True)
        shutil.copyfile(out/'target/x86_64-unknown-uefi/release/musha-boot.efi', esp/'BOOTX64.EFI')
        shutil.copyfile(a.firmware_dir/'edk2-i386-vars.fd', d/'vars.fd')
        debug = d/'debug.log'
        cmd = [a.qemu, '-machine', 'q35,accel=tcg', '-cpu', 'qemu64,+syscall,+nx', '-m', '256M',
               '-drive', f'if=pflash,format=raw,readonly=on,file={a.firmware_dir}/edk2-x86_64-code.fd',
               '-drive', f'if=pflash,format=raw,file={d}/vars.fd',
               '-drive', f'if=ide,format=raw,file=fat:rw:{d}/esp', '-device', 'qemu-xhci',
               '-vga', 'std', '-display', 'none', '-monitor', 'none', '-serial', 'none',
               '-debugcon', f'file:{debug}', '-global', 'isa-debugcon.iobase=0xe9', '-no-reboot']
        (d/'command.json').write_text(json.dumps(cmd, indent=2)+'\n')
        with (d/'qemu.log').open('w') as log:
            proc = subprocess.Popen(cmd, stdout=log, stderr=log)
            try:
                deadline = time.monotonic()+60
                while time.monotonic() < deadline:
                    text = debug.read_text() if debug.exists() else ''
                    if any(end in text for end in ['RING3_EXIT_OK', 'RING3_REJECT', 'EXCEPTION VECTOR=']):
                        break
                    if proc.poll() is not None:
                        raise RuntimeError((d/'qemu.log').read_text())
                    time.sleep(.05)
                else:
                    raise RuntimeError('Timeout: '+text)
                assert 'RING3_ELF_RUNNING' in text and 'RING3_QUERY KERNEL_STACK_OK' in text, text
                assert marker in text and 'RING3_EXIT_OK' in text, text
                assert 'EXCEPTION' not in text and 'RING3_REJECT' not in text, text
                if name == 'start-record-rust':
                    for line in ['arena base  0x0000000010000000', 'arena bytes 0x0000000000100000',
                                 'stack base  0x000000006FFF0000', 'stack bytes 0x0000000000010000']:
                        assert line in text, text
                report['cases'].append({'case': name, 'passed': True, 'elf_sha256': hashlib.sha256(elf.read_bytes()).hexdigest()})
                print('PASS Rust CPL3 '+name, flush=True)
            finally:
                proc.terminate()
                try:
                    proc.wait(timeout=5)
                except subprocess.TimeoutExpired:
                    proc.kill()
                    proc.wait()
except Exception as error:
    report['failure'] = str(error)
    raise
finally:
    (out/'report.json').write_text(json.dumps(report, indent=2)+'\n')
