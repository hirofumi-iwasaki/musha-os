#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""Validate PM-epoch timing pairs captured by a diagnostic QEMU Hello run."""
import argparse
import json
import re
from pathlib import Path
p = argparse.ArgumentParser(description=__doc__)
p.add_argument('log', type=Path)
p.add_argument('--hub-depth', type=int, default=0)
a = p.parse_args()
text = a.log.read_text()
lines = [x.removeprefix('MUSHA: TIMING ') for x in text.splitlines() if x.startswith('MUSHA: TIMING ')]
rows = []
for i, line in enumerate(lines):
    match = re.fullmatch(r'TIME ([0-9A-F]{4}) (.+) START Some\((\d+)\)', line)
    if not match:
        continue
    end = re.fullmatch(r'END Some\((\d+)\) MS Some\((\d+)\) (.+)', lines[i+1])
    assert end, lines[i+1]
    start, stop, elapsed = int(match[3]), int(end[1]), int(end[2])
    assert stop >= start and stop-start == elapsed, (line, lines[i+1])
    assert end[3] not in ('EARLY RETURN', 'TIMEOUT', 'CLOCK STALLED'), end[3]
    rows.append(dict(bdf=match[1], stage=match[2], start_ms=start, end_ms=stop, elapsed_ms=elapsed, result=end[3]))
required = {'OWNERSHIP','INITIAL HALT','RESET','ATTACH WAIT','SS WAIT','ENUMERATE','QUIESCE','ELF READ','LIVE'}
assert required <= {r['stage'] for r in rows}, required - {r['stage'] for r in rows}
assert any('WPR COUNT' in x for x in lines)
assert 'TIMING LIMIT' not in text
if a.hub_depth:
    assert text.count('MUSHA: USB2_HUB_READY') >= a.hub_depth
output = a.log.with_name('timings.json')
output.write_text(json.dumps({'scope':'QEMU only, not Ryzen measurements','phases':rows},indent=2)+'\n')
print(f'PASS: {len(rows)} timing spans; {output}')
