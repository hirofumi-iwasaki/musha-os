/* SPDX-License-Identifier: Apache-2.0 */
/* External-file screen fixtures; never linked into the kernel. */
#include <musha/api.h>
#ifndef EXTERNAL_CASE
#define EXTERNAL_CASE 0
#endif
int musha_main(const musha_start_info_v1 *info) {
    uint64_t features = 0, written = 0;
    (void)info;
    if (musha_query(&features)) return 1;
#if EXTERNAL_CASE == 0
    return 42;
#elif EXTERNAL_CASE == 1
    *(volatile unsigned char *)(uintptr_t)0x70000000 = 1;
#elif EXTERNAL_CASE == 2
    __asm__ volatile("ud2");
#elif EXTERNAL_CASE == 3
    static char text[4096];
    for (unsigned i = 0; i < sizeof text; ++i) text[i] = 'Z';
    if (musha_log(text, sizeof text, &written) || written != sizeof text) return 2;
#elif EXTERNAL_CASE == 4
    const char text[] = "ASCII\t\r\033\001\177 \xe6\x97\xa5\xe6\x9c\xac\nEND\n";
    if (musha_log(text, sizeof text - 1, &written) || written != sizeof text - 1) return 3;
#endif
#if defined(EXTERNAL_OBSERVE_LOG) && (EXTERNAL_CASE == 3 || EXTERNAL_CASE == 4)
    /* Normal builds have no debug transcript. Keep the actual LOG raster visible
       for a finite fixture-only interval before the expected viewer return. */
    for (volatile unsigned observe = 0; observe < 100000000U; ++observe) {}
#endif
    (void)written;
    return 0;
}
