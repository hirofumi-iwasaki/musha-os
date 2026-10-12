#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""Validate Actions input before any build; never create or change refs."""
import os
from pathlib import Path
import subprocess
from distribution import version_check

def identity(version,tag,commit,resolve,create_draft=False):
    version_check(version,tag or None)
    if create_draft and not tag: raise ValueError('Draft requires an approved existing tag')
    if tag and resolve('refs/tags/'+tag+'^{commit}')!=commit: raise ValueError('Checkout differs from tag')
    return tag,commit

if __name__=='__main__':
    run=lambda ref:subprocess.check_output(['git','rev-parse',ref],text=True).strip()
    tag=os.environ.get('REQUEST_TAG','')
    if os.environ.get('EVENT_REF','').startswith('refs/tags/'): tag=os.environ['EVENT_REF'][10:]
    tag,commit=identity(Path('VERSION').read_text().strip(),tag,run('HEAD'),run,os.environ.get('CREATE_DRAFT')=='true')
    with open(os.environ['GITHUB_OUTPUT'],'a') as out: out.write(f'tag={tag}\ncommit={commit}\n')
