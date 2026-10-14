#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""Packaging gates, including version, provenance and optional docs snapshot."""
import importlib.util,subprocess,sys,unittest
from pathlib import Path
spec=importlib.util.spec_from_file_location('distribution_tests',Path(__file__).with_name('test-distribution.py'));d=importlib.util.module_from_spec(spec);spec.loader.exec_module(d)
class PackageTests(unittest.TestCase):
    def setUp(self):d.DistributionTests.setUp(self)
    def test_reject_bad_version_and_flags_before_output(self):
        for kwargs,provenance in [({'version':'0.3.0'},self.prov),({},dict(self.prov,features=['probe'])),({},dict(self.prov,RUSTFLAGS='bad'))]:
            with self.assertRaises(ValueError):d.release.stage(self.root/'out',self.efi,provenance,**kwargs)
            self.assertFalse((self.root/'out').exists())
    def test_docs_snapshot_requires_stage2_index(self):
        with self.assertRaisesRegex(ValueError,'stage2 index'):d.release.stage(self.root/'out',self.efi,self.prov,docs_complete=True)
        self.assertFalse((self.root/'out').exists())
    def test_old_provenance_cannot_relabel_efi(self):
        with self.assertRaises(ValueError):d.release.validate_efi(self.efi,{'version':'0.3.0'},d.VERSION,'v'+d.VERSION,'a'*40,'')
    def test_cli_version_mismatch_precedes_build(self):
        r=subprocess.run([sys.executable,'-B',str(d.release.ROOT/'tools/prepare-release.py'),'--version','0.3.0'],capture_output=True,text=True)
        self.assertNotEqual(r.returncode,0);self.assertIn('VERSION mismatch',r.stderr)
if __name__=='__main__':unittest.main()
