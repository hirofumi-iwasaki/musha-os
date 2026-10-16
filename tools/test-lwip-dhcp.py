#!/usr/bin/env python3
# Copyright 2026 Hirofumi Iwasaki
# SPDX-License-Identifier: Apache-2.0
"""Native pinned lwIP: real DHCP packets/ACD/renew/rebind/expiry and endpoint reuse."""
import argparse,ctypes,importlib.util,json,re,subprocess,sys
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
p=argparse.ArgumentParser(description=__doc__);p.add_argument('--output',type=Path,required=True);a=p.parse_args();out=a.output.resolve();out.mkdir(parents=True,exist_ok=True)
s=importlib.util.spec_from_file_location('dhcp_fixture',ROOT/'tools/dhcp-fixture.py');fixture=importlib.util.module_from_spec(s);s.loader.exec_module(fixture)
assert_source=out/'assert.c';assert_source.write_text('''#include "lwip/stats.h"
#include "lwip/memp.h"
_Noreturn void musha_lwip_assert(void){__builtin_trap();}
int musha_test_pools(int save){static unsigned used[MEMP_MAX],heap;for(unsigned i=0;i<MEMP_MAX;i++){if(save)used[i]=lwip_stats.memp[i]->used;else if(used[i]!=lwip_stats.memp[i]->used)return 1;}if(save)heap=lwip_stats.mem.used;else if(heap!=lwip_stats.mem.used)return 2;return 0;}
''')
source=(ROOT/'boot/build.rs').read_text().split('let sources = [',1)[1].split('];',1)[0]
files=[ROOT/'third_party/lwip/src'/n for n in re.findall(r'"([^"]+\.c)"',source)]
libpath=out/('lwip.dylib' if sys.platform=='darwin' else 'lwip.so')
cmd=['clang','-dynamiclib' if sys.platform=='darwin' else '-shared','-fPIC','-std=c11','-O2','-I',str(ROOT/'net/port'),'-I',str(ROOT/'third_party/lwip/src/include'),*[str(x)for x in files],str(ROOT/'net/port/shim.c'),str(assert_source),'-o',str(libpath)]
with (out/'build.log').open('w')as log:subprocess.run(cmd,stdout=log,stderr=subprocess.STDOUT,check=True)
lib=ctypes.CDLL(str(libpath));ptr=ctypes.POINTER(ctypes.c_ubyte)
lib.musha_lwip_init.argtypes=[ptr,ctypes.c_int];lib.musha_lwip_tx.argtypes=[ptr,ctypes.c_uint16];lib.musha_lwip_input.argtypes=[ptr,ctypes.c_uint16];lib.musha_lwip_address.argtypes=[ptr];lib.musha_lwip_poll.argtypes=[ctypes.c_uint32]
mac=(ctypes.c_ubyte*6)(2,0,0,0,0,1);buf=(ctypes.c_ubyte*1514)();now=0;rows=[]
def address():
 b=(ctypes.c_ubyte*4)();valid=lib.musha_lwip_address(b);return bytes(b)if valid else None
def start(echo_enabled=False):
 assert lib.musha_lwip_init(mac,int(echo_enabled))==0
 assert address() is None,'static or previous address survived restart'
def tick(server,drop_renew=False):
 global now
 # Bound work like the runtime: an immediate NAK may enqueue another DISCOVER.
 for _ in range(8):
  n=lib.musha_lwip_tx(buf,len(buf))
  if not n:break
  assert 14<=n<=1514
  packet=bytes(buf[:n]);reply=server.handle(packet)
  if drop_renew and lib.musha_lwip_dhcp_state()==5:reply=None
  if reply:
   data=(ctypes.c_ubyte*len(reply)).from_buffer_copy(reply);assert lib.musha_lwip_input(data,len(reply))==0
 now+=100;lib.musha_lwip_poll(now)
def acquire(server):
 for _ in range(200):
  tick(server)
  if address():break
 assert address()==server.ip and address()!=bytes([10,0,2,15]),'no dynamic lease'
 assert {x['kind']for x in server.events}>={1,3}
def stop():lib.musha_lwip_stop();assert address()is None;assert lib.musha_lwip_tx(buf,len(buf))==0
server=fixture.Server('10.0.2.77',lease=20,renew=4,rebind=12);start(echo_enabled=True);acquire(server);requests=sum(x['kind']==3 for x in server.events)
for _ in range(100):tick(server)
assert address()==server.ip and sum(x['kind']==3 for x in server.events)>requests
stop();rows.append({'case':'dynamic-lease-and-renew','passed':True})
for name,configure in [('wrong-xid',lambda s:setattr(s,'wrong_xid',True)),('nak',lambda s:setattr(s,'nak',True)),('no-server',lambda s:setattr(s,'drop',True))]:
 server=fixture.Server('10.0.2.77');configure(server);start()
 for _ in range(310):tick(server);assert address()is None
 assert server.events;stop();rows.append({'case':name,'passed':True})
server=fixture.Server('10.0.2.77',lease=20,renew=4,rebind=8);start();acquire(server);states=set()
for _ in range(120):
 states.add(lib.musha_lwip_dhcp_state());tick(server,drop_renew=True)
assert 4 in states and address()==server.ip
stop();rows.append({'case':'renew-drop-rebind-ack','passed':True})
server=fixture.Server('10.0.2.77',lease=12,renew=4,rebind=8);start();acquire(server);server.drop=True
for _ in range(200):tick(server)
assert address()is None
stop();rows.append({'case':'lease-expiry-no-fallback','passed':True})
assert lib.musha_test_pools(1)==0
for _ in range(50):
 server=fixture.Server('10.0.2.77');start();acquire(server);stop();assert lib.musha_test_pools(0)==0
rows.append({'case':'50-successful-lease-restarts-pools-stable','passed':True})
(out/'report.json').write_text(json.dumps({'scope':'native real pinned lwIP DHCP packets, no physical NIC evidence','cases':rows,'monotonic_ms':now,'command':cmd},indent=2)+'\n')
print('PASS native DHCP',len(rows),'scenarios; 50 lease restarts; no fixed-IP fallback')
