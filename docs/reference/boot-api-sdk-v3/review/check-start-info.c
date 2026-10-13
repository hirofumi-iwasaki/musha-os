/* Document-only layout test; not an SDK or syscall implementation. */
#include <stdint.h>
#include <stddef.h>

typedef struct musha_start_info_v1 {
    uint32_t struct_size;        /* offset  0 */
    uint16_t abi_major;          /* offset  4 */
    uint16_t abi_minor;          /* offset  6 */
    uint32_t flags;              /* offset  8 */
    uint32_t reserved0;          /* offset 12: zero */
    uint64_t arena_base;         /* offset 16: user virtual address */
    uint64_t arena_bytes;        /* offset 24 */
    uint64_t stack_base;         /* offset 32: mapped usable range */
    uint64_t stack_bytes;        /* offset 40: guard pages excluded */
    uint64_t boot_report_handle; /* offset 48: zero if unavailable */
    uint64_t reserved1;          /* offset 56: zero */
} musha_start_info_v1;

_Static_assert(sizeof(musha_start_info_v1) == 64, "start info size");
_Static_assert(_Alignof(musha_start_info_v1) == 8, "start info alignment");
_Static_assert(offsetof(musha_start_info_v1, arena_base) == 16, "arena offset");
_Static_assert(offsetof(musha_start_info_v1, boot_report_handle) == 48,
               "boot report offset");

_Static_assert(sizeof(int) == 4, "int width");
_Static_assert(sizeof(long) == 8, "long width");
_Static_assert(sizeof(void *) == 8, "pointer width");
_Static_assert(offsetof(musha_start_info_v1, struct_size) == 0, "struct_size");
_Static_assert(offsetof(musha_start_info_v1, abi_major) == 4, "abi_major");
_Static_assert(offsetof(musha_start_info_v1, abi_minor) == 6, "abi_minor");
_Static_assert(offsetof(musha_start_info_v1, flags) == 8, "flags");
_Static_assert(offsetof(musha_start_info_v1, reserved0) == 12, "reserved0");
_Static_assert(offsetof(musha_start_info_v1, arena_base) == 16, "arena_base");
_Static_assert(offsetof(musha_start_info_v1, arena_bytes) == 24, "arena_bytes");
_Static_assert(offsetof(musha_start_info_v1, stack_base) == 32, "stack_base");
_Static_assert(offsetof(musha_start_info_v1, stack_bytes) == 40, "stack_bytes");
_Static_assert(offsetof(musha_start_info_v1, boot_report_handle) == 48, "boot_report_handle");
_Static_assert(offsetof(musha_start_info_v1, reserved1) == 56, "reserved1");
unsigned musha_document_layout_size(void) { return (unsigned)sizeof(musha_start_info_v1); }
