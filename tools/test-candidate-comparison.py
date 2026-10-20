#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
import importlib.util,json,unittest,copy,subprocess
from unittest.mock import patch
from pathlib import Path
spec=importlib.util.spec_from_file_location('distribution_tests',Path(__file__).with_name('test-distribution.py'));d=importlib.util.module_from_spec(spec);spec.loader.exec_module(d)
class ComparisonTests(unittest.TestCase):
    def setUp(self):
        d.DistributionTests.setUp(self)
        self.left=d.release.stage(self.root/'left',self.efi,self.prov)
        self.right=d.release.stage(self.root/'right',self.efi,self.prov)
    def test_full_asset_same_source(self):self.assertTrue(d.comparison.compare(self.left,self.right,'same-source')['passed'])
    def test_corrupt_archive(self):
        (self.right/f'musha-os-{d.VERSION}-c-sdk.zip').write_bytes(b'corrupt')
        with self.assertRaisesRegex(ValueError,'Checksum'):d.comparison.compare(self.left,self.right,'same-source')
    def test_missing_checksum(self):
        p=self.right/'SHA256SUMS';p.write_text('\n'.join(p.read_text().splitlines()[1:])+'\n')
        with self.assertRaises(ValueError):d.comparison.compare(self.left,self.right,'efi')
    def test_size_metadata_is_remeasured_from_zip_bytes(self):
        p=self.right/'build-manifest.json';m=json.loads(p.read_text());m['efi_size']['sections']['.data']['virtual_bytes']+=1;p.write_text(json.dumps(m))
        checks=self.right/'SHA256SUMS';checks.write_text(''.join(f'{d.digest(f)}  {f.name}\n' for f in sorted(self.right.iterdir()) if f.name!='SHA256SUMS'))
        with self.assertRaisesRegex(ValueError,'EFI size manifest differs'):d.comparison.inventory(self.right)

    def test_source_change_not_same_source(self):
        p=self.right/'build-manifest.json';m=json.loads(p.read_text());m['commit']='b'*40;p.write_text(json.dumps(m))
        checks=self.right/'SHA256SUMS';checks.write_text(''.join(f'{d.digest(f)}  {f.name}\n' for f in sorted(self.right.iterdir()) if f.name!='SHA256SUMS'))
        self.assertFalse(d.comparison.compare(self.left,self.right,'same-source')['passed']);self.assertTrue(d.comparison.compare(self.left,self.right,'efi')['passed'])
    def test_docs_only_classifier_requires_complete_identical_runtime(self):
        # Simulate already validated stage4 inventories; this is not Rust execution evidence.
        a=d.comparison.inventory(self.left);b=copy.deepcopy(a);b['source_commit']='b'*40
        for inventory in (a,b):
            for language in ('c','rust'):
                entry={'bytes':8,'sha256':('c' if language=='c' else 'd')*64}
                inventory['sdk_status'][language]={'executable_sdk':True,'sample_elfs':{'fixture.elf':entry}}
                inventory['runtime_payload'][language+' SDK/fixture.elf']=entry
        def compare(paths):
            with patch.object(d.comparison,'inventory',side_effect=[a,b]),patch.object(d.comparison.subprocess,'check_output',return_value=paths),patch.object(d.comparison.subprocess,'run',return_value=subprocess.CompletedProcess([],0)):
                return d.comparison.compare(self.left,self.right,'docs-only')
        self.assertTrue(compare('docs/README.md\n')['passed'])
        for path in ('VERSION','tools/distribution.py','docs/file-viewer.md','docs/sdk-guide.md'):
            self.assertFalse(compare(path+'\n')['passed'])
        b['runtime_payload']['QEMU image']={'bytes':1,'sha256':'e'*64}
        self.assertFalse(compare('docs/README.md\n')['passed'])
        b['runtime_payload']=copy.deepcopy(a['runtime_payload']);b['runtime_payload']['rust SDK/fixture.elf']={'bytes':1,'sha256':'f'*64}
        self.assertFalse(compare('docs/README.md\n')['passed'])

    def test_docs_only_requires_actual_successor_ancestry(self):
        repo=self.root/'history';repo.mkdir()
        def git(*args):
            return subprocess.check_output(['git',*args],cwd=repo,text=True).strip()
        git('init','-q')
        git('config','user.name','Comparison Test')
        git('config','user.email','comparison@example.invalid')
        (repo/'docs').mkdir()
        (repo/'docs/README.md').write_text('base\n')
        git('add','docs/README.md');git('commit','-qm','base')
        base=git('rev-parse','HEAD')
        (repo/'docs/README.md').write_text('successor\n')
        git('commit','-qam','docs successor')
        successor=git('rev-parse','HEAD')
        git('checkout','-q','-b','other',base)
        (repo/'docs/README.md').write_text('other branch\n')
        git('commit','-qam','divergent docs')
        divergent=git('rev-parse','HEAD')
        a=d.comparison.inventory(self.left)
        for language in ('c','rust'):
            entry={'bytes':8,'sha256':('c' if language=='c' else 'd')*64}
            a['sdk_status'][language]={'executable_sdk':True,'sample_elfs':{'fixture.elf':entry}}
            a['runtime_payload'][language+' SDK/fixture.elf']=entry
        def compare(left_commit,right_commit):
            left=copy.deepcopy(a);right=copy.deepcopy(a)
            left['source_commit']=left_commit;right['source_commit']=right_commit
            with patch.object(d.comparison,'ROOT',repo),patch.object(d.comparison,'inventory',side_effect=[left,right]):
                return d.comparison.compare(self.left,self.right,'docs-only')
        self.assertTrue(compare(base,successor)['passed'])
        self.assertFalse(compare(successor,base)['passed'])
        self.assertFalse(compare(successor,divergent)['passed'])

if __name__=='__main__':unittest.main()
