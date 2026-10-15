#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
import importlib.util, sys, struct, json, time, socket, subprocess, shutil
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
                wait(b'STARTUP FAILURE' if failure else b'VIEWER_FRAME LIST' if observe else b'STARTUP READY HOLD')
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
                    scale = min(4, max(1, (h + 300) // 600))
                    rows = 16 * scale
                    assert before[-w * rows * 3:] == after[-w * rows * 3:], 'ready footer lost after viewer repaint'
                    assert before[:-w * rows * 3] != after[:-w * rows * 3], 'viewer did not change'
                if failure:
                    raw = (out / 'ready.ppm').read_bytes().split(b'\n255\n', 1)[1]
                    assert any((raw[i] > 200 and raw[i + 1] < 120 and (raw[i + 2] < 120) for i in range(0, len(raw), 3))), 'quiet failure not visible'
                text = log.read_text()
                assert expected in text, (expected, text[-2000:])
                assert 'CONFIG FILE CLOSE ERROR' not in text
                if name == 'quiet':
                    assert (out / 'baseline.ppm').read_bytes() == (out / 'ready.ppm').read_bytes(), 'quiet changed screen'
                (out / 'result.json').write_text(json.dumps({'result': 'PASS', 'expected': expected, 'baseline_equals_ready': (out / 'baseline.ppm').read_bytes() == (out / 'ready.ppm').read_bytes()}, indent=2) + '\n')
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
        run('quiet-failure', b'log=quiet\n', 'STARTUP FAILURE REQUIRED CLOCK', efi=args.failure_efi.resolve(), failure=True)
    if args.viewer_efi:
        run('viewer-ready', b'log=ready\ndisplay=640x480\n', 'Ready Exact(640, 480) false Loaded', efi=args.viewer_efi.resolve(), observe=True)
    run('quiet', b'log=quiet\n', 'Quiet Max false Loaded')
    run('ready', b'log=ready\n', 'Ready Max false Loaded')
    run('phases', b'log=phases\n', 'Phases Max false Loaded')
    run('devices', b'log=devices\n', 'Devices Max false Loaded')
    run('missing-file', None, 'Ready Max false MissingFile')
    run('missing-directory', None, 'Ready Max false MissingDirectory', missing=True)
    run('malformed', b'log=quiet\nbad\n', 'Ready Max false Malformed')
    run('oversized', b'#' + b'a' * 4096, 'Ready Max false Malformed')
    run('non-ascii', b'log=quiet\n#\xff', 'Ready Max false Malformed')
    run('display-low', b'log=ready\ndisplay=640x480\n', 'Ready Exact(640, 480) false Loaded')
    run('display-good', b'log=phases\ndisplay=1024x768\npair=on\n', 'Phases Exact(1024, 768) true Loaded')
    run('display-fallback', b'log=phases\ndisplay=1234x567\n', 'requested mode unavailable; max fallback')
    run('f8', b'log=quiet\n', 'devices forced by F8/8', key='f8')
    run('digit8', b'log=quiet\n', 'devices forced by F8/8', key='8')
