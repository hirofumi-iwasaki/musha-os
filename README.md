# Musha-OS

## English

**v0.3.0 release candidate (not published) — developer preview: run your own C program at Ring 3**

[Published v0.2.0 download](https://github.com/hirofumi-iwasaki/musha-os/releases/tag/v0.2.0) · [Release notes](docs/release-notes-0.3.0.md) · [Release scope](docs/release-scope-0.3.0.md) · [C SDK](docs/sdk.md)

Build a C program with the SDK, copy it to the USB as `AUTORUN.ELF`, and Musha-OS runs it at Ring 3 and shows its output. The ABI is experimental and may change. External Hello was observed on development builds on NUC5 and a Ryzen 7 machine; candidate hardware acceptance is pending; MacBook Pro 2018 boots but does not recognize USB storage in this release. See [known issues](docs/open-issues-0.3.0.md).

Published v0.2.0 assets are unchanged. The Intel routing/attachment-grace fix is implemented; repeat hardware acceptance on the next candidate remains pending.

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
3. **Application:** if the root of the FAT32/exFAT data medium contains `AUTORUN.ELF`, the kernel validates it, stops devices, and runs it at Ring 3; its `log` output, exit status, fault or refusal appears on screen, and the machine halts until reboot. Otherwise the bundled **`USB file viewer | READ ONLY`** starts: it lists root files and shows selected contents as hexadecimal and ASCII. The viewer does not write or delete files.

Boot screens distinguish OS startup from the SDK example: before ExitBootServices, `Musha-OS <VERSION>  starting...`; after arena initialization, `Musha-OS <VERSION>  runtime` and `CORE READY`; device enumeration adds `DEVICE DISCOVERY` on the left. VERSION is embedded from the repository file. The external console heading is `AUTORUN.ELF  (Ring 3)`. `Hello Musha-OS!` is SDK sample output only. When the boot-file reader explicitly reports absence, the viewer list status says `AUTORUN.ELF not found in root. Starting file viewer.`; unreadable media keep the existing error and rejected files still halt. There is no application launcher; one application runs per boot.
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

The external application path, kernel console, GOP mode selection and Basic Data partition type are part of the v0.3.0 normal build. The [C1](docs/ring3-prototype.md), [C2](docs/ring3-elf.md) and [C3](docs/ring3-lifecycle.md) probes remain opt-in; returning to the OS after application exit is not a normal-build feature. The [ABI](docs/application-abi.md) is experimental candidate r1. Rust guest execution, practical application I/O (input, files, display commands, network) and physical NIC networking remain pending. See [policy](docs/boot-readiness-policy.md), [validation](docs/boot-api-validation.md) and [open issues](docs/open-issues-0.3.0.md).

Development builds ran Ring 3 Hello on NUC5 and Ryzen 7. Candidate cold-boot acceptance remains incomplete; NUC5 device discovery is under investigation. MacBook Pro 2018 reaches boot and external keyboard detection, but USB storage is not recognized. See [hardware acceptance](docs/hardware-acceptance-0.3.0.md).

### Policy and hardware documentation

#### Current policies and specifications

- [v0.3.0 release scope](docs/release-scope-0.3.0.md)
- [v0.3.0 hardware acceptance](docs/hardware-acceptance-0.3.0.md)
- [Release writing and publication rules](docs/release-editorial-policy.md)
- [I218-V Rust port plan](docs/i218-rust-port-plan.md)
- [Experimental I218 PHY probe](docs/i218-phy-probe.md)
- [Multi-controller and hub implementation plan](docs/usb-multi-controller-hub-plan.md)
- [Multi-controller USB diagnosis](docs/usb-controller-diagnosis.md)
- [Hardware diagnostics](docs/hardware-diagnostics.md)
- [Hardware test record template](docs/hardware-test-record-template.md)
- [Memory attribute policy](docs/memory-attribute-policy.md)
- [ACPI firmware-table read policy](docs/acpi-read-policy.md)
- [Latest physical hardware verification status](docs/hardware-verification-status.md)
- [Distribution policy](docs/release-distribution-policy.md)
- [Boot readiness policy](docs/boot-readiness-policy.md)
- [External application ABI](docs/application-abi.md)
- [C SDK](docs/sdk.md)
- [External applications](docs/external-app.md)
- [v0.3.0 open issues](docs/open-issues-0.3.0.md)
- [Development and build instructions](docs/development.md)
- [Test USB build](docs/test-usb-build.md)

#### Historical records

- [v0.3.0 release notes](docs/release-notes-0.3.0.md)
- [v0.2.0 release notes](docs/release-notes-0.2.0.md)
- [MI68 release preparation](docs/mi68-release-preparation.md)
- [Preparation results](docs/mi68-preparation-results.md)
- [v0.1.0 release notes](docs/release-notes-0.1.0.md)
- [Work plan before hardware arrival](docs/pre-hardware-work-plan.md)
- [Musha-OS baseline policy 0.1.0](docs/policy-v0.1.md)
- [0.1.0 architecture design](docs/design-0.1.0.md)
- [Future architecture: Ring 3 application isolation](docs/future-design.md)
- [0.1.0 implementation plan and design completion criteria](docs/implementation-plan-0.1.0.md)
- [H3 hardware USB diagnostics plan](docs/usb-h3-diagnostics-plan.md)
- [Mac early memory diagnostics](docs/mac-memory-diagnostics.md)
- [Ryzen H4 hardware results](docs/ryzen-h4-hardware-results.md)
- [ACPI timer diagnosis plan](docs/acpi-timer-diagnostics-plan.md)

### Repository layout

```text
boot/         UEFI boot, runtime and driver integration (musha-boot)
platform/     Boot state and diagnostic records
memory/       Page tables, arena and ELF validation
xhci/         xHCI, USB and BOT
fs/           FAT32, exFAT and GPT
api/          Internal application API, stream files and bootfile
viewer/       Read-only file viewer
framebuffer/  GOP rendering and console
input/        Input source selection
net/          lwIP and I218 port (only QEMU 82574 implemented)
bce/          T2 BCE model (not connected)
sdk/          C SDK: ABI schema, crt0, libmusha and samples
tools/        Image creation, deployment, QEMU tests and release preparation
docs/         Policies, specifications, release and hardware records
```

### Development and publication

The default branch is `main`. Releases use a pull request from `release/<version>` to `main`.
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

SDK で C プログラムを作り、`AUTORUN.ELF` として USB へコピーすると、Musha-OS が Ring 3 で実行して出力を表示します。ABI は実験版で変更の可能性があります。外部 Hello は開発版で NUC5 と Ryzen 7 機の動作を観測しました。候補 EFI の実機受入は未完了です。MacBook Pro 2018 は起動しますが本版では USB ストレージを認識しません。[既知の問題](docs/open-issues-0.3.0.md)を参照。

公開済み v0.2.0 配布物は変更していません。Intelルーティング・接続猶予の修正は実装済みで、次候補による実機再受入は未完了です。

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
3. **アプリ**：FAT32/exFAT データ媒体のルートに `AUTORUN.ELF` があれば、カーネルが検査し、機器を停止して Ring 3 で実行します。`log` 出力・終了コード・例外・拒否理由を画面に表示し、再起動まで停止します。なければ同梱の **`USB file viewer | READ ONLY`** が起動し、ルートのファイル一覧と選択したファイルの内容を16進数と ASCII で表示します。書込み・削除は行いません。

起動表示はSDKサンプルと区別します。ExitBootServices前は `Musha-OS <VERSION>  starting...`、arena初期化後は `Musha-OS <VERSION>  runtime` と `CORE READY`、機器探索開始時は左列に `DEVICE DISCOVERY` を表示します。版はリポジトリのVERSIONを埋め込みます。外部アプリの見出しは `AUTORUN.ELF  (Ring 3)` で、`Hello Musha-OS!` はSDKサンプルの出力専用です。起動ファイルの不在が確定した場合だけ、ファイラー一覧の状態行に `AUTORUN.ELF not found in root. Starting file viewer.` を表示します。媒体を読めない場合の文言とrejected時の停止は変更しません。アプリを選ぶランチャーはなく、1起動につき1アプリです。
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

外部アプリの実行経路、カーネルコンソール、GOP モード選択、Basic Data 種別は v0.3.0 の通常版に含まれます。[C1](docs/ring3-prototype.md)・[C2](docs/ring3-elf.md)・[C3](docs/ring3-lifecycle.md) は任意試験構成のままで、アプリ終了後の OS 復帰は通常版の機能ではありません。[ABI](docs/application-abi.md) は実験候補 r1 です。Rust guest、アプリ向け実用 I/O（入力・ファイル・描画命令・通信）、実機 NIC 通信は残件です。[方針](docs/boot-readiness-policy.md)・[検証](docs/boot-api-validation.md)・[残件](docs/open-issues-0.3.0.md)を参照してください。

開発版ではNUC5・Ryzen 7でRing 3 Helloが動作しました。公開候補の冷間起動受入は未達で、NUC5の機器探索を調査中です。MacBook Pro 2018は起動・外付けキーボード検出までで、USBストレージは未認識です。[実機受入](docs/hardware-acceptance-0.3.0.md)を参照してください。

### 方針・実機文書

#### 現行の方針・仕様

- [v0.3.0 公開範囲](docs/release-scope-0.3.0.md)
- [v0.3.0 実機受入](docs/hardware-acceptance-0.3.0.md)
- [リリース文書・公開手順の規約](docs/release-editorial-policy.md)
- [I218-V Rust移植方針](docs/i218-rust-port-plan.md)
- [実験用I218 PHY診断](docs/i218-phy-probe.md)
- [複数コントローラー・ハブ対応設計](docs/usb-multi-controller-hub-plan.md)
- [複数USBコントローラーの診断](docs/usb-controller-diagnosis.md)
- [実機診断](docs/hardware-diagnostics.md)
- [実機試験記録様式](docs/hardware-test-record-template.md)
- [メモリー属性判定方針](docs/memory-attribute-policy.md)
- [ACPI表の読取方針](docs/acpi-read-policy.md)
- [配布資料用の最新実機確認状況](docs/hardware-verification-status.md)
- [配布方針](docs/release-distribution-policy.md)
- [起動とAPIの統合方針](docs/boot-readiness-policy.md)
- [外部アプリABI](docs/application-abi.md)
- [C SDK](docs/sdk.md)
- [外部アプリ](docs/external-app.md)
- [v0.3.0の残件](docs/open-issues-0.3.0.md)
- [開発・ビルド手順](docs/development.md)
- [一括USBビルド](docs/test-usb-build.md)

#### 過去の記録

- [v0.3.0 リリースノート](docs/release-notes-0.3.0.md)
- [v0.2.0リリースノート](docs/release-notes-0.2.0.md)
- [MI68公開準備](docs/mi68-release-preparation.md)
- [MI68準備結果](docs/mi68-preparation-results.md)
- [v0.1.0リリースノート](docs/release-notes-0.1.0.md)
- [実機到着前の作業方針](docs/pre-hardware-work-plan.md)
- [Musha-OS 基本方針 0.1.0](docs/policy-v0.1.md)
- [0.1.0 アーキテクチャ設計](docs/design-0.1.0.md)
- [将来設計：Ring 3によるアプリ分離](docs/future-design.md)
- [0.1.0 実装計画と設計完了条件](docs/implementation-plan-0.1.0.md)
- [H3実機USB診断方針](docs/usb-h3-diagnostics-plan.md)
- [Mac早期メモリ診断](docs/mac-memory-diagnostics.md)
- [Ryzen H4実機結果](docs/ryzen-h4-hardware-results.md)
- [ACPIタイマー診断計画](docs/acpi-timer-diagnostics-plan.md)

### リポジトリ構成

```text
boot/         UEFI 起動・ランタイム・ドライバー統合（musha-boot）
platform/     起動状態・診断記録
memory/       ページテーブル・arena・ELF 検査
xhci/         xHCI・USB・BOT
fs/           FAT32・exFAT・GPT
api/          内部アプリ API・stream file・bootfile
viewer/       読出し専用ファイルビューアー
framebuffer/  GOP 描画・コンソール
input/        入力源の選択
net/          lwIP・I218 移植（QEMU 82574 のみ実装）
bce/          T2 BCE モデル（未接続）
sdk/          C SDK（ABI スキーマ・crt0・libmusha・サンプル）
tools/        イメージ生成・配備・QEMU 試験・リリース準備
docs/         方針・仕様・公開記録・実機記録
```

### 開発と公開

既定ブランチは `main` です。リリースは `release/<version>` から `main` へのプルリクエストで取り込みます。
GitHub: https://github.com/hirofumi-iwasaki/musha-os

### ライセンス

Musha-OSの独自コード、文書、設定ファイルは、個別に別の条件を明記したものを除き、
[Apache License 2.0](LICENSE)（SPDX: `Apache-2.0`）で提供します。

Copyright 2026 Hirofumi Iwasaki

第三者コードは各コードの元のライセンスと著作権表示を保持します。
取り込み前に配布条件と互換性を確認し、採用元・版・変更内容を記録します。
