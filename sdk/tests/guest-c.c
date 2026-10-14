/* SPDX-License-Identifier: Apache-2.0 */
#include <musha/api.h>
#include "syscall.h"
#ifndef C2_CASE
#define C2_CASE 0
#endif
static volatile unsigned char bss[16384];
static volatile double number = 1.5;
__attribute__((noinline)) static double add(double a,double b) { return a+b; }
int musha_main(const musha_start_info_v1 *s) {
 uint64_t features=0, written=0; unsigned short cs;
 __asm__ volatile("mov %%cs,%0":"=r"(cs));
 if ((cs&3)!=3 || !s || s->struct_size!=64 || s->abi_major!=MUSHA_ABI_MAJOR || s->abi_minor!=MUSHA_ABI_MINOR) return 1;
 if(s->flags || s->reserved0 || s->reserved1 || s->boot_report_handle || s->arena_base!=MUSHA_ARENA_BASE || s->stack_base+s->stack_bytes!=MUSHA_STACK_TOP) return 2;
 for(unsigned i=0;i<sizeof(bss);i++) if(bss[i]) return 3;
 volatile unsigned char *arena=(void *)s->arena_base;
 for(uint64_t i=0;i<s->arena_bytes;i++) if(arena[i]) return 4;
 arena[0]=0x55; arena[s->arena_bytes-1]=0xaa; bss[sizeof(bss)-1]=0x5a;
 if(musha_query(&features) || features!=MUSHA_FEATURE_BASE || add(number,number)!=3.0) return 5;
 if(C2_CASE==1) { *(volatile uint32_t *)s=0; return 6; }
 if(C2_CASE==2) { volatile unsigned char v=*((volatile unsigned char *)s->stack_base-1); return v+7; }
 if(C2_CASE==4) return 42;
 if(C2_CASE==5) { __asm__ volatile("ud2"); return 15; }
 if(C2_CASE==6) { __asm__ volatile("cli"); return 16; }
 if(C2_CASE==7) { __asm__ volatile("xor %%ecx,%%ecx; xor %%edx,%%edx; mov $1,%%eax; div %%ecx" ::: "rax","rcx","rdx","cc"); return 17; }
 if(C2_CASE==3) return musha_log("denied",6,&written)!=MUSHA_ERR_DENIED;
 musha_raw_result r=musha_raw(UINT64_MAX,0,0,0,0,0,0);
 if(r.status!=MUSHA_ERR_UNKNOWN_CALL || r.value) return 8;
 r=musha_raw(MUSHA_SYS_LOG,UINT64_MAX,2,0,0,0,0);
 if(r.status!=MUSHA_ERR_INVALID || r.value) return 9;
 r=musha_raw(MUSHA_SYS_LOG,s->arena_base,4097,0,0,0,0);
 if(r.status!=MUSHA_ERR_LIMIT || r.value) return 10;
 r=musha_raw(MUSHA_SYS_LOG,s->arena_base+s->arena_bytes-1,2,0,0,0,0);
 if(r.status!=MUSHA_ERR_INVALID || r.value) return 11;
 char bad_utf8[]={ (char)0xff };
 if(musha_log(bad_utf8,1,&written)!=MUSHA_ERR_INVALID) return 12;
 /* A log spans a page boundary inside the separately mapped user arena. */
 for(unsigned i=0;i<8;i++) arena[4093+i]="C2 COPY\n"[i];
 if(musha_log((void *)(s->arena_base+4093),8,&written) || written!=8) return 13;
 if(musha_log("C2 ABI BSS ARENA OK\n",20,&written) || written!=20) return 14;
 return 0;
}
