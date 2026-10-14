#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""Generate Rust startup marker constants from the exact removable README."""
import argparse
from pathlib import Path
from distribution import usb_readme,fnv
ROOT=Path(__file__).resolve().parents[1]
def render():
    data=usb_readme((ROOT/'VERSION').read_text().strip())
    return f'pub const BYTES: usize = {len(data)};\npub const HASH: u64 = 0x{fnv(data):016x};\n'
if __name__=='__main__':
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('output',type=Path);a=p.parse_args();a.output.write_text(render())
