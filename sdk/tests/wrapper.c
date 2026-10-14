/* SPDX-License-Identifier: Apache-2.0 */
#include "../src/syscall.h"
musha_raw_result six(uint64_t a,uint64_t b,uint64_t c,uint64_t d,uint64_t e,uint64_t f){return musha_raw(MUSHA_SYS_QUERY,a,b,c,d,e,f);}
