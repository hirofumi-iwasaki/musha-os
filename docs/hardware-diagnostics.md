# Hardware diagnostics / 実機診断

Status: current (single boot-log contract, 2026-10-16)

## English — current v0.4.0 display

Use the [single sequential boot log](boot-log.md) and [record template](hardware-test-record-template.md) for newly built v0.4.0 diagnostics. Startup, device decisions, application output and fatal errors share one white-on-black console across the GOP frame. It begins at the top-left cell without a startup background fill; each printed cell paints its own background. Lines wrap, and the console scrolls when full. There are no left/right panels, rotating diagnostic pages, colored status words, separate application box, ready footer or red exception band. The existing 6x8 font/height-based scale is retained; framebuffer WC mapping and actual display details must be recorded from the tested build, not inferred from appearance.

Set `log=devices` in the identified FAT32 boot volume's SETTINGS/CONFIG.TXT, recording exact bytes/hash and EFI/features/source. Each displayed OS event includes a PM-epoch timestamp and a subsystem tag. Capture PCI IDs/revision/BDF, USB IDs/controller/port/route/speed, GOP and memory geometry, storage capacity/marker hash, input selection, LAN/BT decisions and confirmed stop/failure reasons as sequential lines. A summary count or an empty field does not establish compatibility. Hexadecimal IDs/addresses are distinct from decimal vector numbers and PM times; preserve the complete printed line and its prefix.

The 64 KiB ring retains devices-level whole records even when a quieter level hides them; eviction removes complete oldest records. Pre-EBS events retain an early flag and display 0 ms, not measured elapsed time. F8/8 startup override exposes devices detail through the same formatter; firmware recognition must still be recorded. No serial/UDP/file mirror is promised in this version: serial mirroring is a v0.5.0 goal. Screen scrolling can remove an early line, so preserve the complete capture and any available ring dump; do not reconstruct a missing observation from a later success. When the viewer owns the screen, its existing layout applies. A viewer-launched application clears the console to black and starts at the top; app return lets the viewer repaint.

Required failures appear at every level as `ERROR: <subsystem>: <reason>` and `Musha-OS halted`. Kernel exceptions include their name and decimal vector, error or `n/a`, RIP and CR2 fields; CR2 is meaningful for a page fault, not evidence of a page fault for another vector. Optional LAN/BT unavailability remains a reasoned phase event; a continued boot does not prove those devices work. App exit/fault/refusal is appended to the same log after the relevant cleanup decision. Do not infer DMA stop merely from a CPU exception or app termination.

The current mapping uses an existing PAT WC entry when available and falls back to UC without rewriting PAT. Record the actual result; the specification’s WC requirement must not be declared satisfied by a UC fallback. Final implementation validation and new hardware acceptance are PENDING until exact-EFI observations are reviewed. Keep [historical hardware status](hardware-verification-status.md) and version-specific acceptance separate. [PCI notes](pci.md), [NUC5 capture history](nuc5-diagnostic-capture.md) and the original records below preserve older diagnosis. Published v0.3.0 formatter trials continue to use their unchanged EFI and viewer-arrival criterion.

## 日本語 — 現行v0.4.0表示

新規v0.4.0診断は[単一逐次bootlog](boot-log.md)と[記録様式](hardware-test-record-template.md)を使用。起動・device判断・app出力・致命errorをGOP全体の白文字/黒背景consoleへ追記します。左上から始め起動時背景全消去はせず、描くcellの背景だけを塗り、桁折返し/行scrollします。左右panel・診断巡回page・色付き状態語・独立app枠・ready footer・赤例外帯は使いません。従来6x8 font/高さ由来scaleを保持し、framebuffer WCとdisplay値は実buildの記録で確認します。

識別したFAT32起動媒体のSETTINGS/CONFIG.TXTへ`log=devices`を設定し、正確な内容/hash・EFI/features/sourceを保存。OS eventはPM起点timestamp・subsystem付きの逐次行です。PCI ID/revision/BDF、USB ID/controller/port/route/speed、GOP/memory、容量/marker hash、入力元、LAN/BT判断、停止確認/失敗理由を撮影。件数や空欄を互換成功にせず、16進ID/addressと10進vector/PM時刻を区別して行全体を転記します。

64 KiB ringはquiet側で画面に出さないdevices eventも保持し、古いrecord単位で退役。EBS前eventはearly flagを保持し0 ms表示で、実測経過時間ではありません。F8/8の起動overrideは同formatterのdevices詳細を表示し、firmwareでの認識結果も記録。serialはv0.5.0目標で、現版のserial/UDP/file mirrorを約束しません。scrollで早期行は画面から消えるため動画・取得できたring dumpを保存し、後の成功から欠落値を作りません。viewer所有中は従来viewer layout。viewer起動appはconsoleを黒へclearして先頭から出力し、復帰でviewerが再描画。

必須失敗は全levelで`ERROR: <subsystem>: <reason>`と`Musha-OS halted`。kernel例外は名前/10進vector/error又はn/a/RIP/CR2を記録し、CR2はpage fault以外をpage faultと判断する根拠にしません。LAN/BT任意機器は理由付きphase行で、boot継続は通信成功ではありません。app終了/fault/拒否も同logへcleanup判断後に追記。CPU例外やapp終了だけでDMA停止を推測しません。

現mappingは既存PAT WC entryがあれば使用し、無ければPATを書換えずUCへfallback。実値を記録し、UC fallbackを仕様のWC要件達成としません。最終実装検証・正確なEFIの実機reviewまで新受入はPENDING。[過去実機状況](hardware-verification-status.md)と版毎受入を分離し、[PCI](pci.md)・[NUC5採取履歴](nuc5-diagnostic-capture.md)・以下原記録を保持。公開v0.3.0 formatter試験は元EFIとviewer到達条件のままです。

## Historical display and validation / 従来表示と検証の原記録

The complete previous text below describes its dated builds and is preserved verbatim. Its panel coordinates, colors, screenshots and status words do not specify the new boot log. / 以下の従来本文は当時のbuildの記録として全体を保持。panel座標・色・写真・状態語を新bootlogの仕様にしません。


Status: current

> Development update: [boot policy 3.0](boot-readiness-policy.md), [ABI candidates](application-abi.md), [SDK](sdk.md). A/B is static/cooperative; Ring 3 remains pending.
> 開発更新：[起動方針3.0](boot-readiness-policy.md)・[ABI候補](application-abi.md)・[SDK](sdk.md)。Ring 3は後続です。


> 最新の実機確認状況（2026-10-10）は[配布資料用の実機確認状況](hardware-verification-status.md)を参照。以下の過去の準備・検証記録は、その記録日時点の内容です。

## English

### Current v0.3.0 boot display

Boot screens distinguish OS startup from the SDK example: before ExitBootServices, `Musha-OS <VERSION>  starting...`; after arena initialization, `Musha-OS <VERSION>  runtime` and `CORE READY`; device enumeration adds `DEVICE DISCOVERY` on the left. VERSION is embedded from the repository file. The external console heading is `AUTORUN.ELF  (Ring 3)`. `Hello Musha-OS!` is SDK sample output only. When the boot-file reader explicitly reports absence, the viewer list status says `AUTORUN.ELF not found in root. Starting file viewer.`; unreadable media keep the existing error and rejected files still halt.

The earlier preparation and validation records below remain historical. Current hardware acceptance is tracked separately in [hardware acceptance](hardware-acceptance-0.3.0.md).

The normal build now shows a persistent hardware panel after leaving UEFI Boot Services. It records the current runtime phase, the actually selected xHCI and first detected Intel Ethernet PCI vendor/device IDs and BDFs, USB initialization stages, up to eight USB Device Descriptor records (port, slot, VID/PID, negotiated speed), the last storage capacity and file result, and final network counters. Discovery is read-only and is not a compatibility claim. The H2 panel lists up to eight xHCI controllers. Retained controllers are probed sequentially in BDF order; `SCANNED` records discovery, `*ACTIVE` identifies the selected input controller, and errors are recorded per controller. USB details show the current controller, while earlier controller summaries and stored file metadata remain visible. See [multi-controller and hub design](usb-multi-controller-hub-plan.md). Device class/subclass/protocol and interface classification distinguish unsupported hubs from matched keyboard/storage interfaces. See [USB controller diagnosis](usb-controller-diagnosis.md). Slot numbers can be reused after a device is released; use the port and VID/PID to identify a USB device.

All boot, diagnostic and console text uses framebuffer/metrics: 6x8 glyph cells, scale clamp(round(height/600),1,4), 10*scale line pitch, and two-cell margins. At widths >=1200 the diagnostic panel starts at half the total grid columns; on narrower screens it uses the full width below the status rows (row 20), preventing overlap. Wrapping and visible rows follow the grid, capped by the 64-byte line and 42-row diagnostic buffers. At 1600x1200 the cells are 12x16 pixels with 20-pixel line pitch. Unknown fields remain blank; a blank is not a pass.

`USB STEP` distinguishes capability checks, firmware ownership, halt/reset, rings/commands, Device Descriptor, configuration, BOT/capacity, sector reads, FAT32 file reads, and keyboard polling. `NET STEP` distinguishes resources, reset, EEPROM/MAC, link, DMA/lwIP, and polling. On error, `FAIL AT` and `USB ERROR` / `NET ERROR` retain the failure stage and reason while the current step can advance to `STOPPED DMA DISABLED`. A different subsequent failure supersedes the previous failure. A reset failure before a runtime-owned DMA session starts can leave the current step at the failed stage. A quiesce failure is shown separately and does not claim successful shutdown.

`STATE SESSION COMPLETE` means that the diagnostic application session returned, including recoverable device failures. Check error rows and file results separately. `STOPPED DMA DISABLED` is emitted only after the respective driver verifies its shutdown condition. The panel does not certify cleanup after arbitrary CPU exceptions or firmware errors before the runtime starts.

PCI and USB IDs, packed BDF, block count, sector bytes, file byte count, file hash, and network counters are hexadecimal. A PCI BDF encodes `(bus << 8) | (device << 3) | function`; `0018` is bus 0, device 3, function 0. File hash is FNV-1a, not SHA-256. Record the SHA-256 of the EFI executable and complete disk image independently. Capacity and file fields describe the last probed storage device, not necessarily the application snapshot source when multiple drives are present.

Only the QEMU Intel 82574 (`8086:10D3`) driver is implemented. NUC5 I218-V and NUC8 I219-V display `NET STEP DRIVER UNSUPPORTED / ABSENT`; this is expected and is not a hardware-network test pass. The PCI ID remains observable independently. Zero packet counters are not proof of communication. USB2 hub boot enumeration is implemented; full-speed QEMU hub paths are verified, while high-speed TT traffic is unverified and USB3.0/5Gbps hub initialization is implemented but physical transfers are unverified; USB3.1/3.2 hubs remain unsupported. USB records include root port and route string. The H2 heading and `SS HUB / BOOT ENUMERATION UNVERIFIED` distinguish a configured SS hub from a verified child transfer; unsupported hub generations show the actual USB version. Wi-Fi, Bluetooth, audio, native GPU drivers, AHCI/NVMe, SMP and POSIX remain outside the initial scope.

Use the [test record template](hardware-test-record-template.md) with the [NUC5 preparation guide](nuc5-bringup.md) and [USB image instructions](usb-image.md). Photograph the full screen before and after Esc. If initialization stops, record the last stage and exact error before powering down. Physical NUC5/NUC8 results remain pending.

Debug builds also emit `MUSHA: DIAG ...` lines at session completion (or application failure) so QEMU can verify the same stored information. The normal build needs no debug-port reader.

### Verification

Host tests cover bounded ASCII records, failure-stage preservation, screen layout, and compact-text clipping with framebuffer padding/guard words. QEMU checks cover the shared GPT image and its unchanged hash, supported and unsupported Intel NIC identification, USB descriptors/file metadata, USB read timeout, and LAN link-loss diagnostics. Normal-build screenshots are checked separately. See the recorded results below after running the checks.

### Recorded validation — 2026-10-08


- 52 host tests passed with Rust 1.99.0. UEFI normal/debug builds passed. QEMU 11.1.2, q35/TCG, 256MiB, bundled EDK2 were used.
- The shared GPT/FAT32 image booted with xHCI storage and a keyboard; PCI `1B36:000D` and Intel `8086:10D3`, USB VID/PID/speed, 512-byte sectors, 16-byte DIAGNOSE.TXT and FNV-1a `9A42A948C590F507` matched the stored panel snapshot. The diagnostic session stopped both drivers.
- `--nic e1000` displayed detected PCI `8086:100E` and `DRIVER UNSUPPORTED / ABSENT`, while keyboard and file checks completed.
- Five fault cases passed: xHCI halt timeout, command timeout, Device Descriptor timeout, storage read timeout, and corrupt GPT/FAT32 fixture. Each snapshot retained the expected failure stage and error after cleanup where applicable.
- Cooperative traffic passed ARP, ICMP, 257 UDP echoes, file rechecks, ring wraps, checksum rejection and driver shutdown. Final panel counters were UDP `00000101`, ARP/ICMP `00000001`. Link-down retained `FAIL AT NET STEP POLLING` / `NET ERROR LINK DOWN` and allowed file/input progress before USB shutdown.
- The normal build was booted separately without debug-port output. Shift+A, Esc, completed file/arena checks, PCI/USB panel and shutdown status were checked in a screenshot; the complete image hash remained unchanged.

Artifacts are local, ignored build outputs; they are not release assets. The test disk images are **64MiB QEMU fixtures**. For the acquired 32GB physical USB drive, generate a new image using its exact measured capacity as described in [USB image instructions](usb-image.md), so the backup GPT header is at the physical end. No physical drive was written or tested in this work.

| Artifact | SHA-256 |
|---|---|
| `out/esp/EFI/BOOT/BOOTX64.EFI` (normal) | `42d286aa25416f8f735eeb90603638907a232612008022e59ce0a5a1dcd05375` |
| `out/musha-usb-hardware-diag-release.img` | `ff1d2fac174ffccc11ca85de99202d29b133d9d2ab81f5470bbb907452351a78` |
| `out/musha-usb-hardware-diag-final-debug.img` | `d7c9bab3ec02baefe233efcc2e0d0b598287bc007ed23ebe66f7297017bc916d` |

Normal-build screenshot: `out/qemu-hardware-diag-release/screen.ppm`. Host results: `out/hardware-diag-host-tests.log`. Debug snapshots: `out/qemu-normal/debug.log`, `out/qemu-cooperative-traffic/debug.log`, `out/qemu-cooperative-link-down/debug.log` and the corresponding fault-case directories. These paths can be overwritten by later test runs.

Reproduce the standard diagnostic checks after building with `qemu-debug` and staging the EFI:

```sh
python3 tools/make-usb-image.py out/hardware-check-new.img
python3 tools/smoke-qemu.py --firmware-dir /opt/homebrew/share/qemu \
  --qemu /opt/homebrew/bin/qemu-system-x86_64 --usb-image out/hardware-check-new.img
# Use --nic e1000 to check the unsupported Intel NIC path.
python3 tools/smoke-cooperative.py --firmware-dir /opt/homebrew/share/qemu \
  --qemu /opt/homebrew/bin/qemu-system-x86_64 --usb-image out/hardware-check-new.img --case traffic
python3 tools/smoke-cooperative.py --firmware-dir /opt/homebrew/share/qemu \
  --qemu /opt/homebrew/bin/qemu-system-x86_64 --usb-image out/hardware-check-new.img --case link-down
```

Use a fresh output filename; the image generator refuses to replace existing files. Fault tests require their respective features except `fat-corrupt`, which uses a corrupted fixture. Finish by rebuilding normally with `sh tools/build-esp.sh`; never prepare a physical USB from an injected-fault build. Firmware and QEMU paths depend on the host.


## 日本語

### 現在のv0.3.0起動表示

起動表示はSDKサンプルと区別します。ExitBootServices前は `Musha-OS <VERSION>  starting...`、arena初期化後は `Musha-OS <VERSION>  runtime` と `CORE READY`、機器探索開始時は左列に `DEVICE DISCOVERY` を表示します。版はリポジトリのVERSIONを埋め込みます。外部アプリの見出しは `AUTORUN.ELF  (Ring 3)` で、`Hello Musha-OS!` はSDKサンプルの出力専用です。起動ファイルの不在が確定した場合だけ、ファイラー一覧の状態行に `AUTORUN.ELF not found in root. Starting file viewer.` を表示します。媒体を読めない場合の文言とrejected時の停止は変更しません。

以下の過去の準備・検証記録は当時の内容を保持しています。今回の受入は[実機受入](hardware-acceptance-0.3.0.md)に分けて記録します。

通常ビルドでも、UEFI Boot Services終了後に実機診断パネルを表示する。実行段階、実際に選択したxHCIと最初に検出したIntel EthernetのPCI vendor/device ID・BDF、USB初期化段階、最大8件のUSB Device Descriptor情報（ポート、slot、VID/PID、接続速度）、最後に調べたストレージの容量・ファイル結果、終了時の通信カウンターを記録する。列挙は読出し専用で、互換性を保証するものではない。H2では最大8個のxHCIを保持しBDF順に調査する。SCANNEDは探索済み、*ACTIVEは入力選択先を示し、失敗をコントローラー別に記録する。USB詳細は現在の候補を表示し、先の候補の概要とファイル情報を保持する。USB2ハブ配下の起動時列挙を追加した。機器行にroot portとrouteを表示する。高速TTは未検証、USB3.0／5Gbps hub初期化は実装済み・実転送未検証、USB3.1／3.2 hubは未対応。slot番号は機器解放後に再利用されるため、USB機器はポートとVID/PIDでも識別する。

起動・診断・コンソールはframebuffer/metricsを共有します。基本セル6×8、倍率clamp(round(高さ/600),1,4)、行ピッチ10×倍率、余白2セル。幅1200以上では総列数の半分から右側、未満では状態行より下の20行目から全幅を使い、重なりを防ぎます。折返し・表示行数はグリッドから計算し、診断バッファの64バイト・42行を上限とします。1600×1200では12×16ピクセル、行ピッチ20ピクセルです。未到達項目は空欄であり、合格を意味しません。

`USB STEP`はcapability、firmware ownership、halt/reset、ring/command、Device Descriptor、configuration、BOT/capacity、sector read、FAT32 file read、keyboard pollingを区別する。`NET STEP`はresource、reset、EEPROM/MAC、link、DMA/lwIP、pollingを区別する。エラー時は`FAIL AT`と`USB ERROR`／`NET ERROR`が失敗段階・理由を保持し、現在の段階だけが`STOPPED DMA DISABLED`へ進む。後から別のエラーが生じた場合は後者を記録する。ランタイムのDMA開始前のreset失敗では、現在の段階が失敗箇所に残る場合がある。停止検証に失敗した場合は別のエラーを表示し、停止成功とは扱わない。

`STATE SESSION COMPLETE`は診断アプリが終了したことを示し、回復可能な機器エラーも含む。エラー行とファイル結果を別途確認する。`STOPPED DMA DISABLED`は各ドライバが停止条件を検証した後だけ表示する。任意のCPU例外後や、ランタイム開始前のfirmwareエラーに対する停止保証ではない。

PCI・USB ID、BDF、block数、sector byte数、file byte数、file hash、通信カウンターは16進表示。BDFは`(bus << 8) | (device << 3) | function`で、`0018`はbus 0・device 3・function 0。ファイルhashはFNV-1aで、SHA-256ではない。EFIとディスクイメージ全体のSHA-256は別に記録する。容量・ファイル欄は最後に調べたストレージの情報で、複数ドライブ時はアプリが保持するsnapshotの取得元と一致するとは限らない。

実装済みLANドライバはQEMUのIntel 82574（`8086:10D3`）だけ。NUC5のI218-V、NUC8のI219-Vでは`NET STEP DRIVER UNSUPPORTED / ABSENT`が想定表示であり、実機通信試験の合格ではない。PCI IDは独立して確認できる。通信カウンターが0でも通信成功を意味しない。hub、Wi-Fi、Bluetooth、audio、GPU直接制御、AHCI/NVMe、SMP、POSIXは初期対象外。

[試験記録様式](hardware-test-record-template.md)を[NUC5準備手順](nuc5-bringup.md)・[USBイメージ手順](usb-image.md)とともに使用する。Escの前後で画面全体を撮影する。初期化が止まったら、電源を切る前に最後の段階とエラー全文を記録する。NUC5／NUC8の実機結果は未取得。

debugビルドでは終了時（またはアプリ失敗時）に`MUSHA: DIAG ...`を出力し、QEMUで同じ記録内容を検証できる。通常ビルドの画面確認にdebug port読出しは不要。

### 検証

ホスト試験でASCII記録の上限、失敗段階の保持、画面配置、padding／guardを含む小文字サイズ描画の境界を確認する。QEMUでは共通GPTイメージとhash不変、対応／未対応Intel NIC識別、USB descriptor・file情報、USB読出しtimeout、LAN切断時の表示を確認する。通常ビルドの画面も別に確認する。実行後の結果を下記へ記録する。

### 検証記録 — 2026-10-08


- Rust 1.99.0でホスト試験52件成功。通常／debug UEFIビルド成功。QEMU 11.1.2、q35/TCG、RAM 256MiB、同梱EDK2で検証。
- 共通GPT/FAT32イメージからxHCI storageとkeyboardを起動。PCI `1B36:000D`・Intel `8086:10D3`、USB VID/PID・速度、sector 512 byte、DIAGNOSE.TXT 16 byte、FNV-1a `9A42A948C590F507`が診断記録と一致。終了時に両ドライバを停止。
- `--nic e1000`でPCI `8086:100E`と`DRIVER UNSUPPORTED / ABSENT`を表示し、keyboard・file確認は完了。
- xHCI halt timeout、command timeout、Device Descriptor timeout、storage read timeout、GPT/FAT32 fixture破損の5ケース成功。該当する停止処理後も、想定した失敗段階・理由を保持。
- 協調通信試験でARP、ICMP、UDP echo 257件、file再確認、ring周回、checksum拒否、停止処理を確認。最終表示はUDP `00000101`、ARP／ICMP `00000001`。link切断では`FAIL AT NET STEP POLLING`／`NET ERROR LINK DOWN`を保持し、file・inputはUSB停止まで進行。
- debug port出力のない通常版を別途起動。Shift+A、Esc、file・arena確認、PCI・USBパネル、停止表示を画像で確認。image全体のhashは不変。

生成物はローカルのGit除外対象で、リリース配布物ではない。検証imageは**QEMU用64MiB fixture**。確保した32GBの実物USBには、[USBイメージ手順](usb-image.md)に従い、測定した正確な容量で新しいimageを生成してGPT副headerを媒体末尾に置く。本作業では実物USBへの書込み・実機試験を行っていない。

通常EFIと通常／debug imageのSHA-256は上表を参照。通常版画像は`out/qemu-hardware-diag-release/screen.ppm`、ホスト結果は`out/hardware-diag-host-tests.log`。debug記録は上記の各`debug.log`にある。後日の試験で上書きされる場合がある。

通常の再検証コマンドは上記を使用する。最初に`qemu-debug`でbuildしEFIを配置する。image生成は既存ファイルを置換しないため、新しい出力名を使う。fault試験は各featureが必要（`fat-corrupt`は破損fixtureを使用）。最後は`sh tools/build-esp.sh`で通常版へ戻す。故障注入版から実物USBを準備しない。firmware・QEMUのパスは環境に合わせる。

現行パネルは上記の共通グリッドで配置します。未調査はNOT PROBED、調査中はPROBING、探索済みはSCANNED、入力先は*ACTIVEです。[複数コントローラーとハブ対応](usb-multi-controller-hub-plan.md)を参照してください。

## Related PCI and NUC5 records / 関連PCI・NUC5記録

- [PCI diagnostic inventory](pci.md): current diagnostic interpretation / 診断inventory解釈。
- [NUC5 diagnostic capture history](nuc5-diagnostic-capture.md): dated capture procedures, not current acceptance requirements / 当時の採取手順。現受入条件とは区別。
- [Current release scope](release-scope-0.4.0.md) / 現行受入範囲。
