#!/usr/bin/env python3
# Copyright 2026 Hirofumi Iwasaki
# SPDX-License-Identifier: Apache-2.0
"""Create a new sparse FAT32 test image; never operates on a device."""
import argparse,pathlib,struct
import importlib.util
PAYLOAD=b'Hello Musha-OS!\n'+b'FAT32 file read over USB.\n'*45
def create(path,layout='mbr',corrupt=False):
 size=512;clusters=65536;reserved=32;fatsecs=(clusters+2)*4//size+1
 start=2048 if layout in ['mbr','gpt'] else 0
 data=reserved+2*fatsecs;total=data+clusters
 def sector(file,lba,contents):file.seek(lba*size);file.write(contents)
 with pathlib.Path(path).open('xb') as file:
  blocks=start+total+(33 if layout=='gpt' else 0)
  file.truncate(blocks*size)
  if layout=='gpt':
   spec=importlib.util.spec_from_file_location('usb_image',pathlib.Path(__file__).with_name('make-usb-image.py'))
   module=importlib.util.module_from_spec(spec);spec.loader.exec_module(module)
   mbr,primary,backup,array=module.gpt_tables(blocks,start,start+total-1,'fixture')
   sector(file,0,mbr);sector(file,1,primary);sector(file,2,array)
   sector(file,blocks-33,array);sector(file,blocks-1,backup)
  elif start:
   mbr=bytearray(size);mbr[510:512]=b'\x55\xaa';mbr[450]=0x0c
   struct.pack_into('<II',mbr,454,start,total);sector(file,0,mbr)
  boot=bytearray(size);boot[:3]=b'\xeb\x58\x90';boot[3:11]=b'MUSHA-OS'
  struct.pack_into('<H',boot,11,size);boot[13]=1;struct.pack_into('<H',boot,14,reserved)
  boot[16]=2;boot[21]=0xf8;struct.pack_into('<II',boot,28,start,total)
  struct.pack_into('<I',boot,36,fatsecs);struct.pack_into('<I',boot,44,2)
  struct.pack_into('<HH',boot,48,1,6);boot[64]=128;boot[66]=0x29
  struct.pack_into('<I',boot,67,0x4d555348);boot[71:82]=b'MUSHA TEST ';boot[82:90]=b'FAT32   '
  boot[510:512]=b'\x55\xaa';sector(file,start,boot);sector(file,start+6,boot)
  info=bytearray(size);struct.pack_into('<I',info,0,0x41615252)
  struct.pack_into('<III',info,484,0x61417272,0xffffffff,0xffffffff)
  struct.pack_into('<I',info,508,0xaa550000);sector(file,start+1,info);sector(file,start+7,info)
  fat=bytearray(size)
  for n,next_cluster in [(0,0x0ffffff8),(1,0xffffffff),(2,4),(4,0x0fffffff),
                          (3,3 if corrupt else 5),(5,7),(7,0x0fffffff)]:
   struct.pack_into('<I',fat,n*4,next_cluster)
  for offset in [reserved,reserved+fatsecs]:sector(file,start+offset,fat)
  root=bytearray(size)
  for i in range(0,size,32):root[i]=0xe5
  sector(file,start+data,root)
  root=bytearray(size);root[:11]=b'MUSHA   TXT';root[11]=32
  struct.pack_into('<H',root,26,3);struct.pack_into('<I',root,28,len(PAYLOAD))
  sector(file,start+data+2,root)
  for index,cluster in enumerate([3,5,7]):
   chunk=PAYLOAD[index*size:(index+1)*size]
   sector(file,start+data+cluster-2,chunk.ljust(size,b'\0'))
 return len(PAYLOAD)
if __name__=='__main__':
 parser=argparse.ArgumentParser()
 parser.add_argument('output');parser.add_argument('--layout',choices=['mbr','superfloppy','gpt'],default='mbr')
 parser.add_argument('--corrupt-chain',action='store_true');args=parser.parse_args()
 print('File bytes:',create(args.output,args.layout,args.corrupt_chain))
