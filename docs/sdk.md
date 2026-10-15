# C SDK for Musha-OS 0.3.0 candidates

## English

### Stage4 r2 runtime checkpoint —2026-10-15

Experimental r2 now connects real TIME/WAIT/INPUT and relative USB boot mouse to C and Rust guests. r1 values/NOTE acceptance and the default C build remain supported. No public ABI compatibility is frozen. See the [r2 evidence](stage4-r2-0.4.0-progress.md) and [ABI ledger](application-abi.md); full regression and real NUC5/Ryzen acceptance remain open.

C: `musha_time(&ms)`, `musha_wait(timeout_ms, flags, &reason)`, `musha_input(&event)`. Rust guest wrappers: `time()`, `wait(timeout_ms, input)`, `input()` return owned values. TIME permission2 returns monotonic sampled PM-epoch boot milliseconds; WAIT permission4 supports0–60000ms, flags0 or WAIT_INPUT1 (also requires INPUT permission8), returns0 timeout/poll or1 pending input. INPUT copies one48-byte LE record, returns ERR_AGAIN when empty; insufficient capacity is ERR_LIMIT, invalid/unwritable ranges ERR_INVALID, missing permissions ERR_DENIED. Errors return zero RDX and leave the queue/output unchanged. Reserved syscall arguments/record bytes must be zero. Timestamp precision depends on cooperative sampling; long unsampled intervals beyond a PM counter wrap cannot be reconstructed.

Only r2 with INPUT permission reopens USB after the old runtime releases its arena borrow. Hosts/rings remain on the OS stack, callbacks are synchronous and never escape the non-returning guest path. WAIT polls a bounded USB step; halt/BME-off runs on exit/fault/reject and poll failure. Failed stop quarantines permanent memory; no viewer return/reuse is added. USB hub/control work may delay a deadline by one bounded poll (control requests have finite1s deadlines); WAIT is not a hard-real-time deadline. Stalled-clock watchdog counts consecutive nonadvancing samples, not all healthy iterations.

Keyboard down/up includes HID usage/modifiers. Mouse-only sessions work without a keyboard. Relative X/Y/buttons use boot format; wheel is enabled only for the validated signed8 relative descriptor subset, otherwise boot mode supplies wheel0. Absolute usb-tablet is not converted. The current live keyboard/mouse Host (or mouse-only Host) is serviced; simultaneous mouse input on a separate third controller is not integrated. Overflow preserves64 older FIFO records then reports a saturated dropped count; consumers must reset held-state assumptions because lost clicks/motion are not reconstructed. Viewer mouse UI is excluded, gamepad production/Bluetooth is stage6, viewer return stage5.

Build standalone C r2 with `python3 tools/check-sdk.py --c-only --abi-revision 2 --source tests/io-c.c --output build` (NOTE permissions default15; --permissions can restrict them). Rust: `python3 tools/build-rust.py --r2 --output build`. These use generated NOTE/types and the existing ELF checker. Rust uses pinned1.99.0/rust-src, custom static SSE2/no-red-zone target and rebuilt core/compiler_builtins; Bootstrap enables build-std/JSON target only. Never execute guest ELFs on the host. Separate SDK archives remain development-qualified; the Rust release gate stays closed pending remaining stage4/full acceptance.


Start with the [C programming guide](sdk-guide.md) to write your own program.

The separate `musha-os-vVERSION-x64-c-sdk.zip` contains C sources, crt0, minimal libmusha/memory routines, linker script, headers, ABI schema, ELF checker, tests, C Hello and a CMake entry point. Extract into any directory, including a path containing spaces. It has no dependency on the OS source checkout. The ABI is **experimental candidate r1**, not frozen compatibility. Rust is excluded from this C archive; its separate development SDK is described above. No libc/POSIX or practical application I/O is supplied.

Required: Python 3, Clang with x86-64 ELF support, and LLVM LLD (`ld.lld` or `rust-lld`). CMake 3.20+ is optional. `CC` selects Clang; `LD_LLD` selects LLD. With a standalone LLD, Rust is not required. If LLD is omitted, the checker can use the installed Rust toolchain's LLD (repository validation uses pinned Rust 1.99.0). Record the exact versions in `build/report.json`; Hello hashes are reproducible for the same compiler/linker versions and options, not promised across toolchain versions.

From the extracted SDK root:

```sh
export CC=clang
export LD_LLD=/absolute/path/to/ld.lld
python3 tools/check-sdk.py --c-only --sdk . --output "build result"
python3 tools/app-check.py "build result/hello-c.elf"
```

The checker compiles and links the freestanding sample, executes only a native host layout test, rejects malformed ELF fixtures and records `hello_sha256`. It never executes guest syscalls on the host. Copy `build result/hello-c.elf` to the selected USB filesystem root as **AUTORUN.ELF**. See [running an application](external-app.md) for boot, read limits and failure handling. Do not execute the ELF on macOS/Linux/Windows.

Optional CMake uses the same canonical checked build, so it produces the same ELF:

```sh
cmake -S . -B "cmake build"
cmake --build "cmake build"
# output: cmake build/checked/hello-c.elf
```

To modify Hello, edit `examples/hello-c/main.c` and repeat. The linker emits a static, non-PIC x86-64 ELF with the MUSHA NOTE; baseline SSE2, no AVX, no red zone and no host libc/crt. The exposed operations remain query, log and exit. The kernel validates the file independently.

Inside the OS repository the equivalent command is `python3 tools/check-sdk.py --c-only --sdk sdk --output out/sdk-c`. Omitting `--c-only` additionally performs repository Rust compile-only checks; it is not a Rust guest build. [ABI ledger](application-abi.md), [validation](boot-api-validation.md).

NUC5/Ryzen development builds have owner-reported Hello success. **Mac external applications are currently unsupported because the development build does not recognize its storage.** Candidate cold-boot acceptance is pending. Published v0.2.0 assets remain unchanged.

## 日本語

### 工程4 r2実接続到達点 —2026-10-15

実験r2のTIME/WAIT/INPUTと相対USB boot mouseをC/Rust guestへ実接続しました。r1番号/NOTE受入と既定C buildは保持し、公開互換性は固定しません。[r2証拠](stage4-r2-0.4.0-progress.md)と[ABI台帳](application-abi.md)参照。全回帰・NUC5/Ryzen実機受入は未完です。

Cはmusha_time(&ms)、musha_wait(timeout_ms,flags,&reason)、musha_input(&event)。Rust guestのtime()/wait(timeout_ms,input)/input()は所有値を返します。TIME権限2で単調なsample済みPM起動epoch ms、WAIT権限4で0–60000ms、flags0またはWAIT_INPUT1（入力権限8も必要）、結果0はtimeout/poll、1はpending入力です。INPUTは48byte LE record1個をコピー。空ERR_AGAIN、容量不足ERR_LIMIT、異常/非書込み範囲ERR_INVALID、権限無しERR_DENIED。エラー時RDX0でqueue/outputを変更しません。予約引数/record bytesはゼロ。時刻精度は協調sampleに依存し、PM counter周回より長くsampleしない経過時間は復元できません。

r2かつINPUT権限の場合だけ、旧runtimeのarena借用終了後にUSBを再開します。Host/ringはOS stack上に保持し、同期callbackを非復帰guest経路の外へ持ち出しません。WAITは上限付きUSB1stepをpollし、exit/fault/reject/poll失敗時にhalt/BME停止。停止未確認でも永続メモリを隔離し、viewer復帰や再利用は追加しません。hub/control処理により期限が上限付き1poll分（control各1秒期限）遅れることがあり、hard realtime保証ではありません。clock watchdogは全反復数でなく時刻非進行の連続sample数を数えます。

keyboard down/upはHID usage/modifier付き。keyboard無しmouse単独sessionも動作します。相対XY/buttonはboot形式、wheelは検証したsigned8相対descriptor部分形式だけ有効で、それ以外のboot modeはwheel0。絶対usb-tabletは変換しません。現live keyboard/mouse Hostまたはmouse単独Hostを処理し、別の第3controllerの同時mouse収集は未統合。overflowは古い64件を保持後、飽和drop数を通知します。失われたclick/移動は復元できないためconsumerは保持状態の前提をリセットします。viewer mouse UIは対象外、gamepad producer/Bluetoothは工程6、viewer復帰は工程5です。

単独C r2 buildは `python3 tools/check-sdk.py --c-only --abi-revision 2 --source tests/io-c.c --output build`（NOTE権限は既定15、--permissionsで制限）。Rustは `python3 tools/build-rust.py --r2 --output build`。生成NOTE/型と既存ELF checkerを使います。Rustは固定1.99.0/rust-src、専用static SSE2/red-zone無しtarget、再構築core/compiler_builtinsで、Bootstrapはbuild-std/JSON target用だけ。guest ELFをhost実行しません。SDK archiveは分離した開発版で、Rust公開gateは工程4残件/全受入まで閉じています。


自作プログラムの書き方は[Cプログラミングガイド](sdk-guide.md)を参照してください。

独立配布の`musha-os-vVERSION-x64-c-sdk.zip`にCソース、crt0、libmusha／メモリ処理、リンカースクリプト、ヘッダー、ABIスキーマ、ELF検査、試験、Hello、CMake入口を含めます。空白入りを含む任意の場所へ展開でき、OSリポジトリを必要としません。ABIは**実験候補r1**で変更の可能性があります。Rustはリポジトリ内の**compile-only**検査のみで、ZIPには同梱しません。libc／POSIXやアプリ向け実用I/Oはありません。

必要ツールはPython 3、x86-64 ELF対応Clang、LLVM LLDです。任意でCMake 3.20以上を使えます。`CC`と`LD_LLD`でツールを指定します。独立LLDならRust不要、未指定なら導入済みRustのLLDも利用可能です。リポジトリ検証はRust 1.99.0付属LLDを使います。具体的な版・コマンド・Hello SHA-256は`report.json`へ記録します。ハッシュ再現性は同じツール版・オプションで確認し、異なる版での一致は保証しません。

展開したSDKのルートで上記の`check-sdk.py --c-only`と`app-check.py`を実行してください。ホストで実行するのはネイティブの型配置検査のみです。生成された`hello-c.elf`を**AUTORUN.ELF**としてUSBの選択対象ファイルシステムのルートへ配置します。ゲストELFをmacOS等で直接実行しないでください。CMakeも同じ検証ビルドを呼び、`cmake build/checked/hello-c.elf`を生成します。サンプル変更は`examples/hello-c/main.c`を編集して再実行します。

実行方法・64バイト～16MiB・5秒期限・画面出力・終了／例外・再起動による復帰は[外部アプリ手順](external-app.md)を参照してください。NUC5／Ryzenの開発版Hello成功は報告済みですが公開候補の冷間起動確認は残件です。**Macは開発版ストレージ未認識のため外部アプリ未対応**。公開済みv0.2.0配布物は不変です。
