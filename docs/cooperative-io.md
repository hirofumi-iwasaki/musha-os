# Cooperative I/O runtime / 協調I/Oランタイム

## English

Implemented on 2026-10-08 as stage 4 of the [pre-hardware work plan](pre-hardware-work-plan.md). Keyboard input, application reads of the verified boot-file snapshot, lwIP timers and actual QEMU NIC traffic now progress in the same session on the boot CPU. This is still an initial diagnostic runtime: arbitrary asynchronous disk requests and an application-facing UDP socket/handle API remain unimplemented. NUC LAN initialization and physical verification are pending.

### Initialization and ownership

USB discovery, configuration and bounded synchronous BOT/FAT32 reads finish before starting the live loop. The first supported Boot Keyboard keeps its slot but has no interrupt-IN transfer outstanding while later ports are initialized; other slots are released after their diagnostics. This avoids a keyboard waiting for Esc before a later storage device is discovered. Extra keyboards are reported as unsupported and released. The verified file is copied into the application Context's owned, at-most-4KiB snapshot. On discovery completion, the runtime marks the snapshot available/unavailable, initializes the 82574/lwIP session, and reports `RUNTIME_POLL_READY`.

No mutable device-DMA slice or lwIP pbuf is exposed to the application. NIC and USB DMA allocations remain separate, reserved and UC/RW/NX. NIC ownership is represented by a single Session; its Drop implementation disables RX/TX and verifies PCI bus-master disablement. Recoverable application/clock errors leave through USB cleanup, invoke application shutdown, then drop the NIC session before propagating the error. A lwIP assertion requests and verifies bus-master disablement for both controllers before halting. CPU exceptions and unrelated panics retain the existing no-cleanup contract; reserved DMA memory is never reused.

### Each live iteration

1. Check the keyboard's attachment and examine at most one xHCI event without waiting for a key. Completed reports become bounded key transitions; otherwise the iteration proceeds with no transitions.
2. Update the monotonic PM-timer sample, advance lwIP timers, and process at most eight valid RX frames into CPU-owned buffers.
3. Observe at most one pending TX descriptor. If DD is absent, return to the next services instead of spinning. A pending descriptor has a 100ms deadline and a 5,000,000-observation limit. On completion, retire its buffer; at most one queued frame is then posted. A slot cannot be reused before a clean completion. Hardware errors, backwards time and timeout stop the NIC session.
4. Publish key transitions and call one application step. The diagnostic consumes at most 64 queued key events, reads at most 64 file bytes, and probes at most 64 RAM pages per step. New successful UDP traffic requests a new file read/length/hash/EOF/close/stale-handle check after the previous check has finished, demonstrating progress of the file API while traffic continues. File checking uses the owned snapshot, not new disk I/O.

The callbacks are serialized on one CPU with interrupts disabled; this is cooperative execution, not threads or preemption. Applications must return from step. The diagnostic reports cycle count, maximum observed service gap, completed file rechecks, and cycles exceeding the observational 1ms budget. These measurements do not establish a real-time guarantee. Recheck logs appear only at powers of two to bound logging volume; the final counter reports all completed checks.

### Termination and failure isolation

The normal build keeps the first keyboard session active until Esc. A `qemu-debug` build without `input-persistent` ends that session after ten seconds; the old five-second window was extended for integrated tests. Esc ends the input session and stops both DMA controllers after the file/RAM diagnostic completes. Unsatisfied TX can be discarded during termination; no DMA buffer is reclaimed.

No keyboard, or a disconnected keyboard, causes USB cleanup and a fallback cooperative loop. Healthy networking continues until ten seconds have elapsed since NIC startup, while file/RAM work progresses. If no NIC is supported or it has failed, the remaining application work completes without a network wait. A link loss or TX timeout stops only the NIC session; the keyboard/file application continues until its normal termination. A file recheck is scheduled after NIC failure, following completion of the initial check if necessary. Fault tests require the recheck marker to follow NIC DMA disablement. Reconnection, USB hubs, multiple active keyboards, general hotplug recovery and NUC PHY initialization remain outside this implementation.

### Validation

Host tests: 48 passed, including pending/completed/error TX observations, deadline and stalled-clock limits, arithmetic overflow, existing DMA layout checks, file handle lifetime and FIFO behavior.
QEMU 11.1.2 / Rust 1.99.0 packet tests use a loopback socket Ethernet backend and a real GPT/FAT32 image mounted read-only:

| Case | Required result |
|---|---|
| `traffic` | ARP, ICMP, 257 checksum-verified UDP echoes, a/b/c press/release during active NIC service, file rechecks, Esc, both DMA disabled |
| `input-idle` | Same traffic and file progress without any keyboard input, then debug deadline cleanup |
| `no-keyboard` | Same traffic and file progress with no keyboard device |
| `keyboard-disconnect` | USB disconnect detected and DMA stopped; packet and snapshot-file progress continues |
| `link-down` | NIC stopped; subsequent key press/release and file progress succeeds; Esc stops USB |
| `tx-timeout` | Withheld TX doorbell produces bounded NIC timeout; subsequent input/file progress and cleanup succeeds |
| `app-error` | Injected error after a traffic-triggered file recheck stops both DMA controllers before `APP_FAILED`; no success marker |

Positive cases require at least 16 RX and 32 TX ring wraps, reject bad IPv4/UDP checksums, accept subsequent valid traffic, and verify unchanged image SHA-256. The later-storage regression places a FAT32 fixture after the keyboard on USB2 ports, validating discovery before input polling. Debug builds and fault-injection images are for QEMU only.

```sh
cargo test --workspace --exclude musha-boot
cargo build --locked --release --target x86_64-unknown-uefi -p musha-boot --features qemu-debug
cp target/x86_64-unknown-uefi/release/musha-boot.efi out/esp/EFI/BOOT/BOOTX64.EFI
python3 tools/make-usb-image.py out/cooperative-test.img
python3 tools/smoke-cooperative.py --qemu /opt/homebrew/bin/qemu-system-x86_64 --firmware-dir /opt/homebrew/share/qemu --usb-image out/cooperative-test.img --case traffic
```

Repeat the last command with `input-idle`, `no-keyboard`, `keyboard-disconnect` or `link-down` as the case. For `tx-timeout`, build with only `--features net-tx-timeout`; for `app-error`, build with only `--features app-step-error`. Copy the new EFI and generate a new, differently named image before each fault test. After testing, restore a normal build with `sh tools/build-esp.sh`. The image generator deliberately refuses to replace existing files. Results/screenshots are under `out/qemu-cooperative-CASE`. Do not infer hardware compatibility or 0.1.0 completion from these tests. Next is stage 5, hardware diagnostics and test records.

### Image verification record

The final debug GPT/FAT32 image passed the traffic/Esc and link-loss tests unchanged:
`e357b3854e2ad5925c81491ee82d6d8e5c1e1e0ae84bd66d22abf6654a535b00`.
Idle/no-keyboard/disconnect tests also passed on the initial integrated image; TX-timeout and application-error tests passed on their dedicated fault builds.
The normal image `out/musha-usb-cooperative-final.img`, without debug/fault features, was booted read-only, received Shift+A and Esc, and showed `APP FILE READ OK`, `APP COMPLETE` and `NET TEST OK`. The final normal and traffic screens were visually checked. Its SHA-256 remained
`678dbd9cfa6705ab36c43ca0cc463b6e4476870bd820ddc93f096948910c0fc4`.
These are 64MiB test images. Follow [shared USB image sizing](usb-image.md) to regenerate an image for the exact capacity of the physical 32GB drive before writing it. `out/NOTICE` accompanies local binary artifacts. Physical boot remains untested.

## 日本語

[実機到着前の作業方針](pre-hardware-work-plan.md)第4段階として2026-10-08に実装した。起動CPU上の同一セッションで、キー入力、検証済み起動ファイルのsnapshot読出し、lwIP timer、QEMU NICの実通信を進める。初期の診断ランタイムであり、任意の非同期disk要求とアプリ向けUDP socket/handle APIは未実装。NUC向けLAN初期化と実機検証も未実施。

### 初期化と所有権

live loop開始前にUSB探索・設定と上限付き同期BOT/FAT32読出しを終える。最初の対応Boot Keyboardはslotを維持するが、後続ポートを初期化している間はinterrupt-IN転送を投入しない。その他のslotは診断後に解放する。これにより、キーボードのEsc待ちで後続ストレージ探索が止まる問題を解消する。追加のキーボードは非対応と報告して解放する。検証済みファイルはアプリContext所有の最大4KiB snapshotへコピーする。探索完了後に利用可否を確定し、82574/lwIP sessionを開始して`RUNTIME_POLL_READY`を報告する。

device DMAの可変sliceやlwIP pbufはアプリへ公開しない。NICとUSBのDMA領域は別々に予約し、UC/RW/NXとする。NICは単一Sessionが所有し、DropでRX/TXを止めてPCI bus mastering無効化を確認する。回復可能なアプリ・clock errorはUSB cleanup、アプリshutdown、NIC Sessionの破棄を経て伝播する。lwIP assertionは両controllerのbus mastering無効化を試み、結果を確認して停止する。CPU例外・その他panicは既存のcleanupを保証しない契約を維持する。予約DMA領域は再利用しない。

### 各周回の処理

1. キーボードの接続を確認し、キーを待たずにxHCI eventを最大1件調べる。完了reportから上限付きキー遷移を作り、未完了なら遷移なしで次へ進む。
2. 単調PM timerを更新してlwIP timerを進め、RX frameを最大8件、CPU所有bufferへ取り込む。
3. 未完了TX descriptorを最大1件観測する。DDがなければ待ち続けず他の処理へ戻る。未完了転送には100msと500万回の観測上限を設ける。完了時にbufferを解放し、queueから最大1frameを投入する。正常完了前にslotを再利用しない。hardware error、clock逆行、timeoutではNIC sessionを停止する。
4. キー遷移を公開してアプリstepを1回呼ぶ。診断は最大64件のキーevent、最大64byteのfile read、最大64ページのRAM試験を1stepで進める。正常UDP通信が進むと、前回検査完了後にfileの再読出し・長さ/hash・EOF・close・失効handle拒否を再検査する。通信中のfile APIの進行を確認するためであり、新たなdisk I/Oではない。

interruptを無効にした単一CPUから順番に呼び出す協調処理で、threadやpreemptionではない。アプリはstepから戻る必要がある。周回数、観測した最大サービス間隔、file再検査完了数、観測上の1ms budgetを超えた周回を報告する。real-time保証を示す測定ではない。ログ量を抑えるため再検査markerは2のべき乗の回数だけで表示し、最終counterへ全完了数を記録する。

### 終了と障害の分離

通常版は最初のキーボードをEscまで待ち受ける。`input-persistent`なしの`qemu-debug`版は10秒で入力セッションを終える。統合試験のため従来の5秒を延長した。Esc後はfile/RAM診断完了を待ち、両DMA controllerを停止する。終了時の未完了TXは破棄でき、DMA buffer自体は回収しない。

キーボードなし・切断時はUSBを停止し、協調fallback loopへ進む。正常な通信はNIC開始から10秒まで継続し、file/RAM処理も進める。非対応NIC・NIC停止後は残るアプリ処理を完了して終了する。link切断・TX timeoutはNICだけを停止し、キーボード・fileアプリは通常の終了まで継続する。NIC障害後にfile再検査を予約し、必要なら初回検査完了後に開始する。故障試験はNIC DMA停止後に再検査成功markerが出る順序も要求する。再接続、hub、複数キーボードの同時処理、汎用hotplug復旧、NUC PHY初期化は未対応。

### 検証

ホスト試験48件に合格。TX未完了・完了・error、期限・clock停止時の観測上限、算術overflow、既存DMA配置、file handle寿命、FIFOを検査する。
QEMU 11.1.2 / Rust 1.99.0で、loopback socket Ethernetと実際のGPT/FAT32読出し専用イメージを使用した。

| case | 要求する結果 |
|---|---|
| `traffic` | ARP、ICMP、checksum検証済みUDP echo 257回、NIC動作中のa/b/c press/release、file再検査、Esc、両DMA停止 |
| `input-idle` | キー入力なしでも同じ通信・file処理を行い、debug期限で停止 |
| `no-keyboard` | キーボードdeviceがなくても同じ通信・file処理が進む |
| `keyboard-disconnect` | USB切断を検知しDMA停止。通信・snapshot file処理は継続 |
| `link-down` | NICを止め、その後のkey press/releaseとfile処理が成功。EscでUSB停止 |
| `tx-timeout` | TX doorbellを抑止して期限内にNIC timeout。以後の入力・file処理とcleanupが成功 |
| `app-error` | 通信によるfile再検査後にerrorを注入し、`APP_FAILED`前に両DMA停止。成功markerを出さない |

正常系ではRX ring 16周以上・TX ring 32周以上、不正IPv4/UDP checksum拒否、その後の正常通信、イメージSHA-256不変も確認する。後続ストレージの回帰試験はUSB2 port上でkeyboardより後ろへFAT32 fixtureを接続し、入力poll前の探索を検査する。debug版と故障注入イメージはQEMU専用。

英語節のコマンドで試験する。最後のcaseを`input-idle`、`no-keyboard`、`keyboard-disconnect`、`link-down`へ変えて各試験を行う。`tx-timeout`は`--features net-tx-timeout`だけ、`app-error`は`--features app-step-error`だけでbuildし、EFIを配置して毎回別名のイメージを新規生成する。試験後は`sh tools/build-esp.sh`で通常版へ戻す。生成ツールは既存fileの上書きを拒否する。結果・画面は`out/qemu-cooperative-CASE`へ保存する。これらの試験から実機互換性・0.1.0完成を宣言しない。次は第5段階の実機診断・試験記録へ進む。

### イメージ検証記録

最終debug GPT/FAT32イメージで通信・Esc終了・link切断試験が成功し、SHA-256不変を確認した:
`e357b3854e2ad5925c81491ee82d6d8e5c1e1e0ae84bd66d22abf6654a535b00`。
無入力・キーボードなし・切断試験は初期統合イメージでも成功。TX timeoutとアプリerrorは各専用故障注入版で成功した。
debug/fault機能を含まない通常版`out/musha-usb-cooperative-final.img`も読出し専用で起動し、Shift+AとEscを送信した。`APP FILE READ OK`、`APP COMPLETE`、`NET TEST OK`を表示し、通常版・通信試験の最終画面を目視確認した。通常版のSHA-256は不変:
`678dbd9cfa6705ab36c43ca0cc463b6e4476870bd820ddc93f096948910c0fc4`。
これらは64MiBの試験イメージ。実物32GB USBへ書く前に[共通USBイメージの容量指定](usb-image.md)に従い、媒体の正確な容量で再生成する。ローカルbinary成果物へ`out/NOTICE`を添付した。実機起動は未検証。
