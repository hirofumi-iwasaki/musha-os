#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
import importlib.util,json,unittest,copy,subprocess,gzip,hashlib,struct
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

    def replace_image_and_rehash(self, image):
        prefix=f'musha-os-{d.VERSION}'
        (self.right/(prefix+'-x64-qemu.img.gz')).write_bytes(gzip.compress(image,mtime=0))
        manifest_path=self.right/'build-manifest.json';manifest=json.loads(manifest_path.read_text())
        manifest['runtime_payload']['QEMU image']={'bytes':len(image),'sha256':hashlib.sha256(image).hexdigest()}
        asset=prefix+'-x64-qemu.img.gz'
        manifest['assets'][asset]={'bytes':(self.right/asset).stat().st_size,'sha256':d.digest(self.right/asset)}
        manifest_path.write_text(json.dumps(manifest))
        (self.right/'SHA256SUMS').write_text(''.join(f'{d.digest(f)}  {f.name}\n' for f in sorted(self.right.iterdir()) if f.name!='SHA256SUMS'))

    def test_candidate_image_managed_content_must_match_zip(self):
        original=d.drive_payload(self.efi,d.VERSION)
        for kind in ('readme','docs','missing-settings'):
            with self.subTest(kind=kind):
                image_path=self.root/(kind+'.img')
                if kind == 'missing-settings':
                    image=gzip.decompress((self.left/f'musha-os-{d.VERSION}-x64-qemu.img.gz').read_bytes())
                    image=bytearray(image)
                    boot=image[2048*512:2049*512]
                    start=2048+32+2*struct.unpack_from('<I',boot,36)[0]
                    offset=(start+struct.unpack_from('<I',boot,44)[0]-2)*512
                    entry=image.find(b'SETTINGS   ',offset,offset+512)
                    self.assertGreaterEqual(entry,offset)
                    image[entry:entry+11]=b'LOSTDIR    '
                    image=bytes(image)
                else:
                    altered=dict(original)
                    if kind == 'readme': altered['README.TXT']+=b'Changed image README\r\n'
                    else: altered['DOCS/file-viewer.md']+=b'\nChanged image guide\n'
                    d.usb.create(image_path,self.efi,payload=altered)
                    image=image_path.read_bytes()
                self.replace_image_and_rehash(image)
                with self.assertRaisesRegex(ValueError,'Image and drive managed files differ|Candidate root layout differs'):
                    d.comparison.inventory(self.right)

    def test_candidate_image_reader_rejects_extra_and_cyclic_directories(self):
        reader=d.load('candidate-image')
        original=gzip.decompress((self.left/f'musha-os-{d.VERSION}-x64-qemu.img.gz').read_bytes())
        boot=original[2048*512:2049*512]
        fat_sectors=struct.unpack_from('<I',boot,36)[0]
        fat_offset=(2048+32)*512
        data_sector=2048+32+2*fat_sectors
        def location(cluster):return (data_sector+cluster-2)*512
        def find(directory,short):
            return next(directory[i:i+32] for i in range(0,len(directory),32) if directory[i:i+11]==short)
        def cluster(entry):return (struct.unpack_from('<H',entry,20)[0]<<16)|struct.unpack_from('<H',entry,26)[0]
        root=cluster(find(original[location(2):location(2)+512],b'DOCS       '))
        settings=cluster(find(original[location(2):location(2)+512],b'SETTINGS   '))
        with self.subTest(kind='nonempty-settings'):
            image=bytearray(original);slot=location(settings)+64
            image[slot:slot+11]=b'SECRET  TXT';image[slot+11]=32
            with self.assertRaisesRegex(ValueError,'SETTINGS must initially be empty'):
                reader.managed_files(image)
        with self.subTest(kind='cyclic-directory'):
            image=bytearray(original);struct.pack_into('<I',image,fat_offset+root*4,root)
            with self.assertRaisesRegex(ValueError,'Invalid or shared FAT chain'):
                reader.managed_files(image)
        with self.subTest(kind='extra-empty-docs-directory'):
            image=bytearray(original)
            free=next(c for c in range(2,fat_sectors*512//4) if struct.unpack_from('<I',image,fat_offset+c*4)[0]==0)
            for offset in (fat_offset,fat_offset+fat_sectors*512):
                struct.pack_into('<I',image,offset+free*4,0x0fffffff)
            directory=location(root)
            while True:
                slot=next((directory+i for i in range(0,512,32) if image[directory+i]==0),None)
                if slot is not None:break
                following=struct.unpack_from('<I',image,fat_offset+(2+(directory//512-data_sector))*4)[0]&0x0fffffff
                self.assertLess(following,0x0ffffff8)
                directory=location(following)
            image[slot:slot+11]=b'EXTRA      ';image[slot+11]=16;struct.pack_into('<H',image,slot+26,free)
            for index,(name,target) in enumerate(((b'.          ',free),(b'..         ',root))):
                position=location(free)+index*32
                image[position:position+11]=name;image[position+11]=16;struct.pack_into('<H',image,position+26,target)
            with self.assertRaisesRegex(ValueError,'DOCS tree exceeds bound'):
                reader.managed_files(image)

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
