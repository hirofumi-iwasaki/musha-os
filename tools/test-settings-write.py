#!/usr/bin/env python3
# Copyright 2026 Hirofumi Iwasaki
# SPDX-License-Identifier: Apache-2.0
"""Stage3 development tests on NEW disposable regular images; never devices."""
import argparse, importlib.util, json, pathlib, shutil, socket, struct, subprocess, time, hashlib
ROOT = pathlib.Path(__file__).resolve().parent.parent
spec = importlib.util.spec_from_file_location('usb', ROOT / 'tools/make-usb-image.py')
usb = importlib.util.module_from_spec(spec)
spec.loader.exec_module(usb)

def volume_start(image):
    return 0 if struct.unpack_from('<H', image, 11)[0] in (512, 4096) else 2048 * 512

def sector_size(image):
    return struct.unpack_from('<H', image, volume_start(image) + 11)[0]

def variant_fixture(path, size, spc):
    """New sparse superfloppy with actual FAT32 cluster count and dirty free slack."""
    clusters = 65525
    reserved = 32
    fz = ((clusters + 2) * 4 + size - 1) // size
    data = reserved + 2 * fz
    total = data + clusters * spc
    cb = size * spc
    marker = usb.usb_readme((ROOT / 'VERSION').read_text().strip())
    count = (len(marker) + cb - 1) // cb
    free = 4 + count
    boot = bytearray(size)
    boot[:3] = b'\xebX\x90'
    boot[3:11] = b'MUSHA-OS'
    struct.pack_into('<H', boot, 11, size)
    boot[13] = spc
    struct.pack_into('<H', boot, 14, reserved)
    boot[16] = 2
    boot[21] = 248
    struct.pack_into('<II', boot, 32, total, fz)
    struct.pack_into('<IHH', boot, 44, 2, 1, 6)
    boot[64] = 128
    boot[66] = 41
    struct.pack_into('<I', boot, 67, 1297437512)
    boot[71:82] = b'MUSHA-OS   '
    boot[82:90] = b'FAT32   '
    boot[510:512] = b'U\xaa'
    info = bytearray(size)
    struct.pack_into('<I', info, 0, 1096897106)
    struct.pack_into('<III', info, 484, 1631679090, clusters - (free - 2), free)
    struct.pack_into('<I', info, 508, 2857697280)
    fat = bytearray(fz * size)
    for (c, v) in [(0, 268435448), (1, 268435455), (2, 268435455), (3, 268435455)] + [(c, c + 1 if c + 1 < free else 268435455) for c in range(4, free)]:
        struct.pack_into('<I', fat, c * 4, v)

    def entry(name, attr, c=0, length=0):
        e = bytearray(32)
        e[:11] = name
        e[11] = attr
        struct.pack_into('<H', e, 20, c >> 16)
        struct.pack_into('<HI', e, 26, c & 65535, length)
        return e
    root = entry(b'MUSHA-OS   ', 8) + entry(b'README  TXT', 32, 4, len(marker)) + entry(b'SETTINGS   ', 16, 3)
    settings = entry(b'.          ', 16, 3) + entry(b'..         ', 16, 0)
    with path.open('xb') as f:
        f.truncate(total * size)

        def put(offset, b):
            f.seek(offset)
            f.write(b)
        put(0, boot)
        put(6 * size, boot)
        put(size, info)
        put(7 * size, info)
        put(reserved * size, fat)
        put((reserved + fz) * size, fat)
        put(data * size, root)
        put((data + spc) * size, settings)
        put((data + 2 * spc) * size, marker)
        for c in range(free, free + (4096 + cb - 1) // cb):
            put((data + (c - 2) * spc) * size, bytes([165]) * cb)

def geometry(image):
    start = volume_start(image)
    ss = sector_size(image)
    b = image[start:start + ss]
    reserved = struct.unpack_from('<H', b, 14)[0]
    fz = struct.unpack_from('<I', b, 36)[0]
    data = start + (reserved + 2 * fz) * ss
    cb = b[13] * ss
    root = struct.unpack_from('<I', b, 44)[0]
    ro = data + (root - 2) * cb
    at = image.index(b'SETTINGS   ', ro, ro + cb)
    cluster = struct.unpack_from('<H', image, at + 26)[0] | struct.unpack_from('<H', image, at + 20)[0] << 16
    return (start, fz, start + reserved * ss, data, cb, at, data + (cluster - 2) * cb)

def checker(path, out):
    image = path.read_bytes()
    if image[3:11] == b'EXFAT   ':
        return 'not applicable (exFAT byte-identical refusal)'
    start = volume_start(image)
    ss = sector_size(image)
    total = struct.unpack_from('<I', image, start + 32)[0]
    part = out / 'partition.fat'
    part.write_bytes(image[start:start + total * ss])
    command = [str(shutil.which('fsck.fat') or '/sbin/fsck_msdos'), '-n', str(part)]
    p = subprocess.run(command, capture_output=True, text=True)
    (out / 'fsck.log').write_text(p.stdout + p.stderr)
    part.unlink()
    assert p.returncode == 0, (command, p.returncode, p.stdout, p.stderr)
    return command[0]

def verify_changes(before, after, allow_write):
    assert len(before) == len(after)
    if not allow_write:
        assert before == after
        return 0
    (start, fz, fat, data, cb, at, settings) = geometry(before)
    ss = sector_size(before)
    allowed = {settings // ss, (start + ss) // ss, (start + 7 * ss) // ss}
    entry = after.index(b'PAIRING DAT', settings, settings + cb)
    assert struct.unpack_from('<I', after, entry + 28)[0] == 4096
    c = struct.unpack_from('<H', after, entry + 26)[0] | struct.unpack_from('<H', after, entry + 20)[0] << 16
    seen = []
    while c < 268435448:
        assert c >= 2 and c not in seen
        seen.append(c)
        allowed.update(range((data + (c - 2) * cb) // ss, (data + (c - 1) * cb) // ss))
        allowed.add(fat // ss + c * 4 // ss)
        allowed.add(fat // ss + fz + c * 4 // ss)
        c = struct.unpack_from('<I', after, fat + c * 4)[0] & 268435455
    assert len(seen) == (4096 + cb - 1) // cb
    assert after[fat:fat + fz * ss] == after[fat + fz * ss:fat + 2 * fz * ss]
    changed = {i // ss for i in range(0, len(before), ss) if before[i:i + ss] != after[i:i + ss]}
    assert changed <= allowed, sorted(changed - allowed)
    assert before[settings:entry] == after[settings:entry]
    assert before[entry + 32:settings + cb] == after[entry + 32:settings + cb]
    payload = b''.join((after[data + (c - 2) * cb:data + (c - 1) * cb] for c in seen))
    assert not any(payload[4096:])
    return len(changed)

def qemu(a, out, boot, data, readonly=False):
    fw = pathlib.Path(a.firmware_dir)
    shutil.copyfile(fw / 'edk2-i386-vars.fd', out / 'vars.fd')
    log = out / 'debug.log'
    log.write_text('')
    sock = ROOT / 'out' / ('sw-' + hashlib.sha256(str(out).encode()).hexdigest()[:12] + '.sock')
    sock.unlink(missing_ok=True)
    cmd = [a.qemu, '-machine', 'q35,accel=tcg', '-m', '256M', '-drive', f'if=pflash,format=raw,readonly=on,file={fw}/edk2-x86_64-code.fd', '-drive', f'if=pflash,format=raw,file={out}/vars.fd', '-drive', f'if=ide,format=raw,snapshot=on,file={boot}', '-drive', f"if=none,id=data,format=raw,readonly={('on' if readonly else 'off')},cache={('writeback' if a.case == 'cache-enabled' else 'writethrough')},file={data}", '-device', 'qemu-xhci,id=x', '-device', f'usb-storage,id=storage,drive=data,bus=x.0,port=1,logical_block_size={a.sector_size},physical_block_size={a.sector_size}', '-device', 'usb-kbd,bus=x.0', '-display', 'none', '-debugcon', f'file:{log}', '-global', 'isa-debugcon.iobase=0xe9', '-qmp', f'unix:{sock},server=on,wait=off', '-no-reboot']
    (out / 'command.json').write_text(json.dumps(cmd, indent=2))
    with (out / 'qemu.log').open('w') as err:
        p = subprocess.Popen(cmd, stdout=err, stderr=err)
        try:
            deadline = time.monotonic() + 180
            removed = False
            while True:
                text = log.read_text()
                if a.case == 'removal' and (not removed) and ('SETTINGS_WRITE_DATA_PENDING' in text):
                    with socket.socket(socket.AF_UNIX) as q:
                        q.connect(str(sock))
                        s = q.makefile('rwb')
                        s.readline()
                        for (name, args) in [('qmp_capabilities', {}), ('device_del', {'id': 'storage'})]:
                            s.write((json.dumps({'execute': name, 'arguments': args}) + '\n').encode())
                            s.flush()
                            while 'return' not in json.loads(s.readline()):
                                pass
                    removed = True
                if a.case == 'cache-enabled' and 'SETTINGS_CACHE_REFUSED' in text and 'XHCI_QUIESCED DMA_DISABLED' in text:
                    break
                if 'SETTINGS_WRITE_BEGIN' in text and text.rfind('XHCI_QUIESCED DMA_DISABLED') > text.index('SETTINGS_WRITE_BEGIN'):
                    break
                assert p.poll() is None and time.monotonic() < deadline, text[-5000:]
                time.sleep(0.01)
            if a.case == 'cache-enabled':
                assert 'SETTINGS_CACHE_REFUSED' in text and 'SETTINGS_WRITE_DATA_PENDING' not in text, text
                return
            assert 'CONFIG_WRITE_REFUSED' in text, text
            if a.bounds:
                for marker in ['SETTINGS_ZERO_PAYLOAD_OK', 'SETTINGS_MAX_BONDS_OK', 'SETTINGS_OVERFLOW_REFUSED']:
                    assert marker in text, text
            if a.case in ('create', 'fragmented', 'config'):
                assert 'SETTINGS_WRITE_RESULT Ok(' in text and 'SETTINGS_OVERWRITE_RESULT Ok(false)' in text and ('SETTINGS_READBACK_OK' in text), text
            elif a.case == 'protected':
                assert 'SETTINGS_WRITE_PROTECTED' in text and 'SETTINGS_WRITE_RESULT Err(Refused(Protected))' in text, text
            elif a.case in ('timeout', 'removal', 'cancel'):
                assert 'SETTINGS_WRITE_RESULT Err(Io(Io))' in text, text
            else:
                assert 'SETTINGS_WRITE_RESULT Err(' in text, text
        finally:
            p.terminate()
            try:
                p.wait(timeout=10)
            except subprocess.TimeoutExpired:
                p.kill()
                p.wait()
            sock.unlink(missing_ok=True)

def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--efi', type=pathlib.Path, required=True)
    p.add_argument('--output', type=pathlib.Path, required=True)
    p.add_argument('--case', choices=['create', 'protected', 'missing', 'unowned', 'full', 'no-space', 'fragmented', 'timeout', 'removal', 'exfat', 'cancel', 'config', 'cache-enabled'], default='create')
    p.add_argument('--sector-size', type=int, choices=[512, 4096], default=512)
    p.add_argument('--sectors-per-cluster', type=int, choices=[1, 2, 4, 8, 16, 32, 64, 128], default=1)
    p.add_argument('--bounds', action='store_true', help='Require zero/maximum/overflow probe evidence')
    p.add_argument('--qemu')
    p.add_argument('--firmware-dir', default='/opt/homebrew/share/qemu')
    p.add_argument('--harness', type=pathlib.Path, default=ROOT / 'target/debug/examples/settings-write')
    a = p.parse_args()
    out = a.output.resolve()
    out.mkdir(parents=True, exist_ok=False)
    image = out / 'data.img'
    if a.sector_size != 512 or a.sectors_per_cluster != 1:
        assert a.qemu and a.case == 'create', 'Geometry variants use QEMU create only'
        variant_fixture(image, a.sector_size, a.sectors_per_cluster)
    else:
        usb.create(image, a.efi.resolve())
    raw = bytearray(image.read_bytes())
    (start, fz, fat, data, cb, at, settings) = geometry(raw)
    if a.case == 'missing':
        raw[at:at + 11] = b'MISSING    '
    if a.case == 'unowned':
        free = [c for c in range(2, (len(raw) - data) // cb) if struct.unpack_from('<I', raw, fat + c * 4)[0] & 268435455 == 0][:8]
        for (i, c) in enumerate(free):
            for f in (fat, fat + fz * 512):
                struct.pack_into('<I', raw, f + c * 4, free[i + 1] if i + 1 < len(free) else 268435455)
        off = settings + 64
        raw[off:off + 11] = b'PAIRING DAT'
        raw[off + 11] = 32
        struct.pack_into('<H', raw, off + 26, free[0])
        struct.pack_into('<I', raw, off + 28, 4096)
        for n in (start + 512, start + 7 * 512):
            struct.pack_into('<II', raw, n + 488, struct.unpack_from('<I', raw, n + 488)[0] - 8, 4294967295)
    if a.case == 'full':
        for n in range(64, cb, 32):
            raw[settings + n:settings + n + 11] = f'F{n:07d}DAT'.encode()
            raw[settings + n + 11] = 32
    if a.case == 'no-space':
        clusters = (struct.unpack_from('<I', raw, start + 32)[0] - (32 + 2 * fz)) // (cb // 512)
        free = [c for c in range(2, clusters + 2) if struct.unpack_from('<I', raw, fat + c * 4)[0] & 268435455 == 0]
        for (i, c) in enumerate(free):
            for f in (fat, fat + fz * 512):
                struct.pack_into('<I', raw, f + c * 4, free[i + 1] if i + 1 < len(free) else 268435455)
        ro = data
        off = next((n for n in range(ro, ro + cb, 32) if raw[n] == 0))
        raw[off:off + 11] = b'FILLER  DAT'
        raw[off + 11] = 32
        struct.pack_into('<H', raw, off + 20, free[0] >> 16)
        struct.pack_into('<H', raw, off + 26, free[0] & 65535)
        struct.pack_into('<I', raw, off + 28, len(free) * cb)
        for n in (start + 512, start + 7 * 512):
            struct.pack_into('<II', raw, n + 488, 0, 4294967295)
    image.write_bytes(raw)
    if a.case == 'fragmented':
        subprocess.run([str(a.harness), str(image), 'initial'], check=True, capture_output=True)
        raw = bytearray(image.read_bytes())
        entry = raw.index(b'PAIRING DAT', settings, settings + cb)
        c = struct.unpack_from('<H', raw, entry + 26)[0]
        old = []
        while c < 268435448:
            old.append(c)
            c = struct.unpack_from('<I', raw, fat + c * 4)[0] & 268435455
        candidates = [c for c in range(2, (len(raw) - data) // cb) if struct.unpack_from('<I', raw, fat + c * 4)[0] & 268435455 == 0][::2][:len(old)]
        payload = [raw[data + (c - 2) * cb:data + (c - 1) * cb] for c in old]
        for c in old:
            for f in (fat, fat + fz * 512):
                struct.pack_into('<I', raw, f + c * 4, 0)
        for (i, c) in enumerate(candidates):
            raw[data + (c - 2) * cb:data + (c - 1) * cb] = payload[i]
            for f in (fat, fat + fz * 512):
                struct.pack_into('<I', raw, f + c * 4, candidates[i + 1] if i + 1 < len(candidates) else 268435455)
        struct.pack_into('<H', raw, entry + 20, candidates[0] >> 16)
        struct.pack_into('<H', raw, entry + 26, candidates[0] & 65535)
        image.write_bytes(raw)
    if a.case == 'config':
        spec = importlib.util.spec_from_file_location('startup', ROOT / 'tools/smoke-startup.py')
        startup = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(startup)
        startup.configure_fixture(image, b'# user-owned exact bytes\r\nlog=devices\r\npair=on\r\n')
    if a.case == 'exfat':
        image.unlink()
        spec = importlib.util.spec_from_file_location('exfat', ROOT / 'tools/make-exfat-fixture.py')
        exfat = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(exfat)
        exfat.create(image)
    baseline = checker(image, out)
    before = image.read_bytes()
    if a.qemu:
        boot = out / 'boot.img'
        usb.create(boot, a.efi.resolve())
        qemu(a, out, boot, image, a.case == 'protected')
    else:
        cmd = [str(a.harness), str(image), 'updated', 'PAIRING.DAT'] + (['protected'] if a.case == 'protected' else [])
        r = subprocess.run(cmd, text=True, capture_output=True)
        (out / 'host.log').write_text(r.stdout + r.stderr)
        assert (r.returncode == 0) == (a.case in ('create', 'fragmented', 'config')), (r.returncode, r.stdout, r.stderr)
    after = image.read_bytes()
    changes = verify_changes(before, after, a.case in ('create', 'fragmented', 'config'))
    if a.case == 'fragmented':
        for (off, n) in [(fat, 2 * fz * 512), (settings, cb), (start + 512, 512), (start + 7 * 512, 512)]:
            assert before[off:off + n] == after[off:off + n]
    checker(image, out)
    result = {'sector_size': a.sector_size, 'sectors_per_cluster': a.sectors_per_cluster, 'bounds': a.bounds, 'case': a.case, 'transport': 'qemu' if a.qemu else 'host', 'passed': True, 'changed_sectors': changes, 'checker': baseline, 'sha256': hashlib.sha256(image.read_bytes()).hexdigest()}
    (out / 'result.json').write_text(json.dumps(result, indent=2) + '\n')
    print(json.dumps(result))
if __name__ == '__main__':
    main()
