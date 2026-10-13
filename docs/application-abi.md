# External ABI — experimental candidate r1

## English

Source: [policy 3.0 section 12](reference/boot-api-sdk-v3/docs/boot-readiness-policy.md). Adopt SysV AMD64 / LP64 C ABI, static little-endian ELF64 and Musha startup/syscall conventions. No iBCS/iBCS2, Linux binary compatibility or POSIX compatibility. Kernel EFI/Win64 remains separate and needs an explicit assembly bridge.

| Item | Policy/candidate | Implemented | Execution evidence | Public compatibility fixed |
|---|---|---|---|---|
| SysV LP64, no red zone | adopted | C flags/crt0 | host layout | no |
| Start info | candidate r1 | generated C/Rust assertions | host + x64 compile | no |
| syscall convention | candidate r1 | wrappers + C1/C2 dispatcher | QEMU assembly and SDK C probes | no |
| ELF/NOTE | candidate r1 | host/kernel validators + C ELF | host and QEMU rejection tests | no |
| x86-64/SSE2 | candidate | FP compile fixtures/custom target | no C↔Rust guest FP | no |
| Ring 3/IRETQ/copyin-out | C | C1/C2 + owned-mapping copies; general copies pending | QEMU assembly and SDK C ELF | no |
| External practical I/O | D | internal services only | cooperative tests only | no |

Canonical values are `sdk/abi/experimental.json`; generator --check detects stale public C/Rust declarations. The following local allocations are experimental and may change before public fixation. Document 3.0 is not ABI 3.0. Syscall allocation starts at 0x100 as a private candidate namespace, not a borrowed Linux table.

| Candidate constant | Decimal |
|---|---|
| `ABI_MAJOR` | `0` |
| `ABI_MINOR` | `1` |
| `CANDIDATE_REVISION` | `1` |
| `CPU_PROFILE` | `1` |
| `START_SIZE` | `64` |
| `SYS_QUERY` | `256` |
| `SYS_LOG` | `257` |
| `SYS_EXIT` | `258` |
| `OK` | `0` |
| `ERR_UNKNOWN_CALL` | `-1` |
| `ERR_UNSUPPORTED` | `-2` |
| `ERR_INVALID` | `-3` |
| `ERR_DENIED` | `-4` |
| `ERR_LIMIT` | `-5` |
| `ERR_AGAIN` | `-6` |
| `ERR_TIMEOUT` | `-7` |
| `ERR_CANCELED` | `-8` |
| `ERR_DISCONNECTED` | `-9` |
| `ERR_IO` | `-10` |
| `ERR_NO_MEMORY` | `-11` |
| `ERR_NOT_FOUND` | `-12` |
| `NOTE_TYPE` | `1297437512` |
| `NOTE_SIZE` | `64` |
| `FEATURE_BASE` | `1` |
| `PERMISSION_LOG` | `1` |
| `MAX_LOG_BYTES` | `4096` |
| `IMAGE_BASE` | `4194304` |
| `IMAGE_LIMIT` | `16777216` |
| `ARENA_BASE` | `268435456` |
| `ARENA_LIMIT` | `67108864` |
| `STACK_TOP` | `1879048192` |
| `STACK_LIMIT` | `1048576` |
| `START_BASE` | `536870912` |
| `MAX_SEGMENTS` | `32` |

### Start record

`musha_start_info_v1` is 64 bytes, alignment 8. v1 is a record name, not published ABI 1.0. Every field offset is asserted in both languages.

| Field | Fixed-width type | Offset |
|---|---|---|
| `struct_size` | `u32` | 0 |
| `abi_major` | `u16` | 4 |
| `abi_minor` | `u16` | 6 |
| `flags` | `u32` | 8 |
| `reserved0` | `u32` | 12 |
| `arena_base` | `u64` | 16 |
| `arena_bytes` | `u64` | 24 |
| `stack_base` | `u64` | 32 |
| `stack_bytes` | `u64` | 40 |
| `boot_report_handle` | `u64` | 48 |
| `reserved1` | `u64` | 56 |

OS supplies a zero-initialized, read-only user copy, never internal BootInfo. Arena/stack are user virtual addresses, stack excludes guards, report handle zero means unavailable. Arena/stack RW/NX, record R/NX, stack guard unmapped; all loaded/arena/stack/record/guard ranges must be canonical, nonoverlapping, overflow-checked and live until teardown. Candidate r1 requires exactly 64 bytes, known flags and zero reserved. Future longer-record negotiation is undecided, not implicit. Input length and output capacity are separate; output padding must be initialized, insufficient capacity reports required size without overflow.

Opaque u64 handles use kernel-side kind/owner/permissions/generation validation; zero is invalid/unavailable. Bit allocation and report wire record remain undecided. Host record validation cannot prove mapped access, ownership or DMA lifetime. C must check every page and use fault-safe copies.

### ELF / NOTE / first instruction

Require ELFCLASS64/ELFDATA2LSB/EM_X86_64/static ET_EXEC, bounded PH/file sizes, alignment congruence, file-backed executable entry, page-wise W^X and disjoint ranges. Kernel zeroes BSS (`memsz-filesz`). Reject PT_INTERP/PT_DYNAMIC/PT_TLS/unknown runtime PHs; debug and resolved static sections alone are not errors. Host app-check does not replace kernel checks/mapping or grant permissions.

The generated `musha_note_v1`/`MushaNoteV1` also has checked 64-byte size, alignment 8 and every field offset. File NOTE alignment is only 4: decode/copy bytes rather than casting an unaligned file pointer to this native type.

Exactly one NOTE: name `MUSHA\0`, namesz 6, type NOTE_TYPE, zero padding, 4-byte alignment. Little-endian descriptor is 64 bytes: u32 size@0, u32 revision@4, u16 major@8, u16 minor@10, u32 CPU@12; u64 features@16, permissions@24, arena_bytes@32, stack_bytes@40, flags@48, reserved@56. Reject missing/duplicate/inconsistent versions, unsupported features/permissions, unaligned/out-of-limit RAM/stack and nonzero flags/reserved. Requested permissions are not granted permissions.

Planned CPL3 jump: RSP%16=0, RDI=start copy, DF=0, no return address; other GPRs zero, FS/GS user bases zero, privileged flags masked. Candidate FPU defaults x87=0x037f, MXCSR=0x1f80, SSE2 supported, AVX absent. crt0 calls C with entry RSP%16=8, then exit; unexpected exit return traps. APP_ENTERING is OS-side; first user query syscall must prove future APP_RUNNING.

### Raw syscall

RAX=number; args RDI/RSI/RDX/R10/R8/R9. Return RAX=sign-extended int32 status (zero success, negative error), RDX=value, zero on error. Preserve raw GPRs except RAX/RDX/RCX/R11; flags clobbered and DF clear at language boundaries. Preserve supported x87/SSE state. C wrapper follows ordinary SysV caller/callee rules, which differ from raw preservation. C fourth arg RCX moves to R10; RDX is input/output; asm declares RCX/R11, flags and memory effects. Unknown/unsupported calls are errors, never success stubs.

### C review gate (partially implemented in C1–C3)

The [C3 lifecycle probe](ring3-lifecycle.md) adds bounded rollback, repeated launches and return to the static OS path; this is not general scheduling or resource management.

The [C1 probe](ring3-prototype.md) records the implemented selectors/MSRs, fixed-payload tests and remaining limitations. [C2](ring3-elf.md) adds SDK C startup and bounded ELF loading; Rust startup and the full review gate below remain open.

SYSCALL does not change RSP. Save user RSP and switch kernel stack before any compiler prologue; TSS.RSP0 alone is insufficient. Review GDT selectors and STAR/LSTAR/SFMASK/EFER together, stack guards, transition frame, per-CPU ownership and EFI/Win64 bridge. Validate canonical RIP/RSP, user mappings/selectors/flags before IRETQ. SYSRET is deferred. Handle NMI/exception reentry during stack transition through reviewed IST/recovery even with interrupts disabled.

Define CR0/CR4, CPUID checks, FPU/SSE init and aligned save format on syscall/interrupt/fault paths. C1 selects prototype selectors/MSRs and records its consulted manual revision; production exception recovery and complete current-manual review remain C work. Guest tests must cover raw preservation/result errors, stack switching, invalid pointers/lengths, noncanonical return, guards, FP state and C↔Rust integers/double/mixed structs/stack arguments.

Rust 1.99.0 built-in x86_64-unknown-none uses softfloat and disables SSE. Integer Hello does not establish FP ABI compatibility. The custom SSE2 user target is candidate-only; pinned rust-src/core/alloc rebuilding and mixed-language guest tests remain required.

## 日本語

第12節のSysV／LP64、静的ELF、独自起動／syscallを採用方針とします。表の番号・NOTE・CPU ID・上限・仮想配置は実験候補で、公開互換性固定はありません。C/Rust公開型は単一定義から生成し全offsetを検査します。

起動情報はユーザー空間への読出し専用コピーです。内部BootInfoや物理アドレスは公開しません。ハンドルのビット構成とレポート形式は未確定です。raw syscallの保存規約とC関数規約を分離します。SDKコンパイル試験はCPL3遷移、IRETQ、安全なコピー、FPU保存の実行証拠ではありません。専用Rust targetのcore再構築とC/Rust相互呼出しをCの必須条件に残します。

C1の[固定payload遷移試験](ring3-prototype.md)を追加しました。[C2](ring3-elf.md)でSDK C ELF実行を追加しました。Rust実行・汎用コピー・異常終了からの回復は未完了です。
