#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
import json,pathlib,sys
root=pathlib.Path(__file__).resolve().parents[1]
s=json.loads((root/'abi/experimental.json').read_text())
c=['/* Generated experimental definitions; no public ABI guarantee. */','#ifndef MUSHA_ABI_H','#define MUSHA_ABI_H','#include <stdint.h>','#include <stddef.h>']
r=['// Generated experimental definitions; no public ABI guarantee.']
for k,v in s['constants'].items():
 c.append(f'#define MUSHA_{k} ({v})')
 r.append(f'pub const {k}: {"i32" if v<0 else "u64"} = {v};')
for cn,rn,key in [('musha_start_info_v1','MushaStartInfoV1','start_fields'),('musha_note_v1','MushaNoteV1','note_fields'),('musha_input_event','MushaInputEvent','input_fields')]:
 size=48 if key=='input_fields' else 64
 c.append(f'typedef struct {cn} {{');r.extend(['#[repr(C)]','#[derive(Clone, Copy, Debug, Default)]',f'pub struct {rn} {{'])
 for n,t,o in s[key]:
  c.append(f'    {("uint" if t[0]=="u" else "int")}{t[1:]}_t {n};');r.append(f'    pub {n}: {t},')
 c.extend([f'}} {cn};',f'_Static_assert(sizeof({cn})=={size},"size");',f'_Static_assert(_Alignof({cn})==8,"alignment");'])
 r.extend(['}','const _: () = {',f'    assert!(core::mem::size_of::<{rn}>() == {size});',f'    assert!(core::mem::align_of::<{rn}>() == 8);'])
 for n,t,o in s[key]:
  c.append(f'_Static_assert(offsetof({cn},{n})=={o},"{n}");');r.append(f'    assert!(core::mem::offset_of!({rn}, {n}) == {o});')
 r.append('};')
c.append('#endif')
for path,lines in [('sysroot/include/musha/abi.h',c),('rust/musha-sys/src/generated.rs',r)]:
 if '--c-only' in sys.argv and path.startswith('rust/'): continue
 p=root/path;data='\n'.join(lines)+'\n'
 if '--check' in sys.argv: assert p.read_text()==data, str(p)+' differs from schema'
 else:p.parent.mkdir(parents=True,exist_ok=True);p.write_text(data)
ledger=root/'docs/application-abi.md'
if not ledger.exists(): ledger=root.parent/'docs/application-abi.md'
if '--check' in sys.argv and ledger.exists():
 text=ledger.read_text()
 for n,v in s['constants'].items():assert f'| `{n}` | `{v}` |' in text,n+' ledger mismatch'
 for n,t,o in s['start_fields']:assert f'| `{n}` | `{t}` | {o} |' in text,n+' offset mismatch'
