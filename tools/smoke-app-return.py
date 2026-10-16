#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""Local disposable QEMU ELF -> scrub -> fresh viewer lifecycle probe."""
import argparse
import hashlib
import importlib.util
import json
import re
from pathlib import Path
import shutil
import struct
import socket
import subprocess
import time
from echo_fixture import boot as configure_echo_boot

ROOT = Path(__file__).resolve().parents[1]
p = argparse.ArgumentParser(description=__doc__)
p.add_argument('--efi', type=Path, required=True)
p.add_argument('--elf', type=Path, required=True)
p.add_argument('--output', type=Path, required=True)
p.add_argument('--firmware-dir', type=Path, required=True)
p.add_argument('--qemu', default='qemu-system-x86_64')
p.add_argument('--format', choices=['fat32','exfat'], default='fat32')
p.add_argument('--selected-name', action='store_true', help='Rename fixture to SECOND.ELF and launch from viewer, checking its real display name')
p.add_argument('--lan', action='store_true', help='Require a real UDP echo after every viewer restart')
p.add_argument('--echo-disabled', action='store_true', help='Verify default-off with a real DHCP lease and no UDP reply after each restart')
p.add_argument('--launches', type=int, choices=range(1, 4), default=3)
p.add_argument('--quarantine', action='store_true', help='Expect stop-failure injection and forbid arena reuse')
p.add_argument('--screen-reference', type=Path, help='Feature-free screen-only comparison against verified return frame')
p.add_argument('--input', action='store_true', help='Drive existing r2 keyboard input sample with A/Q down/up')
p.add_argument('--detail', type=int, help='Expected exit status or fault vector')
p.add_argument('--outcome', type=int, choices=[0, 1, 2], default=0)
a = p.parse_args()
if a.lan and a.echo_disabled: p.error('Choose echo-on or default-off')
o = a.output.resolve()
o.mkdir(parents=True, exist_ok=False)
spec = importlib.util.spec_from_file_location('external_fixture', ROOT/'tools/smoke-external-elf.py')
fixture = importlib.util.module_from_spec(spec)
spec.loader.exec_module(fixture)
image = o/'data.img'
fixture.fixture(image, a.efi.resolve(), a.elf.read_bytes(), a.format)
if a.selected_name:
    if a.format!='fat32' or a.quarantine or a.screen_reference: raise ValueError('selected-name needs FAT32 functional fixture')
    with image.open('r+b') as media:
        media.seek(2048*512);boot=media.read(512)
        directory=(2048+32+2*struct.unpack_from('<I',boot,36)[0])*512
        media.seek(directory);entries=media.read(512)
        slot=next(i for i in range(0,512,32) if entries[i:i+11]==b'AUTORUN ELF')
        media.seek(directory+slot);media.write(b'SECOND  ELF')
before = hashlib.sha256(image.read_bytes()).hexdigest()
esp = o/'esp/EFI/BOOT'
esp.mkdir(parents=True)
shutil.copyfile(a.efi, esp/'BOOTX64.EFI')
shutil.copyfile(a.firmware_dir/'edk2-i386-vars.fd', o/'vars.fd')
sock = ROOT/'out'/('return-'+hashlib.sha256(str(o).encode()).hexdigest()[:10]+'.sock')
assert not sock.exists()
debug = o/'debug.log'
cmd = [a.qemu, '-machine', 'q35,accel=tcg', '-cpu', 'qemu64,+syscall,+nx', '-m', '256M',
       '-drive', f'if=pflash,format=raw,readonly=on,file={a.firmware_dir}/edk2-x86_64-code.fd',
       '-drive', f'if=pflash,format=raw,file={o}/vars.fd',
       '-drive', f'if=ide,format=raw,file=fat:rw:{o}/esp',
       '-device', 'qemu-xhci,id=x', '-device', 'usb-kbd,id=k,bus=x.0',
       '-drive', f'if=none,id=data,format=raw,readonly=on,file={image}',
       '-device', 'usb-storage,drive=data,bus=x.0', '-device', 'usb-mouse,bus=x.0',
       '-display', 'none', '-vga', 'std', '-debugcon', f'file:{debug}',
       '-qmp', f'unix:{sock},server=on,wait=off', '-no-reboot']
lan_port = None
if a.lan or a.echo_disabled:
    with socket.socket(socket.AF_INET, socket.SOCK_DGRAM) as reservation:
        reservation.bind(('127.0.0.1', 0))
        lan_port = reservation.getsockname()[1]
    cmd += ['-netdev', f'user,id=n,hostfwd=udp:127.0.0.1:{lan_port}-:12345', '-device', 'e1000e,netdev=n']
if a.lan or a.echo_disabled:
    configure_echo_boot(cmd, o, efi=a.efi, enabled=a.lan)
(o/'command.json').write_text(json.dumps(cmd, indent=2)+'\n')
report = {'scope': 'QEMU only; no physical USB acceptance', 'passed': False, 'format': a.format, 'lan': a.lan, 'launches': [], 'efi_sha256': hashlib.sha256(a.efi.read_bytes()).hexdigest(), 'elf_sha256': hashlib.sha256(a.elf.read_bytes()).hexdigest()}
with (o/'qemu.log').open('w') as log:
    proc = subprocess.Popen(cmd, stdout=log, stderr=log)
    try:
        deadline = time.monotonic()+60
        while not sock.exists():
            if proc.poll() is not None or time.monotonic()>deadline:
                raise RuntimeError('QMP startup')
            time.sleep(.05)
        with socket.socket(socket.AF_UNIX) as s:
            s.settimeout(10)
            s.connect(str(sock))
            f = s.makefile('rwb')
            f.readline()
            def call(name, args=None):
                f.write((json.dumps({'execute': name, 'arguments': args or {}})+'\n').encode())
                f.flush()
                while True:
                    response = json.loads(f.readline())
                    if 'error' in response:
                        raise RuntimeError(response)
                    if 'return' in response:
                        return
            def wait(marker, start=0):
                end = time.monotonic()+60
                while time.monotonic()<end:
                    text = debug.read_text() if debug.exists() else ''
                    if marker in text[start:]:
                        return text.index(marker, start)
                    if 'QUARANTINED' in text[start:] or 'EXCEPTION VECTOR=' in text[start:]:
                        raise RuntimeError(text[start:])
                    time.sleep(.05)
                raise RuntimeError('Timeout '+marker+'\n'+text[start:])
            def key(code):
                call('send-key', {'keys': [{'type': 'qcode', 'data': code}], 'hold-time': 30})
                time.sleep(.12)
            call('qmp_capabilities')
            start = 0
            if a.selected_name:
                wait('VIEWER_FRAME LIST')
                key('end')
                wait('VIEWER_FRAME LIST 0000000000000001')
                start = len(debug.read_text())
                key('ret')
                wait('VIEWER_ELF_SELECTED SAME_VALIDATION', start)
            if a.quarantine:
                wait('APP_RECOVERY_STOP_UNCONFIRMED QUARANTINED')
                text = debug.read_text()
                assert 'APP_ARENA_SCRUBBED' not in text and 'APP_RECOVERED' not in text
                report['quarantine_without_reuse'] = True
                print('PASS unconfirmed stop forbids reuse', flush=True)
            if a.screen_reference:
                expected = a.screen_reference.read_bytes()
                end = time.monotonic()+60
                while time.monotonic()<end:
                    call('screendump', {'filename': str(o/'viewer-normal.ppm')})
                    if (o/'viewer-normal.ppm').read_bytes()==expected:
                        break
                    time.sleep(.2)
                assert (o/'viewer-normal.ppm').read_bytes()==expected, 'Normal return screen differs'
                assert not debug.exists() or debug.read_bytes()==b'', 'Normal EFI emitted debug output'
                report['screen_only_normal'] = True
                print('PASS normal return pixels and no debug output', flush=True)
            for launch in range(0 if a.quarantine or a.screen_reference else a.launches):
                if a.input:
                    wait('R2 INPUT READY', start)
                    key('a')
                    key('q')
                recovered = wait('APP_RECOVERED KIND='+format(a.outcome, '016X'), start)
                scrub = wait('APP_ARENA_SCRUBBED VIEWER_RESTART', recovered)
                frame = wait('VIEWER_FRAME LIST', scrub)
                text = debug.read_text()
                if a.selected_name:
                    assert 'app: starting SECOND.ELF' in text[start:scrub],text[start:scrub]
                    assert 'app: SECOND.ELF exited, status' in text[start:scrub],text[start:scrub]
                    assert 'app: starting AUTORUN.ELF' not in text[start:scrub],text[start:scrub]
                assert 'FILE_IDENTITIES_RELEASED AFTER_DMA_STOP' in text[start:scrub]
                assert 'XHCI_QUIESCED DMA_DISABLED' in text[start:scrub]
                assert 'STREAM_MOUNT_'+a.format.upper() in text[scrub:frame]
                if a.lan or a.echo_disabled:
                    wait('NET_READY', scrub)
                    text = debug.read_text()
                    assert ('NET_READY IP=10.0.2.15 UDP=12345 DHCP' if a.lan else 'NET_READY IP=10.0.2.15 ECHO=disabled DHCP') in text[scrub:] and 'LWIP INIT' not in text, text
                    payload = ('stage5-'+a.format+'-mount-'+str(launch)).encode()
                    with socket.socket(socket.AF_INET, socket.SOCK_DGRAM) as peer:
                        peer.settimeout(.5)
                        end = time.monotonic()+(1.5 if a.echo_disabled else 10)
                        while True:
                            peer.sendto(payload, ('127.0.0.1', lan_port))
                            try:
                                echoed, _ = peer.recvfrom(2048)
                                assert a.lan, 'Default-off UDP service replied'
                                assert echoed == payload, (echoed, payload)
                                break
                            except socket.timeout:
                                if time.monotonic()>end:
                                    if a.echo_disabled: break
                                    raise RuntimeError('No UDP echo after viewer restart')
                call('screendump', {'filename': str(o/('viewer-'+str(launch)+'.ppm'))})
                detail = int(re.search(r'APP_RECOVERED KIND=[0-9A-F]{16} DETAIL=([0-9A-F]{16})', text[recovered:]).group(1), 16)
                if a.detail is not None:
                    assert detail == a.detail, (detail, a.detail)
                report['launches'].append({'index': launch, 'outcome': a.outcome, 'detail': detail, 'lan_echo': a.lan, 'echo_disabled_verified': a.echo_disabled, 'viewer': True, 'scrub_after_dma': True})
                print('PASS app return '+str(launch), flush=True)
                if launch+1<a.launches:
                    start = len(debug.read_text())
                    selected_index = (len(fixture.module('make-exfat-fixture').build()[1]['files']) if a.format=='exfat' else 1)
                    key('end')  # AUTORUN was appended after all original root files.
                    wait('VIEWER_FRAME LIST '+format(selected_index, '016X'), start)
                    key('ret')
                    wait('VIEWER_ELF_SELECTED SAME_VALIDATION', start)
            report['passed'] = True
            report['image_unchanged'] = before == hashlib.sha256(image.read_bytes()).hexdigest()
            assert report['image_unchanged']
    finally:
        proc.terminate()
        try:
            proc.wait(timeout=5)
        except subprocess.TimeoutExpired:
            proc.kill()
            proc.wait()
        sock.unlink(missing_ok=True)
        (o/'report.json').write_text(json.dumps(report, indent=2)+'\n')
