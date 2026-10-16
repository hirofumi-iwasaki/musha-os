# Running an external C application (development preview)

Status: current

## English

### Stage4 r2 runtime checkpoint —2026-10-15

Experimental r2 now connects real TIME/WAIT/INPUT and relative USB boot mouse to C and Rust guests. r1 values/NOTE acceptance and the default C build remain supported. No public ABI compatibility is frozen. See the [r2 evidence](stage4-r2-0.4.0-progress.md) and [ABI ledger](application-abi.md); full regression and real NUC5/Ryzen acceptance remain open.

C: `musha_time(&ms)`, `musha_wait(timeout_ms, flags, &reason)`, `musha_input(&event)`. Rust guest wrappers: `time()`, `wait(timeout_ms, input)`, `input()` return owned values. TIME permission2 returns monotonic sampled PM-epoch boot milliseconds; WAIT permission4 supports0–60000ms, flags0 or WAIT_INPUT1 (also requires INPUT permission8), returns0 timeout/poll or1 pending input. INPUT copies one48-byte LE record, returns ERR_AGAIN when empty; insufficient capacity is ERR_LIMIT, invalid/unwritable ranges ERR_INVALID, missing permissions ERR_DENIED. Errors return zero RDX and leave the queue/output unchanged. Reserved syscall arguments/record bytes must be zero. Timestamp precision depends on cooperative sampling; long unsampled intervals beyond a PM counter wrap cannot be reconstructed.

Only r2 with INPUT permission reopens USB after the old runtime releases its arena borrow. Hosts/rings remain on the OS stack. A saved guest continuation is inside the synchronous USB callback; detach/halt/BME-off precedes normal callback and session cleanup. Only confirmed cleanup permits mapping rollback, arena scrub and a fresh viewer. Failed stop quarantines memory without reuse. WAIT polls a bounded USB step. USB hub/control work may delay a deadline by one bounded poll (control requests have finite1s deadlines); WAIT is not a hard-real-time deadline. Stalled-clock watchdog counts consecutive nonadvancing samples, not all healthy iterations.

Keyboard down/up includes HID usage/modifiers. Mouse-only sessions work without a keyboard. Relative X/Y/buttons use boot format; wheel is enabled only for the validated signed8 relative descriptor subset, otherwise boot mode supplies wheel0. Absolute usb-tablet is not converted. The current live keyboard/mouse Host (or mouse-only Host) is serviced; simultaneous mouse input on a separate third controller is not integrated. Overflow preserves64 older FIFO records then reports a saturated dropped count; consumers must reset held-state assumptions because lost clicks/motion are not reconstructed. Viewer mouse UI is excluded, gamepad production/Bluetooth is stage6, viewer return is implemented locally in stage5; physical acceptance remains open.

Build standalone C r2 with `python3 tools/check-sdk.py --c-only --abi-revision 2 --source tests/io-c.c --output build` (NOTE permissions default15; --permissions can restrict them). Rust: `python3 tools/build-rust.py --r2 --output build`. These use generated NOTE/types and the existing ELF checker. Rust uses pinned1.99.0/rust-src, custom static SSE2/no-red-zone target and rebuilt core/compiler_builtins; Bootstrap enables build-std/JSON target only. Never execute guest ELFs on the host. Separate SDK archives remain development-qualified; the Rust release gate stays closed pending remaining stage4/full acceptance.


Stage1 v0.4.0 development: this C/r1 behavior is retained; Rust r1/r2 execution is covered above; viewer launch/return is implemented locally in stage5 with the same r1 validation. The drive now has DOCS and empty SETTINGS and README is the startup marker. See [distribution usage](distribution-usage.md).

### Historical r1 delivery notes / 旧r1配備記録

Owner decision (2026-10-14): the automatic boot filename is `AUTORUN.ELF`; the previous `MUSHA.ELF` is not accepted. This implementation choice does not change the ABI, syscalls, NOTE or SDK-generated ELF. When AUTORUN.ELF is present, the viewer does not start.

MacBook Pro 2018 currently boots but does not recognize storage in this development build; external applications are unsupported there. Internal keyboard support is absent because T2 is unimplemented. NUC5/Ryzen development Hello results do not replace candidate cold-boot acceptance.

The current `release/0.3.0` development build supports one external C ELF per boot, including the feature-free release-profile EFI. This is an implementation and local QEMU result, **not hardware acceptance or a published release**. Published v0.2.0 downloads are unchanged. The [ABI](application-abi.md) remains **experimental candidate r1**, with no public compatibility freeze.

Build the SDK C example using [the SDK instructions](sdk.md) and place the resulting ELF as `AUTORUN.ELF` in the root directory of the data medium selected by the existing file service. FAT32 and exFAT are supported through that service. ASCII case differences are accepted; duplicate matching names are rejected. There is no launcher, directory search, or new multiple-medium search. If the file is absent, the existing read-only viewer starts. A machine without a readable file service retains its existing recovery display.

A matching file must be 64 bytes to 16 MiB. The kernel reads at most 4096 bytes per request into its own staging memory, with a **5000 ms deadline**. The size ceiling is not a throughput promise: local tests completed 16 KiB, while 1 MiB and 16 MiB reached the deadline before completion. Start with the 13,312-byte SDK Hello. See [read measurements and memory rationale](external-elf.md). A partial, interrupted or malformed image is never executed.

The log prints `Musha-OS 0.4.0 ready` after required USB-host, boot-media and keyboard discovery and before any application starts. Ready level also shows app start, exit, fault, rejection and raw log bytes. The sequential log records the detected AUTORUN.ELF metadata and `app: starting <name>` at launch, with the actual ASCII-displayed filename (`AUTORUN.ELF` or the selected catalog name). After reading and verified device shutdown, the kernel checks ELF/NOTE and memory policy, then starts the application. It records OS_READY for the external application, APP_ENTERING before the privilege transition, and APP_RUNNING on the first `query`. It does not mark the static viewer as running for this path.

### Current shared console and lifecycle

`log` appends to the [shared boot console](boot-log.md), starting directly after `app: starting <name>`. Its dimensions are the full GOP text grid, with the existing6x8 font/height-based scale and10*scale row pitch at the top-left. There is no80x24 app box, banner or footer. White/black cells wrap and scroll; each cell paints its background and startup has no whole-screen fill. The application receives neither the framebuffer address nor hardware registers. UTF-8 validation, printable-ASCII/newline rendering, non-renderable-byte handling, LOG permission,4096-byte call limit, accepted-byte count and return status remain API contracts.

On confirmed completion the OS appends `app: <name> exited, status N`, a reasoned rejection, or `app: <name> faulted, vector=13 error=0x18 rip=0x...` as appropriate; the latter is an illustrative format, not a recorded result. Kernel CPU exceptions use `ERROR: cpu: ...` followed by `Musha-OS halted`, never a red band or a fabricated application fault. No-error-code vectors use `error n/a`. Device stop remains a separate condition and unconfirmed stop never permits reuse. When launched from the viewer, the console is cleared to black and output starts at its top; return lets the viewer repaint its existing UI. App logging remains visible even at quiet startup level. Devices now selects concise OS device decisions; trace selects detailed transfer/stage/timing events, and F8/8 requests trace. The ring keeps all events without inserting debug output between app start, raw log and completion. Serial mirroring is a future v0.5.0 goal.

The normal path now promotes C3 rollback/scrub/device restart and viewer selection; AUTORUN automatically runs once, selected ELF uses the same checks. A stop or kernel invariant failure still halts without reuse. Application file/network/display APIs, scheduling and a public ABI freeze remain outside this work; Rust and r2 input/time calls are implemented as described above. SDK distribution and three recorded hardware cold boots remain release gates.

## 日本語

### 工程4 r2実接続到達点 —2026-10-15

実験r2のTIME/WAIT/INPUTと相対USB boot mouseをC/Rust guestへ実接続しました。r1番号/NOTE受入と既定C buildは保持し、公開互換性は固定しません。[r2証拠](stage4-r2-0.4.0-progress.md)と[ABI台帳](application-abi.md)参照。全回帰・NUC5/Ryzen実機受入は未完です。

Cはmusha_time(&ms)、musha_wait(timeout_ms,flags,&reason)、musha_input(&event)。Rust guestのtime()/wait(timeout_ms,input)/input()は所有値を返します。TIME権限2で単調なsample済みPM起動epoch ms、WAIT権限4で0–60000ms、flags0またはWAIT_INPUT1（入力権限8も必要）、結果0はtimeout/poll、1はpending入力です。INPUTは48byte LE record1個をコピー。空ERR_AGAIN、容量不足ERR_LIMIT、異常/非書込み範囲ERR_INVALID、権限無しERR_DENIED。エラー時RDX0でqueue/outputを変更しません。予約引数/record bytesはゼロ。時刻精度は協調sampleに依存し、PM counter周回より長くsampleしない経過時間は復元できません。

r2かつINPUT権限の場合だけ、旧runtimeのarena借用終了後にUSBを再開します。Host/ringはOS stack上。同期USB callback内でguest保存継続を囲み、detach/halt/BME停止確認後にcallbackとsession cleanupを通常returnします。確認済みcleanup後だけmapping復元・arena消去・新viewerへ進み、停止未確認は再利用せず隔離。WAITは上限付きUSB1stepをpollします。hub/control処理により期限が上限付き1poll分（control各1秒期限）遅れることがあり、hard realtime保証ではありません。clock watchdogは全反復数でなく時刻非進行の連続sample数を数えます。

keyboard down/upはHID usage/modifier付き。keyboard無しmouse単独sessionも動作します。相対XY/buttonはboot形式、wheelは検証したsigned8相対descriptor部分形式だけ有効で、それ以外のboot modeはwheel0。絶対usb-tabletは変換しません。現live keyboard/mouse Hostまたはmouse単独Hostを処理し、別の第3controllerの同時mouse収集は未統合。overflowは古い64件を保持後、飽和drop数を通知します。失われたclick/移動は復元できないためconsumerは保持状態の前提をリセットします。viewer mouse UIは対象外、gamepad producer/Bluetoothは工程6、viewer復帰は工程5でローカル実装済み、実機受入は未確認です。

単独C r2 buildは `python3 tools/check-sdk.py --c-only --abi-revision 2 --source tests/io-c.c --output build`（NOTE権限は既定15、--permissionsで制限）。Rustは `python3 tools/build-rust.py --r2 --output build`。生成NOTE/型と既存ELF checkerを使います。Rustは固定1.99.0/rust-src、専用static SSE2/red-zone無しtarget、再構築core/compiler_builtinsで、Bootstrapはbuild-std/JSON target用だけ。guest ELFをhost実行しません。SDK archiveは分離した開発版で、Rust公開gateは工程4残件/全受入まで閉じています。


v0.4.0工程1：以下のC/r1動作を保持し、Rust r1/r2実行は上記工程4に到達、viewer起動/復帰は工程5。媒体はDOCS/空SETTINGS/README起動markerへ変更。[配布利用](distribution-usage.md)参照。

### 旧r1配備記録

所有者決定（2026-10-14）：自動実行名は `AUTORUN.ELF` とし、旧 `MUSHA.ELF` は受理しません。実装上の選択でありABI・syscall・NOTE・SDK生成ELFは変更しません。AUTORUN.ELFがあるとビューアーは起動しません。

MacBook Pro 2018は現行開発版で起動しますがストレージ未認識のため外部アプリ未対応です。内蔵キーボードもT2未実装のため未対応です。NUC5／Ryzenの開発版Hello成功と、公開候補の冷間起動受入は区別します。

`release/0.3.0`の開発版は、feature指定なしのrelease構成を含め、起動ごとに1個の外部C ELFを実行します。ローカルQEMUでの実装検証であり、**実機合格やリリース公開を意味しません**。公開済みv0.2.0配布物は不変です。ABIは**実験候補r1**のままで、公開互換性を固定していません。

[SDK手順](sdk.md)でCサンプルをビルドし、既存ファイルサービスが選択するデータ媒体のルートへ`AUTORUN.ELF`として配置します。FAT32／exFATに対応し、ASCIIの大小文字は区別せず、同名重複は拒否します。ランチャー、ディレクトリ探索、複数媒体の追加探索はありません。不在なら従来の読出し専用ビューアー、読める媒体がなければ従来の復旧表示へ進みます。

サイズは64バイト～16MiB、1要求最大4096バイト、読込み期限は**5000ms**です。容量上限と期限内に読める容量は別です。QEMUでは16KiBが完読し、1MiB／16MiBは未完読で期限終了しました。まず13,312バイトのSDK Helloを使用してください。[測定・メモリ配置の根拠](external-elf.md)に詳細を記載しています。不完全・中断・不正なELFは実行しません。

必須USB host・起動媒体・keyboard探索後、app開始より前に`Musha-OS 0.4.0 ready`を表示。readyでもapp開始/終了/fault/拒否・生log byteを表示します。検出metadataと起動時の`app: starting <name>`を逐次logへ追記（nameはAUTORUN.ELF又は選択catalogの実名ASCII表示）し、読込みと機器停止を確認してからELF／NOTE・メモリ検査と実行へ進みます。外部アプリ用のOS_READY、特権遷移前のAPP_ENTERING、最初の`query`でAPP_RUNNINGを記録し、静的ビューアーの起動記録と区別します。

### 現行共通consoleとlifecycle

`log`は`app: starting <name>`直後から[共通boot console](boot-log.md)へ追記。GOP全体のtext grid、既存6x8 font/高さ由来scale・10*scale行ピッチ、左上起点です。80x24専用枠/banner/footerを使わず白黒cellを折返し/scroll。cell背景を塗り起動時の全画面fillは無し。framebuffer address/機器registerをappへ渡しません。UTF-8検査、printable ASCII/改行・非表示byte扱い、LOG権限、1call4096byte、受理byte数/戻り値のAPI契約は維持。

確認済終了は`app: <name> exited, status N`、拒否理由又は`app: <name> faulted, vector=13 error=0x18 rip=0x...`等を同consoleへ追記（例は書式で実測結果ではない）。kernel CPU例外は`ERROR: cpu: ...`と`Musha-OS halted`で赤帯/app faultへのすり替え無し。error codeをpushしないvectorは`error n/a`。device停止は別条件で未確認なら再利用しません。viewerから起動時はconsoleを黒clearし先頭から出力、復帰で従来viewerを再描画。quietでもapp logは可視。devicesはOSの簡潔な機器判断、traceは詳細transfer/stage/timing、F8/8はtraceを要求。ringは全eventを保持し、app開始・生log・終了の間にdebug出力を挟みません。serial mirrorはv0.5.0目標。

通常版へC3復元・消去・機器再開・viewer選択起動を昇格し、AUTORUN自動起動は初回だけ、選択ELFも同じ検査です。app向けfile/network/display API・スケジューリング・ABI固定は対象外で、Rustとr2 time/inputは上記の実装済み契約に従います。SDK配布と実機3回のコールドブートは公開までの残件です。
