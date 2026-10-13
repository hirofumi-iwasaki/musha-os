# Musha-OS

## English

**v0.3.0 release candidate (not published) — developer preview: run your own C program at Ring 3**

[Published v0.2.0 download](https://github.com/hirofumi-iwasaki/musha-os/releases/tag/v0.2.0) · [Release notes](docs/release-notes-0.3.0.md) · [Release scope](docs/release-scope-0.3.0.md) · [C SDK](docs/sdk.md)

Build a C program with the SDK, copy it to the USB as `MUSHA.ELF`, and Musha-OS runs it at Ring 3 and shows its output. The ABI is experimental and may change. External Hello was observed on development builds on NUC5 and a Ryzen 7 machine; candidate hardware acceptance is pending; MacBook Pro 2018 boots but does not recognize USB storage in this release. See [known issues](docs/open-issues-0.3.0.md).

Published v0.2.0 assets are unchanged.

Musha-OS is a thin bare-metal execution environment based on x86-64 / UEFI.
It aims to make use of older PCs and provide applications with a display, input,
USB storage, wired networking, and a RAM region they can use freely.

The oldest hardware reference is the Intel NUC5i5RYH / RYK; the comparison machine
is the NUC8 (NUC8i5BEH). The goal is to boot the same USB image in QEMU and on both machines.

The implementation primarily uses Rust (`no_std`), with C for lwIP and assembly
for operations such as switching CPU execution state.

### What starts after boot

Musha-OS initializes its runtime, then automatically runs built-in checks and one application.

1. **OS startup:** boot from UEFI, select the largest GOP mode, and initialize memory, CPU state, timers, and the runtime.
2. **Checks:** test RAM and discover USB controllers and connected devices. These checks progress cooperatively with application execution.
3. **Application:** if the root of the FAT32/exFAT data medium contains `MUSHA.ELF`, the kernel validates it, stops devices, and runs it at Ring 3; its `log` output, exit status, fault or refusal appears on screen, and the machine halts until reboot. Otherwise the bundled **`USB file viewer | READ ONLY`** starts: it lists root files and shows selected contents as hexadecimal and ASCII. The viewer does not write or delete files.

Boot screens distinguish OS startup from the SDK example: before ExitBootServices, `Musha-OS <VERSION>  starting...`; after arena initialization, `Musha-OS <VERSION>  runtime` and `CORE READY`; device enumeration adds `DEVICE DISCOVERY` on the left. VERSION is embedded from the repository file. The external console heading is `MUSHA.ELF  (Ring 3)`. `Hello Musha-OS!` is SDK sample output only. When the boot-file reader explicitly reports absence, the viewer list status says `MUSHA.ELF not found in root. Starting file viewer.`; unreadable media keep the existing error and rejected files still halt. There is no application launcher; one application runs per boot.
Connect an external USB keyboard to operate the viewer. Q ends the application session; restart to use it again.
See the [application guide](docs/external-app.md), [SDK](docs/sdk.md), [viewer guide](docs/file-viewer.md) and [diagnostic screen guide](docs/hardware-diagnostics.md).

### Downloads and USB updates

[GitHub Releases](https://github.com/hirofumi-iwasaki/musha-os/releases) provides OS binaries. GitHub's **Source code** archives contain source only.
v0.3.0 separates the small initial/recovery image (`*-usb.img.gz`), the normal file update (`*-files.zip`) and the C SDK (`*-c-sdk.zip`). The boot partition type is now Microsoft Basic Data, so macOS and Windows show the USB as a normal drive; existing ESP media still boot.
See the [distribution instructions](docs/distribution-usage.md) and [distribution policy](docs/release-distribution-policy.md).

The compatibility target is x64 PCs, with NUC5 as the oldest reference. UEFI x64, GOP, a supported xHCI controller, and an external USB Boot Keyboard are required.
This is a target, not a compatibility guarantee; see [hardware evidence](docs/hardware-verification-status.md).
Initial provisioning erases the target USB. The sequence is download, verify hashes, identify the target, deploy and adjust capacity, verify, safely eject, and boot through UEFI.
A physical-device writer for the small initial image is not provided. Do not raw-copy it to a larger USB; retain the existing verified FAT32 file-placement path.

Normal updates back up, verify, and replace only changed managed files, preserving GPT/FAT32 and user data.
They do not generate, write, or hash an entire USB device, reformat it, or fall back to a whole-disk rewrite.
See [macOS](docs/usb-install-macos.md), [Windows](docs/usb-install-windows.md), [Linux](docs/usb-install-linux.md), and [recovery and limitations](docs/distribution-usage.md).
Windows/Linux device adapters remain unimplemented. Unsigned EFI requires appropriate Secure Boot settings. Internal Mac keyboard support remains optional and unavailable.

### Current status

Implementation has begun. Direct greeting output through GOP and the display of
`CORE READY` after exiting UEFI Boot Services and switching to a dedicated stack
have been verified in QEMU.
Our own GDT / IDT / TSS and CPU exception diagnostics are implemented;
#UD / #GP / #DF have been verified in QEMU.
Our own page tables, reserved-memory management, and RAM arena are also implemented.
A 64MiB arena and page protection have been verified in QEMU.
The PM timer, PCI enumeration, xHCI ownership handoff, reset, DMA / ring diagnostics,
and step-based Rust application execution have been added.
Completion of 600 No-Op commands, ring wraparound, and controller shutdown have been verified in QEMU.
USB port reset and Device Descriptor reads from USB storage and a keyboard have been verified.
See [USB enumeration scope and limitations](docs/usb-enumeration.md).
Boot Keyboard press / release diagnostics have been verified in QEMU.
See [keyboard diagnostics](docs/usb-keyboard.md) and [NUC5 test preparation](docs/nuc5-bringup.md).
Continuous input from a single keyboard to the application has been added.
The v0.2.0 development build uses the [read-only file viewer](docs/file-viewer.md): arrows select, Enter opens, Page Up/Down browse, Esc returns, and Q ends the session.
USB BOT capacity queries and first / last sector reads have been verified in QEMU.
See [storage diagnostics](docs/usb-storage.md).
[FAT32 read diagnostics](docs/fat32.md) have been added; reading MUSHA.TXT from the root has been verified in QEMU.
[GPT support and shared bootable USB images](docs/usb-image.md) are implemented and verified in QEMU.
An initial read-only [application file API](docs/file-api.md) is implemented for the verified boot file, up to 4KiB. Initial [QEMU ARP / ICMP / UDP diagnostics](docs/network.md) are implemented with lwIP. NUC NIC initialization and the application UDP API remain pending. The [I218 Rust port](docs/i218-rust-port-plan.md) has started with a BSD-derived bounded PHY-access library and an opt-in identity probe; I218 networking is not yet implemented.
[Cooperative input, snapshot-file reads and QEMU networking](docs/cooperative-io.md) now run in the same session, including idle input and isolated device failures.
The diagnostic application is designed to
check the display, input, RAM, file, and UDP status.
Diagnostic builds retain the [hardware diagnostic panel](docs/hardware-diagnostics.md); use the [hardware test record template](docs/hardware-test-record-template.md) for physical tests.
[USB2 hub boot enumeration](docs/usb-multi-controller-hub-plan.md) now handles bounded child traversal and sequential multi-controller discovery. Up to five full-speed QEMU hub levels have been verified; high-speed TT traffic remains unverified. USB3.0/5Gbps hub initialization is implemented but has not been exercised on hardware; USB3.1/3.2 hubs remain unsupported.
See [development and build instructions](docs/development.md).
Build a test image and FAT32 USB bundle with the [one-command USB build](docs/test-usb-build.md).

### Development status

The external application path, kernel console, GOP mode selection and Basic Data partition type are part of the v0.3.0 normal build. The [C1](docs/ring3-prototype.md), [C2](docs/ring3-elf.md) and [C3](docs/ring3-lifecycle.md) probes remain opt-in; returning to the OS after application exit is not a normal-build feature. The [ABI](docs/application-abi.md) is experimental candidate r1. Rust guest execution, practical application I/O (input, files, display commands, network) and physical NIC networking remain pending. See [policy](docs/boot-readiness-policy.md), [validation](docs/boot-api-validation.md) and [open issues](docs/open-issues-0.3.0.md).

### Policy documents

- [v0.3.0 release notes](docs/release-notes-0.3.0.md)
- [v0.3.0 release scope](docs/release-scope-0.3.0.md)
- [v0.3.0 hardware acceptance](docs/hardware-acceptance-0.3.0.md)
- [Release writing and publication rules](docs/release-editorial-policy.md)
- [v0.2.0 release notes](docs/release-notes-0.2.0.md)

- [MI68 release preparation](docs/mi68-release-preparation.md)
- [Preparation results](docs/mi68-preparation-results.md)
- [Release notes](docs/release-notes-0.1.0.md)

- [I218-V Rust port plan](docs/i218-rust-port-plan.md)
- [Experimental I218 PHY probe](docs/i218-phy-probe.md)
- [Multi-controller and hub implementation plan](docs/usb-multi-controller-hub-plan.md)
- [Multi-controller USB diagnosis](docs/usb-controller-diagnosis.md)
- [Hardware diagnostics](docs/hardware-diagnostics.md)
- [Hardware test record template](docs/hardware-test-record-template.md)
- [Work plan before hardware arrival](docs/pre-hardware-work-plan.md)

- [Musha-OS baseline policy 0.1.0](docs/policy-v0.1.md)
- [0.1.0 architecture design](docs/design-0.1.0.md)
- [Future architecture: Ring 3 application isolation](docs/future-design.md)
- [0.1.0 implementation plan and design completion criteria](docs/implementation-plan-0.1.0.md)

Version `0.1.0` is an initial development release with the limited scope linked above.

### Additional hardware documentation

- [H3 hardware USB diagnostics plan](docs/usb-h3-diagnostics-plan.md)
- [Mac early memory diagnostics](docs/mac-memory-diagnostics.md)
- [Ryzen H4 hardware results](docs/ryzen-h4-hardware-results.md)
- [Memory attribute policy](docs/memory-attribute-policy.md)
- [ACPI timer diagnosis plan](docs/acpi-timer-diagnostics-plan.md)
- [ACPI firmware-table read policy](docs/acpi-read-policy.md)
- [Latest physical hardware verification status](docs/hardware-verification-status.md)

### Repository layout

```text
README.md             Project overview
.gitignore            Exclusions for locally generated files
 docs/policy-v0.1.md   Baseline policy, scope, and validation criteria
```

### Development and publication

The default branch is `main`; the design branch is `design/0.1.0`.
GitHub: https://github.com/hirofumi-iwasaki/musha-os

### License

Musha-OS original code, documentation, and configuration files are provided under
[Apache License 2.0](LICENSE) (SPDX: `Apache-2.0`), except where different terms
are explicitly stated for individual files.

Copyright 2026 Hirofumi Iwasaki

Third-party code retains its original licenses and copyright notices.
Before incorporating it, we check distribution terms and compatibility,
and record its source, version, and changes.

---

## 日本語

**v0.3.0 公開候補（未公開）— 開発者プレビュー：自作 C プログラムを Ring 3 で実行**

[公開済み v0.2.0 のダウンロード](https://github.com/hirofumi-iwasaki/musha-os/releases/tag/v0.2.0) · [リリースノート](docs/release-notes-0.3.0.md) · [公開範囲](docs/release-scope-0.3.0.md) · [C SDK](docs/sdk.md)

SDK で C プログラムを作り、`MUSHA.ELF` として USB へコピーすると、Musha-OS が Ring 3 で実行して出力を表示します。ABI は実験版で変更の可能性があります。外部 Hello は開発版で NUC5 と Ryzen 7 機の動作を観測しました。候補 EFI の実機受入は未完了です。MacBook Pro 2018 は起動しますが本版では USB ストレージを認識しません。[既知の問題](docs/open-issues-0.3.0.md)を参照。

公開済み v0.2.0 配布物は変更していません。

Musha-OSは、x86-64 / UEFIベースの薄いベアメタル実行環境です。
古いPCを活用し、アプリケーションに必要な画面、入力、USBストレージ、
有線ネットワークと自由に使えるRAM領域を提供することを目指します。

最古リファレンスはIntel NUC5i5RYH / RYK、比較機はNUC8（NUC8i5BEH）です。
QEMUと両実機で同一のUSBイメージを起動する構成を目標とします。

実装はRust（`no_std`）を主体とし、lwIPはC、CPU切替などはアセンブリを使用します。

### 起動後に動くもの

Musha-OS は、OS の実行環境を初期化した後、組込みのチェック処理と1つのアプリケーションを自動実行します。

1. **OS 起動**：UEFI から起動し、最大の GOP モードを選び、メモリー・CPU・タイマー・ランタイムを初期化します。
2. **チェック処理**：RAM の検査と USB コントローラー・接続機器の探索を行います。これらはアプリの実行と協調して進みます。
3. **アプリ**：FAT32/exFAT データ媒体のルートに `MUSHA.ELF` があれば、カーネルが検査し、機器を停止して Ring 3 で実行します。`log` 出力・終了コード・例外・拒否理由を画面に表示し、再起動まで停止します。なければ同梱の **`USB file viewer | READ ONLY`** が起動し、ルートのファイル一覧と選択したファイルの内容を16進数と ASCII で表示します。書込み・削除は行いません。

起動表示はSDKサンプルと区別します。ExitBootServices前は `Musha-OS <VERSION>  starting...`、arena初期化後は `Musha-OS <VERSION>  runtime` と `CORE READY`、機器探索開始時は左列に `DEVICE DISCOVERY` を表示します。版はリポジトリのVERSIONを埋め込みます。外部アプリの見出しは `MUSHA.ELF  (Ring 3)` で、`Hello Musha-OS!` はSDKサンプルの出力専用です。起動ファイルの不在が確定した場合だけ、ファイラー一覧の状態行に `MUSHA.ELF not found in root. Starting file viewer.` を表示します。媒体を読めない場合の文言とrejected時の停止は変更しません。アプリを選ぶランチャーはなく、1起動につき1アプリです。
ビューアーの操作には外付け USB キーボードが必要です。Q でセッションを終了し、再度使う場合は再起動します。
[アプリの利用方法](docs/external-app.md)、[SDK](docs/sdk.md)、[ファイルビューアー](docs/file-viewer.md)、[実機診断表示](docs/hardware-diagnostics.md)を参照してください。

### ダウンロードとUSB更新

[GitHub Releases](https://github.com/hirofumi-iwasaki/musha-os/releases)からOSバイナリーを取得できます。GitHubの **Source code** アーカイブはソースコードのみです。
v0.3.0 では初回・復旧用の小型イメージ（`*-usb.img.gz`）、通常更新用 ZIP（`*-files.zip`）、C SDK（`*-c-sdk.zip`）を分けています。起動パーティションの種別を Microsoft Basic Data に変更し、macOS・Windows で通常の USB として表示されます。既存の ESP 媒体も起動できます。
[配備手順](docs/distribution-usage.md)と[配布方針](docs/release-distribution-policy.md)を参照してください。

互換性の目標はx64機で、最古の基準機はNUC5です。UEFI x64・GOP・対応xHCI・外付けUSB Boot Keyboardが必要です。
全機種への対応を保証するものではありません。[実機確認状況](docs/hardware-verification-status.md)を参照してください。
初回作成は対象USBのデータを消去します。取得→ハッシュ照合→対象確認→配置・容量調整→検証→安全な取り外し→UEFI起動の順で進めます。
小型イメージの実USB向け初回書込みアダプターは未提供です。大容量USBへの単純なrawコピーは使わず、既存の検証済みFAT32配置経路を維持します。

通常更新は変更ファイルだけを退避・照合して差し替え、GPT/FAT32とユーザーデータを保持します。
USB全容量のイメージ生成・書込み・ハッシュ走査や再フォーマットは行わず、失敗時の全量書込みへの切替も行いません。
[macOS](docs/usb-install-macos.md)、[Windows](docs/usb-install-windows.md)、[Linux](docs/usb-install-linux.md)、[復旧と制限](docs/distribution-usage.md)を参照してください。
Windows/Linuxの実デバイス用アダプターは未実装です。未署名EFIに適したSecure Boot設定が必要です。Mac内蔵キーボード対応はオプショナルで、現状は未対応です。

### 現在の状態


[USB2ハブ配下の起動時列挙](docs/usb-multi-controller-hub-plan.md)と複数controllerの順次探索を追加。QEMU full-speed hubは最大5段で確認済み。高速TT通信は未検証。USB3.0／5Gbps hubの初期化を実装したが実転送は未検証、USB3.1／3.2 hubは未対応。

実装を開始しました。GOPで挨拶を直接表示し、UEFI終了・専用スタックへの切替後に
`CORE READY` を表示するところまでQEMUで確認済みです。
自前GDT / IDT / TSSとCPU例外診断を実装し、QEMUで#UD / #GP / #DFを確認済みです。
自前ページテーブルと予約メモリ管理、RAM arenaも実装し、
QEMUで64MiB arenaとページ保護を確認済みです。
PM timer、PCI列挙、xHCI所有権移行・リセット・DMA / ring診断とRustアプリのstep実行を追加しました。
QEMUでNo-Op 600回の完了、リング周回と停止を確認済みです。
USBポートをリセットし、USBメモリとキーボードのDevice Descriptor読出しを確認済みです。
[USB列挙の範囲と制約](docs/usb-enumeration.md)を参照。Boot Keyboardの押下・解放診断をQEMUで確認済みです。
[キーボード診断](docs/usb-keyboard.md)と[NUC5試験準備](docs/nuc5-bringup.md)を参照。
[I218 Rust移植](docs/i218-rust-port-plan.md)に着手し、BSD由来の上限付きPHYアクセスと実験用識別probeを追加しました。I218での通信は未実装です。
診断用ビルドの[実機診断表示](docs/hardware-diagnostics.md)と[試験記録様式](docs/hardware-test-record-template.md)も追加済みです。
v0.2.0開発版は[読出し専用ファイルビューアー](docs/file-viewer.md)を表示します。上下で選択、Enterで開く、Page Up/Downで移動、Escで一覧へ戻り、Qで終了します。FAT32/exFATをホストとQEMUで検証済み、実機確認は工程7です。
USB BOTの容量取得と先頭・末尾セクタ読出しをQEMUで確認済みです。
[ストレージ診断](docs/usb-storage.md)を参照。[FAT32読出し診断](docs/fat32.md)を追加し、ルートのMUSHA.TXTをQEMUで確認済みです。
[GPT対応と共通の起動用USBイメージ](docs/usb-image.md)を実装し、QEMUで確認済みです。
検証済みの起動ファイル（最大4KiB）向けに、初期の読出し専用[アプリファイルAPI](docs/file-api.md)を実装しました。lwIPによる初期の[QEMU ARP・ICMP・UDP診断](docs/network.md)を実装しました。NUC向けNIC初期化とアプリUDP APIは未実装です。[協調I/O](docs/cooperative-io.md)により入力・snapshot file read・QEMU通信が同一セッションで進み、無入力・機器障害時の動作も検証しました。診断アプリは
画面・入力・RAM・ファイル・UDPの状態を確認する構成です。
[開発・ビルド手順](docs/development.md)を参照してください。

### 開発状況

外部アプリの実行経路、カーネルコンソール、GOP モード選択、Basic Data 種別は v0.3.0 の通常版に含まれます。[C1](docs/ring3-prototype.md)・[C2](docs/ring3-elf.md)・[C3](docs/ring3-lifecycle.md) は任意試験構成のままで、アプリ終了後の OS 復帰は通常版の機能ではありません。[ABI](docs/application-abi.md) は実験候補 r1 です。Rust guest、アプリ向け実用 I/O（入力・ファイル・描画命令・通信）、実機 NIC 通信は残件です。[方針](docs/boot-readiness-policy.md)・[検証](docs/boot-api-validation.md)・[残件](docs/open-issues-0.3.0.md)を参照してください。

### 方針書

- [v0.3.0 リリースノート](docs/release-notes-0.3.0.md)
- [v0.3.0 公開範囲](docs/release-scope-0.3.0.md)
- [v0.3.0 実機受入](docs/hardware-acceptance-0.3.0.md)
- [リリース文書・公開手順の規約](docs/release-editorial-policy.md)
- [v0.2.0リリースノート](docs/release-notes-0.2.0.md)

- [I218-V Rust移植方針](docs/i218-rust-port-plan.md)
- [実験用I218 PHY診断](docs/i218-phy-probe.md)
- [複数コントローラー・ハブ対応設計](docs/usb-multi-controller-hub-plan.md)
- [複数USBコントローラーの診断](docs/usb-controller-diagnosis.md)
- [実機診断](docs/hardware-diagnostics.md)
- [実機試験記録様式](docs/hardware-test-record-template.md)
- [実機到着前の作業方針](docs/pre-hardware-work-plan.md)

- [Musha-OS 基本方針 0.1.0](docs/policy-v0.1.md)

- [0.1.0 アーキテクチャ設計](docs/design-0.1.0.md)
- [将来設計：Ring 3によるアプリ分離](docs/future-design.md)
- [0.1.0 実装計画と設計完了条件](docs/implementation-plan-0.1.0.md)

`0.1.0`は上記の検証範囲に限定した初期開発版です。開発は継続中です。

テストイメージとFAT32 USB用ファイル一式の生成方法は[一括USBビルド](docs/test-usb-build.md)を参照してください。

### リポジトリ構成

```text
README.md             プロジェクト概要
.gitignore            ローカル生成物の除外
 docs/policy-v0.1.md   基本方針・対象範囲・検証条件
```

### 開発と公開

既定ブランチは `main`、設計ブランチは `design/0.1.0` です。
GitHub: https://github.com/hirofumi-iwasaki/musha-os

### ライセンス

Musha-OSの独自コード、文書、設定ファイルは、個別に別の条件を明記したものを除き、
[Apache License 2.0](LICENSE)（SPDX: `Apache-2.0`）で提供します。

Copyright 2026 Hirofumi Iwasaki

第三者コードは各コードの元のライセンスと著作権表示を保持します。
取り込み前に配布条件と互換性を確認し、採用元・版・変更内容を記録します。

- [H3実機USB診断方針](docs/usb-h3-diagnostics-plan.md)

- [Mac早期メモリ診断](docs/mac-memory-diagnostics.md)

- [Ryzen H4実機結果](docs/ryzen-h4-hardware-results.md)

- [メモリー属性判定方針](docs/memory-attribute-policy.md)

- [ACPIタイマー診断計画](docs/acpi-timer-diagnostics-plan.md)

- [ACPI表の読取方針](docs/acpi-read-policy.md)

- [配布資料用の最新実機確認状況](docs/hardware-verification-status.md)（2026-10-10）
