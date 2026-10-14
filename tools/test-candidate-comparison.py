#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
import importlib.util,json,unittest,copy
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
            with patch.object(d.comparison,'inventory',side_effect=[a,b]),patch.object(d.comparison.subprocess,'check_output',return_value=paths):
                return d.comparison.compare(self.left,self.right,'docs-only')
        self.assertTrue(compare('docs/README.md\n')['passed'])
        for path in ('VERSION','tools/distribution.py','docs/file-viewer.md','docs/sdk-guide.md'):
            self.assertFalse(compare(path+'\n')['passed'])
        b['runtime_payload']['QEMU image']={'bytes':1,'sha256':'e'*64}
        self.assertFalse(compare('docs/README.md\n')['passed'])
        b['runtime_payload']=copy.deepcopy(a['runtime_payload']);b['runtime_payload']['rust SDK/fixture.elf']={'bytes':1,'sha256':'f'*64}
        self.assertFalse(compare('docs/README.md\n')['passed'])

if __name__=='__main__':unittest.main()
