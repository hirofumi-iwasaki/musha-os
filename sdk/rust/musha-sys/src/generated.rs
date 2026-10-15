// Generated experimental definitions; no public ABI guarantee.
pub const ABI_MAJOR: u64 = 0;
pub const ABI_MINOR: u64 = 1;
pub const CANDIDATE_REVISION: u64 = 1;
pub const CPU_PROFILE: u64 = 1;
pub const START_SIZE: u64 = 64;
pub const SYS_QUERY: u64 = 256;
pub const SYS_LOG: u64 = 257;
pub const SYS_EXIT: u64 = 258;
pub const OK: u64 = 0;
pub const ERR_UNKNOWN_CALL: i32 = -1;
pub const ERR_UNSUPPORTED: i32 = -2;
pub const ERR_INVALID: i32 = -3;
pub const ERR_DENIED: i32 = -4;
pub const ERR_LIMIT: i32 = -5;
pub const ERR_AGAIN: i32 = -6;
pub const ERR_TIMEOUT: i32 = -7;
pub const ERR_CANCELED: i32 = -8;
pub const ERR_DISCONNECTED: i32 = -9;
pub const ERR_IO: i32 = -10;
pub const ERR_NO_MEMORY: i32 = -11;
pub const ERR_NOT_FOUND: i32 = -12;
pub const NOTE_TYPE: u64 = 1297437512;
pub const NOTE_SIZE: u64 = 64;
pub const FEATURE_BASE: u64 = 1;
pub const PERMISSION_LOG: u64 = 1;
pub const MAX_LOG_BYTES: u64 = 4096;
pub const IMAGE_BASE: u64 = 4194304;
pub const IMAGE_LIMIT: u64 = 16777216;
pub const ARENA_BASE: u64 = 268435456;
pub const ARENA_LIMIT: u64 = 67108864;
pub const STACK_TOP: u64 = 1879048192;
pub const STACK_LIMIT: u64 = 1048576;
pub const START_BASE: u64 = 536870912;
pub const MAX_SEGMENTS: u64 = 32;
pub const R2_REVISION: u64 = 2;
pub const R2_MINOR: u64 = 2;
pub const SYS_TIME: u64 = 259;
pub const SYS_WAIT: u64 = 260;
pub const SYS_INPUT: u64 = 261;
pub const FEATURE_INPUT: u64 = 2;
pub const PERMISSION_TIME: u64 = 2;
pub const PERMISSION_WAIT: u64 = 4;
pub const PERMISSION_INPUT: u64 = 8;
pub const INPUT_SIZE: u64 = 48;
pub const INPUT_KEY: u64 = 1;
pub const INPUT_MOUSE: u64 = 2;
pub const INPUT_GAMEPAD_BUTTON: u64 = 3;
pub const INPUT_GAMEPAD_AXIS: u64 = 4;
pub const INPUT_OVERFLOW: u64 = 5;
pub const WAIT_INPUT: u64 = 1;
pub const WAKE_TIMEOUT: u64 = 0;
pub const WAKE_INPUT: u64 = 1;
pub const MAX_WAIT_MS: u64 = 60000;
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct MushaStartInfoV1 {
    pub struct_size: u32,
    pub abi_major: u16,
    pub abi_minor: u16,
    pub flags: u32,
    pub reserved0: u32,
    pub arena_base: u64,
    pub arena_bytes: u64,
    pub stack_base: u64,
    pub stack_bytes: u64,
    pub boot_report_handle: u64,
    pub reserved1: u64,
}
const _: () = {
    assert!(core::mem::size_of::<MushaStartInfoV1>() == 64);
    assert!(core::mem::align_of::<MushaStartInfoV1>() == 8);
    assert!(core::mem::offset_of!(MushaStartInfoV1, struct_size) == 0);
    assert!(core::mem::offset_of!(MushaStartInfoV1, abi_major) == 4);
    assert!(core::mem::offset_of!(MushaStartInfoV1, abi_minor) == 6);
    assert!(core::mem::offset_of!(MushaStartInfoV1, flags) == 8);
    assert!(core::mem::offset_of!(MushaStartInfoV1, reserved0) == 12);
    assert!(core::mem::offset_of!(MushaStartInfoV1, arena_base) == 16);
    assert!(core::mem::offset_of!(MushaStartInfoV1, arena_bytes) == 24);
    assert!(core::mem::offset_of!(MushaStartInfoV1, stack_base) == 32);
    assert!(core::mem::offset_of!(MushaStartInfoV1, stack_bytes) == 40);
    assert!(core::mem::offset_of!(MushaStartInfoV1, boot_report_handle) == 48);
    assert!(core::mem::offset_of!(MushaStartInfoV1, reserved1) == 56);
};
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct MushaNoteV1 {
    pub struct_size: u32,
    pub candidate_revision: u32,
    pub abi_major: u16,
    pub abi_minor: u16,
    pub cpu_profile: u32,
    pub required_features: u64,
    pub requested_permissions: u64,
    pub arena_bytes: u64,
    pub stack_bytes: u64,
    pub flags: u64,
    pub reserved: u64,
}
const _: () = {
    assert!(core::mem::size_of::<MushaNoteV1>() == 64);
    assert!(core::mem::align_of::<MushaNoteV1>() == 8);
    assert!(core::mem::offset_of!(MushaNoteV1, struct_size) == 0);
    assert!(core::mem::offset_of!(MushaNoteV1, candidate_revision) == 4);
    assert!(core::mem::offset_of!(MushaNoteV1, abi_major) == 8);
    assert!(core::mem::offset_of!(MushaNoteV1, abi_minor) == 10);
    assert!(core::mem::offset_of!(MushaNoteV1, cpu_profile) == 12);
    assert!(core::mem::offset_of!(MushaNoteV1, required_features) == 16);
    assert!(core::mem::offset_of!(MushaNoteV1, requested_permissions) == 24);
    assert!(core::mem::offset_of!(MushaNoteV1, arena_bytes) == 32);
    assert!(core::mem::offset_of!(MushaNoteV1, stack_bytes) == 40);
    assert!(core::mem::offset_of!(MushaNoteV1, flags) == 48);
    assert!(core::mem::offset_of!(MushaNoteV1, reserved) == 56);
};
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct MushaInputEvent {
    pub struct_size: u16,
    pub kind: u16,
    pub flags: u32,
    pub timestamp_ms: u64,
    pub device: u32,
    pub code: u16,
    pub modifiers: u16,
    pub value0: i32,
    pub value1: i32,
    pub value2: i32,
    pub buttons: u32,
    pub reserved: u64,
}
const _: () = {
    assert!(core::mem::size_of::<MushaInputEvent>() == 48);
    assert!(core::mem::align_of::<MushaInputEvent>() == 8);
    assert!(core::mem::offset_of!(MushaInputEvent, struct_size) == 0);
    assert!(core::mem::offset_of!(MushaInputEvent, kind) == 2);
    assert!(core::mem::offset_of!(MushaInputEvent, flags) == 4);
    assert!(core::mem::offset_of!(MushaInputEvent, timestamp_ms) == 8);
    assert!(core::mem::offset_of!(MushaInputEvent, device) == 16);
    assert!(core::mem::offset_of!(MushaInputEvent, code) == 20);
    assert!(core::mem::offset_of!(MushaInputEvent, modifiers) == 22);
    assert!(core::mem::offset_of!(MushaInputEvent, value0) == 24);
    assert!(core::mem::offset_of!(MushaInputEvent, value1) == 28);
    assert!(core::mem::offset_of!(MushaInputEvent, value2) == 32);
    assert!(core::mem::offset_of!(MushaInputEvent, buttons) == 36);
    assert!(core::mem::offset_of!(MushaInputEvent, reserved) == 40);
};
