# Musha-OS experimental C SDK

## English

### Stage4 r2 runtime checkpoint —2026-10-15

Experimental r2 now connects real TIME/WAIT/INPUT and relative USB boot mouse to C and Rust guests. r1 values/NOTE acceptance and the default C build remain supported. No public ABI compatibility is frozen. See the [r2 evidence](../docs/stage4-r2-0.4.0-progress.md) and [ABI ledger](../docs/application-abi.md); full regression and real NUC5/Ryzen acceptance remain open.

C: `musha_time(&ms)`, `musha_wait(timeout_ms, flags, &reason)`, `musha_input(&event)`. Rust guest wrappers: `time()`, `wait(timeout_ms, input)`, `input()` return owned values. TIME permission2 returns monotonic sampled PM-epoch boot milliseconds; WAIT permission4 supports0–60000ms, flags0 or WAIT_INPUT1 (also requires INPUT permission8), returns0 timeout/poll or1 pending input. INPUT copies one48-byte LE record, returns ERR_AGAIN when empty; insufficient capacity is ERR_LIMIT, invalid/unwritable ranges ERR_INVALID, missing permissions ERR_DENIED. Errors return zero RDX and leave the queue/output unchanged. Reserved syscall arguments/record bytes must be zero. Timestamp precision depends on cooperative sampling; long unsampled intervals beyond a PM counter wrap cannot be reconstructed.

Only r2 with INPUT permission reopens USB after the old runtime releases its arena borrow. Hosts/rings remain on the OS stack. A saved guest continuation is inside the synchronous USB callback; detach/halt/BME-off precedes normal callback and session cleanup. Only confirmed cleanup permits mapping rollback, arena scrub and a fresh viewer. Failed stop quarantines memory without reuse. WAIT polls a bounded USB step. USB hub/control work may delay a deadline by one bounded poll (control requests have finite1s deadlines); WAIT is not a hard-real-time deadline. Stalled-clock watchdog counts consecutive nonadvancing samples, not all healthy iterations.

Keyboard down/up includes HID usage/modifiers. Mouse-only sessions work without a keyboard. Relative X/Y/buttons use boot format; wheel is enabled only for the validated signed8 relative descriptor subset, otherwise boot mode supplies wheel0. Absolute usb-tablet is not converted. The current live keyboard/mouse Host (or mouse-only Host) is serviced; simultaneous mouse input on a separate third controller is not integrated. Overflow preserves64 older FIFO records then reports a saturated dropped count; consumers must reset held-state assumptions because lost clicks/motion are not reconstructed. Viewer mouse UI is excluded, gamepad production/Bluetooth is stage6, viewer return is implemented locally in stage5; physical acceptance remains open.

Build standalone C r2 with `python3 tools/check-sdk.py --c-only --abi-revision 2 --source tests/io-c.c --output build` (NOTE permissions default15; --permissions can restrict them). Rust: `python3 tools/build-rust.py --r2 --output build`. These use generated NOTE/types and the existing ELF checker. Rust uses pinned1.99.0/rust-src, custom static SSE2/no-red-zone target and rebuilt core/compiler_builtins; Bootstrap enables build-std/JSON target only. Never execute guest ELFs on the host. Separate SDK archives remain development-qualified; the Rust release gate stays closed pending remaining stage4/full acceptance.


Documentation: [Writing a C program](../docs/sdk-guide.md).

Build the C Hello with Python 3, Clang and LLD: `python3 tools/check-sdk.py --c-only --sdk . --output build` from the extracted archive. In the source repository use `python3 ../tools/check-sdk.py --c-only --sdk . --output build` from sdk/. Optional CMake: `cmake -S . -B build-cmake` then `cmake --build build-cmake`.

The archive includes local `docs/sdk-guide.md`, `docs/sdk.md`, `docs/external-app.md` and `docs/application-abi.md`; in the repository these documents are in ../docs/. They describe required tools, AUTORUN.ELF placement, 64-byte–16MiB/5-second bounds and reboot after exit/fault. Do not execute the guest ELF on the host. ABI candidate r1 is experimental and may change. The C archive excludes Rust; the separate development Rust SDK now builds runnable r1 samples as described above. Mac external applications are currently unsupported due to unrecognized storage. Candidate hardware acceptance is pending.

## 日本語

### 工程4 r2実接続到達点 —2026-10-15

実験r2のTIME/WAIT/INPUTと相対USB boot mouseをC/Rust guestへ実接続しました。r1番号/NOTE受入と既定C buildは保持し、公開互換性は固定しません。[r2証拠](../docs/stage4-r2-0.4.0-progress.md)と[ABI台帳](../docs/application-abi.md)参照。全回帰・NUC5/Ryzen実機受入は未完です。

Cはmusha_time(&ms)、musha_wait(timeout_ms,flags,&reason)、musha_input(&event)。Rust guestのtime()/wait(timeout_ms,input)/input()は所有値を返します。TIME権限2で単調なsample済みPM起動epoch ms、WAIT権限4で0–60000ms、flags0またはWAIT_INPUT1（入力権限8も必要）、結果0はtimeout/poll、1はpending入力です。INPUTは48byte LE record1個をコピー。空ERR_AGAIN、容量不足ERR_LIMIT、異常/非書込み範囲ERR_INVALID、権限無しERR_DENIED。エラー時RDX0でqueue/outputを変更しません。予約引数/record bytesはゼロ。時刻精度は協調sampleに依存し、PM counter周回より長くsampleしない経過時間は復元できません。

r2かつINPUT権限の場合だけ、旧runtimeのarena借用終了後にUSBを再開します。Host/ringはOS stack上。同期USB callback内でguest保存継続を囲み、detach/halt/BME停止確認後にcallbackとsession cleanupを通常returnします。確認済みcleanup後だけmapping復元・arena消去・新viewerへ進み、停止未確認は再利用せず隔離。WAITは上限付きUSB1stepをpollします。hub/control処理により期限が上限付き1poll分（control各1秒期限）遅れることがあり、hard realtime保証ではありません。clock watchdogは全反復数でなく時刻非進行の連続sample数を数えます。

keyboard down/upはHID usage/modifier付き。keyboard無しmouse単独sessionも動作します。相対XY/buttonはboot形式、wheelは検証したsigned8相対descriptor部分形式だけ有効で、それ以外のboot modeはwheel0。絶対usb-tabletは変換しません。現live keyboard/mouse Hostまたはmouse単独Hostを処理し、別の第3controllerの同時mouse収集は未統合。overflowは古い64件を保持後、飽和drop数を通知します。失われたclick/移動は復元できないためconsumerは保持状態の前提をリセットします。viewer mouse UIは対象外、gamepad producer/Bluetoothは工程6、viewer復帰は工程5でローカル実装済み、実機受入は未確認です。

単独C r2 buildは `python3 tools/check-sdk.py --c-only --abi-revision 2 --source tests/io-c.c --output build`（NOTE権限は既定15、--permissionsで制限）。Rustは `python3 tools/build-rust.py --r2 --output build`。生成NOTE/型と既存ELF checkerを使います。Rustは固定1.99.0/rust-src、専用static SSE2/red-zone無しtarget、再構築core/compiler_builtinsで、Bootstrapはbuild-std/JSON target用だけ。guest ELFをhost実行しません。SDK archiveは分離した開発版で、Rust公開gateは工程4残件/全受入まで閉じています。


Documentation: [Cプログラムの作成ガイド](../docs/sdk-guide.md)。

展開ZIPでは`python3 tools/check-sdk.py --c-only --sdk . --output build`でC Helloを生成します。リポジトリのsdk/内では`../tools/check-sdk.py`を指定します。Python 3、Clang、LLDが必要です。任意のCMake手順は上記を参照してください。

ZIP内のdocs/（リポジトリでは../docs/）にSDK、外部アプリ、ABIの手順を同梱します。AUTORUN.ELF配置、64バイト～16MiB／5秒期限、終了・障害後の再起動を説明しています。ホストでゲストELFを実行しないでください。ABIは変更可能な実験候補r1です。C ZIPへRustは含めません。別の開発Rust SDKは上記のr1実行sampleを生成します。Macはストレージ未認識のため外部アプリ未対応、公開候補の実機受入は未完了です。
