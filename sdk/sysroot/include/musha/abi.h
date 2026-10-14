/* Generated experimental definitions; no public ABI guarantee. */
#ifndef MUSHA_ABI_H
#define MUSHA_ABI_H
#include <stdint.h>
#include <stddef.h>
#define MUSHA_ABI_MAJOR (0)
#define MUSHA_ABI_MINOR (1)
#define MUSHA_CANDIDATE_REVISION (1)
#define MUSHA_CPU_PROFILE (1)
#define MUSHA_START_SIZE (64)
#define MUSHA_SYS_QUERY (256)
#define MUSHA_SYS_LOG (257)
#define MUSHA_SYS_EXIT (258)
#define MUSHA_OK (0)
#define MUSHA_ERR_UNKNOWN_CALL (-1)
#define MUSHA_ERR_UNSUPPORTED (-2)
#define MUSHA_ERR_INVALID (-3)
#define MUSHA_ERR_DENIED (-4)
#define MUSHA_ERR_LIMIT (-5)
#define MUSHA_ERR_AGAIN (-6)
#define MUSHA_ERR_TIMEOUT (-7)
#define MUSHA_ERR_CANCELED (-8)
#define MUSHA_ERR_DISCONNECTED (-9)
#define MUSHA_ERR_IO (-10)
#define MUSHA_ERR_NO_MEMORY (-11)
#define MUSHA_ERR_NOT_FOUND (-12)
#define MUSHA_NOTE_TYPE (1297437512)
#define MUSHA_NOTE_SIZE (64)
#define MUSHA_FEATURE_BASE (1)
#define MUSHA_PERMISSION_LOG (1)
#define MUSHA_MAX_LOG_BYTES (4096)
#define MUSHA_IMAGE_BASE (4194304)
#define MUSHA_IMAGE_LIMIT (16777216)
#define MUSHA_ARENA_BASE (268435456)
#define MUSHA_ARENA_LIMIT (67108864)
#define MUSHA_STACK_TOP (1879048192)
#define MUSHA_STACK_LIMIT (1048576)
#define MUSHA_START_BASE (536870912)
#define MUSHA_MAX_SEGMENTS (32)
typedef struct musha_start_info_v1 {
    uint32_t struct_size;
    uint16_t abi_major;
    uint16_t abi_minor;
    uint32_t flags;
    uint32_t reserved0;
    uint64_t arena_base;
    uint64_t arena_bytes;
    uint64_t stack_base;
    uint64_t stack_bytes;
    uint64_t boot_report_handle;
    uint64_t reserved1;
} musha_start_info_v1;
_Static_assert(sizeof(musha_start_info_v1)==64,"size");
_Static_assert(_Alignof(musha_start_info_v1)==8,"alignment");
_Static_assert(offsetof(musha_start_info_v1,struct_size)==0,"struct_size");
_Static_assert(offsetof(musha_start_info_v1,abi_major)==4,"abi_major");
_Static_assert(offsetof(musha_start_info_v1,abi_minor)==6,"abi_minor");
_Static_assert(offsetof(musha_start_info_v1,flags)==8,"flags");
_Static_assert(offsetof(musha_start_info_v1,reserved0)==12,"reserved0");
_Static_assert(offsetof(musha_start_info_v1,arena_base)==16,"arena_base");
_Static_assert(offsetof(musha_start_info_v1,arena_bytes)==24,"arena_bytes");
_Static_assert(offsetof(musha_start_info_v1,stack_base)==32,"stack_base");
_Static_assert(offsetof(musha_start_info_v1,stack_bytes)==40,"stack_bytes");
_Static_assert(offsetof(musha_start_info_v1,boot_report_handle)==48,"boot_report_handle");
_Static_assert(offsetof(musha_start_info_v1,reserved1)==56,"reserved1");
typedef struct musha_note_v1 {
    uint32_t struct_size;
    uint32_t candidate_revision;
    uint16_t abi_major;
    uint16_t abi_minor;
    uint32_t cpu_profile;
    uint64_t required_features;
    uint64_t requested_permissions;
    uint64_t arena_bytes;
    uint64_t stack_bytes;
    uint64_t flags;
    uint64_t reserved;
} musha_note_v1;
_Static_assert(sizeof(musha_note_v1)==64,"size");
_Static_assert(_Alignof(musha_note_v1)==8,"alignment");
_Static_assert(offsetof(musha_note_v1,struct_size)==0,"struct_size");
_Static_assert(offsetof(musha_note_v1,candidate_revision)==4,"candidate_revision");
_Static_assert(offsetof(musha_note_v1,abi_major)==8,"abi_major");
_Static_assert(offsetof(musha_note_v1,abi_minor)==10,"abi_minor");
_Static_assert(offsetof(musha_note_v1,cpu_profile)==12,"cpu_profile");
_Static_assert(offsetof(musha_note_v1,required_features)==16,"required_features");
_Static_assert(offsetof(musha_note_v1,requested_permissions)==24,"requested_permissions");
_Static_assert(offsetof(musha_note_v1,arena_bytes)==32,"arena_bytes");
_Static_assert(offsetof(musha_note_v1,stack_bytes)==40,"stack_bytes");
_Static_assert(offsetof(musha_note_v1,flags)==48,"flags");
_Static_assert(offsetof(musha_note_v1,reserved)==56,"reserved");
#endif
