/* SPDX-License-Identifier: Apache-2.0 */
#include <musha/api.h>
static inline musha_raw_result musha_raw(uint64_t nr,uint64_t a,uint64_t b,uint64_t c,uint64_t d,uint64_t e,uint64_t f) {
 register uint64_t r10 __asm__("r10")=d;
 register uint64_t r8 __asm__("r8")=e;
 register uint64_t r9 __asm__("r9")=f;
 __asm__ volatile("syscall" : "+a"(nr), "+d"(c) : "D"(a),"S"(b),"r"(r10),"r"(r8),"r"(r9) : "rcx","r11","cc","memory");
 return (musha_raw_result){(int64_t)nr,c};
}
/* Reject malformed raw return values; do not silently truncate status. */
static inline musha_status musha_decode(musha_raw_result r,uint64_t *out) {
 if(r.status>0 || r.status<INT32_MIN || (r.status<0 && r.value!=0))return MUSHA_ERR_INVALID;
 if(r.status==0 && out)*out=r.value;
 return (int32_t)r.status;
}
