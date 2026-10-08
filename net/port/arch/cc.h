// Copyright 2026 Hirofumi Iwasaki
// SPDX-License-Identifier: Apache-2.0
#ifndef MUSHA_CC_H
#define MUSHA_CC_H
#include <stddef.h>
#define BYTE_ORDER LITTLE_ENDIAN
#define LWIP_NO_STDINT_H 1
#define LWIP_NO_INTTYPES_H 1
#define LWIP_NO_CTYPE_H 1
#define LWIP_NO_LIMITS_H 1
#define INT_MAX 2147483647
#define LWIP_HAVE_INT64 1
typedef unsigned char u8_t;
typedef signed char s8_t;
typedef unsigned short u16_t;
typedef signed short s16_t;
typedef unsigned int u32_t;
typedef signed int s32_t;
typedef unsigned long long u64_t;
typedef signed long long s64_t;
typedef __UINTPTR_TYPE__ mem_ptr_t;
#define X8_F "02x"
#define U16_F "u"
#define S16_F "d"
#define X16_F "x"
#define U32_F "u"
#define S32_F "d"
#define X32_F "x"
#define SZT_F "llu"
#define LWIP_PLATFORM_DIAG(x) do {} while (0)
_Noreturn void musha_lwip_assert(void);
#define LWIP_PLATFORM_ASSERT(x) musha_lwip_assert()
#endif
