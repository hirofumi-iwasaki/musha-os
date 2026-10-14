/* SPDX-License-Identifier: Apache-2.0 */
#include "syscall.h"
musha_status musha_query(uint64_t *out){if(!out)return MUSHA_ERR_INVALID;return musha_decode(musha_raw(MUSHA_SYS_QUERY,0,0,0,0,0,0),out);}
musha_status musha_log(const void *p,uint64_t n,uint64_t *out){if((n&&!p)||!out||n>MUSHA_MAX_LOG_BYTES)return MUSHA_ERR_INVALID;return musha_decode(musha_raw(MUSHA_SYS_LOG,(uint64_t)p,n,0,0,0,0),out);}
_Noreturn void musha_exit(int32_t status){musha_raw(MUSHA_SYS_EXIT,(uint64_t)(int64_t)status,0,0,0,0,0);__builtin_trap();}
