# SPDX-License-Identifier: Apache-2.0
"""Explicit echo-on CONFIG on a new FAT32 boot fixture, never the supplied data."""
from pathlib import Path
import hashlib
import importlib.util
import io
import json
import tempfile

ROOT = Path(__file__).resolve().parents[1]


def load(name):
    spec = importlib.util.spec_from_file_location(name, ROOT / ('tools/' + name + '.py'))
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def boot(cmd, output, *, image=None, efi=None, enabled=True):
    """Replace/add SATA boot only; original USB/exFAT media remain read-only."""
    directory = Path(tempfile.mkdtemp(prefix='echo-config-', dir=output))
    if image is not None:
        image = Path(image)
        if image.is_symlink() or not image.is_file() or image.stat().st_size > 256 * 1024 * 1024:
            raise ValueError('Expected bounded generated boot image')
        raw = image.read_bytes()
        data = load('deploy-image').inspect(io.BytesIO(raw), len(raw))['efi']
        efi = directory / 'source.efi'
        efi.write_bytes(data)
    elif efi is None:
        raise ValueError('Boot image or EFI required')
    config = b'log=devices\nlan.echo=on\n' if enabled else b'log=phases\n'
    path = directory / 'boot.img'
    # SATA firmware boot requires an ESP; the original USB data may be Basic Data.
    load('make-usb-image').create(path, Path(efi), partition_type='esp')
    load('smoke-startup').configure_fixture(path, config)
    replacement = f'if=none,id=echo_boot,format=raw,snapshot=on,file={path}'
    for i, value in enumerate(cmd):
        if value.startswith('if=ide,'):
            cmd[i] = replacement
            break
    else:
        cmd.extend(['-drive', replacement])
    if '-boot' in cmd:
        at = cmd.index('-boot')
        del cmd[at:at + 2]
    # OVMF can otherwise prefer the data USB despite legacy -boot order=c.
    cmd.extend(['-device', 'ide-hd,drive=echo_boot,bootindex=1'])
    (directory / 'config.json').write_text(json.dumps({
        'echo_enabled': enabled, 'config': config.decode('ascii'),
        'config_sha256': hashlib.sha256(config).hexdigest(),
        'boot_image': str(path), 'original_media_modified': False,
    }, indent=2) + '\n')
    return path
