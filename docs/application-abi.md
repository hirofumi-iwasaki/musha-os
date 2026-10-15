# External ABI — experimental candidates r1/r2

Status: current

## English

### Stage4 r2 runtime checkpoint —2026-10-15

Experimental r2 now connects real TIME/WAIT/INPUT and relative USB boot mouse to C and Rust guests. r1 values/NOTE acceptance and the default C build remain supported. No public ABI compatibility is frozen. See the [r2 evidence](stage4-r2-0.4.0-progress.md) and [ABI ledger](application-abi.md); full regression and real NUC5/Ryzen acceptance remain open.

C: `musha_time(&ms)`, `musha_wait(timeout_ms, flags, &reason)`, `musha_input(&event)`. Rust guest wrappers: `time()`, `wait(timeout_ms, input)`, `input()` return owned values. TIME permission2 returns monotonic sampled PM-epoch boot milliseconds; WAIT permission4 supports0–60000ms, flags0 or WAIT_INPUT1 (also requires INPUT permission8), returns0 timeout/poll or1 pending input. INPUT copies one48-byte LE record, returns ERR_AGAIN when empty; insufficient capacity is ERR_LIMIT, invalid/unwritable ranges ERR_INVALID, missing permissions ERR_DENIED. Errors return zero RDX and leave the queue/output unchanged. Reserved syscall arguments/record bytes must be zero. Timestamp precision depends on cooperative sampling; long unsampled intervals beyond a PM counter wrap cannot be reconstructed.

Only r2 with INPUT permission reopens USB after the old runtime releases its arena borrow. Hosts/rings remain on the OS stack. A saved guest continuation is inside the synchronous USB callback; detach/halt/BME-off precedes normal callback and session cleanup. Only confirmed cleanup permits mapping rollback, arena scrub and a fresh viewer. Failed stop quarantines memory without reuse. WAIT polls a bounded USB step. USB hub/control work may delay a deadline by one bounded poll (control requests have finite1s deadlines); WAIT is not a hard-real-time deadline. Stalled-clock watchdog counts consecutive nonadvancing samples, not all healthy iterations.

Keyboard down/up includes HID usage/modifiers. Mouse-only sessions work without a keyboard. Relative X/Y/buttons use boot format; wheel is enabled only for the validated signed8 relative descriptor subset, otherwise boot mode supplies wheel0. Absolute usb-tablet is not converted. The current live keyboard/mouse Host (or mouse-only Host) is serviced; simultaneous mouse input on a separate third controller is not integrated. Overflow preserves64 older FIFO records then reports a saturated dropped count; consumers must reset held-state assumptions because lost clicks/motion are not reconstructed. Viewer mouse UI is excluded, gamepad production/Bluetooth is stage6, viewer return is implemented locally in stage5; physical acceptance remains open.

Build standalone C r2 with `python3 tools/check-sdk.py --c-only --abi-revision 2 --source tests/io-c.c --output build` (NOTE permissions default15; --permissions can restrict them). Rust: `python3 tools/build-rust.py --r2 --output build`. These use generated NOTE/types and the existing ELF checker. Rust uses pinned1.99.0/rust-src, custom static SSE2/no-red-zone target and rebuilt core/compiler_builtins; Bootstrap enables build-std/JSON target only. Never execute guest ELFs on the host. Separate SDK archives remain development-qualified; the Rust release gate stays closed pending remaining stage4/full acceptance.


Source: [policy 3.0 section 12](reference/boot-api-sdk-v3/docs/boot-readiness-policy.md). Adopt SysV AMD64 / LP64 C ABI, static little-endian ELF64 and Musha startup/syscall conventions. No iBCS/iBCS2, Linux binary compatibility or POSIX compatibility. Kernel EFI/Win64 remains separate and needs an explicit assembly bridge.

| Item | Policy/candidate | Implemented | Execution evidence | Public compatibility fixed |
|---|---|---|---|---|
| SysV LP64, no red zone | adopted | C flags/crt0 | host layout | no |
| Start info | candidate r1 | generated C/Rust assertions | host + x64 compile | no |
| syscall convention | candidate r1 | wrappers + C1/C2 dispatcher | QEMU assembly and SDK C probes | no |
| ELF/NOTE | candidate r1 | host/kernel validators + C ELF | host and QEMU rejection tests | no |
| x86-64/SSE2 | candidate | FP compile fixtures/custom target | no C↔Rust guest FP | no |
| Ring 3/IRETQ/copyin-out | C | C1/C2 + owned-mapping copies; general copies pending | QEMU assembly and SDK C ELF | no |
| External practical I/O | experimental r2 | TIME/WAIT/INPUT, keyboard/relative mouse | local C/Rust QEMU; hardware pending | no |

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

### Experimental r2 allocations (r1 retained)

| Constant | Decimal |
|---|---|
| `R2_REVISION` | `2` |
| `R2_MINOR` | `2` |
| `SYS_TIME` | `259` |
| `SYS_WAIT` | `260` |
| `SYS_INPUT` | `261` |
| `FEATURE_INPUT` | `2` |
| `PERMISSION_TIME` | `2` |
| `PERMISSION_WAIT` | `4` |
| `PERMISSION_INPUT` | `8` |
| `INPUT_SIZE` | `48` |
| `INPUT_KEY` | `1` |
| `INPUT_MOUSE` | `2` |
| `INPUT_GAMEPAD_BUTTON` | `3` |
| `INPUT_GAMEPAD_AXIS` | `4` |
| `INPUT_OVERFLOW` | `5` |
| `WAIT_INPUT` | `1` |
| `WAKE_TIMEOUT` | `0` |
| `WAKE_INPUT` | `1` |
| `MAX_WAIT_MS` | `60000` |

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

The [C1 probe](ring3-prototype.md) records the implemented selectors/MSRs, fixed-payload tests and remaining limitations. [C2](ring3-elf.md) adds SDK C startup and bounded ELF loading; Rust r1 startup is now locally executed as described above; the full review gate below remains open.

SYSCALL does not change RSP. Save user RSP and switch kernel stack before any compiler prologue; TSS.RSP0 alone is insufficient. Review GDT selectors and STAR/LSTAR/SFMASK/EFER together, stack guards, transition frame, per-CPU ownership and EFI/Win64 bridge. Validate canonical RIP/RSP, user mappings/selectors/flags before IRETQ. SYSRET is deferred. Handle NMI/exception reentry during stack transition through reviewed IST/recovery even with interrupts disabled.

Define CR0/CR4, CPUID checks, FPU/SSE init and aligned save format on syscall/interrupt/fault paths. C1 selects prototype selectors/MSRs and records its consulted manual revision; production exception recovery and complete current-manual review remain C work. Guest tests must cover raw preservation/result errors, stack switching, invalid pointers/lengths, noncanonical return, guards, FP state and C↔Rust integers/double/mixed structs/stack arguments.

Rust 1.99.0 built-in x86_64-unknown-none uses softfloat and disables SSE. Integer Hello does not establish FP ABI compatibility. The custom SSE2 user target is candidate-only; pinned rust-src/core rebuilding is now verified; alloc and mixed-language guest tests remain outside this checkpoint.

### Experimental r2 contract (local implementation, no compatibility freeze)

Implemented experimental allocations: TIME0x103, WAIT0x104, INPUT0x105; permission bits TIME2/WAIT4/INPUT8. Existing0x100–0x102 and LOG1 are retained. No public compatibility is fixed. The schema/generator, host/kernel NOTE validators, dispatcher and C/Rust wrappers now implement r2 alongside r1. NOTE r2 requires revision2/minor2/features3; requested permissions are a subset of15. r1 still requires revision1/minor1/features1 and LOG-only permissions.

INPUT copies one48-byte little-endian value: size u16@0, kind u16@2, flags u32@4, boot timestamp u64@8, source/device u32@16, code u16@20, modifiers u16@22, three signed i32 values@24/28/32, buttons u32@36, reserved zero bytes40–47. Kind1 keyboard (flags0 up/1 down, HID usage0–255, modifier mask),2 relative mouse (X/Y/wheel, absent wheel0),3 gamepad button,4 gamepad axis,5 overflow. An absolute tablet is not a relative mouse producer. The model preserves64 older FIFO records and drops further input until draining, then returns an overflow record with saturated lost count in buttons; it cannot reconstruct lost movement/clicks or held state. Consumers reset held-state assumptions after overflow; source IDs1/2 identify the current keyboard/mouse session, not hardware handles. Disconnect ends that input session; no reconnect-generation reuse is attempted.

TIME returns sampled monotonic PM-epoch boot milliseconds. WAIT uses relative0–60000ms, flags0 or INPUT1, and returns timeout0 or input1 (input wins if both are observed). INPUT wake additionally needs INPUT permission. The consecutive stalled-sample watchdog reports ERR_IO; a failed input poll stops DMA and reports ERR_DISCONNECTED. INPUT checks every writable supervisor-alias byte before polling/removing a record, leaving output/queue unchanged on invalid/capacity errors, returning ERR_AGAIN on empty and RDX0 on errors. Reserved arguments are rejected. No user virtual reference escapes a call. QEMU exercises actual copyout, read-only/overflow/cross-boundary rejection and permissions; the host queue model alone is not that evidence.

## 日本語

### 工程4 r2実接続到達点 —2026-10-15

実験r2のTIME/WAIT/INPUTと相対USB boot mouseをC/Rust guestへ実接続しました。r1番号/NOTE受入と既定C buildは保持し、公開互換性は固定しません。[r2証拠](stage4-r2-0.4.0-progress.md)と[ABI台帳](application-abi.md)参照。全回帰・NUC5/Ryzen実機受入は未完です。

Cはmusha_time(&ms)、musha_wait(timeout_ms,flags,&reason)、musha_input(&event)。Rust guestのtime()/wait(timeout_ms,input)/input()は所有値を返します。TIME権限2で単調なsample済みPM起動epoch ms、WAIT権限4で0–60000ms、flags0またはWAIT_INPUT1（入力権限8も必要）、結果0はtimeout/poll、1はpending入力です。INPUTは48byte LE record1個をコピー。空ERR_AGAIN、容量不足ERR_LIMIT、異常/非書込み範囲ERR_INVALID、権限無しERR_DENIED。エラー時RDX0でqueue/outputを変更しません。予約引数/record bytesはゼロ。時刻精度は協調sampleに依存し、PM counter周回より長くsampleしない経過時間は復元できません。

r2かつINPUT権限の場合だけ、旧runtimeのarena借用終了後にUSBを再開します。Host/ringはOS stack上。同期USB callback内でguest保存継続を囲み、detach/halt/BME停止確認後にcallbackとsession cleanupを通常returnします。確認済みcleanup後だけmapping復元・arena消去・新viewerへ進み、停止未確認は再利用せず隔離。WAITは上限付きUSB1stepをpollします。hub/control処理により期限が上限付き1poll分（control各1秒期限）遅れることがあり、hard realtime保証ではありません。clock watchdogは全反復数でなく時刻非進行の連続sample数を数えます。

keyboard down/upはHID usage/modifier付き。keyboard無しmouse単独sessionも動作します。相対XY/buttonはboot形式、wheelは検証したsigned8相対descriptor部分形式だけ有効で、それ以外のboot modeはwheel0。絶対usb-tabletは変換しません。現live keyboard/mouse Hostまたはmouse単独Hostを処理し、別の第3controllerの同時mouse収集は未統合。overflowは古い64件を保持後、飽和drop数を通知します。失われたclick/移動は復元できないためconsumerは保持状態の前提をリセットします。viewer mouse UIは対象外、gamepad producer/Bluetoothは工程6、viewer復帰は工程5でローカル実装済み、実機受入は未確認です。

単独C r2 buildは `python3 tools/check-sdk.py --c-only --abi-revision 2 --source tests/io-c.c --output build`（NOTE権限は既定15、--permissionsで制限）。Rustは `python3 tools/build-rust.py --r2 --output build`。生成NOTE/型と既存ELF checkerを使います。Rustは固定1.99.0/rust-src、専用static SSE2/red-zone無しtarget、再構築core/compiler_builtinsで、Bootstrapはbuild-std/JSON target用だけ。guest ELFをhost実行しません。SDK archiveは分離した開発版で、Rust公開gateは工程4残件/全受入まで閉じています。


第12節のSysV／LP64、静的ELF、独自起動／syscallを採用方針とします。表の番号・NOTE・CPU ID・上限・仮想配置は実験候補で、公開互換性固定はありません。C/Rust公開型は単一定義から生成し全offsetを検査します。

起動情報はユーザー空間への読出し専用コピーです。内部BootInfoや物理アドレスは公開しません。ハンドルのビット構成とレポート形式は未確定です。raw syscallの保存規約とC関数規約を分離します。SDKコンパイル試験はCPL3遷移、IRETQ、安全なコピー、FPU保存の実行証拠ではありません。専用Rust targetのcore再構築とC/Rust相互呼出しをCの必須条件に残します。

C1の[固定payload遷移試験](ring3-prototype.md)を追加しました。[C2](ring3-elf.md)でSDK C ELF実行を追加しました。Rust r1実行は上記工程4で追加検証済み。汎用コピー・通常版の回復は未完了です。

### 実験r2契約（local実装・公開互換固定なし）

TIME0x103、WAIT0x104、INPUT0x105、権限TIME2/WAIT4/INPUT8、既存0x100–0x102/LOG1を保持。schema/生成型/host・kernel NOTE検証/dispatcher/C・Rust wrapperをr1と併存実装。NOTE r2はrevision2/minor2/features3、権限は15の部分集合。r1はrevision1/minor1/features1とLOGだけのままです。

INPUTは48byte LE値。size u16@0、kind u16@2、flags u32@4、起動時刻u64@8、source u32@16、code u16@20、modifiers u16@22、signed i32値3個@24/28/32、buttons u32@36、予約ゼロ40–47。kind1 keyboard（flags0 up/1 down、usage0–255/modifier）、2相対mouse（XY/wheel、無し0）、3gamepad button、4axis、5overflow。gamepad producerは工程6。絶対tabletは相対mouseではありません。古い64件を保持し、drain後buttonsに飽和drop数を入れたoverflowを返すためconsumerは保持状態前提をリセットします。source1/2は現keyboard/mouse sessionでhardware handleではなく、切断後の再接続/generation再利用はしません。

TIMEはsample済み単調PM起動epoch ms。WAIT相対0–60000ms、flags0/INPUT1、結果timeout0/input1（同時観測時input優先）。入力wakeはINPUT権限も必要。clock非進行連続sample watchdogはERR_IO、poll失敗はDMA停止後ERR_DISCONNECTED。INPUTはpoll/pop前に全byteの書込み可能supervisor aliasを検査し、異常buffer/容量エラーではoutput/queue不変、空ERR_AGAIN、エラーRDX0。予約引数を拒否し、user仮想参照をcall外へ返しません。QEMUで実copyout・読取り専用/overflow/cross-boundary拒否・権限を検証し、純粋queue modelと区別します。
