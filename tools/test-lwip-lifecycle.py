#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""Compile the pinned lwIP + original shim natively and test 1000 safe restarts."""
import argparse
import json
from pathlib import Path
import re
import subprocess
ROOT = Path(__file__).resolve().parents[1]
p = argparse.ArgumentParser(description=__doc__)
p.add_argument('--output', type=Path, required=True)
a = p.parse_args()
o = a.output.resolve()
o.mkdir(exist_ok=True, parents=True)
source = (ROOT/'boot/build.rs').read_text().split('let sources = [',1)[1].split('];',1)[0]
files = [ROOT/'third_party/lwip/src'/name for name in re.findall(r'"([^"]+\.c)"',source)]
cmd = ['clang','-std=c11','-O2','-fno-builtin','-I',str(ROOT/'net/port'),'-I',str(ROOT/'third_party/lwip/src/include'),*[str(x) for x in files],str(ROOT/'net/port/shim.c'),str(ROOT/'net/tests/lifecycle.c'),'-o',str(o/'lifecycle')]
with (o/'build.log').open('w') as log:
    subprocess.run(cmd,check=True,stdout=log,stderr=subprocess.STDOUT)
subprocess.run([str(o/'lifecycle')],check=True)
(o/'report.json').write_text(json.dumps({'passed':True,'sessions':1000,'scope':'native pinned lwIP endpoint/pbuf/ARP/queue/time lifetime; no NIC hardware proof','command':cmd},indent=2)+'\n')
print('PASS 1000 lwIP endpoint restarts, no queue/pbuf/netif/UDP leak or clock reset')
