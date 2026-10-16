#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
import importlib.util, sys, struct, json, time, socket, subprocess, shutil, re
from pathlib import Path
root = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(root / 'tools'))
spec = importlib.util.spec_from_file_location('usb', root / 'tools/make-usb-image.py')
usb = importlib.util.module_from_spec(spec)
spec.loader.exec_module(usb)

def fixture(path, efi, config, missing=False):
    usb.create(path, efi)
    configure_fixture(path, config, missing)

def configure_fixture(path, config, missing=False):
    """Edit only a newly generated deterministic development image, never a device."""
    if not path.is_file() or path.is_symlink():
        raise ValueError('Expected a generated regular fixture')
    with path.open('r+b') as f:

        def read(off, n):
            f.seek(off)
            return f.read(n)

        def write(off, data):
            f.seek(off)
            f.write(data)
        first = 2048 * 512
        b = read(first, 512)
        reserved = struct.unpack_from('<H', b, 14)[0]
        nf = b[16]
        fz = struct.unpack_from('<I', b, 36)[0]
        spc = b[13]
        cb = spc * 512
        fat = bytearray(read(first + reserved * 512, fz * 512))
        data = first + (reserved + nf * fz) * 512
        rootc = struct.unpack_from('<I', b, 44)[0]
        ro = data + (rootc - 2) * cb
        directory = read(ro, cb)
        at = next((i for i in range(0, len(directory), 32) if directory[i:i + 11] == b'SETTINGS   '))
        if missing:
            write(ro + at, b'MISSING    ')
            return
        if config is None:
            return
        c = struct.unpack_from('<H', directory, at + 26)[0] | struct.unpack_from('<H', directory, at + 20)[0] << 16
        free = [i for i in range(2, len(fat) // 4) if struct.unpack_from('<I', fat, i * 4)[0] == 0][:max(1, (len(config) + cb - 1) // cb)]
        for (i, n) in enumerate(free):
            struct.pack_into('<I', fat, n * 4, free[i + 1] if i + 1 < len(free) else 268435455)
        for i in range(nf):
            write(first + (reserved + i * fz) * 512, fat)
        entry = bytearray(32)
        entry[:11] = b'CONFIG  TXT'
        entry[11] = 32
        struct.pack_into('<H', entry, 20, free[0] >> 16)
        struct.pack_into('<HI', entry, 26, free[0] & 65535, len(config))
        write(data + (c - 2) * cb + 64, entry)
        for (i, n) in enumerate(free):
            write(data + (n - 2) * cb, config[i * cb:(i + 1) * cb].ljust(cb, b'\x00'))
        for off in (512, 7 * 512):
            info = bytearray(read(first + off, 512))
            count = struct.unpack_from('<I', info, 488)[0]
            struct.pack_into('<II', info, 488, count - len(free), free[-1] + 1)
            write(first + off, info)

def run(name, config, expected, key=None, missing=False, efi=None, failure=False, observe=False):
    if args.case and name not in args.case.split(','):
        return
    out = args.output / name
    out.mkdir(exist_ok=False)
    image = out / 'boot.img'
    fixture(image, efi or args.efi, config, missing)
    fw = args.firmware_dir
    shutil.copyfile(fw / 'edk2-i386-vars.fd', out / 'vars.fd')
    sock = args.output / 'qmp.sock'
    sock.unlink(missing_ok=True)
    log = out / 'debug.log'
    cmd = [args.qemu, '-machine', 'q35,accel=tcg', '-m', '256M', '-drive', f'if=pflash,format=raw,readonly=on,file={fw}/edk2-x86_64-code.fd', '-drive', f'if=pflash,format=raw,file={out}/vars.fd', '-drive', f'if=ide,format=raw,snapshot=on,file={image}', '-drive', f'if=none,id=data,format=raw,readonly=on,file={args.data_image}', '-device', 'qemu-xhci,id=x', '-device', 'usb-storage,drive=data,bus=x.0,port=1', '-device', 'usb-kbd,bus=x.0', '-vga', 'std', '-display', 'none', '-debugcon', f'file:{log}', '-global', 'isa-debugcon.iobase=0xe9', '-qmp', f'unix:{sock},server=on,wait=off', '-no-reboot']
    with (out / 'qemu.log').open('w') as err:
        p = subprocess.Popen(cmd, stdout=err, stderr=err)
        try:
            end = time.monotonic() + 90
            while not sock.exists():
                assert p.poll() is None and time.monotonic() < end
                time.sleep(0.05)
            with socket.socket(socket.AF_UNIX) as q:
                q.settimeout(10)
                q.connect(str(sock))
                s = q.makefile('rwb')
                s.readline()

                def control(n, a={}):
                    s.write((json.dumps({'execute': n, 'arguments': a}) + '\n').encode())
                    s.flush()
                    while True:
                        r = json.loads(s.readline())
                        if 'error' in r:
                            raise RuntimeError(r)
                        if 'return' in r:
                            return r
                control('qmp_capabilities')

                def wait(marker):
                    while marker not in log.read_bytes():
                        assert time.monotonic() < end and p.poll() is None, marker
                        time.sleep(0.02)
                wait(b'CONFIG KEY WINDOW')
                if key:
                    control('send-key', {'keys': [{'type': 'qcode', 'data': key}], 'hold-time': 80})
                wait(b'CONFIG FRAMEBUFFER CAPTURED')
                control('screendump', {'filename': str(out / 'baseline.ppm')})
                wait(b'Musha-OS halted' if failure else b'VIEWER_FRAME LIST' if observe else b'MUSHA: STARTUP_READY')
                # Earlier diagnostic snapshots may already contain RING_END.
                # Require a complete latest dump containing this run's terminal event.
                terminal = b'Musha-OS halted' if failure else b'Musha-OS 0.4.0 ready'
                while True:
                    raw = log.read_bytes()
                    at = raw.rfind(b'MUSHA: RING_BEGIN\n')
                    end_at = raw.find(b'MUSHA: RING_END', at) if at >= 0 else -1
                    if end_at > at >= 0 and terminal in raw[at:end_at]:
                        break
                    assert time.monotonic() < end and p.poll() is None, 'terminal retained ring dump'
                    time.sleep(0.02)
                time.sleep(0.1)
                control('screendump', {'filename': str(out / 'ready.ppm')})
                if observe:
                    control('send-key', {'keys': [{'type': 'qcode', 'data': 'ret'}], 'hold-time': 80})
                    wait(b'VIEWER_FRAME PAGE')
                    time.sleep(0.1)
                    control('screendump', {'filename': str(out / 'page.ppm')})

                    def ppm(path):
                        raw = path.read_bytes()
                        (header, body) = raw.split(b'\n255\n', 1)
                        (w, h) = map(int, header.splitlines()[1].split())
                        return (w, h, body)
                    (w, h, before) = ppm(out / 'ready.ppm')
                    (w2, h2, after) = ppm(out / 'page.ppm')
                    assert (w, h) == (w2, h2)
                    assert before != after, 'viewer did not change'
                text = log.read_text()
                primary = [line.removeprefix('MUSHA: LOG ') for line in text.splitlines()
                           if line.startswith('MUSHA: LOG ')]
                assert primary, 'missing primary single-log records'
                begin = text.rfind('MUSHA: RING_BEGIN\n')
                finish = text.find('MUSHA: RING_END', begin)
                assert begin >= 0 and finish > begin, 'missing retained ring dump'
                retained = [line.removeprefix('MUSHA: RING ') for line in text[begin:finish].splitlines()
                            if line.startswith('MUSHA: RING ')]
                assert retained, 'empty retained ring'
                assert expected.lower() in '\n'.join(primary).lower(), (expected, primary[-20:])
                assert expected.lower() in '\n'.join(retained).lower(), (expected, retained[-20:])
                plain = [re.sub(r'^\[\s*\d+\.\d{3}\] ', '', line) for line in primary]
                assert primary[0] == 'Musha-OS 0.4.0 starting', primary[:3]
                assert primary == retained, 'primary/retained startup record mismatch'
                if key:
                    assert any('settings: Loaded log=Trace ' in line for line in plain), plain
                if failure:
                    assert any(line.startswith('ERROR: ') for line in plain), primary[-20:]
                    assert 'Musha-OS halted' in plain, primary[-20:]
                elif not observe:
                    assert 'Musha-OS 0.4.0 ready' in plain, primary[-20:]
                if not observe:
                    def body(path):
                        return path.read_bytes().split(b'\n255\n', 1)[1]
                    old = body(out / 'baseline.ppm')
                    new = body(out / 'ready.ppm')
                    assert len(old) == len(new)
                    changed = [new[i:i+3] for i in range(0, len(new), 3)
                               if old[i:i+3] != new[i:i+3]]
                    assert all(pixel in (b'\0\0\0', b'\xff\xff\xff') for pixel in changed), 'non-monochrome startup paint'
                    if failure or name != 'quiet':
                        assert changed, 'visible startup log missing'
                        assert len(changed) < len(new) // 3, 'startup filled every pixel'
                assert 'CONFIG FILE CLOSE ERROR' not in text
                if name == 'quiet':
                    assert (out / 'baseline.ppm').read_bytes() == (out / 'ready.ppm').read_bytes(), 'quiet changed screen'
                (out / 'result.json').write_text(json.dumps({'result': 'PASS', 'expected': expected, 'primary_records': primary, 'retained_records': retained, 'baseline_equals_ready': (out / 'baseline.ppm').read_bytes() == (out / 'ready.ppm').read_bytes()}, indent=2) + '\n')
                control('quit')
                p.wait(timeout=5)
            print('PASS', name, flush=True)
        finally:
            if p.poll() is None:
                p.terminate()
                p.wait(timeout=5)
            sock.unlink(missing_ok=True)
if __name__ == '__main__':
    import argparse
    p = argparse.ArgumentParser(description='Development-only startup configuration QEMU checks; no physical device access.')
    p.add_argument('--case', help='Comma-separated subset of checks')
    p.add_argument('--efi', required=True, type=Path)
    p.add_argument('--data-image', required=True, type=Path)
    p.add_argument('--firmware-dir', required=True, type=Path)
    p.add_argument('--output', required=True, type=Path)
    p.add_argument('--failure-efi', type=Path)
    p.add_argument('--viewer-efi', type=Path)
    p.add_argument('--qemu', default='qemu-system-x86_64')
    args = p.parse_args()
    for name in ('efi', 'data_image', 'firmware_dir', 'output'):
        setattr(args, name, getattr(args, name).resolve())
    args.output.mkdir(exist_ok=False, parents=True)
    if args.failure_efi:
        run('quiet-failure', b'log=quiet\n', 'ERROR: clock:', efi=args.failure_efi.resolve(), failure=True)
    if args.viewer_efi:
        run('viewer-ready', b'log=ready\ndisplay=640x480\n', 'settings: loaded log=ready display=exact(640, 480) pair=false', efi=args.viewer_efi.resolve(), observe=True)
    run('quiet', b'log=quiet\n', 'settings: loaded log=quiet display=max pair=false')
    run('ready', b'log=ready\n', 'settings: loaded log=ready display=max pair=false')
    run('phases', b'log=phases\n', 'settings: loaded log=phases display=max pair=false')
    run('devices', b'log=devices\n', 'settings: loaded log=devices display=max pair=false')
    run('trace', b'log=trace\n', 'settings: loaded log=trace display=max pair=false')
    run('trace-malformed', b'log=trace\nbad\n', 'settings: malformed log=ready display=max pair=false')
    run('invalid-level', b'log=verbose\n', 'settings: malformed log=ready display=max pair=false')
    run('missing-file', None, 'settings: missingfile log=ready display=max pair=false')
    run('missing-directory', None, 'settings: missingdirectory log=ready display=max pair=false', missing=True)
    run('malformed', b'log=quiet\nbad\n', 'settings: malformed log=ready display=max pair=false')
    run('oversized', b'#' + b'a' * 4096, 'settings: malformed log=ready display=max pair=false')
    run('non-ascii', b'log=quiet\n#\xff', 'settings: malformed log=ready display=max pair=false')
    run('display-low', b'log=ready\ndisplay=640x480\n', 'settings: loaded log=ready display=exact(640, 480) pair=false')
    run('display-good', b'log=phases\ndisplay=1024x768\npair=on\n', 'settings: loaded log=phases display=exact(1024, 768) pair=true')
    run('display-fallback', b'log=phases\ndisplay=1234x567\n', 'requested mode unavailable; maximum selected')
    run('f8', b'log=quiet\n', 'trace selected by F8/8', key='f8')
    run('digit8', b'log=quiet\n', 'trace selected by F8/8', key='8')
