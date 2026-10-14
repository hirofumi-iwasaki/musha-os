# SPDX-License-Identifier: Apache-2.0
"""Strict log checks shared by QEMU smoke tests and host regressions."""
import re


def check_hub_enumerations(text, depth):
    """The single-controller hub topology is enumerated at discovery and input."""
    starts = list(re.finditer(
        r"^MUSHA: XHCI_(SCAN|INPUT) BDF=([0-9A-F]{16})$", text, re.MULTILINE))
    if [m.group(1) for m in starts] != ['SCAN', 'INPUT']:
        raise RuntimeError('Hub topology requires one discovery and one input pass')
    if starts[0].group(2) != starts[1].group(2):
        raise RuntimeError('Hub input controller differs from discovery controller')
    if text[:starts[0].start()].splitlines().count('MUSHA: USB2_HUB_READY'):
        raise RuntimeError('Hub marker outside enumeration pass')
    counts = []
    for i, start in enumerate(starts):
        end = starts[i + 1].start() if i + 1 < len(starts) else len(text)
        lines = text[start.end():end].splitlines()
        completions = [n for n, line in enumerate(lines)
                       if line.startswith('MUSHA: USB_ENUMERATION_OK COUNT=')]
        if len(completions) != 1:
            raise RuntimeError(f'Hub {start.group(1)} pass needs one completed enumeration')
        boundary = completions[0]
        count = lines[:boundary].count('MUSHA: USB2_HUB_READY')
        if count != depth or 'MUSHA: USB2_HUB_READY' in lines[boundary:]:
            raise RuntimeError(f'Hub {start.group(1)} count differs: expected {depth}, got {count}')
        counts.append(count)
    return counts
