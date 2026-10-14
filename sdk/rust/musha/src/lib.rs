// SPDX-License-Identifier: Apache-2.0
#![no_std]
pub use musha_sys::MushaStartInfoV1;
#[derive(Debug,PartialEq,Eq)]pub struct Error(pub i32);
/// Pure conversion only. OS-backed safe wrappers wait for the C-stage runtime.
pub fn decode(status:i64,value:u64)->Result<u64,Error>{
 if status>0 || status<i32::MIN as i64 || (status<0 && value!=0){return Err(Error(musha_sys::ERR_INVALID));}
 if status==0 {Ok(value)}else{Err(Error(status as i32))}
}
/// Validate a copied record. Does not prove mapping, ownership or lifetime.
pub fn validate_start(s:&MushaStartInfoV1)->Result<(),Error>{
 let range=|base:u64,len:u64|base>=4096 && base%4096==0 && len>0 && len%4096==0 && base.checked_add(len).is_some_and(|end|end<=0x8000_0000);
 if s.struct_size!=64 || s.abi_major!=0 || s.abi_minor!=1 || s.flags!=0 || s.reserved0!=0 || s.reserved1!=0 || !range(s.arena_base,s.arena_bytes) || !range(s.stack_base,s.stack_bytes) || s.arena_bytes>musha_sys::ARENA_LIMIT || s.stack_bytes>musha_sys::STACK_LIMIT || s.arena_base<s.stack_base+s.stack_bytes && s.stack_base<s.arena_base+s.arena_bytes {return Err(Error(musha_sys::ERR_INVALID));}
 Ok(())
}
#[cfg(test)]mod tests {use super::*;
#[test]fn raw_results(){assert_eq!(decode(0,91),Ok(91));assert_eq!(decode(-7,0),Err(Error(-7)));for (s,v) in [(1,0),(-1,42),(0xffff_ffff,0),(-2147483649,0)]{assert_eq!(decode(s,v),Err(Error(-3)));}}
#[test]fn records(){let mut s=MushaStartInfoV1{struct_size:64,abi_minor:1,arena_base:0x10000000,arena_bytes:4096,stack_base:0x70000000-4096,stack_bytes:4096,..Default::default()};assert!(validate_start(&s).is_ok());s.reserved1=1;assert!(validate_start(&s).is_err());s.reserved1=0;s.stack_base=s.arena_base;assert!(validate_start(&s).is_err());s.arena_base=u64::MAX;assert!(validate_start(&s).is_err());}
}
