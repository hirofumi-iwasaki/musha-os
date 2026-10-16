# Writing a C program for Musha-OS 0.3.0

Status: current

## English

### Stage4 r2 runtime checkpoint —2026-10-15

Experimental r2 now connects real TIME/WAIT/INPUT and relative USB boot mouse to C and Rust guests. r1 values/NOTE acceptance and the default C build remain supported. No public ABI compatibility is frozen. See the [r2 evidence](stage4-r2-0.4.0-progress.md) and [ABI ledger](application-abi.md); full regression and real NUC5/Ryzen acceptance remain open.

C: `musha_time(&ms)`, `musha_wait(timeout_ms, flags, &reason)`, `musha_input(&event)`. Rust guest wrappers: `time()`, `wait(timeout_ms, input)`, `input()` return owned values. TIME permission2 returns monotonic sampled PM-epoch boot milliseconds; WAIT permission4 supports0–60000ms, flags0 or WAIT_INPUT1 (also requires INPUT permission8), returns0 timeout/poll or1 pending input. INPUT copies one48-byte LE record, returns ERR_AGAIN when empty; insufficient capacity is ERR_LIMIT, invalid/unwritable ranges ERR_INVALID, missing permissions ERR_DENIED. Errors return zero RDX and leave the queue/output unchanged. Reserved syscall arguments/record bytes must be zero. Timestamp precision depends on cooperative sampling; long unsampled intervals beyond a PM counter wrap cannot be reconstructed.

Only r2 with INPUT permission reopens USB after the old runtime releases its arena borrow. Hosts/rings remain on the OS stack. A saved guest continuation is inside the synchronous USB callback; detach/halt/BME-off precedes normal callback and session cleanup. Only confirmed cleanup permits mapping rollback, arena scrub and a fresh viewer. Failed stop quarantines memory without reuse. WAIT polls a bounded USB step. USB hub/control work may delay a deadline by one bounded poll (control requests have finite1s deadlines); WAIT is not a hard-real-time deadline. Stalled-clock watchdog counts consecutive nonadvancing samples, not all healthy iterations.

Keyboard down/up includes HID usage/modifiers. Mouse-only sessions work without a keyboard. Relative X/Y/buttons use boot format; wheel is enabled only for the validated signed8 relative descriptor subset, otherwise boot mode supplies wheel0. Absolute usb-tablet is not converted. The current live keyboard/mouse Host (or mouse-only Host) is serviced; simultaneous mouse input on a separate third controller is not integrated. Overflow preserves64 older FIFO records then reports a saturated dropped count; consumers must reset held-state assumptions because lost clicks/motion are not reconstructed. Viewer mouse UI is excluded, gamepad production/Bluetooth is stage6, viewer return is implemented locally in stage5; physical acceptance remains open.

Build standalone C r2 with `python3 tools/check-sdk.py --c-only --abi-revision 2 --source tests/io-c.c --output build` (NOTE permissions default15; --permissions can restrict them). Rust: `python3 tools/build-rust.py --r2 --output build`. These use generated NOTE/types and the existing ELF checker. Rust uses pinned1.99.0/rust-src, custom static SSE2/no-red-zone target and rebuilt core/compiler_builtins; Bootstrap enables build-std/JSON target only. Never execute guest ELFs on the host. Separate SDK archives remain development-qualified; the Rust release gate stays closed pending remaining stage4/full acceptance.


Stage1 v0.4.0 development: this C/r1 behavior is retained; Rust r1/r2 execution is covered above; viewer launch/return remains stage5. The drive now has DOCS and empty SETTINGS and README is the startup marker. See [distribution usage](distribution-usage.md).

This guide is for people who want to write their own program, not only build the sample. Tool commands are in the [SDK reference](sdk.md); how the OS loads and runs the file is in [running an application](external-app.md); the exact numbers are in the [ABI ledger](application-abi.md). The ABI is **experimental candidate r1** and may change in a later version.

### 1. What a program can do in 0.3.0

For the current v0.4.0 console, required USB-host/boot-media/keyboard discovery → `Musha-OS 0.4.0 ready` → application start → raw `musha_log` output → exit/fault are consecutive output in the same console. Ready level also shows app start/exit/fault/rejection and app bytes. Startup verbosity controls OS events: devices is concise per-device decisions, trace adds transfer/stage/timing detail, and F8/8 selects trace. The ring retains all detail; these display changes do not change the syscall ABI or MAX_LOG_BYTES (4096). The version-specific 0.3.0 behavior below is historical where lifecycle differs.

A program runs at Ring 3 with its own stack and a private memory arena. It can call exactly three operations:

| Call | Does | Returns |
|---|---|---|
| `musha_query(uint64_t *features)` | Reports the feature mask (`MUSHA_FEATURE_BASE` = 1 in r1) | `MUSHA_OK` or an error |
| `musha_log(const void *bytes, uint64_t length, uint64_t *written)` | Appends UTF-8 text to the [shared full-GOP boot console](boot-log.md). Printable ASCII and newline are rendered; other bytes appear as `.`. The syscall ABI and 4096-byte limit are unchanged | `MUSHA_OK` with `*written` = bytes accepted, or an error |
| `musha_exit(int32_t status)` | Ends the program. The screen shows `AUTORUN.ELF exited: status <n>`; the machine halts until reboot | never returns |

There is no libc: no `printf`, `malloc`, file, keyboard, timer, network or drawing call. Output is `musha_log` only. The SDK supplies `memcpy`, `memset`, `memmove` and `memcmp` because the compiler may emit calls to them.

Status values are `int32_t`: 0 on success, negative on error (`MUSHA_ERR_INVALID` -3, `MUSHA_ERR_DENIED` -4, `MUSHA_ERR_LIMIT` -5, and the others in `musha/abi.h`). `musha_log` rejects a null pointer with nonzero length and any length above `MUSHA_MAX_LOG_BYTES` (4096) before reaching the kernel.

### 2. Entry point and start information

The SDK's `crt0` calls `int musha_main(const musha_start_info_v1 *info)` and passes its return value to `musha_exit`. Returning 0 is the normal end. `info` is a read-only 64-byte record:

| Field | Meaning |
|---|---|
| `struct_size` | 64 in r1; check it before using other fields |
| `abi_major`, `abi_minor` | 0 and 1 in r1 |
| `arena_base`, `arena_bytes` | A zero-filled, writable, non-executable region you may use freely (1 MiB in the sample build, at virtual address 0x10000000) |
| `stack_base`, `stack_bytes` | Your stack (64 KiB in the sample build, below 0x70000000, with an unmapped guard page beneath it) |
| `boot_report_handle` | 0 in r1 (not available) |

Static and global variables work: `.data` is loaded from the file and `.bss` is zeroed by the kernel. The arena is the place for larger buffers; writing outside the arena, stack or `.data`/`.bss` faults.

### 3. Memory and CPU rules

- Image: up to 16 MiB file, loaded at 0x400000. Code is read-only and executable; data is writable and non-executable. The linker script `linker/app.ld` lays this out; do not change section permissions.
- Stack: 64 KiB. Do not place large arrays on the stack; use the arena or static storage.
- Floating point: x87 and SSE2 are available; AVX is not. The compiler flags already select `-march=x86-64 -mno-avx`.
- No red zone (`-mno-red-zone`), no stack protector, no PIC. Keep the SDK flags.
- Privileged instructions, port I/O and access to kernel memory fault. A fault shows `AUTORUN.ELF faulted` with the vector, error code, RIP and RSP, then halts.
- Infinite loops are not interrupted: the program owns the CPU until it calls `musha_exit`. Reboot to recover.

### 4. The NOTE: what the file declares

Every ELF carries a `MUSHA` note (built by `check-sdk.py`) that declares the ABI revision, requested permissions (`MUSHA_PERMISSION_LOG`), arena size and stack size. The kernel refuses files whose note is missing, duplicated, from another revision, or asks for more than the limits (arena up to 64 MiB, stack up to 1 MiB). The sample build requests the log permission, a 1 MiB arena and a 64 KiB stack. A program without the log permission gets `MUSHA_ERR_DENIED` from `musha_log`.

### 5. Build, place, run

1. Install Python 3, Clang with x86-64 ELF support and LLVM LLD.
   - macOS: Xcode command-line tools give `clang`; `brew install llvm` gives `ld.lld` (set `LD_LLD=/opt/homebrew/opt/llvm/bin/ld.lld`).
   - Linux: `apt install clang lld` (or the distribution equivalent).
   - Windows: the LLVM release installer provides both; run the commands from a prompt where `clang` and `ld.lld` are on `PATH`.
2. Edit `examples/hello-c/main.c` (or point the checker at your own file by replacing it; the sample build compiles that path).
3. `python3 tools/check-sdk.py --c-only --sdk . --output build`
   The checker compiles, links, verifies the ELF layout and note, and records the SHA-256 in `build/report.json`.
4. `python3 tools/app-check.py build/hello-c.elf` — the same validation the kernel performs, on the host.
5. Copy `build/hello-c.elf` to the root of the USB drive as `AUTORUN.ELF` and boot the drive. Without the file, the file viewer starts instead.

Do not run the ELF on the host: it has no OS entry point and uses `syscall` directly.

### 6. A second example: show the start record

```c
#include <musha/api.h>

static void put(const char *s) { uint64_t n = 0; while (s[n]) n++; uint64_t w; musha_log(s, n, &w); }
static void hex(uint64_t v) {
    char buf[] = "0x0000000000000000\n";
    for (int i = 17; i >= 2; i--) { buf[i] = "0123456789ABCDEF"[v & 15]; v >>= 4; }
    uint64_t w; musha_log(buf, sizeof(buf) - 1, &w);
}
int musha_main(const musha_start_info_v1 *info) {
    if (!info || info->struct_size != MUSHA_START_SIZE) return 1;
    uint64_t features = 0;
    if (musha_query(&features) != MUSHA_OK) return 2;
    put("arena base  "); hex(info->arena_base);
    put("arena bytes "); hex(info->arena_bytes);
    put("stack bytes "); hex(info->stack_bytes);
    unsigned char *arena = (unsigned char *)(uintptr_t)info->arena_base;
    arena[0] = 42;                       /* writable */
    return arena[0] == 42 ? 0 : 3;
}
```

Replace `examples/hello-c/main.c` with this and rebuild. Expected screen: three hexadecimal lines, then `AUTORUN.ELF exited: status 0`.

Validated on 2026-10-14 by temporarily replacing the actual sample, running `check-sdk.py --c-only` and `app-check.py`, and executing it in QEMU with the normal EFI. The three values were `0x0000000010000000`, `0x0000000000100000`, and `0x0000000000010000`, followed by exit status 0. The original Hello sample was restored. Local evidence: `out/sdk-guide-validation/report.json` and `out/sdk-guide-validation/qemu/hello-fat32/screen.txt`. This is QEMU validation, not hardware acceptance.

### 7. When it does not work

| Screen | Cause | Fix |
|---|---|---|
| `AUTORUN.ELF not found in root. Starting file viewer.` | Wrong name or not in the root directory | Name must be `AUTORUN.ELF` (case-insensitive), in the root of the FAT32/exFAT drive |
| `rejected: File size outside 64 bytes to 16 MiB` | Empty or oversized file | Check the build output |
| `rejected: Ambiguous AUTORUN.ELF name` | Two root entries match (for example `autorun.elf` and `AUTORUN.ELF`) | Keep one |
| `rejected: File read deadline exceeded` | Read took more than 5 seconds | Large files may not fit the deadline; keep programs small (the sample is 13 KB) |
| `rejected: Invalid ELF or ABI NOTE` | Built with other flags, another linker script, or a different ABI revision | Rebuild with the SDK checker; run `app-check.py` first |
| `rejected: ELF_MEMORY` / `ELF_COLLISION` | Requested arena or stack does not fit | Reduce the note's arena/stack request |
| `AUTORUN.ELF faulted` | Page fault, privileged instruction or bad pointer in your code | RIP tells where; check pointers against the arena and stack ranges |
| `musha_log` returns `MUSHA_ERR_DENIED` | Note lacks the log permission | Use the SDK's note generation |
| `No medium` or viewer without your file | The OS did not read the drive | See [hardware status](https://github.com/hirofumi-iwasaki/musha-os/blob/main/docs/hardware-verification-status.md); MacBook Pro 2018 is unsupported in 0.3.0 |

### 8. Not available in 0.3.0

r2 input, files, timers, drawing commands, networking, returning to the viewer without reboot, more than one program per boot, and any ABI stability promise.

## 日本語

現行v0.4.0 consoleでは必須USB host/起動媒体/keyboard探索→`Musha-OS 0.4.0 ready`→app開始→生の`musha_log`出力→終了/faultを同じconsoleへ順に表示します。readyでもapp開始/終了/fault/拒否と生byte出力を表示。起動levelはOS eventを選別し、devicesは簡潔な機器判断、traceは詳細transfer/stage/timing、F8/8はtraceを選択。ringは全詳細を保持し、syscall ABI・MAX_LOG_BYTES（4096）は変えません。以下の0.3.0固有動作はlifecycleが異なる箇所では過去の説明です。

### 工程4 r2実接続到達点 —2026-10-15

実験r2のTIME/WAIT/INPUTと相対USB boot mouseをC/Rust guestへ実接続しました。r1番号/NOTE受入と既定C buildは保持し、公開互換性は固定しません。[r2証拠](stage4-r2-0.4.0-progress.md)と[ABI台帳](application-abi.md)参照。全回帰・NUC5/Ryzen実機受入は未完です。

Cはmusha_time(&ms)、musha_wait(timeout_ms,flags,&reason)、musha_input(&event)。Rust guestのtime()/wait(timeout_ms,input)/input()は所有値を返します。TIME権限2で単調なsample済みPM起動epoch ms、WAIT権限4で0–60000ms、flags0またはWAIT_INPUT1（入力権限8も必要）、結果0はtimeout/poll、1はpending入力です。INPUTは48byte LE record1個をコピー。空ERR_AGAIN、容量不足ERR_LIMIT、異常/非書込み範囲ERR_INVALID、権限無しERR_DENIED。エラー時RDX0でqueue/outputを変更しません。予約引数/record bytesはゼロ。時刻精度は協調sampleに依存し、PM counter周回より長くsampleしない経過時間は復元できません。

r2かつINPUT権限の場合だけ、旧runtimeのarena借用終了後にUSBを再開します。Host/ringはOS stack上。同期USB callback内でguest保存継続を囲み、detach/halt/BME停止確認後にcallbackとsession cleanupを通常returnします。確認済みcleanup後だけmapping復元・arena消去・新viewerへ進み、停止未確認は再利用せず隔離。WAITは上限付きUSB1stepをpollします。hub/control処理により期限が上限付き1poll分（control各1秒期限）遅れることがあり、hard realtime保証ではありません。clock watchdogは全反復数でなく時刻非進行の連続sample数を数えます。

keyboard down/upはHID usage/modifier付き。keyboard無しmouse単独sessionも動作します。相対XY/buttonはboot形式、wheelは検証したsigned8相対descriptor部分形式だけ有効で、それ以外のboot modeはwheel0。絶対usb-tabletは変換しません。現live keyboard/mouse Hostまたはmouse単独Hostを処理し、別の第3controllerの同時mouse収集は未統合。overflowは古い64件を保持後、飽和drop数を通知します。失われたclick/移動は復元できないためconsumerは保持状態の前提をリセットします。viewer mouse UIは対象外、gamepad producer/Bluetoothは工程6、viewer復帰は工程5でローカル実装済み、実機受入は未確認です。

単独C r2 buildは `python3 tools/check-sdk.py --c-only --abi-revision 2 --source tests/io-c.c --output build`（NOTE権限は既定15、--permissionsで制限）。Rustは `python3 tools/build-rust.py --r2 --output build`。生成NOTE/型と既存ELF checkerを使います。Rustは固定1.99.0/rust-src、専用static SSE2/red-zone無しtarget、再構築core/compiler_builtinsで、Bootstrapはbuild-std/JSON target用だけ。guest ELFをhost実行しません。SDK archiveは分離した開発版で、Rust公開gateは工程4残件/全受入まで閉じています。


v0.4.0工程1：以下のC/r1動作を保持し、Rust r1/r2実行は上記工程4に到達、viewer起動/復帰は工程5。媒体はDOCS/空SETTINGS/README起動markerへ変更。[配布利用](distribution-usage.md)参照。

本書はサンプルをビルドするだけでなく、自分のプログラムを書きたい人向けの案内です。ツールのコマンドは[SDK リファレンス](sdk.md)、OS がファイルを読み込んで実行する手順は[アプリの実行](external-app.md)、正確な数値は[ABI 台帳](application-abi.md)にあります。ABI は**実験候補 r1** で、後続の版で変わる可能性があります。

### 1. 0.3.0 でプログラムにできること

プログラムは Ring 3 で、専用のスタックと専用のメモリー領域（arena）を持って動きます。呼べる操作はちょうど 3 つです。

| 呼出し | 内容 | 戻り値 |
|---|---|---|
| `musha_query(uint64_t *features)` | 機能マスクを返す（r1 では `MUSHA_FEATURE_BASE` = 1） | `MUSHA_OK` またはエラー |
| `musha_log(const void *bytes, uint64_t length, uint64_t *written)` | UTF-8テキストを[共通のGOP全体boot console](boot-log.md)へ追記。ASCII印字可能文字と改行だけ描画し、他のバイトは `.`。syscall ABI・4096byte上限は不変 | `MUSHA_OK` と受理バイト数、またはエラー |
| `musha_exit(int32_t status)` | プログラムを終了。画面に `AUTORUN.ELF exited: status <n>` を表示し、再起動まで停止 | 戻らない |

libc はありません。`printf`・`malloc`・ファイル・キーボード・タイマー・通信・描画の呼出しはなく、出力は `musha_log` だけです。コンパイラーが呼ぶことがあるため、SDK は `memcpy`・`memset`・`memmove`・`memcmp` を用意しています。

状態値は `int32_t` で、成功は 0、エラーは負（`MUSHA_ERR_INVALID` -3、`MUSHA_ERR_DENIED` -4、`MUSHA_ERR_LIMIT` -5 ほか、`musha/abi.h` 参照）。`musha_log` は長さ付きの NULL ポインターと `MUSHA_MAX_LOG_BYTES`（4096）超の長さをカーネルに渡す前に拒否します。

### 2. エントリーポイントと起動情報

SDK の `crt0` が `int musha_main(const musha_start_info_v1 *info)` を呼び、戻り値を `musha_exit` に渡します。0 を返すのが正常終了です。`info` は読出し専用の 64 byte 構造体です。

| フィールド | 意味 |
|---|---|
| `struct_size` | r1 では 64。他のフィールドを使う前に確認する |
| `abi_major`, `abi_minor` | r1 では 0 と 1 |
| `arena_base`, `arena_bytes` | ゼロ初期化済み・書込み可・実行不可の自由領域（サンプルのビルドでは 1 MiB、仮想アドレス 0x10000000） |
| `stack_base`, `stack_bytes` | スタック（サンプルでは 64 KiB、0x70000000 の下。さらに下は未マップのガードページ） |
| `boot_report_handle` | r1 では 0（未提供） |

静的変数・グローバル変数は使えます。`.data` はファイルから読み込まれ、`.bss` はカーネルがゼロ初期化します。大きなバッファーは arena に置きます。arena・スタック・`.data`/`.bss` の外への書込みは例外になります。

### 3. メモリーと CPU の規則

- イメージ：ファイル最大 16 MiB、0x400000 に配置。コードは読出し専用・実行可、データは書込み可・実行不可。配置は `linker/app.ld` が決めるので、セクションの権限を変えないこと。
- スタック：64 KiB。大きな配列はスタックに置かず、arena か静的領域へ。
- 浮動小数点：x87 と SSE2 が使えます。AVX は使えません（SDK のフラグ `-march=x86-64 -mno-avx` が既に選んでいます）。
- red zone なし（`-mno-red-zone`）、スタックプロテクターなし、PIC なし。SDK のフラグを変えないこと。
- 特権命令・ポート I/O・カーネルメモリーへのアクセスは例外になります。例外時は `AUTORUN.ELF faulted` と vector・error code・RIP・RSP を表示して停止します。
- 無限ループは止められません。`musha_exit` を呼ぶまで CPU はプログラムのものです。復帰は再起動です。

### 4. NOTE：ファイルが宣言するもの

すべての ELF は `MUSHA` ノート（`check-sdk.py` が生成）を持ち、ABI 版・要求する権限（`MUSHA_PERMISSION_LOG`）・arena の大きさ・スタックの大きさを宣言します。ノートが無い・重複している・版が違う・上限（arena 64 MiB、スタック 1 MiB）を超える要求は拒否されます。サンプルのビルドはログ権限・arena 1 MiB・スタック 64 KiB を要求します。ログ権限の無いプログラムは `musha_log` で `MUSHA_ERR_DENIED` を受け取ります。

### 5. ビルド・配置・実行

1. Python 3、x86-64 ELF 対応の Clang、LLVM LLD を用意します。
   - macOS：Xcode コマンドラインツールの `clang` と、`brew install llvm` の `ld.lld`（`LD_LLD=/opt/homebrew/opt/llvm/bin/ld.lld` を指定）。
   - Linux：`apt install clang lld`（相当のパッケージ）。
   - Windows：LLVM のリリースインストーラーで両方入ります。`clang` と `ld.lld` が `PATH` にあるプロンプトで実行します。
2. `examples/hello-c/main.c` を編集します（自分のファイルで置き換えても構いません。サンプルのビルドはこのパスをコンパイルします）。
3. `python3 tools/check-sdk.py --c-only --sdk . --output build`
   コンパイル・リンク・ELF 配置とノートの検査を行い、SHA-256 を `build/report.json` に記録します。
4. `python3 tools/app-check.py build/hello-c.elf` — カーネルと同じ検査をホストで行います。
5. `build/hello-c.elf` を USB のルートに `AUTORUN.ELF` としてコピーし、USB から起動します。ファイルが無ければファイルビューアーが起動します。

ELF をホストで実行しないでください。OS 向けのエントリーポイントを持たず、`syscall` を直接使います。

### 6. 2 つ目の例：起動情報を表示する

```c
#include <musha/api.h>

static void put(const char *s) { uint64_t n = 0; while (s[n]) n++; uint64_t w; musha_log(s, n, &w); }
static void hex(uint64_t v) {
    char buf[] = "0x0000000000000000\n";
    for (int i = 17; i >= 2; i--) { buf[i] = "0123456789ABCDEF"[v & 15]; v >>= 4; }
    uint64_t w; musha_log(buf, sizeof(buf) - 1, &w);
}
int musha_main(const musha_start_info_v1 *info) {
    if (!info || info->struct_size != MUSHA_START_SIZE) return 1;
    uint64_t features = 0;
    if (musha_query(&features) != MUSHA_OK) return 2;
    put("arena base  "); hex(info->arena_base);
    put("arena bytes "); hex(info->arena_bytes);
    put("stack bytes "); hex(info->stack_bytes);
    unsigned char *arena = (unsigned char *)(uintptr_t)info->arena_base;
    arena[0] = 42;                       /* 書込み可 */
    return arena[0] == 42 ? 0 : 3;
}
```

`examples/hello-c/main.c` をこれに置き換えて再ビルドします。期待する画面：16 進数 3 行のあと `AUTORUN.ELF exited: status 0`。

2026-10-14、実際のサンプルを一時的に差し替え、`check-sdk.py --c-only` と `app-check.py`、通常版 EFI による QEMU 実行で検証しました。3 行の値は順に `0x0000000010000000`、`0x0000000000100000`、`0x0000000000010000`、終了状態は 0 でした。元の Hello サンプルは復元済みです。ローカル証拠：`out/sdk-guide-validation/report.json` と `out/sdk-guide-validation/qemu/hello-fat32/screen.txt`。これは QEMU 検証であり、実機受入ではありません。

### 7. うまくいかないとき

| 画面 | 原因 | 対処 |
|---|---|---|
| `AUTORUN.ELF not found in root. Starting file viewer.` | 名前が違う、またはルート直下にない | 名前は `AUTORUN.ELF`（大小文字は区別しない）、FAT32/exFAT のルート直下 |
| `rejected: File size outside 64 bytes to 16 MiB` | 空ファイルか大きすぎる | ビルド結果を確認 |
| `rejected: Ambiguous AUTORUN.ELF name` | ルートに一致する名前が 2 つある（`autorun.elf` と `AUTORUN.ELF` など） | 1 つにする |
| `rejected: File read deadline exceeded` | 読込みが 5 秒を超えた | 大きなファイルは期限に収まらないことがある。小さく保つ（サンプルは 13 KB） |
| `rejected: Invalid ELF or ABI NOTE` | 別のフラグ・別のリンカースクリプト・別の ABI 版でビルドした | SDK の検査ツールで再ビルドし、先に `app-check.py` を通す |
| `rejected: ELF_MEMORY` / `ELF_COLLISION` | 要求した arena かスタックが収まらない | ノートの要求量を減らす |
| `AUTORUN.ELF faulted` | ページフォルト・特権命令・不正ポインター | RIP が場所を示す。ポインターを arena・スタックの範囲と照合 |
| `musha_log` が `MUSHA_ERR_DENIED` | ノートにログ権限がない | SDK のノート生成を使う |
| `No medium` やビューアーに自分のファイルが出ない | OS が USB を読めていない | [実機確認状況](https://github.com/hirofumi-iwasaki/musha-os/blob/main/docs/hardware-verification-status.md)を参照。MacBook Pro 2018 は 0.3.0 では未対応 |

### 8. 0.3.0 で使えないもの

r2入力、ファイル、タイマー、描画命令、通信、再起動なしでのビューアー復帰、1 起動に 2 つ以上のプログラム、ABI の安定性の保証。
