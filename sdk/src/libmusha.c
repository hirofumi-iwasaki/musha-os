/* SPDX-License-Identifier: Apache-2.0 */
#include "syscall.h"
musha_status musha_query(uint64_t *out){if(!out)return MUSHA_ERR_INVALID;return musha_decode(musha_raw(MUSHA_SYS_QUERY,0,0,0,0,0,0),out);}
musha_status musha_log(const void *p,uint64_t n,uint64_t *out){if((n&&!p)||!out||n>MUSHA_MAX_LOG_BYTES)return MUSHA_ERR_INVALID;return musha_decode(musha_raw(MUSHA_SYS_LOG,(uint64_t)p,n,0,0,0,0),out);}
_Noreturn void musha_exit(int32_t status){musha_raw(MUSHA_SYS_EXIT,(uint64_t)(int64_t)status,0,0,0,0,0);__builtin_trap();}

musha_status musha_time(uint64_t *out) {
    if (!out) return MUSHA_ERR_INVALID;
    return musha_decode(musha_raw(MUSHA_SYS_TIME,0,0,0,0,0,0),out);
}
musha_status musha_wait(uint64_t timeout,uint64_t flags,uint64_t *out) {
    if (!out || (flags & ~MUSHA_WAIT_INPUT)) return MUSHA_ERR_INVALID;
    return musha_decode(musha_raw(MUSHA_SYS_WAIT,timeout,flags,0,0,0,0),out);
}
musha_status musha_input(musha_input_event *event) {
    if (!event) return MUSHA_ERR_INVALID;
    uint64_t n=0;
    musha_status status=musha_decode(musha_raw(MUSHA_SYS_INPUT,(uint64_t)event,sizeof(*event),0,0,0,0),&n);
    if (status) return status;
    return n==MUSHA_INPUT_SIZE && event->struct_size==MUSHA_INPUT_SIZE && event->reserved==0 ? MUSHA_OK : MUSHA_ERR_INVALID;
}
