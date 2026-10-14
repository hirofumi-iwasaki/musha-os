#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""Build/boot the opt-in C1 fixture. Local QEMU only; no USB or release actions."""
import argparse, hashlib, json, os, pathlib, shutil, socket, subprocess, time
ROOT = pathlib.Path(__file__).resolve().parents[1]
CASES = ['normal', 'ro', 'nx', 'guard', 'supervisor', 'rsp', 'privileged', 'nmi-transition']
p = argparse.ArgumentParser()
p.add_argument('--firmware-dir', required=True, type=pathlib.Path)
p.add_argument('--qemu', default='qemu-system-x86_64')
p.add_argument('--case', choices=['all', *CASES], default='all')
p.add_argument('--output',type=pathlib.Path)
a = p.parse_args()
out = a.output.resolve() if a.output else ROOT / 'out/ring3'; out.mkdir(parents=True, exist_ok=True)
report = {'scope': 'C1 fixed assembly fixture; not SDK ELF execution', 'cases': [], 'commands': []}
try:
    for case in CASES if a.case == 'all' else [a.case]:
        d = out / case; d.mkdir(exist_ok=True)
        env = dict(os.environ, MUSHA_RING3_CASE=case, CARGO_TARGET_DIR=str(out / 'target'))
        build = ['cargo', 'build', '--locked', '--release', '--target', 'x86_64-unknown-uefi', '-p', 'musha-boot', '--features', 'ring3-probe']
        report['commands'].append({'argv': build, 'MUSHA_RING3_CASE': case, 'CARGO_TARGET_DIR': env['CARGO_TARGET_DIR']})
        with (d / 'build.log').open('w') as log:
            subprocess.run(build, cwd=ROOT, env=env, stdout=log, stderr=log, check=True)
        esp = d / 'esp/EFI/BOOT'; esp.mkdir(parents=True, exist_ok=True)
        efi = esp / 'BOOTX64.EFI'
        shutil.copyfile(out / 'target/x86_64-unknown-uefi/release/musha-boot.efi', efi)
        shutil.copyfile(a.firmware_dir / 'edk2-i386-vars.fd', d / 'vars.fd')
        debug = d / 'debug.log'; debug.write_text('')
        cmd = [a.qemu, '-machine', 'q35,accel=tcg', '-cpu', 'qemu64,+syscall,+nx', '-m', '256M',
               '-drive', f'if=pflash,format=raw,readonly=on,file={a.firmware_dir / "edk2-x86_64-code.fd"}',
               '-drive', f'if=pflash,format=raw,file={d / "vars.fd"}',
               '-drive', f'if=ide,format=raw,file=fat:rw:{d / "esp"}',
               '-device', 'qemu-xhci', '-vga', 'std', '-display', 'none', '-monitor', 'none', '-serial', 'none',
               '-debugcon', f'file:{debug}', '-global', 'isa-debugcon.iobase=0xe9', '-no-reboot']
        qmp = out / 'nmi.sock'
        qmp.unlink(missing_ok=True)
        cmd += ['-qmp', f'unix:{qmp},server=on,wait=off']
        report['commands'].append({'argv': cmd})
        with (d / 'qemu.log').open('w') as log:
            proc = subprocess.Popen(cmd, stdout=log, stderr=log)
            try:
                injected = False
                deadline = time.monotonic() + 60
                while time.monotonic() < deadline:
                    text = debug.read_text()
                    if case == 'nmi-transition' and '@@' in text and not injected:
                        with socket.socket(socket.AF_UNIX) as sock:
                            sock.settimeout(5); sock.connect(str(qmp))
                            with sock.makefile('rwb') as stream:
                                stream.readline()
                                for name in ['qmp_capabilities', 'inject-nmi']:
                                    stream.write((json.dumps({'execute': name}) + '\n').encode()); stream.flush()
                                    while True:
                                        reply = json.loads(stream.readline())
                                        if 'error' in reply: raise RuntimeError(reply)
                                        if 'return' in reply: break
                        injected = True
                    if text.endswith('\n') and any(m in text for m in ['RING3_EXIT_OK', 'RING3_REJECT', 'MUSHA: EXCEPTION VECTOR=']): break
                    if proc.poll() is not None: raise RuntimeError('QEMU exited: ' + (d / 'qemu.log').read_text())
                    time.sleep(.05)
                else: raise RuntimeError('Timeout: ' + text[-4000:])
                assert 'MUSHA: RING3_ENTERING C1_PROBE' in text, text
                if case != 'nmi-transition':
                    assert 'MUSHA: RING3_QUERY KERNEL_STACK_OK' in text, text
                    assert 'RING3 USER HELLO\n' in text, text
                if case == 'normal':
                    assert 'MUSHA: RING3_EXIT_OK' in text and 'RING3_REJECT' not in text and 'EXCEPTION' not in text, text
                elif case == 'nmi-transition':
                    assert 'RING3_FATAL_IST_OK' in text, text
                    assert injected and 'EXCEPTION VECTOR=0000000000000002 ERROR=0000000000000000' in text, text
                    assert 'RING3_FAULT CS=0000000000000008' in text, text
                    assert 'RING3_FAULT_RSP=000000006FFFFFE0' in text, text
                    assert 'RING3_EXIT_OK' not in text and 'RING3_REJECT' not in text, text
                elif case == 'rsp':
                    assert 'MUSHA: RING3_REJECT RETURN' in text and 'EXCEPTION' not in text and 'RING3_EXIT_OK' not in text, text
                else:
                    vector, error = (13, 0) if case == 'privileged' else (14, {'ro':7, 'nx':21, 'guard':4, 'supervisor':5}[case])
                    assert f'EXCEPTION VECTOR={vector:016X} ERROR={error:016X}' in text, text
                    assert 'RING3_FAULT_KERNEL_STACK_OK' in text, text
                    assert 'RING3_FAULT CS=0000000000000033' in text and 'RING3_EXIT_OK' not in text, text
                report['cases'].append({'case': case, 'passed': True, 'efi_sha256': hashlib.sha256(efi.read_bytes()).hexdigest()})
                print('PASS Ring3 ' + case, flush=True)
            finally:
                proc.terminate()
                try: proc.wait(timeout=5)
                except subprocess.TimeoutExpired: proc.kill(); proc.wait()
finally:
    (out / 'report.json').write_text(json.dumps(report, indent=2) + '\n')
