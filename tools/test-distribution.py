#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
import gzip
import hashlib
import importlib.util
import io
import json
from pathlib import Path
import struct
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch
import zipfile
import zlib
from distribution import make_zip, read_zip, digest, json_bytes, usb_readme

def load(name):
    spec=importlib.util.spec_from_file_location(name,Path(__file__).with_name(name+'.py'))
    m=importlib.util.module_from_spec(spec); spec.loader.exec_module(m); return m
usb=load('make-usb-image'); deploy=load('deploy-image'); update=load('update-usb'); release=load('prepare-release'); draft=load('draft-release'); ident=load('candidate-identity')

VERSION=(release.ROOT/'VERSION').read_text().strip()

class DistributionTests(unittest.TestCase):
    def setUp(self):
        self.tmp=tempfile.TemporaryDirectory(); self.addCleanup(self.tmp.cleanup); self.root=Path(self.tmp.name).resolve()
        self.efi=self.root/'test.efi'; b=bytearray(1800); b[:2]=b'MZ'; struct.pack_into('<I',b,60,128); b[128:132]=b'PE\0\0'
        struct.pack_into('<H',b,132,0x8664); struct.pack_into('<H',b,152,0x20b); struct.pack_into('<H',b,220,10); self.efi.write_bytes(b)
        self.prov={'commit':'a'*40,'features':[],'working_tree':'','tag':f'v{VERSION}'}
    def package(self):
        return release.stage(self.root/'release',self.efi,self.prov)
    def zip(self,payload=None):
        p=self.root/'update.zip'; make_zip(p,payload or {'EFI/BOOT/BOOTX64.EFI':self.efi.read_bytes(),'DIAGNOSE.TXT':b'text'},f'{VERSION}',self.prov); return p
    def volume(self):
        p=self.root/'volume'; (p/'EFI/BOOT').mkdir(parents=True); (p/'EFI/BOOT/BOOTX64.EFI').write_bytes(b'old'); (p/'user.txt').write_bytes(b'keep'); return p
    def test_image_zip_payload_and_manifest(self):
        out=self.package(); m=json.loads((out/'build-manifest.json').read_text()); version=f'musha-os-v{VERSION}-x64'
        image=gzip.decompress((out/(version+'-usb.img.gz')).read_bytes())
        inspected=deploy.inspect(io.BytesIO(image),len(image))
        um,payload=read_zip(out/(version+'-files.zip'))
        self.assertEqual(inspected['efi'],self.efi.read_bytes()); self.assertEqual(payload['EFI/BOOT/BOOTX64.EFI'],inspected['efi'])
        self.assertEqual(payload['README.TXT'], usb_readme(VERSION))
        self.assertIn('README.TXT', m['payload'])
        self.assertEqual(m['update_schema'],2)
        self.assertEqual(m['retired_files'],['MUSHA.TXT'])
        self.assertEqual(m['image_uncompressed'],{'bytes':len(image),'sha256':hashlib.sha256(image).hexdigest()})
        for name in ['LICENSE','NOTICE','third_party/lwip/COPYING','third_party/freebsd-e1000/COPYING','third_party/freebsd-apple-bce/COPYING','third_party/r-efi/AUTHORS']: self.assertIn(name,payload)
        with zipfile.ZipFile(out/(version+'-deployment.zip')) as z: self.assertIn('tools/update-usb.py',z.namelist())
        draft.validate(out,f'v{VERSION}','a'*40)
    def test_readme_contract_and_fat_chain(self):
        expected = usb_readme(VERSION)
        self.assertLess(len(expected), 4096)
        self.assertTrue(expected.isascii())
        self.assertNotIn(b'\n', expected.replace(b'\r\n', b''))
        self.assertTrue(all(len(line) <= 72 for line in expected.splitlines()))
        self.assertIn(f'musha-os-v{VERSION}-x64-c-sdk.zip'.encode(), expected)
        self.assertIn(b'AUTORUN.ELF', expected)
        for kind in ('esp', 'basic-data'):
            image = usb.create(self.root/(kind+'-readme.img'), self.efi, partition_type=kind).read_bytes()
            first = struct.unpack_from('<Q', image, 1024+32)[0]
            boot = image[first*512:(first+1)*512]
            spc = boot[13]; reserved = struct.unpack_from('<H', boot, 14)[0]
            fat_size = struct.unpack_from('<I', boot, 36)[0]
            fat = (first+reserved)*512
            data = (first+reserved+2*fat_size)*512
            root = image[data:data+spc*512]
            entries = [root[i:i+32] for i in range(0,len(root),32)]
            entry = next(e for e in entries if e[:11] == b'README  TXT')
            cluster = struct.unpack_from('<H',entry,26)[0]
            length = struct.unpack_from('<I',entry,28)[0]
            contents = b''
            while cluster < 0x0ffffff8:
                offset = data+(cluster-2)*spc*512
                contents += image[offset:offset+spc*512]
                cluster = struct.unpack_from('<I',image,fat+cluster*4)[0] & 0x0fffffff
            self.assertEqual(contents[:length], expected)

    def test_partition_types_reproducible_and_preserved(self):
        for kind in ('basic-data', 'esp'):
            with self.subTest(kind=kind):
                src=usb.create(self.root/(kind+'.img'),self.efi,partition_type=kind)
                twin=usb.create(self.root/(kind+'-twin.img'),self.efi,partition_type=kind)
                self.assertEqual(digest(src),digest(twin))
                dst=self.root/(kind+'-deployed.img')
                deploy.adjust(src,dst,128*1024**2,apply=True)
                with src.open('rb') as f: original=deploy.inspect(f,src.stat().st_size)
                with dst.open('rb') as f: result=deploy.inspect(f,dst.stat().st_size)
                self.assertEqual(result['array'],original['array'])
                self.assertEqual(result['array'][:16],usb.PARTITION_TYPES[kind])
                self.assertEqual(result['array'][56:128].decode('utf-16le').rstrip('\0'),'Musha-OS')
                self.assertEqual(result['efi'],self.efi.read_bytes())
        with self.assertRaises(ValueError): usb.create(self.root/'bad-kind.img',self.efi,partition_type='unknown')
        self.assertFalse((self.root/'bad-kind.img').exists())

    def test_update_identity_accepts_both_partition_types_only(self):
        import plistlib
        v=self.root/'volume'
        part={'ParentWholeDisk':'disk99','MountPoint':str(v),'FilesystemName':'MS-DOS FAT32','WritableVolume':True,'VolumeUUID':'test','DiskUUID':'partition'}
        disk={'Internal':False,'BusProtocol':'USB','VirtualOrPhysical':'Physical','WholeDisk':True,'DeviceBlockSize':512,'DeviceIdentifier':'disk99','MediaName':'fixture','TotalSize':128*1024**2}
        for kind in ('EFI','Microsoft Basic Data','Apple_APFS'):
            with patch.object(update.sys,'platform','darwin'), patch('subprocess.check_output',side_effect=[plistlib.dumps(dict(part,Content=kind)),plistlib.dumps(disk)]):
                if kind=='Apple_APFS':
                    with self.assertRaises(ValueError): update.mac_identity(v)
                else: self.assertEqual(update.mac_identity(v)['partition_type'],kind)

    def test_gpt_capacity_and_bounded_ranges(self):
        image=usb.create(self.root/'source.img',self.efi)
        for capacity in [64*1024**2,128*1024**2,32*1024**3+512]:
            out=self.root/f'{capacity}.img'
            dry=deploy.adjust(image,out,capacity); self.assertFalse(out.exists())
            plan=deploy.adjust(image,out,capacity,apply=True)
            self.assertEqual(sum(n for _,n in plan['write_ranges']),64*1024**2)
            self.assertEqual(out.stat().st_size,capacity)
            with out.open('rb') as f:
                result=deploy.inspect(f,capacity); self.assertEqual(result['efi'],self.efi.read_bytes())
                # Independent CRC and location checks, not the adjuster validator.
                f.seek(capacity-512); header=bytearray(f.read(512)); crc=struct.unpack_from('<I',header,16)[0]; header[16:20]=b'\0'*4
                self.assertEqual(zlib.crc32(header[:92]),crc)
                self.assertEqual(struct.unpack_from('<QQ',header,24),(capacity//512-1,1))
                if capacity>128*1024**2:
                    f.seek(128*1024**2); self.assertEqual(f.read(512),bytes(512))
            self.assertLess(out.stat().st_blocks*512,128*1024**2)
            with self.assertRaises(ValueError): deploy.adjust(image,out,capacity,apply=True)
    def test_reject_size_sector_corruption_before_write(self):
        image=usb.create(self.root/'source.img',self.efi); out=self.root/'out.img'
        for size,sector in [(1024,512),(64*1024**2+1,512),(128*1024**2,4096)]:
            with self.assertRaises(ValueError): deploy.adjust(image,out,size,sector,True)
            self.assertFalse(out.exists())
        with image.open('r+b') as f: f.seek(512+16); f.write(b'BAD!')
        with self.assertRaises(ValueError): deploy.adjust(image,out,128*1024**2,apply=True)
        self.assertFalse(out.exists())
    def test_changed_only_backup_no_subprocess(self):
        z=self.zip(); v=self.volume(); (v/'DIAGNOSE.TXT').write_bytes(b'text'); before=(v/'DIAGNOSE.TXT').stat().st_mtime_ns
        with patch('subprocess.run',side_effect=AssertionError('No image/raw command')),patch('subprocess.check_output',side_effect=AssertionError('No raw read')):
            dry=update.update(z,v,self.root/'backup'); self.assertEqual(dry['changed'],['EFI/BOOT/BOOTX64.EFI','update-manifest.json']); self.assertFalse((self.root/'backup').exists())
            result=update.update(z,v,self.root/'backup',True)
            self.assertTrue(result['applied']); self.assertEqual((v/'user.txt').read_bytes(),b'keep')
            self.assertEqual((v/'DIAGNOSE.TXT').stat().st_mtime_ns,before)
            self.assertEqual((self.root/'backup/EFI/BOOT/BOOTX64.EFI').read_bytes(),b'old')
            self.assertEqual(update.update(z,v,self.root/'unused',True)['changed'],[]); self.assertFalse((self.root/'unused').exists())
    def test_retired_marker_dry_run_backup_remove_and_verified_files(self):
        out=self.package(); z=out/f'musha-os-v{VERSION}-x64-files.zip'
        m,payload=read_zip(z); v=self.volume()
        (v/'MUSHA.TXT').write_bytes(b'locally edited marker')
        (v/'AUTORUN.ELF').write_bytes(b'user program')
        (v/'musha-install.json').write_text('{"layout_version":1}')
        dry=update.update(z,v,self.root/'backup')
        self.assertEqual(dry['removed'],['MUSHA.TXT'])
        self.assertTrue((v/'MUSHA.TXT').exists())
        self.assertFalse((self.root/'backup').exists())
        result=update.update(z,v,self.root/'backup',True)
        self.assertEqual((self.root/'backup/MUSHA.TXT').read_bytes(),b'locally edited marker')
        self.assertFalse((v/'MUSHA.TXT').exists())
        self.assertEqual(result['verified-absent'],['MUSHA.TXT'])
        self.assertEqual(set(result['verified-files']),set(payload)|{'update-manifest.json'})
        self.assertEqual(sum(n.startswith('third_party/') for n in result['verified-files']),8)
        self.assertEqual(json.loads((v/'update-manifest.json').read_text()),m)
        self.assertEqual((v/'AUTORUN.ELF').read_bytes(),b'user program')
        again=update.update(z,v,self.root/'unused',True)
        self.assertFalse(again['changed'] or again['removed'])
        self.assertEqual(again['verified-files'],result['verified-files'])
        self.assertFalse((self.root/'unused').exists())

    def test_retirement_only_and_failure_before_retirement(self):
        z=self.zip(); v=self.volume()
        update.update(z,v,self.root/'first',True)
        (v/'MUSHA.TXT').write_bytes(b'old marker')
        result=update.update(z,v,self.root/'retirement',True)
        self.assertEqual(result['changed'],[])
        self.assertEqual(result['removed_completed'],['MUSHA.TXT'])
        (v/'MUSHA.TXT').write_bytes(b'retry marker')
        (v/'EFI/BOOT/BOOTX64.EFI').write_bytes(b'old EFI')
        with patch('os.replace',side_effect=OSError('interrupted')):
            with self.assertRaises(OSError):update.update(z,v,self.root/'interrupted',True)
        self.assertEqual((v/'MUSHA.TXT').read_bytes(),b'retry marker')
        self.assertEqual((self.root/'interrupted/MUSHA.TXT').read_bytes(),b'retry marker')

    def test_legacy_schema_and_retirement_rejections(self):
        z=self.zip(); m,payload=read_zip(z)
        for i,retired in enumerate([['user.txt'],['../outside'],['MUSHA.TXT','MUSHA.TXT'],['musha.txt'],[]]):
            bad=self.root/f'retired-{i}.zip'
            with zipfile.ZipFile(bad,'w') as f:
                for n,b in payload.items():f.writestr(n,b)
                f.writestr('update-manifest.json',json_bytes(dict(m,retired_files=retired)))
            with self.assertRaises(ValueError):read_zip(bad)
        legacy=self.root/'legacy.zip'
        with zipfile.ZipFile(legacy,'w') as f:
            for n,b in payload.items():f.writestr(n,b)
            old=dict(m,schema=1); old.pop('retired_files')
            f.writestr('update-manifest.json',json_bytes(old))
        v=self.volume(); (v/'MUSHA.TXT').write_bytes(b'keep on legacy update')
        update.update(legacy,v,self.root/'legacy-backup',True)
        self.assertTrue((v/'MUSHA.TXT').exists())
        (v/'MUSHA.TXT').unlink(); (v/'MUSHA.TXT').symlink_to(self.root/'outside')
        with self.assertRaises(ValueError):update.update(z,v,self.root/'symlink-backup',True)
        self.assertFalse((self.root/'symlink-backup').exists())

    def test_failure_preserves_old_and_records_backup(self):
        z=self.zip(); v=self.volume()
        with patch('os.replace',side_effect=OSError('simulated failure')):
            with self.assertRaises(OSError): update.update(z,v,self.root/'backup',True)
        self.assertEqual((v/'EFI/BOOT/BOOTX64.EFI').read_bytes(),b'old')
        self.assertIn('error',json.loads((self.root/'backup/update-result.json').read_text()))
        self.assertEqual(list(v.rglob('.musha-update-*')),[])
    def test_zip_traversal_alias_symlink_and_hash(self):
        for i,name in enumerate(['../escape','/absolute','a\\b','EFI/../bad','a:b','NUL.txt']):
            z=self.root/f'bad{i}.zip'
            with zipfile.ZipFile(z,'w') as f: f.writestr(name,b'x')
            with self.assertRaises(ValueError): read_zip(z)
        z=self.root/'link.zip'
        with zipfile.ZipFile(z,'w') as f:
            n=zipfile.ZipInfo('link'); n.external_attr=0o120777<<16; f.writestr(n,b'/tmp')
        with self.assertRaises(ValueError): read_zip(z)
        z=self.root/'alias.zip'
        with zipfile.ZipFile(z,'w') as f: f.writestr('ABC',b'a'); f.writestr('abc',b'b')
        with self.assertRaises(ValueError): read_zip(z)
        z=self.zip(); v=self.volume(); (v/'DIAGNOSE.TXT').symlink_to(self.root/'outside')
        with self.assertRaises(ValueError): update.update(z,v,self.root/'backup',True)
        self.assertFalse((self.root/'backup').exists())
    def test_unknown_layout_space_and_changed_device(self):
        z=self.zip(); v=self.volume(); (v/'musha-install.json').write_text('{"layout_version":2}')
        with self.assertRaises(ValueError): update.update(z,v,self.root/'backup',True)
        (v/'musha-install.json').unlink()
        with patch('shutil.disk_usage',return_value=type('Usage',(),{'free':0})()):
            with self.assertRaises(ValueError): update.update(z,v,self.root/'backup',True)
        calls=iter([{'id':1},{'id':2}])
        with self.assertRaises(ValueError): update.update(z,v,self.root/'backup',True,lambda _:next(calls))
        self.assertEqual((v/'EFI/BOOT/BOOTX64.EFI').read_bytes(),b'old')
    def test_bad_versions_features_and_dirty_draft(self):
        for version in ['../bad','1.2','0.2.0\n','$(bad)']:
            with self.assertRaises(ValueError): release.stage(self.root/'bad',self.efi,self.prov,version=version)
        for field,value in [('features',['qemu-debug']),('RUSTFLAGS','--cfg foo')]:
            with self.assertRaises(ValueError): release.stage(self.root/'bad',self.efi,dict(self.prov,**{field:value}))
        with self.assertRaises(ValueError): ident.identity(f'{VERSION}','v0.1.0','a'*40,lambda _: 'a'*40)
        with self.assertRaises(ValueError): ident.identity(f'{VERSION}',f'v{VERSION}','a'*40,lambda _: 'b'*40)
        out=self.package(); p=out/'build-manifest.json'; m=json.loads(p.read_text()); m['working_tree']='dirty'; p.write_bytes(json_bytes(m))
        with self.assertRaises(ValueError): draft.validate(out,f'v{VERSION}','a'*40)
    def test_draft_api_mock_no_publish_no_tag_creation(self):
        out=self.package(); calls=[]
        def api(*a):
            calls.append(a)
            if 'git/ref/tags' in a[-1]: return json_bytes({'object':{'type':'commit','sha':'a'*40}})
            if '/releases/tags/' in a[-1]: raise subprocess.CalledProcessError(1,a,stderr=b'Not Found (HTTP 404)')
            return b''
        self.assertIn('Dry-run',draft.attach(out,f'v{VERSION}','a'*40,api=api)); self.assertFalse(any(a[0]=='release' for a in calls))
        draft.attach(out,f'v{VERSION}','a'*40,True,api)
        create=[a for a in calls if a[0]=='release'][0]; self.assertIn('--draft',create); self.assertIn('--verify-tag',create); self.assertNotIn('--clobber',create)
        def published(*a):
            if 'git/ref/tags' in a[-1]: return json_bytes({'object':{'type':'commit','sha':'a'*40}})
            return json_bytes({'draft':False})
        with self.assertRaises(ValueError): draft.attach(out,f'v{VERSION}','a'*40,True,published)
    def test_identical_draft_and_mismatch(self):
        out=self.package(); _,names=draft.validate(out,f'v{VERSION}','a'*40)
        def api(*a):
            if 'git/ref/tags' in a[-1]: return json_bytes({'object':{'type':'commit','sha':'a'*40}})
            if '/releases/tags/' in a[-1]: return json_bytes({'draft':True,'body':'Musha source: '+'a'*40,'assets':[{'name':n,'id':i} for i,n in enumerate(names)]})
            return (out/names[int(a[-1].rsplit('/',1)[-1])]).read_bytes()
        self.assertIn('retained',draft.attach(out,f'v{VERSION}','a'*40,True,api))
        def corrupt(*a):
            if '/assets/' in a[-1]: return b'corrupt'
            return api(*a)
        with self.assertRaises(ValueError): draft.attach(out,f'v{VERSION}','a'*40,True,corrupt)

    def test_no_image_work_in_files_only_build(self):
        builder=load('build-usb')
        with patch('importlib.util.module_from_spec') as module_factory:
            module_factory.side_effect=AssertionError('Image module must not load')
            # This tests the legacy build entry point's files-only path.
            builder.package(self.root/'files-only',self.efi,files_only=True)
        self.assertFalse((self.root/'files-only/musha-os.img').exists())

    def test_update_rejects_corrupt_payload_before_writing(self):
        z=self.zip(); v=self.volume()
        with zipfile.ZipFile(z) as src: entries={i.filename:src.read(i) for i in src.infolist()}
        entries['DIAGNOSE.TXT']=b'corrupt'
        bad=self.root/'bad.zip'
        with zipfile.ZipFile(bad,'w') as dst:
            for n,b in entries.items(): dst.writestr(n,b)
        with self.assertRaises(ValueError): update.update(bad,v,self.root/'backup',True)
        self.assertFalse((self.root/'backup').exists())
        self.assertEqual((v/'EFI/BOOT/BOOTX64.EFI').read_bytes(),b'old')

    def test_adjust_actual_io_never_visits_unused_capacity(self):
        image=usb.create(self.root/'source.img',self.efi); output=self.root/'large.img'; capacity=32*1024**3
        original=Path.open; operations=[]
        class Traced:
            def __init__(self,f): self.f=f
            def __enter__(self): return self
            def __exit__(self,*args): self.f.close()
            def __getattr__(self,n): return getattr(self.f,n)
            def write(self,b): operations.append(('write',self.f.tell(),len(b))); return self.f.write(b)
            def read(self,n=-1):
                if n<0: raise AssertionError('Unbounded read')
                operations.append(('read',self.f.tell(),n)); return self.f.read(n)
        def opened(path,*args,**kwargs):
            f=original(path,*args,**kwargs)
            return Traced(f) if path==output else f
        with patch.object(Path,'open',opened): deploy.adjust(image,output,capacity,apply=True)
        self.assertTrue(operations)
        for kind,offset,length in operations:
            self.assertTrue(offset+length<=64*1024**2 or offset>=capacity-33*512,(kind,offset,length))
        self.assertLess(sum(n for _,_,n in operations),140*1024**2)

    def test_cli_version_mismatch_before_build(self):
        result=subprocess.run([sys.executable,str(Path(__file__).with_name('prepare-release.py')),'--version','9.9.9','--allow-dirty','--output',str(self.root/'bad')],capture_output=True,text=True)
        self.assertNotEqual(result.returncode,0); self.assertIn('VERSION mismatch',result.stderr); self.assertFalse((self.root/'bad').exists())

    def test_workflow_privilege_and_success_boundary(self):
        text=(release.ROOT/'.github/workflows/release-candidate.yml').read_text()
        self.assertNotIn('pull_request_target',text); self.assertNotIn('release: published',text)
        self.assertEqual(text.count('contents: write'),1)
        self.assertIn("github.event_name == 'workflow_dispatch'",text)
        self.assertIn('if: always()',text)
        passed=text.split('- name: Preserve passed distribution only')[1].split('- name: Preserve test records')[0]
        self.assertNotIn('always()',passed)
        records=text.split('- name: Preserve test records')[1].split('  draft:')[0]
        self.assertNotIn('dist/',records)
        self.assertIn('cancel-in-progress: false',text)
        import re
        for line in text.splitlines():
            if 'uses:' in line: self.assertRegex(line,r'@[a-f0-9]{40} ')

    def test_verified_efi_reuse_requires_exact_normal_provenance(self):
        original=dict(self.prov,version=f'{VERSION}',status='candidate',target='x86_64-unknown-uefi',profile='release',payload={'EFI/BOOT/BOOTX64.EFI':{'sha256':digest(self.efi)}})
        release.validate_efi(self.efi,original,f'{VERSION}',f'v{VERSION}','a'*40,'')
        for field,value in [('version','999.0.0'),('tag','v999.0.0'),('commit','b'*40),('working_tree','dirty'),('features',['qemu-debug']),('status','preview'),('profile','debug')]:
            with self.assertRaises(ValueError): release.validate_efi(self.efi,dict(original,**{field:value}),f'{VERSION}',f'v{VERSION}','a'*40,'')
        self.efi.write_bytes(b'changed')
        with self.assertRaises(ValueError): release.validate_efi(self.efi,original,f'{VERSION}',f'v{VERSION}','a'*40,'')

if __name__=='__main__': unittest.main()
