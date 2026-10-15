#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""Real r2 syscalls through USB external ELF path; disposable QEMU image only."""
import argparse, hashlib, importlib.util, json, os, shutil, socket, subprocess, time
from pathlib import Path
ROOT = Path(__file__).resolve().parents[1]
p = argparse.ArgumentParser(description=__doc__)
p.add_argument('--elf', type=Path, required=True)
p.add_argument('--output', type=Path, required=True)
p.add_argument('--firmware-dir', type=Path, required=True)
p.add_argument('--qemu', default='qemu-system-x86_64')
p.add_argument('--expect', choices=['normal', 'denied', 'fault', 'overflow', 'removed'], default='normal')
p.add_argument('--mouse-only', action='store_true')
p.add_argument('--mouse', action='store_true')
p.add_argument('--tablet', action='store_true')
a = p.parse_args()
out = a.output.resolve()
out.mkdir(parents=True, exist_ok=False)
report = {'scope': 'QEMU r2 real syscalls, no physical USB acceptance', 'passed': False}
try:
    env = dict(os.environ, CARGO_TARGET_DIR=str(out / 'target'), MUSHA_RING3_CASE='normal')
    with (out / 'build.log').open('w') as log:
        subprocess.run(['cargo', 'build', '--locked', '--release', '--target', 'x86_64-unknown-uefi', '-p', 'musha-boot', '--features', 'external-elf-probe'], cwd=ROOT, env=env, stdout=log, stderr=subprocess.STDOUT, check=True)
    efi = out / 'target/x86_64-unknown-uefi/release/musha-boot.efi'
    spec = importlib.util.spec_from_file_location('external', ROOT / 'tools/smoke-external-elf.py')
    external = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(external)
    external.fixture(out / 'data.img', efi, a.elf.resolve().read_bytes(), 'fat32')
    esp = out / 'esp/EFI/BOOT'
    esp.mkdir(parents=True)
    shutil.copyfile(efi, esp / 'BOOTX64.EFI')
    shutil.copyfile(a.firmware_dir / 'edk2-i386-vars.fd', out / 'vars.fd')
    sock = ROOT / 'out' / ('r2-' + hashlib.sha256(str(out).encode()).hexdigest()[:10] + '.sock')
    sock.unlink(missing_ok=True)
    debug = out / 'debug.log'
    cmd = [a.qemu, '-machine', 'q35,accel=tcg', '-cpu', 'qemu64,+syscall,+nx', '-m', '256M', '-drive', f'if=pflash,format=raw,readonly=on,file={a.firmware_dir}/edk2-x86_64-code.fd', '-drive', f'if=pflash,format=raw,file={out}/vars.fd', '-drive', f'if=ide,format=raw,file=fat:rw:{out}/esp', '-device', 'qemu-xhci,id=x', '-device', 'usb-kbd,id=k,bus=x.0', '-drive', f'if=none,id=data,format=raw,readonly=on,file={out}/data.img', '-device', 'usb-storage,drive=data,bus=x.0', '-display', 'none', '-vga', 'std', '-debugcon', f'file:{debug}', '-qmp', f'unix:{sock},server=on,wait=off', '-no-reboot']
    if a.mouse_only:
        i = cmd.index('usb-kbd,id=k,bus=x.0')
        del cmd[i - 1:i + 1]
    if a.mouse:
        cmd += ['-device', 'usb-mouse,bus=x.0']
    if a.tablet:
        cmd += ['-device', 'usb-tablet,bus=x.0']
    (out / 'command.json').write_text(json.dumps(cmd, indent=2) + '\n')
    with (out / 'qemu.log').open('w') as log:
        proc = subprocess.Popen(cmd, stdout=log, stderr=log)
        try:
            end = time.monotonic() + 60
            while not sock.exists():
                if proc.poll() is not None or time.monotonic() > end:
                    raise RuntimeError('QMP startup')
                time.sleep(0.05)
            with socket.socket(socket.AF_UNIX) as s:
                s.settimeout(10)
                s.connect(str(sock))
                f = s.makefile('rwb')
                f.readline()

                def call(name, args=None):
                    f.write((json.dumps({'execute': name, 'arguments': args or {}}) + '\n').encode())
                    f.flush()
                    while True:
                        r = json.loads(f.readline())
                        if 'error' in r:
                            raise RuntimeError(r)
                        if 'return' in r:
                            return r['return']
                call('qmp_capabilities')
                while time.monotonic() < end:
                    text = debug.read_text() if debug.exists() else ''
                    if {'normal': 'R2 INPUT READY', 'removed': 'R2 INPUT READY', 'overflow': 'R2 OVERFLOW READY', 'denied': 'R2 PERMISSIONS OK', 'fault': 'R2 FAULT READY'}[a.expect] in text:
                        break
                    if 'EXTERNAL_APP_EXIT' in text or 'EXCEPTION VECTOR=' in text:
                        raise RuntimeError(text)
                    time.sleep(0.05)
                else:
                    raise RuntimeError('No input-ready: ' + text)
                if a.expect == 'normal':
                    if a.tablet:
                        call('input-send-event', {'events': [{'type': 'abs', 'data': {'axis': 'x', 'value': 10000}}, {'type': 'abs', 'data': {'axis': 'y', 'value': 20000}}]})
                    if a.mouse:
                        call('input-send-event', {'events': [{'type': 'rel', 'data': {'axis': 'x', 'value': 17}}, {'type': 'rel', 'data': {'axis': 'y', 'value': -9}}, {'type': 'btn', 'data': {'down': True, 'button': 'left'}}, {'type': 'btn', 'data': {'down': True, 'button': 'wheel-up'}}]})
                        time.sleep(0.2)
                        call('input-send-event', {'events': [{'type': 'btn', 'data': {'down': False, 'button': 'left'}}, {'type': 'btn', 'data': {'down': False, 'button': 'wheel-up'}}]})
                    if not a.mouse_only:
                        call('send-key', {'keys': [{'type': 'qcode', 'data': 'a'}], 'hold-time': 100})
                        time.sleep(0.3)
                        call('send-key', {'keys': [{'type': 'qcode', 'data': 'q'}], 'hold-time': 100})
                    end = time.monotonic() + 10
                    while time.monotonic() < end:
                        text = debug.read_text()
                        if 'APP_INPUT_STOPPED DMA_DISABLED' in text:
                            break
                        time.sleep(0.05)
                    assert ('R2 MOUSE EXIT' if a.mouse_only else 'R2 Q EXIT') in text and 'EXTERNAL_APP_EXIT' in text and ('APP_INPUT_STOPPED DMA_DISABLED' in text), text
                    assert 'EXTERNAL_APP_EXIT_STATUS' not in text and 'EXCEPTION VECTOR=' not in text, text
                    if not a.mouse_only:
                        assert text.count('R2 KEY DOWN') >= 2 and text.count('R2 KEY UP') >= 2, text
                    if a.mouse:
                        for marker in ['R2 MOUSE', 'DX 0x0000000000000011', 'DY 0xFFFFFFFFFFFFFFF7', 'BUTTONS 0x0000000000000001']:
                            assert marker in text, text
                        assert 'WHEEL 0x0000000000000001' in text or 'WHEEL 0xFFFFFFFFFFFFFFFF' in text, text
                    if a.tablet:
                        assert 'R2 MOUSE' not in text, text
                else:
                    if a.expect == 'removed':
                        call('device_del', {'id': 'k'})
                    if a.expect == 'overflow':
                        for i in range(80):
                            for down in [True, False]:
                                call('input-send-event', {'events': [{'type': 'key', 'data': {'down': down, 'key': {'type': 'qcode', 'data': 'a'}}}]})
                                time.sleep(0.006)
                    end = time.monotonic() + 10
                    terminal = 'EXTERNAL_APP_EXIT' if a.expect == 'denied' else 'APP_INPUT_STOPPED DMA_DISABLED'
                    while time.monotonic() < end:
                        text = debug.read_text()
                        if terminal in text:
                            break
                        time.sleep(0.05)
                    assert terminal in text, text
                    if a.expect == 'denied':
                        assert 'R2 PERMISSIONS OK' in text and 'APP_INPUT_LIVE' not in text, text
                    elif a.expect == 'fault':
                        assert 'C3_APP_FAULT VECTOR=0000000000000006' in text and 'RING3_FAULT_KERNEL_STACK_OK' in text, text
                    elif a.expect == 'overflow':
                        assert 'R2 OVERFLOW OK' in text, text
                    elif a.expect == 'removed':
                        assert 'R2 DISCONNECTED OK' in text, text
                end = time.monotonic()+60
                while time.monotonic()<end:
                    text = debug.read_text()
                    if 'APP_ARENA_SCRUBBED VIEWER_RESTART' in text and 'VIEWER_FRAME ' in text.split('APP_ARENA_SCRUBBED VIEWER_RESTART')[-1]:
                        break
                    time.sleep(.05)
                assert 'APP_ARENA_SCRUBBED VIEWER_RESTART' in text, text
                assert 'VIEWER_FRAME ' in text.split('APP_ARENA_SCRUBBED VIEWER_RESTART')[-1], text
                if a.expect == 'fault':
                    assert 'APP_RECOVERED KIND=0000000000000001 DETAIL=0000000000000006' in text,text
                if a.expect != 'fault':
                    assert 'RING3_ELF_EXIT_STATUS=0000000000000000' in text, text
            report.update(passed=True, elf_sha256=hashlib.sha256(a.elf.read_bytes()).hexdigest(), efi_sha256=hashlib.sha256(efi.read_bytes()).hexdigest(), mouse=a.mouse, tablet=a.tablet, expected=a.expect)
            print('PASS r2 real TIME/WAIT/INPUT ' + a.elf.name, flush=True)
        finally:
            proc.terminate()
            proc.wait(timeout=10)
            sock.unlink(missing_ok=True)
except Exception as e:
    report['failure'] = str(e)
    raise
finally:
    (out / 'report.json').write_text(json.dumps(report, indent=2) + '\n')
