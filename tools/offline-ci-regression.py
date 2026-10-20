#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""Run four bounded, device-free release regressions with disposable QEMU images."""
import argparse
import json
import os
import subprocess
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def run(args, *, env=None):
    print('+', ' '.join(map(str, args)), flush=True)
    started = time.monotonic()
    subprocess.run(list(map(str, args)), cwd=ROOT, env=env, check=True)
    return round(time.monotonic() - started, 1)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--firmware-dir', type=Path, required=True)
    parser.add_argument('--qemu', default='qemu-system-x86_64')
    args = parser.parse_args()
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=False)
    firmware = args.firmware_dir.resolve()
    for name in ('edk2-x86_64-code.fd', 'edk2-i386-vars.fd'):
        if not (firmware / name).is_file():
            parser.error(f'missing firmware: {firmware / name}')
    timings = {}
    try:
        samples = output / 'r2-samples'
        timings['r2_samples'] = run(['python3', 'tools/build-r2-samples.py', '--output', samples])
        elf = samples / 'io-c.elf'
        common = ['python3', 'tools/smoke-r2-io.py', '--elf', elf,
                  '--firmware-dir', firmware, '--qemu', args.qemu,
                  '--mouse', '--input-topology', 'split']
        timings['split_input'] = run([*common, '--output', output / 'split-input'])
        timings['split_stop_failure'] = run([*common, '--output', output / 'split-stop-failure',
                                             '--expect', 'stop-failure'])

        target = output / 'target'
        env = dict(os.environ, CARGO_TARGET_DIR=str(target))
        efi = target / 'x86_64-unknown-uefi/release/musha-boot.efi'
        base = ['cargo', 'build', '--locked', '--release', '--target', 'x86_64-unknown-uefi',
                '-p', 'musha-boot']
        timings['metadata_build'] = run([*base, '--features', 'settings-write-partial-metadata'], env=env)
        timings['partial_metadata'] = run(['python3', 'tools/test-settings-write.py',
                                           '--efi', efi, '--output', output / 'partial-metadata',
                                           '--case', 'partial-metadata', '--sector-size', '4096',
                                           '--sectors-per-cluster', '1', '--qemu', args.qemu,
                                           '--firmware-dir', firmware])
        timings['lan_off_build'] = run([*base, '--no-default-features', '--features', 'qemu-debug'], env=env)
        timings['lan_off'] = run(['python3', 'tools/smoke-qemu.py', '--efi', efi,
                                  '--output', output / 'lan-off', '--firmware-dir', firmware,
                                  '--qemu', args.qemu, '--expect-network-disabled',
                                  '--keyboard-exit'])
    finally:
        (output / 'timings.json').write_text(json.dumps(timings, indent=2) + '\n')


if __name__ == '__main__':
    main()
