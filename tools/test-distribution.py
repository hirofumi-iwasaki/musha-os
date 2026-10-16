#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""Drive contract, stateless migration, unsafe inputs and fixture packaging."""
import gzip,hashlib,importlib.util,io,json,struct,subprocess,sys,tempfile,unittest,zipfile,zlib
from pathlib import Path
from unittest.mock import patch
from distribution import *

def load(name):
    spec=importlib.util.spec_from_file_location(name,Path(__file__).with_name(name+'.py'));m=importlib.util.module_from_spec(spec);spec.loader.exec_module(m);return m
usb=load('make-usb-image');deploy=load('deploy-image');update=load('update-usb');release=load('prepare-release');draft=load('draft-release');comparison=load('compare-candidates')
VERSION=(ROOT/'VERSION').read_text().strip()

class DistributionTests(unittest.TestCase):
    def setUp(self):
        self.tmp=tempfile.TemporaryDirectory();self.addCleanup(self.tmp.cleanup);self.root=Path(self.tmp.name).resolve()
        self.efi=self.root/'test.efi';self.efi.write_bytes(load('test-efi-size').fixture())
        self.prov={'commit':'a'*40,'features':[],'working_tree':'','tag':f'v{VERSION}'}
    def zip(self,payload=None,previous=None,name='update.zip'):
        path=self.root/name;make_zip(path,payload or drive_payload(self.efi,VERSION),VERSION,self.prov,previous);return path
    def volume(self,efi=None):
        path=self.root/'volume';(path/'EFI/BOOT').mkdir(parents=True);(path/'EFI/BOOT/BOOTX64.EFI').write_bytes(efi or self.efi.read_bytes());return path
    def test_clean_layout_and_metadata_extraction(self):
        archive=self.zip();m,files=read_zip(archive);volume=self.root/'extract'
        with zipfile.ZipFile(archive) as z:z.extractall(volume);self.assertLessEqual(len(z.comment),65535)
        self.assertEqual({p.name for p in volume.iterdir()},{'EFI','DOCS','SETTINGS','README.TXT'})
        self.assertEqual(list((volume/'SETTINGS').iterdir()),[]);self.assertFalse((volume/'update-manifest.json').exists())
        self.assertTrue(all('DOCS/'+n in files for n in USER_DOCS));self.assertEqual(m['layout_version'],2)
    def test_stateless_settings_preservation_idempotence(self):
        z=self.zip();v=self.volume();(v/'SETTINGS').mkdir();(v/'SETTINGS/SECRET.BIN').write_bytes(b'private');stamp=(v/'SETTINGS/SECRET.BIN').stat().st_mtime_ns
        dry=update.update(z,v,self.root/'backup');self.assertFalse((self.root/'backup').exists())
        result=update.update(z,v,self.root/'backup',True);self.assertTrue(result['applied']);self.assertNotIn('SETTINGS/SECRET.BIN',result['verified-files'])
        self.assertEqual((v/'SETTINGS/SECRET.BIN').stat().st_mtime_ns,stamp)
        self.assertEqual(update.update(z,v,self.root/'unused',True)['changed'],[]);self.assertFalse((self.root/'unused').exists())
        self.assertFalse(any((v/n).exists() for n in ('musha-install.json','update-manifest.json')))
    def test_unknown_and_modified_files_rejected_before_writes(self):
        z=self.zip();v=self.volume(b'unknown')
        with self.assertRaisesRegex(ValueError,'User-modified'):update.update(z,v,self.root/'backup',True)
        self.assertFalse((self.root/'backup').exists());self.assertEqual((v/'EFI/BOOT/BOOTX64.EFI').read_bytes(),b'unknown')
    def legacy(self):
        payload={'EFI/BOOT/BOOTX64.EFI':b'old EFI','DIAGNOSE.TXT':b'Hello Musha-OS!\n','LICENSE':b'old license'}
        m={'schema':2,'layout_version':1,'version':'0.3.0','commit':'b'*40,'tag':'v0.3.0','retired_files':['MUSHA.TXT'],
           'files':{n:{'bytes':len(b),'sha256':hashlib.sha256(b).hexdigest()} for n,b in payload.items()}}
        path=self.root/'legacy.zip'
        with zipfile.ZipFile(path,'x') as z:
            for n,b in dict(payload,**{'update-manifest.json':json_bytes(m)}).items():z.writestr(n,b)
        return path,m,payload
    def test_previous_package_migration_and_changed_obsolete_refusal(self):
        prior,old,payload=self.legacy();catalog=previous_catalog([prior]);z=self.zip(previous=catalog);v=self.volume(b'old EFI')
        for n,b in payload.items():d=v/n;d.parent.mkdir(parents=True,exist_ok=True);d.write_bytes(b)
        (v/'update-manifest.json').write_bytes(json_bytes(old));(v/'musha-install.json').write_bytes(json_bytes({'layout_version':1,'version':'0.3.0','commit':'b'*40}))
        (v/'AUTORUN.ELF').write_bytes(b'user program');(v/'LICENSE').write_bytes(b'user change')
        with self.assertRaisesRegex(ValueError,'User-modified'):update.update(z,v,self.root/'bad',True)
        self.assertFalse((self.root/'bad').exists());(v/'LICENSE').write_bytes(b'old license')
        result=update.update(z,v,self.root/'backup',True);self.assertTrue(result['applied']);self.assertEqual((v/'AUTORUN.ELF').read_bytes(),b'user program')
        self.assertTrue(all(not (v/n).exists() for n in ('LICENSE','DIAGNOSE.TXT','update-manifest.json','musha-install.json')))
        self.assertEqual((self.root/'backup/LICENSE').read_bytes(),b'old license')
    def test_failure_backup_and_temporary_cleanup(self):
        z=self.zip(previous={'EFI/BOOT/BOOTX64.EFI':[hashlib.sha256(b'old').hexdigest()]});v=self.volume(b'old')
        with patch('os.replace',side_effect=OSError('injected')):
            with self.assertRaises(OSError):update.update(z,v,self.root/'backup',True)
        self.assertEqual((v/'EFI/BOOT/BOOTX64.EFI').read_bytes(),b'old');self.assertEqual(list(v.rglob('.musha-update-*')),[])
        self.assertIn('error',json.loads((self.root/'backup/update-result.json').read_text()))
    def rewrite(self,name,entries,comment):
        path=self.root/name
        with zipfile.ZipFile(path,'w') as z:
            z.comment=comment
            for n,b in entries:z.writestr(n,b)
        return path
    def test_untrusted_zip_paths_duplicates_symlinks_and_comment(self):
        good=self.zip()
        with zipfile.ZipFile(good) as z:entries=[(i,z.read(i)) for i in z.infolist()];comment=z.comment
        for i,name in enumerate(('../escape','/absolute','a\\b','EFI/../bad','a:b','NUL.txt','SETTINGS/CONFIG.TXT','README.TXT','readme.txt','DOCS')):
            z=self.rewrite(f'bad{i}.zip',entries+[(name,b'x')],comment)
            with self.assertRaises((ValueError,KeyError)):read_zip(z)
        link=zipfile.ZipInfo('DOCS/link');link.external_attr=0o120777<<16
        with self.assertRaises(ValueError):read_zip(self.rewrite('link.zip',entries+[(link,b'/tmp')],comment))
        for i,raw in enumerate((b'{}',COMMENT_PREFIX+b'{"schema":3,"schema":3}',COMMENT_PREFIX+json_bytes(dict(json.loads(comment[len(COMMENT_PREFIX):]),extra=1)))):
            with self.assertRaises((ValueError,KeyError)):read_zip(self.rewrite(f'comment{i}.zip',entries,raw))
    def test_metadata_size_and_invalid_settings(self):
        many={f'DOCS/file{i}.txt':b'x' for i in range(500)};many.update(drive_payload(self.efi,VERSION))
        with self.assertRaisesRegex(ValueError,'comment limit'):self.zip(many)
        self.assertFalse((self.root/'update.zip').exists())
        z=self.zip(name='valid.zip');v=self.volume();(v/'SETTINGS').symlink_to(self.root/'outside',target_is_directory=True)
        with self.assertRaises(ValueError):update.update(z,v,self.root/'backup',True)
        self.assertFalse((self.root/'backup').exists())
    def test_destination_alias_and_symlink(self):
        z=self.zip();v=self.volume();(v/'readme.txt').write_bytes(b'x')
        with self.assertRaises(ValueError):update.update(z,v,self.root/'backup',True)
        (v/'readme.txt').unlink();(v/'README.TXT').symlink_to(self.root/'outside')
        with self.assertRaises(ValueError):update.update(z,v,self.root/'backup',True)
    def test_reproducibility_assets_and_rust_release_gate(self):
        a=release.stage(self.root/'one',self.efi,self.prov);b=release.stage(self.root/'two',self.efi,self.prov)
        result=comparison.compare(a,b,'same-source');self.assertTrue(result['passed']);self.assertFalse(result['sdk_runtime_complete'])
        self.assertEqual(set(json.loads((a/'build-manifest.json').read_text())['assets']),{f'musha-os-{VERSION}{s}' for s in ('-x64.zip','-x64-qemu.img.gz','-c-sdk.zip','-rust-sdk.zip')})
        with self.assertRaisesRegex(ValueError,'SDK implementation'):draft.validate(a,'v'+VERSION,'a'*40)
        draft.validate(a,'v'+VERSION,'a'*40,require_complete=False)
        with self.assertRaises(FileExistsError):release.stage(a,self.efi,self.prov)
    def test_readme_expected_fat_chain_and_fsck(self):
        data=usb_readme(VERSION);self.assertLess(len(data),4096);self.assertTrue(data.isascii());self.assertTrue(all(len(l)<=72 for l in data.splitlines()))
        image=usb.create(self.root/'test.img',self.efi)
        with image.open('rb') as f:info=deploy.inspect(f,image.stat().st_size)
        self.assertEqual(info['efi'],self.efi.read_bytes())
        if Path('/sbin/fsck_msdos').is_file():
            part=self.root/'partition.img'
            with image.open('rb') as f:f.seek(info['first']*512);part.write_bytes(f.read((info['last']-info['first']+1)*512))
            r=subprocess.run(['/sbin/fsck_msdos','-n',str(part)],capture_output=True,text=True)
            self.assertEqual(r.returncode,0,r.stdout+r.stderr)

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
    def test_parent_directory_alias_symlink_and_race_are_rejected(self):
        z=self.zip()
        for i,mode in enumerate(('docs-symlink','old-symlink','docs-alias','backup-symlink')):
            v=self.root/f'volume{i}';(v/'EFI/BOOT').mkdir(parents=True);(v/'EFI/BOOT/BOOTX64.EFI').write_bytes(self.efi.read_bytes())
            outside=self.root/f'outside{i}';outside.mkdir();(outside/'KEEP').write_bytes(b'keep')
            backup=self.root/f'backup{i}'
            if mode=='docs-symlink':(v/'DOCS').symlink_to(outside,target_is_directory=True)
            if mode=='old-symlink':(v/'third_party').symlink_to(outside,target_is_directory=True)
            if mode=='docs-alias':(v/'docs').mkdir()
            if mode=='backup-symlink':backup.symlink_to(outside,target_is_directory=True)
            with self.assertRaises(ValueError):update.update(z,v,backup,True)
            self.assertEqual((outside/'KEEP').read_bytes(),b'keep');self.assertEqual(list(outside.iterdir()),[outside/'KEEP'])
        v=self.volume();calls=iter([{'id':1},{'id':2}])
        original_stat=Path.stat
        class DifferentDevice:
            def __init__(self,wrapped):self.wrapped=wrapped;self.st_dev=-1
            def __getattr__(self,name):return getattr(self.wrapped,name)
        def stat(path,*args,**kwargs):
            value=original_stat(path,*args,**kwargs)
            return DifferentDevice(value) if path==v else value
        with patch.object(Path,'stat',stat):
            with self.assertRaisesRegex(ValueError,'identity changed'):update.update(z,v,self.root/'racebackup',True,lambda _:next(calls))
        self.assertFalse((v/'DOCS').exists())
    def test_insufficient_space_and_source_metadata_corruption(self):
        z=self.zip();v=self.volume()
        with patch('shutil.disk_usage',return_value=type('Usage',(),{'free':0})()):
            with self.assertRaisesRegex(ValueError,'space'):update.update(z,v,self.root/'backup',True)
        self.assertFalse((self.root/'backup').exists())
        with zipfile.ZipFile(z) as a:entries=[(i,a.read(i)) for i in a.infolist()];m=json.loads(a.comment[len(COMMENT_PREFIX):])
        m['files']['README.TXT']['sha256']='0'*64
        corrupted=self.rewrite('corrupt.zip',entries,COMMENT_PREFIX+json_bytes(m))
        with self.assertRaises(ValueError):update.update(corrupted,v,self.root/'backup',True)
        self.assertFalse((self.root/'backup').exists())
    def test_image_embedded_document_change_never_passes_runtime_equivalence(self):
        a=release.stage(self.root/'left',self.efi,self.prov)
        payload=drive_payload(self.efi,VERSION);payload['DOCS/file-viewer.md']+=b'changed docs'
        with patch('distribution.drive_payload',return_value=payload):b=release.stage(self.root/'right',self.efi,self.prov)
        result=comparison.compare(a,b,'same-source')
        self.assertTrue(result['efi_identical']);self.assertFalse(result['runtime_identical']);self.assertFalse(result['passed'])
        self.assertFalse(result['runtime_members']['QEMU image']['identical'])
    def test_scaffold_draft_gate_prevents_all_remote_calls(self):
        out=release.stage(self.root/'draft',self.efi,self.prov)
        with self.assertRaisesRegex(ValueError,'SDK implementation'):draft.attach(out,'v'+VERSION,'a'*40,True,lambda *args:self.fail('Remote access before SDK gate'))

    def test_draft_boundary_published_immutable_remote_tag_and_api_reuse(self):
        # Isolate attachment API rules from the separately tested scaffold gate.
        out=release.stage(self.root/'attach-fixture',self.efi,self.prov)
        validated=draft.validate(out,'v'+VERSION,'a'*40,require_complete=False)
        names=validated[1];calls=[]
        def tag(*args):return json_bytes({'object':{'type':'commit','sha':'a'*40}})
        def missing(*args):
            calls.append(args)
            if 'git/ref/tags' in args[-1]:return tag(*args)
            if '/releases/tags/' in args[-1]:raise subprocess.CalledProcessError(1,args,stderr=b'Not Found (HTTP 404)')
            return b''
        def published(*args):
            if 'git/ref/tags' in args[-1]:return tag(*args)
            return json_bytes({'draft':False})
        def identical(*args):
            if 'git/ref/tags' in args[-1]:return tag(*args)
            if '/releases/tags/' in args[-1]:return json_bytes({'draft':True,'body':'Musha source: '+'a'*40,'assets':[{'name':n,'id':i} for i,n in enumerate(names)]})
            return (out/names[int(args[-1].rsplit('/',1)[-1])]).read_bytes()
        with patch.object(draft,'validate',return_value=validated):
            self.assertIn('Dry-run',draft.attach(out,'v'+VERSION,'a'*40,api=missing));self.assertFalse(any(a[0]=='release' for a in calls))
            draft.attach(out,'v'+VERSION,'a'*40,True,missing)
            create=next(a for a in calls if a[0]=='release');self.assertIn('--draft',create);self.assertIn('--verify-tag',create);self.assertNotIn('--clobber',create)
            with self.assertRaisesRegex(ValueError,'immutable'):draft.attach(out,'v'+VERSION,'a'*40,True,published)
            self.assertIn('retained',draft.attach(out,'v'+VERSION,'a'*40,True,identical))
            with self.assertRaisesRegex(ValueError,'Remote tag mismatch'):draft.attach(out,'v'+VERSION,'a'*40,True,lambda *args:json_bytes({'object':{'type':'commit','sha':'b'*40}}))
            def corrupt(*args):return b'corrupt' if '/assets/' in args[-1] else identical(*args)
            with self.assertRaisesRegex(ValueError,'differs'):draft.attach(out,'v'+VERSION,'a'*40,True,corrupt)
    def test_dirty_draft_and_release_sdk_gates(self):
        out=release.stage(self.root/'dirty-fixture',self.efi,dict(self.prov,working_tree='dirty'))
        with self.assertRaisesRegex(ValueError,'provenance'):draft.validate(out,'v'+VERSION,'a'*40)
        with self.assertRaisesRegex(ValueError,'Release SDK'):release.stage(self.root/'release-gate',self.efi,self.prov,release=True)
        self.assertFalse((self.root/'release-gate').exists())

if __name__=='__main__':unittest.main()
