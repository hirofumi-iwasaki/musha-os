#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""Packaging gates, including version, provenance and optional docs snapshot."""
import importlib.util,subprocess,sys,unittest
from pathlib import Path
from unittest.mock import patch
spec=importlib.util.spec_from_file_location('distribution_tests',Path(__file__).with_name('test-distribution.py'));d=importlib.util.module_from_spec(spec);spec.loader.exec_module(d)
class PackageTests(unittest.TestCase):
    def setUp(self):d.DistributionTests.setUp(self)
    def test_reject_bad_version_and_flags_before_output(self):
        for kwargs,provenance in [({'version':'0.3.0'},self.prov),({},dict(self.prov,features=['probe'])),({},dict(self.prov,RUSTFLAGS='bad'))]:
            with self.assertRaises(ValueError):d.release.stage(self.root/'out',self.efi,provenance,**kwargs)
            self.assertFalse((self.root/'out').exists())
    def test_docs_snapshot_requires_stage2_index(self):
        original_is_file=Path.is_file
        index=d.release.ROOT/'docs/README.md'
        # Model a missing index explicitly; the real checkout now has one.
        with patch.object(Path,'is_file',lambda path:False if path==index else original_is_file(path)):
            with self.assertRaisesRegex(ValueError,'stage2 index'):d.release.stage(self.root/'out',self.efi,self.prov,docs_complete=True)
        self.assertFalse((self.root/'out').exists())
    def test_old_provenance_cannot_relabel_efi(self):
        with self.assertRaises(ValueError):d.release.validate_efi(self.efi,{'version':'0.3.0'},d.VERSION,'v'+d.VERSION,'a'*40,'')
    def test_explicit_sdk_roles_and_input_validation(self):
        sample=self.root/'sample.elf';sample.write_bytes(b'x'*64)
        roles=[f'c:hello.elf={sample}',f'rust:hello.elf={sample}',f'rust:start-record.elf={sample}']
        self.assertEqual(set(d.release.parse_sdk_samples(roles)['rust']),{'hello.elf','start-record.elf'})
        self.assertIsNone(d.release.parse_sdk_samples([]))
        for entries in [roles+[roles[0]],roles[:1],[f'python:hello.elf={sample}'],['bad'],[f'c:no.elf={sample}']]:
            with self.assertRaises(ValueError):d.release.parse_sdk_samples(entries)
        alias=self.root/'alias.elf';alias.symlink_to(sample)
        with self.assertRaises(ValueError):d.release.parse_sdk_samples([f'c:hello.elf={alias}'])
        sample.write_bytes(b'x')
        with self.assertRaises(ValueError):d.release.parse_sdk_samples(roles)
    def test_default_sdk_builder_connects_expected_roles(self):
        with patch.object(d.release.subprocess,'run') as run:
            samples=d.release.build_sdk_samples(self.root/'samples')
        self.assertEqual(run.call_count,2)
        self.assertEqual(samples['c']['hello.elf'].name,'hello-c.elf')
        self.assertEqual(samples['rust']['hello.elf'].name,'hello-rust.elf')
        self.assertEqual(samples['rust']['start-record.elf'].name,'start-record-rust.elf')
        self.assertTrue(all(call.kwargs['check'] for call in run.call_args_list))
    def test_cli_version_mismatch_precedes_build(self):
        r=subprocess.run([sys.executable,'-B',str(d.release.ROOT/'tools/prepare-release.py'),'--version','0.3.0'],capture_output=True,text=True)
        self.assertNotEqual(r.returncode,0);self.assertIn('VERSION mismatch',r.stderr)
if __name__=='__main__':unittest.main()
