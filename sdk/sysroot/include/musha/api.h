/* SPDX-License-Identifier: Apache-2.0 */
/* Experimental compile-only boundary; current EFI has no syscall dispatcher. */
#ifndef MUSHA_API_H
#define MUSHA_API_H
#include <musha/abi.h>
typedef int32_t musha_status;
typedef uint64_t musha_handle;
typedef struct { int64_t status; uint64_t value; } musha_raw_result;
musha_status musha_query(uint64_t *features);
musha_status musha_log(const void *bytes, uint64_t length, uint64_t *written);
_Noreturn void musha_exit(int32_t status);
#endif
