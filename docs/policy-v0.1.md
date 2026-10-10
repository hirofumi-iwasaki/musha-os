# Musha-OS baseline policy 0.1.0

> 2026-10-10: v0.1.0 is approved as a limited initial development release. [Final scope / 確定した公開範囲](release-scope-0.1.0.md) supersedes the original acceptance gates below. Earlier unreleased statements are historical.

## English

- Established: 2026-10-07
- Status: Adopted as the initial policy. Implementation and hardware validation are incomplete.
- Scope: Design, implementation scope, and validation criteria for 0.1.0

### 1. Purpose

Musha-OS will provide a thin bare-metal execution environment based on x86-64 / UEFI.
It prioritizes giving applications access to the display, input, storage, networking,
and RAM on a limited set of devices over reproducing a broad range of general-purpose OS features.
The project targets older PCs and explicitly limits hardware and feature scope to keep development manageable.

### 2. Platform and reference machines

The baseline requirements are x86-64, UEFI, ACPI, PCI / PCIe, xHCI, and UEFI GOP.
Configurations that depend on Legacy BIOS, CSM, or EHCI are outside the scope of 0.1.0.
This does not mean support for every model from NUC5 onward; support applies to validated configurations.

| Validation target | Role | Network design target |
|---|---|---|
| QEMU + x86-64 UEFI firmware | Development and reproducible tests | e1000e / 82574 family |
| Intel NUC5i5RYH / RYK | Oldest hardware reference | I218-V |
| Intel NUC8 (NUC8i5BEH) | Comparison machine for generational differences | I219-V |

Record PCI IDs, firmware settings, and USB connection conditions on each physical machine.
The table defines design targets and does not guarantee operation at this stage.

### 3. Shared USB image and boot

Boot the same USB image in QEMU / NUC5 / NUC8.
Select the necessary initialization after detecting hardware at boot, rather than building separate images per model.
Use the UEFI removable-media boot path `EFI/BOOT/BOOTX64.EFI` and FAT32.
Configure USB storage and xHCI in QEMU as well, and validate differences from physical hardware.

During the UEFI phase, obtain GOP, the memory map, ACPI information, and other required data for the runtime.
Finish preparations before `ExitBootServices`, then call it with the latest memory map and key.
If it fails, obtain the map again. After exit, do not depend on Boot Services.
The runtime initializes and manages USB and the NIC.

### 4. Features provided by 0.1.0

| Area | Policy |
|---|---|
| CPU | x86-64; use only the boot CPU |
| Display | Draw into the framebuffer obtained from UEFI GOP |
| Device discovery | Enumerate PCI / PCIe and use required ACPI information |
| USB | Use an xHCI controller |
| Input | USB HID keyboard; validate direct connections first |
| Storage | USB Mass Storage; initially target Bulk-Only Transport |
| Filesystem | FAT32; initial acceptance requires file reads |
| Networking | Drivers for the target Intel wired NICs and lwIP |
| Memory | A RAM arena applications can use freely |

Carry over the GOP address, resolution, pitch, and pixel format, and draw according to the actual format.
Do not depend on GPU mode changes or acceleration.

For xHCI, account for firmware ownership handoff, required port routing, reset,
DMA regions, and device enumeration. USB working under UEFI does not guarantee
USB operation after Boot Services exit.
Decide on extensions such as hubs, broader HID support, and UAS after validating the initial configuration.

### 5. Network structure

Separate the application communication API, lwIP, the common Ethernet layer, and model-specific initialization.
Share transmit / receive processing where possible across QEMU's 82574 family,
NUC5's I218 family, and NUC8's I219 family, while handling reset, PHY control,
link establishment, and other differences separately.
An Intel brand or an added PCI ID alone is insufficient evidence of completed support.

Use lwIP on a single CPU without an OS, and design an execution model that continuously advances packet and timer processing.
Define a service-call contract so lengthy application work does not stop communication.
Initial tests use static IPv4, ARP, ICMP, and UDP; validate TCP in stages.
Specify the application API separately before implementing it.

### 6. Application RAM arena

Select safely usable RAM based on the UEFI memory map and pass its base and length explicitly to the application.
Do not hand over all RAM unconditionally.

Exclude the runtime, stacks, page tables, boot handoff information, framebuffer,
MMIO, retained ACPI regions, firmware-reserved regions, UEFI Runtime Services regions,
and USB / NIC DMA regions.
If Boot Services regions are reused, first verify that no references remain after exit.

Applications may choose their own allocators and data layouts within the arena.
Specify alignment, ownership, lifetime, address handling, and boundaries with DMA regions in the API.
Process isolation and POSIX-style memory management are not prerequisites for 0.1.0.

### 7. Outside the scope of 0.1.0

- Wi-Fi, Bluetooth, and audio
- Direct GPU control, dedicated GPU drivers, and GPU acceleration
- AHCI and NVMe for internal storage
- SMP and use of multiple CPU cores
- POSIX compatibility and general-purpose OS process / system-call models

Do not introduce excluded features as implicit dependencies.
Decide on extensions after validating the baseline configuration.

### 8. Implementation approach

1. Establish UEFI boot, GOP drawing, memory-map retrieval, and transition to the runtime.
2. Implement exception handling, required memory management, PCI enumeration, and RAM arena handoff.
3. Implement xHCI, USB HID, USB Mass Storage, and FAT32 reads.
4. Connect QEMU's NIC to lwIP and implement initialization differences for NUC5 / NUC8.
5. Verify acceptance criteria in all three environments using the same image and record the results.

Use Rust (no_std) as the primary language, C for lwIP, and minimal assembly for operations such as CPU transitions.
Statically link the validation application in 0.1.0.
Finalize toolchain versions, ABI, and interrupt / timer details in the architecture and detailed designs.
This policy document does not describe features that are already implemented.

### 9. Acceptance criteria for 0.1.0

Verify the following in QEMU, NUC5, and NUC8 using the same image:

- Boot from UEFI and continue execution after Boot Services exit.
- Display output through the GOP framebuffer.
- Obtain USB keyboard input through the runtime's xHCI driver.
- Read FAT32 files from USB storage and compare them with known contents.
- Establish a link and verify ARP, ICMP, and UDP communication using Intel wired Ethernet and lwIP.
- Let the application read and write its arena without overlapping reserved regions.
- Diagnose initialization failures during repeated tests, including reboots.

Record image hashes, commits, machine models, firmware settings, PCI IDs,
USB devices, memory capacity, successes / failures, and logs.
Do not label untested hardware or features as supported.

### 10. Open-source practices

Use Apache License 2.0 for Musha-OS original code, documentation, and configuration files.
Follow individually stated terms where they differ.
Retain third-party licenses and copyright notices; do not change them to Apache-2.0 without authorization.
When using existing drivers or lwIP, check licenses, copyright notices, and distribution terms,
and record incorporated portions and changes.
Exclude secrets, local configuration, and build outputs from the repository.
During this initial setup, do not create a GitHub repository or push to GitHub.

---

## 日本語

**Musha-OS 基本方針 0.1.0**

- 策定日: 2026-10-07
- 状態: 初期方針として採用。実装および実機検証は未完了。
- 対象: 0.1.0の設計、実装範囲、検証条件

### 1. 目的

Musha-OSは、x86-64 / UEFIベースの薄いベアメタル実行環境を構築する。
汎用OSの機能を広く再現することより、限られた機器上でアプリケーションが
画面、入力、ストレージ、ネットワーク、RAMを利用できることを優先する。
古いPCの活用を想定し、対象機器と機能を明示して開発規模を抑える。

### 2. プラットフォームと基準機

基本条件をx86-64、UEFI、ACPI、PCI / PCIe、xHCI、UEFI GOPとする。
Legacy BIOS、CSM、EHCIを前提とした構成は0.1.0では扱わない。
「NUC5以降の全機種対応」を意味するものではなく、検証した構成を対応対象とする。

| 検証対象 | 位置付け | ネットワークの設計対象 |
|---|---|---|
| QEMU + x86-64 UEFIファームウェア | 開発・再現試験 | e1000e / 82574系 |
| Intel NUC5i5RYH / RYK | 最古の実機リファレンス | I218-V |
| Intel NUC8（NUC8i5BEH） | 世代差を確認する比較機 | I219-V |

機種ごとのPCI ID、ファームウェア設定、USB接続条件を実機で記録する。
上表は設計上の対象であり、現時点で動作を保証するものではない。

### 3. 同一USBイメージと起動

QEMU / NUC5 / NUC8で同一のUSBイメージを起動する。
機種ごとにイメージを作り分けず、起動後の機器検出で必要な初期化を選択する。
UEFIのリムーバブルメディア起動パス `EFI/BOOT/BOOTX64.EFI` とFAT32を使用する。
QEMU側もUSBストレージとxHCIを構成し、実機との差を検証する。

UEFI段階でGOP、メモリマップ、ACPI情報などを取得し、ランタイムへ渡す。
`ExitBootServices` 前に必要な準備を完了し、最新のメモリマップとキーで
呼び出す。失敗時はマップを再取得する。終了後はBoot Servicesに依存しない。
USBとNICはランタイム側で初期化して管理する。

### 4. 0.1.0の提供機能

| 領域 | 方針 |
|---|---|
| CPU | x86-64。起動CPUのみを使用 |
| 画面 | UEFI GOPから取得したフレームバッファへ描画 |
| 機器検出 | PCI / PCIe列挙と必要なACPI情報の利用 |
| USB | xHCIコントローラを使用 |
| 入力 | USB HIDキーボード。まず直結構成で検証 |
| ストレージ | USB Mass Storage。初期対象はBulk-Only Transport |
| ファイルシステム | FAT32。初期合格条件はファイルの読出し |
| ネットワーク | 対象Intel有線LANドライバとlwIP |
| メモリ | アプリケーション向け自由RAM arena |

GOPのアドレス、解像度、ピッチ、ピクセル形式を引き継ぎ、実際の形式に
従って描画する。GPUのモード変更やアクセラレーションには依存しない。

xHCIはファームウェアからの所有権引き継ぎ、必要なポートルーティング、
リセット、DMA領域、デバイス列挙を考慮する。UEFIでUSBが使えたことを、
Boot Services終了後のUSB動作保証として扱わない。
ハブ、多様なHID、UASなどへの対応拡張は初期動作確認後に別途判断する。

### 5. ネットワーク構成

アプリケーション向け通信API、lwIP、共通Ethernet層、機種別初期化を分ける。
QEMUの82574系、NUC5のI218系、NUC8のI219系は、共有できる送受信処理を
共有しつつ、リセット、PHY制御、リンク確立などの差を個別に扱う。
Intel製であることやPCI IDの追加だけを根拠に対応完了とは判断しない。

lwIPは単一CPU・OSなしの構成を基本とし、パケット処理とタイマー処理を
継続して進められる実行モデルを設計する。アプリケーションの長時間処理が
通信を停止させないよう、サービス呼出しの契約を定める。
初期試験は固定IPv4、ARP、ICMP、UDPを基本とし、TCPは段階的に検証する。
アプリ向けAPIの詳細は実装前に別途仕様化する。

### 6. アプリケーション向け自由RAM arena

UEFIメモリマップを根拠に、安全に利用できるRAMを選び、アプリへ
領域の基点と長さを明示して渡す。RAM全体を無条件に渡すことはしない。

ランタイム本体、スタック、ページテーブル、ブート引継ぎ情報、
フレームバッファ、MMIO、ACPI保持領域、ファームウェア予約領域、
UEFI Runtime Services領域、USB / NICのDMA領域などを除外する。
Boot Servicesの領域を再利用する場合は、終了後に参照が残らないことを確認する。

アプリはarena内で独自のアロケータやデータ配置を選択できる。
アラインメント、所有権、有効期間、アドレスの扱い、DMA用領域との境界を
APIで明文化する。0.1.0ではプロセス分離やPOSIX的なメモリ管理は前提としない。

### 7. 0.1.0の対象外

- Wi-Fi、Bluetooth、audio
- GPUの直接制御、専用GPUドライバ、GPUアクセラレーション
- 内蔵ストレージ向けAHCI、NVMe
- SMP、複数CPUコアの利用
- POSIX互換、汎用OSのプロセス・システムコール体系

対象外の機能を暗黙の依存として持ち込まない。拡張は基本構成の検証後に判断する。

### 8. 実装の進め方

1. UEFI起動、GOP描画、メモリマップ取得、ランタイムへの移行を確立する。
2. 例外処理、必要なメモリ管理、PCI列挙、RAM arenaの受渡しを実装する。
3. xHCI、USB HID、USB Mass Storage、FAT32読出しを実装する。
4. QEMUのNICとlwIPを接続し、NUC5 / NUC8の初期化差を実装する。
5. 同一イメージで3環境の合格条件を確認し、結果を記録する。

主言語はRust（no_std）とし、lwIPはC、CPU切替などは最小限のアセンブリを使う。
0.1.0は検証アプリを静的リンクする。ツールチェーンの版、ABI、
割込み・タイマー方式の詳細はアーキテクチャ設計と詳細設計で確定する。本方針書は実装済みの機能を表すものではない。

### 9. 0.1.0の合格条件

QEMU、NUC5、NUC8の各環境について、以下を同一イメージで確認する。

- UEFIから起動し、Boot Services終了後も処理が継続する。
- GOPフレームバッファで画面表示ができる。
- ランタイム側のxHCIでUSBキーボード入力を取得できる。
- USBストレージ上のFAT32ファイルを読み、既知の内容と照合できる。
- Intel有線LANとlwIPでリンク確立、ARP、ICMP、UDP通信を確認できる。
- アプリが受け取ったarenaへ読み書きでき、予約領域と重複しない。
- 再起動を含む反復試験で、初期化の失敗を診断できる。

試験結果にはイメージのハッシュ、コミット、機種、ファームウェア設定、
PCI ID、USB機器、メモリ容量、成功・失敗とログを残す。
未検証の機器や機能は対応済みと表示しない。

### 10. オープンソース運用

Musha-OSの独自コード、文書、設定ファイルはApache License 2.0を採用する。
個別に別条件を明記したものはその条件に従う。第三者コードのライセンスと
著作権表示を保持し、Apache-2.0へ無断で変更しない。既存ドライバやlwIPを利用する際は、
ライセンス、著作権表記、配布条件を確認し、採用箇所と変更を記録する。
秘密情報、ローカル設定、ビルド生成物はリポジトリへ含めない。
本初期セットアップではGitHubリポジトリの作成およびpushを行わない。
