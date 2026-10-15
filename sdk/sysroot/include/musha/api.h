/* SPDX-License-Identifier: Apache-2.0 */
/* Experimental r1/r2 guest syscall boundary; public ABI compatibility is not fixed. */
#ifndef MUSHA_API_H
#define MUSHA_API_H
#include <musha/abi.h>
typedef int32_t musha_status;
typedef uint64_t musha_handle;
typedef struct { int64_t status; uint64_t value; } musha_raw_result;
musha_status musha_query(uint64_t *features);
musha_status musha_log(const void *bytes, uint64_t length, uint64_t *written);
musha_status musha_time(uint64_t *milliseconds);
musha_status musha_wait(uint64_t timeout_ms, uint64_t flags, uint64_t *reason);
musha_status musha_input(musha_input_event *event);
_Noreturn void musha_exit(int32_t status);
#endif
