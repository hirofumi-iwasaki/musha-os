# Musha-OS 0.1.0 architecture design

## English

Established: 2026-10-07. Updated: 2026-10-08. Status: Basic design adopted; detailed design ongoing.
See [policy-v0.1.md](policy-v0.1.md) for the baseline policy.
Original code, documentation, and configuration use Apache-2.0.
New source files must include the actual copyright holder and `SPDX-License-Identifier: Apache-2.0`.
Retain original license notices in third-party code.
The v0.1 filename retains the older document name; the first release number is 0.1.0.

### 1. Implementation units

Use Rust as the primary language with the minimum necessary x86-64 assembly.
Keep lwIP in C and build it with Clang / LLD. Custom C drivers are not part of the baseline approach.
Manage Rust code in a Cargo workspace and initially favor a configuration achievable on stable Rust.
The current build pins Rust 1.99.0 and lwIP 2.2.1. The initial 82574 port and selected C source build are documented in [network diagnostics](network.md). NUC initialization remains a design target. The initial [cooperative loop](cooperative-io.md) is implemented for input, cached files and QEMU networking; general asynchronous disk requests and the application UDP API remain pending.

The runtime uses `#![no_std]` and does not use a standard library dependent on OS or UEFI services.
Initially use fixed-capacity buffers and explicit arena allocation, avoiding reliance on a general-purpose heap.
Before adopting a dependency requiring `alloc`, design our own allocator and OOM handling.
Finish processing values that depend on the UEFI allocator or services before ExitBootServices.
Ensure no Drop operation calls those services after exit.

Use `x86_64-unknown-uefi` for the entry target and `extern "efiapi"` at the UEFI boundary.
Do not use Rust's internal ABI as an external contract.
Use an explicit C ABI and fixed-width `#[repr(C)]` structures at C boundaries;
do not pass Rust references, enums, Vecs, or trait objects directly.
Do not assume the UEFI target's C ABI is SysV. Compile lwIP with the same ABI.
Fix the external application ABI separately based on the post-transition CPU configuration.
For the single-EFI approach, the target's default C ABI is the first choice; do not automatically introduce SysV.

Limit `unsafe` to MMIO, DMA, page tables, CPU operations, and FFI.
Document valid ranges, alignment, ownership, lifetime, and concurrent-access assumptions at each use.
Do not treat device-updated DMA memory as ordinary shared references.
Explicitly define CPU / device ownership transitions, volatile accesses, and required barriers.
Volatile access alone does not guarantee synchronization or cache coherence.
Provide safe APIs to higher layers.

On panic, report diagnostics and stop without unwinding. Prohibit unwinding across FFI boundaries.
Do not use the red zone. Initialize CPU state at boot to match the target's FPU / SIMD assumptions,
including compiler-generated code. A blanket prohibition cannot exclude all compiler-generated instructions;
check build settings and generated code.
Select UEFI crates after checking their licenses and conditions for use after Boot Services exit.

For 0.1.0, statically link the runtime and validation application into one PE32+ EFI executable.
Defer dynamic ELF loading, process isolation, and switching between multiple applications to later versions.

The first validation application is a diagnostic application displaying `Hello Musha-OS!` on the GOP screen.
Display the greeting on the initial screen as well; after reaching the application loop,
distinguish completion of boot stages.
Show display, key input, arena capacity / boundary tests, known-file reads,
and UDP round-trip results on one screen.
Distinguish not run, success, and failure; the greeting alone does not mean every feature passed.
The target source layout follows. The implemented tree currently provides the boot, framebuffer, memory, platform, api, xhci, fs and net crates; some planned components are still pending.

```text
boot/              UEFI entry and boot-information construction
arch/x86_64/       GDT, IDT, page tables, and time source
core/              Memory, diagnostics, and cooperative execution
 drivers/          PCI, xHCI, and Intel Ethernet
 usb/              Enumeration, HID, and Mass Storage
 fs/               FAT32 reads
 net/              lwIP integration and application communication
 api/              Safe Rust wrappers and FFI contracts
 include/musha/    C API headers (after contracts are finalized)
 apps/             Validation applications
 tests/            Host and QEMU tests
 tools/            Image creation and test helpers
```

### 2. Boot state transitions

Use `UEFI_ENTRY → PREPARE → EXIT_BOOT_SERVICES → CPU_INIT → MEMORY_INIT
→ DEVICE_INIT → APP_LOOP`.

In PREPARE, select GOP, validate ACPI, and retain PM timer information as values.
Allocate the runtime stack, page tables, boot information, and memory-map buffer.
Store display information as values.
Do not log or allocate between the final GetMemoryMap and ExitBootServices.
On failure, obtain the map again using the preallocated buffer and retry.
Treat insufficient buffer capacity or an exceeded retry limit as boot failures;
do not continue normal boot. After exit, do not call UEFI protocol function pointers.

Switch to our own stack and GDT / IDT, then enable our own page tables.
Keep the code, stack, and boot information required for this transition mapped in both old and new mappings.
The initial version masks external interrupts and polls device completions.
Minimal exception handlers display diagnostics and halt.
Do not call UEFI Runtime Services; keep their regions reserved.

### 3. Boot information and memory

Internal boot_info contains a magic value, structure size, version,
UEFI map base / length / descriptor size, GOP physical address / size / width / height / stride / pixel format,
PM timer information, and a list of reserved ranges.
Always pass each region's size alongside its pointer value.
Walk UEFI descriptors using the returned stride rather than assuming a fixed-size array.

Initially use identity mapping corresponding to physical addresses and map only RAM and MMIO in use.
Aim for non-writable code and non-executable data, and check NX support.
Use cache attributes for MMIO that differ from ordinary RAM; validate the GOP address range as well.
Set page attributes to RO/execute or RW/NX according to PE sections.
Select existing WB / UC PAT entries and preserve MTRRs.
Hardware consistency checks remain. See [memory implementation](memory.md).

Initially allocate only from EfiConventionalMemory after subtracting explicit reserved ranges.
Do not reclaim Boot Services regions in 0.1.0; prioritize safe ownership over capacity.
Separate kernel, DMA, and arena regions, checking start / end arithmetic for overflow.
Initially allocate contiguous DMA regions below 4GiB and apply each device's constraints and alignment requirements.
Do not free or reuse buffers while a device is using them.

Pass one contiguous, page-aligned arena selected from the largest available candidate.
Return an explicit error if it is too small.
The diagnostic application requires at least 16MiB; the initial maximum is 64MiB.
Pass the CPU-accessible virtual base and byte count; do not expose it as a DMA API.
The arena remains valid until application termination and supports custom allocation within its boundaries.
There is no isolation, so the runtime is not guaranteed protection against invalid writes.

### 4. Time and execution model

The runtime owns the loop. Each iteration advances USB, NIC receive / transmit completions,
lwIP timers, input events, and the application step in that order.
Bound the work performed by each operation.
Application steps must not block; split long computations across multiple steps.
Initially target 1ms per step and a normal loop within 10ms, and measure actual performance.
Diagnose budget overruns without providing forced preemption.

Use the ACPI PM timer as the initial time-source candidate and also check HPET availability.
If neither is available, diagnose the configuration as unsupported.
Do not infer TSC timing from CPU frequency. Handle counter wraparound and provide monotonic milliseconds.
Finalize register access and wraparound tests in the detailed design.

### 5. Application API contract

Call statically linked app_init(context), app_step(context), and app_shutdown(context).
The context contains version, size, arena, display information, and an API table.
Provide safe wrappers for the Rust validation application and equivalent functionality through a C ABI.
Fix structure sizes, calling conventions, and integer widths before producing headers.
Pass the arena as a mutable slice to its sole owner; retain no runtime mutable reference to the same region.
This API is a contract with the internal runtime; 0.1.0 does not guarantee binary compatibility with future versions.

| API group | 0.1.0 contract |
|---|---|
| Display | Bounds-checked pixel / rectangle drawing; convert physical formats inside the API |
| Input | Nonblocking key-event retrieval; return key codes and press / release state |
| Time | Return monotonic milliseconds |
| Files | open / read / close; read-only, absolute paths, initially ASCII names |
| UDP | bind / send / receive / close; return received data by copying |
| Diagnostics | Logs, current device status, and error codes |

Return failures as fixed-integer status values. Initial candidates are OK, AGAIN, INVALID,
UNSUPPORTED, NO_MEMORY, IO, TIMEOUT, DISCONNECTED, and NOT_FOUND.
Caller buffer ownership does not transfer. Asynchronous operations copy into runtime-owned buffers.
Advance file I/O as a state machine and return AGAIN until completion.
Use handle generation numbers to detect use after disconnection or close.
The application TCP API is not required for 0.1.0; acceptance covers UDP.

### 6. USB and files

Initialize xHCI in this order: ownership, stop, reset, capability checks,
required scratchpads, DCBAA, command ring, event ring, port reset, and slot / endpoint contexts.
Apply time limits to every wait and validate TRB cycles, ring wraparound, and DMA ordering.
Define register offsets, timeout values, and ring capacities in the detailed driver specifications.

Initially target directly connected USB Boot Keyboards and USB Mass Storage BOT / SCSI transparent devices.
USB2 hub boot enumeration is now an implementation extension; see [hub design and validation](usb-multi-controller-hub-plan.md). USB3.0/5Gbps hub initialization is implemented but physical transfers are unverified. USB3.1/3.2 hubs, hotplug re-enumeration, UAS, and arbitrary HID report descriptors remain unsupported.
Use the keyboard boot protocol and report modifier / press / release changes.
Validate BOT CBW / data / CSW tags, lengths, and status, and design stall and reset recovery.
On USB disconnection, fail pending requests and invalidate old handles.

The image contains GPT, a single FAT32 ESP, EFI/BOOT/BOOTX64.EFI, and validation files.
Implement basic MBR / GPT partition detection and BPB validation for FAT32, initially using 8.3 names.
Check sector sizes and boundaries; reject damaged cluster chains, cycles, and out-of-range access.
Do not provide a disk-write API. Use the same image as a QEMU USB disk and on physical USB media.

### 7. Intel NICs and lwIP

Use an explicit PCI ID allowlist and separate initialization paths for 82574, I218, and I219.
Check BARs, bus mastering, reset, MAC retrieval, PHY / link, descriptor rings,
and transmit / receive completions for each model.
Do not substitute 82574 register settings for I218 / I219 handling.
Diagnose unsupported revisions instead of applying an incorrect driver.

Pin the C implementation of lwIP and connect it to Rust through a small C shim.
Manage pbuf and callback ownership in the integration layer; C must not retain Rust borrows.
Use lwIP with NO_SYS=1 and the raw API internally, calling input and sys_check_timeouts from the same loop.
Do not use socket / netconn APIs.
Do not expose lwIP pbufs to applications; use receive copies and bounded queues.
Supply static IPv4, netmask, and gateway at build time; defer DHCP / DNS / IPv6.
Record drops under congestion and prohibit unbounded memory allocation.

### 8. Errors and diagnostics

Treat critical boot, CPU, and memory errors as panics: display the stage, code,
and related address, then halt.
Retain device failure status and allow the diagnostic application to continue.
However, 0.1.0 does not pass unless every required feature is present.
Use a fixed-capacity RAM log and GOP console, with auxiliary serial logs only in QEMU.
Assume physical machines lack serial output. Do not depend on a write-only USB log.

### 9. Reference specifications

- [Rust UEFI targets and calling conventions](https://doc.rust-lang.org/rustc/platform-support/unknown-uefi.html)
- [Rust FFI contracts](https://doc.rust-lang.org/nomicon/ffi.html)
- [UEFI specification](https://uefi.org/specs/UEFI/2.11/)
- [Intel xHCI specification and initialization](https://cdrdv2-public.intel.com/625472/625472_xHCI_Rev1_2b.pdf)
- [lwIP NO_SYS execution model](https://www.nongnu.org/lwip/2_1_x/group__lwip__nosys.html)

Pin reference specification versions when implementation begins;
do not require the reference hardware to conform to the latest UEFI version.
Add primary NIC, USB class, and FAT references, plus licenses for incorporated code, during detailed design.

### Current API / xHCI implementation

The [Rust application API](app-api.md) and [first stage of xHCI initialization](xhci.md) are implemented.
A C context containing an API table and the complete I/O APIs will be added later.
Extend the contract for preserving original ACPI tables when AML / MCFG is needed;
currently only PM timer information is passed.

### Future application isolation

0.1.0 runs the runtime and trusted diagnostic application at Ring 0 in long mode.
The [future architecture](future-design.md) adopts Ring 3 as the default for separately supplied applications, with private address spaces, checked system calls, fault containment and enforceable CPU budgets. This is outside the 0.1.0 implementation scope.

---

## 日本語

**Musha-OS 0.1.0 アーキテクチャ設計**

策定日: 2026-10-07。更新日: 2026-10-08。状態: 基本設計採用、詳細設計継続中。
基本方針は [policy-v0.1.md](policy-v0.1.md) を参照する。
独自コード・文書・設定のライセンスはApache-2.0とする。新規ソースには
実際の著作権者表示と `SPDX-License-Identifier: Apache-2.0` を付ける。
第三者コードには元のライセンス表示を保持する。
ファイル名のv0.1は旧文書名を保持したもので、初回リリース番号は0.1.0とする。

### 1. 実装単位

主言語はRustとし、必要最小限のx86-64アセンブリを併用する。
lwIPはCのまま採用し、Clang / LLDでビルドする。独自Cドライバを基本構成にはしない。
RustはCargo workspaceで管理し、まずstableで実現できる構成を優先する。
現在のbuildはRust 1.99.0、lwIP 2.2.1へ固定した。初期82574 portと採用Cソースのbuildは[通信診断](network.md)を参照。NUC初期化は設計上の目標として残る。入力・cached file・QEMU通信の初期[協調loop](cooperative-io.md)は実装済みだが、一般の非同期disk要求とアプリUDP APIは未実装。

本体は `#![no_std]` とし、OS・UEFIサービスに依存する標準ライブラリを使わない。
初期は固定容量バッファと明示的なarena割当てを使い、汎用ヒープへの依存を避ける。
`alloc` が必要な依存を採用する場合は、自前アロケータとOOM処理を先に設計する。
UEFI側アロケータやサービスに依存する値はExitBootServices前に処理を終え、
終了後にそのサービスを呼ぶDrop処理が走らない構成とする。

入口は `x86_64-unknown-uefi`、UEFIとの境界は `extern "efiapi"` を使用する。
Rust内部ABIは外部契約に使わない。Cとの境界は明示したC ABIと `#[repr(C)]` の
固定幅構造体を使い、Rustの参照・enum・Vec・trait objectをそのまま渡さない。
UEFIターゲットのC ABIをSysVと決めつけない。lwIP側も同じABIでコンパイルする。
外部アプリABIはCPU切替後の構成を踏まえて別途固定し、単一EFI方式では
ターゲット標準のC ABIを第一候補とする。SysVの導入は自動的に行わない。

MMIO、DMA、ページテーブル、CPU操作、FFIに `unsafe` を限定し、
各箇所に有効範囲・アラインメント・所有権・寿命・同時アクセスの根拠を記す。
デバイスが更新するDMAメモリを通常の共有参照として扱わず、CPU / デバイス間の
所有権遷移、volatileアクセス、必要なbarrierを明示する。volatileだけで
同期やキャッシュ整合が保証されると考えない。上位層には安全なAPIを提供する。

panicはunwindせず診断後に停止する。FFI境界を越えるunwindを禁止する。
red zoneは使用しない。FPU / SIMDはコンパイラ生成コードを含め、
使用するターゲットの前提と整合するCPU状態を起動時に初期化する。
一律禁止だけではコンパイラ生成命令を排除できないため、ビルド設定と生成コードを確認する。
UEFI関連crateはライセンスとBoot Services終了後の利用条件を確認して選ぶ。

0.1.0は一つのPE32+ EFI実行ファイルにランタイムと検証アプリを静的リンクする。
動的ELFロード、プロセス分離、複数アプリ切替は後続版へ送る。

最初の検証アプリは診断アプリとし、GOP画面に `Hello Musha-OS!` と表示する。
初期画面でも表示し、アプリループ到達後には起動段階の完了を区別して表示する。
画面、キー入力、arena容量・境界試験、既知ファイル読出し、UDP往復の
結果を一画面で確認できるようにする。未実行・成功・失敗を区別し、
Hello表示だけを全機能合格と判断しない。
以下を目標のソース構成とする。現在はboot、framebuffer、memory、platform、api、xhci、fs、net crateを実装済みで、予定構成の一部は未実装である。

```text
boot/              UEFI入口、起動情報の構築
arch/x86_64/       GDT、IDT、ページテーブル、時間源
core/              メモリ、診断、協調実行
 drivers/          PCI、xHCI、Intel Ethernet
 usb/              列挙、HID、Mass Storage
 fs/               FAT32読出し
 net/              lwIP接続、アプリ向け通信
 api/              Rustの安全なラッパーとFFI契約
 include/musha/    C向けAPIヘッダー（契約確定後）
 apps/             検証アプリ
 tests/            ホスト側試験、QEMU試験
 tools/            イメージ作成、試験補助
```

### 2. 起動状態遷移

`UEFI_ENTRY → PREPARE → EXIT_BOOT_SERVICES → CPU_INIT → MEMORY_INIT
→ DEVICE_INIT → APP_LOOP` とする。

PREPAREでGOPを選択し、ACPIを検証してPM timer情報を値として保存する。ランタイム用スタック、ページテーブル、
起動情報、メモリマップ保存バッファを確保する。画面情報は値として保存する。
最終GetMemoryMapとExitBootServicesの間でログ出力や新規確保を行わない。
失敗時は事前確保バッファでマップを再取得して再試行する。
不足バッファや再試行上限超過は起動失敗として処理し、通常起動を続けない。
終了後はUEFIプロトコルの関数ポインタを呼ばない。

自前スタックとGDT / IDTへ移行し、自前ページテーブルを有効化する。
その切替に必要なコード、スタック、起動情報を新旧双方のマッピングで保持する。
初期版は外部割込みをマスクし、デバイス完了をポーリングする。
例外は最小ハンドラで診断を表示して停止する。
UEFI Runtime Servicesは呼ばず、その領域は予約したまま保持する。

### 3. 起動情報とメモリ

内部boot_infoはmagic、構造体サイズ、版、UEFIマップの基点・長さ・descriptor size、
GOPの物理アドレス・サイズ・幅・高さ・stride・pixel format、PM timer情報、
予約範囲一覧を持つ。ポインタ値だけでなく各領域のサイズを必ず渡す。
UEFI descriptorは固定長配列と決めつけず、返されたstrideで走査する。

初期版は物理アドレスに対応する恒等マッピングを採用し、使用するRAMとMMIOだけを
マップする。コードは書込不可、データは実行不可を目標とし、NX対応を確認する。
MMIOは通常RAMと異なるキャッシュ属性にする。GOPもアドレス範囲を検証する。
ページ属性はPE sectionに従いRO/executeまたはRW/NXとする。PATは既存のWB / UCを
選択し、MTRRを保持する。実機での整合確認は残る。[メモリ実装](memory.md)を参照。

初期割当て元はEfiConventionalMemoryに限定し、明示予約範囲を差し引く。
Boot Services領域の回収は0.1.0では行わず、容量より安全な所有権を優先する。
カーネル用領域、DMA領域、arenaを分け、開始・終了の算術オーバーフローを検査する。
DMA用にはまず4GiB未満の連続領域を確保し、各機器の制約と必要アラインメントを適用する。
バッファを使用中の機器がある間は解放・再利用しない。

arenaはページ境界に揃った一つの連続領域を渡す。空き領域の最大候補から選び、
容量が必要量に足りなければ明示エラーとする。診断アプリの最低容量は16MiB、初期上限は64MiBとする。
CPUから使える仮想基点とバイト数を渡し、DMA APIとしては使用させない。
アプリ終了まで有効で、アプリは境界内で独自割当てを行える。
隔離はないため、不正書込からランタイムを保護する保証はしない。

### 4. 時間と実行モデル

ランタイムがループを所有し、各周回でUSB、NIC受信・送信完了、lwIPタイマー、
入力イベント、アプリstepを順に進める。各処理に作業量上限を設ける。
アプリstepはブロック禁止で、長い計算は複数stepに分割する。
初期予算は1step 1ms、通常ループ10ms以内を目標とし実測する。
予算超過は診断するが、強制プリエンプトは提供しない。

時間源はACPI PM timerを初期候補とし、HPETの利用可能性も検査する。
どちらも利用できない場合は未対応構成として診断する。
TSCをCPU周波数から推測して時間源にしない。カウンタのwrapを扱い、
単調増加するミリ秒値を提供する。レジスタアクセスとwrap試験は詳細設計で確定する。

### 5. アプリAPI契約

静的リンクされたapp_init(context)、app_step(context)、app_shutdown(context)を呼ぶ。
contextは版、サイズ、arena、画面情報、APIテーブルを持つ。
Rust検証アプリには安全なラッパーを提供し、C ABIで同等機能を利用可能にする。
ヘッダー化前に構造体サイズ、呼出規約、整数幅を固定する。arenaは唯一の所有者へ
可変sliceとして渡し、ランタイム側に同じ領域の可変参照を残さない。
APIは内部ランタイムとの契約で、0.1.0時点で将来版とのバイナリ互換を保証しない。

| API群 | 0.1.0の契約 |
|---|---|
| 画面 | 範囲確認付きピクセル・矩形描画。物理形式をAPI内で変換 |
| 入力 | 非ブロックのkey event取得。キーコードと押下・解放を返す |
| 時間 | 単調ミリ秒値を返す |
| ファイル | open / read / close。読出し専用、絶対パス、初期はASCII名 |
| UDP | bind / send / receive / close。受信はコピーして返す |
| 診断 | ログ、現在の機器状態、エラーコード |

失敗は固定整数のstatusで返す。OK、AGAIN、INVALID、UNSUPPORTED、NO_MEMORY、
IO、TIMEOUT、DISCONNECTED、NOT_FOUNDを基本候補とする。
呼出し側のバッファ所有権は移転しない。非同期処理はランタイム所有バッファへコピーする。
ファイルI/Oは状態機械として進め、完了前はAGAINを返す。
handleの世代番号で切断後やclose後の誤使用を検出する。
TCPのアプリAPIは0.1.0の必須機能には含めず、UDPまでで受入れを行う。

### 6. USBとファイル

xHCIはownership、停止、リセット、capability検査、必要scratchpad、DCBAA、
command ring、event ring、port reset、slot / endpoint contextの順に初期化する。
各待機に時間制限を設け、TRB cycle、リングwrap、DMA orderingを検証する。
register offsetやtimeout値、リング容量は詳細ドライバ仕様で定める。

初期機器は直結USB Boot Keyboardと直結USB Mass Storage BOT / SCSI transparentを
対象とする。USB2ハブの起動時列挙を追加範囲とする。[設計と検証](usb-multi-controller-hub-plan.md)を参照。USB3.0／5Gbps hub初期化は実装したが実転送は未検証。USB3.1／3.2 hub、hotplug再列挙、UAS、任意HID report descriptorは未対応。
キーボードはboot protocolを用い、修飾キーと押下・解放を差分で通知する。
BOTはCBW / data / CSWのtag、長さ、statusを検査し、stallとreset recoveryを設計する。
USB切断時は保留要求を失敗させ、古いhandleを無効化する。

イメージはGPT、単一FAT32 ESP、EFI/BOOT/BOOTX64.EFI、検証ファイルを持つ。
FAT32はMBR / GPTの基本パーティション検出とBPB検証を行い、初期は8.3名を対象とする。
セクタサイズと境界を検査し、壊れたcluster chain、循環、範囲外アクセスを拒否する。
ディスク書込APIは提供しない。同一イメージをQEMUのUSBディスクと実機USBで使用する。

### 7. Intel NICとlwIP

PCI IDによる明示allowlistを使用し、82574、I218、I219を別初期化経路にする。
BAR、bus mastering、reset、MAC取得、PHY/link、descriptor ring、送受信完了を
機種別に確認する。I218 / I219の処理を82574のレジスタ設定だけで代用しない。
未対応revisionは診断し、誤ったドライバを適用しない。

lwIPはC実装を版固定し、小さなC shimを介してRustと接続する。
pbufやcallbackの所有権を接続層で管理し、Rust側の借用をCが保存しない契約にする。
lwIPはNO_SYS=1、raw APIを内部で使用し、同一ループから入力と
sys_check_timeoutsを呼ぶ。socket / netconn APIは使用しない。
アプリにはlwIPのpbufを公開せず、受信コピーと上限付きキューを使う。
設定は静的IPv4、netmask、gatewayをビルド時に与え、DHCP / DNS / IPv6は後続対応とする。
輻輳時はドロップ数を記録し、無制限のメモリ確保を禁止する。

### 8. エラーと診断

起動・CPU・メモリの重大エラーはpanicとして画面へ段階、code、関連アドレスを表示して停止する。
個別デバイスの失敗は状態表示を残し、診断アプリを継続できるようにする。
ただし全必須機能が揃わなければ0.1.0合格とはしない。
固定容量RAMログとGOPコンソールを基本とし、QEMUのみ補助シリアルログを用いる。
実機でシリアルがないことを前提とする。書込専用USBログには依存しない。

### 9. 参照仕様

- [RustのUEFIターゲットと呼出規約](https://doc.rust-lang.org/rustc/platform-support/unknown-uefi.html)
- [Rust FFIの契約](https://doc.rust-lang.org/nomicon/ffi.html)

- [UEFI仕様](https://uefi.org/specs/UEFI/2.11/)
- [Intel xHCI仕様・初期化手順](https://cdrdv2-public.intel.com/625472/625472_xHCI_Rev1_2b.pdf)
- [lwIP NO_SYS実行モデル](https://www.nongnu.org/lwip/2_1_x/group__lwip__nosys.html)

参照仕様の版は実装開始時に固定し、最新UEFI版への準拠を基準機へ要求しない。
NIC、USB class、FATの一次資料と採用コードのライセンスは詳細設計時に追加する。


### 現在のAPI / xHCI実装

[RustアプリAPI](app-api.md)と[xHCI初期化の第1段階](xhci.md)を実装済み。
APIテーブルを持つC contextと全I/O APIは今後追加する。
ACPI原本の保存契約は将来AML / MCFG利用時に拡張し、現在はPM timer情報のみ渡す。

### 将来のアプリ分離

0.1.0では実行環境と信頼された診断アプリをロングモードのRing 0で実行する。[将来設計](future-design.md)では、外部提供アプリにRing 3を標準とし、独立アドレス空間、検証付きsystem call、障害の封じ込め、強制可能なCPU時間制限を導入する。0.1.0の実装範囲には含めない。
