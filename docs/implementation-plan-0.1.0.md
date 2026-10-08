# Musha-OS 0.1.0 implementation plan and design completion criteria

## English

As of 2026-10-08, the baseline policy and architecture are documented.
Detailed design and implementation are ongoing. Direct GOP drawing, ExitBootServices,
and dedicated-stack transition have been verified in QEMU.
Our own GDT / IDT / TSS, exception diagnostics, page tables, reserved-region management,
64MiB arena, and PCI enumeration (xHCI / Intel 82574 identification) have been verified in QEMU.
ACPI PM timer handoff and 100ms measurement have been verified in QEMU.
The Rust application API, xHCI stop / reset, dedicated DMA / rings,
and 600 No-Op commands have been verified in QEMU.
USB enumeration / input / read diagnostics and GPT image generation are implemented. HPET, application file APIs, NIC drivers, and hardware tests remain incomplete.
Do not create a 0.1.0 tag yet.

### Design status

| Area | Decided | Design remaining before implementation |
|---|---|---|
| Language / build | Rust no_std, C lwIP, minimal assembly | Tool versions, crate selection, C ABI, CPU feature settings |
| Boot | Single EFI, static linking, self-management after UEFI exit | Library selection, boot_info Rust types / FFI definitions, transition assembly |
| Memory | Identity mapping, reserved-range exclusion, single arena | Hardware PAT / MTRR compatibility, DMA capacity, dynamic expansion |
| CPU / time | BSP only, polling, exception diagnostics | PM timer / HPET procedures and timeouts, FPU / SIMD state, expanded exception tests |
| Applications | Cooperative steps, asynchronous I/O, versioned context | Rust API and C headers, handle / buffer limits |
| USB | xHCI, boot keyboard, BOT | Ring / context specifications, state machines, recovery procedures |
| FAT32 | Read-only, 8.3 names | BPB constraints, GPT validation, corruption handling |
| Wired LAN | Model-specific initialization, lwIP NO_SYS, static IPv4 | NIC register / PHY specifications, PCI allowlist |
| Distribution | GPT + FAT32 ESP, shared USB image | Tool versions, reproducible builds, QEMU configuration |
| OSS | Apache-2.0 for original code, documentation, and configuration | Dependency licenses and distribution notices |

Design is complete when the remaining pre-implementation items are fixed in documents or headers,
and normal, timeout, disconnect, and invalid-input behavior can be explained for every state transition.
Confirmed hardware compatibility is a separate release requirement.

### Work order and exit criteria

1. Finalize the detailed build, boot-information, and CPU-transition designs.
   Exit: Generate EFI, display GOP output, and run on our own stack after ExitBootServices.
2. Finalize memory, page attributes, arena, time sources, and the application API.
   Exit: Pass reserved-range overlap checks, arena boundary tests, and exception diagnostics.
3. Finalize the xHCI and USB enumeration state machines.
   Exit: Obtain directly connected keyboard input in QEMU and on hardware, and diagnose timeouts.
4. Finalize BOT / FAT32 constraints and recovery.
   Exit: Match known-file lengths and hashes, and reject corrupt images.
5. Finalize initialization for each NIC and lwIP integration.
   Exit: Verify ARP / ICMP / UDP round trips and diagnose link disconnection in all three environments.
6. Run integration tests with the same image and check publication requirements.
   Exit: Document required-feature passes, known limitations, licenses, and reproducibility instructions.

### Test design

Host tests cover memory-range arithmetic, FAT / GPT parsing, ring wraparound,
handle generations, and time-counter wraparound, including invalid sizes, cycles, and disconnections.
Pin the QEMU UEFI + xHCI + USB disk + USB keyboard + e1000e configuration.
Include timeouts and damaged images, checking expected diagnostics and stop / continuation behavior.

Test NUC5 and NUC8 separately. Verify the same image SHA-256,
and perform 10 cold boots, 10 reboots, and a 30-minute simultaneous input / file-read / UDP test on each machine.
Do not write to internal SSDs. Disable Secure Boot for initial tests; handle signing separately.
Record firmware versions, settings, PCI IDs, USB devices, display formats, RAM, and peer devices.
Passing requires every mandatory condition in the baseline policy.
Do not declare support for all 0.1.0 targets if hardware is unavailable or untested.

### Immediate next work

Port reset, Enable Slot, Address Device, and Device Descriptor transfers are implemented.
Configuration Descriptor, Set Configuration, and Boot Keyboard Interrupt IN diagnostics are implemented.
Continuous input from a single keyboard is connected to the API version 2 FIFO.
USB BOT capacity and first / last sector read diagnostics have been added.
MBR / FAT32 short-name root-file reads have been added. GPT validation and shared-image generation are implemented. Next, implement the application file API.
The application C ABI and file / UDP APIs are also incomplete.
HPET fallback for machines without a supported PM timer remains.
Cross-check USB / NIC register specifications against primary sources and collect hardware PCI diagnostics.
Update documents on the design branch and implement units as their designs become ready.

### Agreed practices and remaining checks

- The first validation application is a diagnostic application displaying `Hello Musha-OS!`.
  Show display, input, arena, file, and UDP diagnostic results on one screen.
- The user has secured a NUC5 and 32GB USB drive. NUC8 procurement / testing status is unknown.
  Record machine model, RAM, and USB devices before hardware tests.
- The user authorized GitHub pushes. Checking musha-ic-prog's owner showed the individual
  `hirofumi-iwasaki` account. Public repository creation under the same owner was approved;
  https://github.com/hirofumi-iwasaki/musha-os has been created.
  The initial main and design/0.1.0 pushes are complete.

The implementer can finalize the following during detailed design without asking the user to choose every item:
Rust version, UEFI crate, lwIP version, crate layout, CPU transition, memory attributes,
DMA placement, API types, driver state machines, timeouts, queue capacities, QEMU, and image-generation procedures.
If API, capacity, or CPU requirements change, document the reasons and validation criteria.

---

## 日本語

**Musha-OS 0.1.0 実装計画と設計完了条件**

2026-10-08時点: 基本方針とアーキテクチャは文書化済み。
詳細設計・実装を継続中。GOP直接描画、ExitBootServices、専用スタック移行は
QEMUで確認済み。自前GDT / IDT / TSS、例外診断、ページテーブル、予約領域管理、
64MiB arenaとPCI列挙（xHCI / Intel 82574識別）はQEMUで確認済み。ACPIのPM timer情報引継ぎと100ms計測はQEMUで確認済み。
RustアプリAPIとxHCI停止・リセット、専用DMA / ring、No-Op 600回までQEMUで確認済み。
USB列挙・入力・読出し診断とGPTイメージ生成は実装済み。HPET、アプリ向けfile API、NICドライバと実機試験は未完了。
0.1.0タグはまだ作らない。

### 設計状況

| 領域 | 決定済み | 実装前に残る設計 |
|---|---|---|
| 言語 / ビルド | Rust no_std、C版lwIP、最小限のアセンブリ | ツール版、crate選定、CとのABI、CPU機能設定 |
| 起動 | 単一EFI、静的リンク、UEFI終了後自主管理 | ライブラリ選定、boot_infoのRust型・FFI定義、切替アセンブリ |
| メモリ | 恒等マップ、予約範囲除外、単一arena | 実機PAT / MTRR適合性、DMA容量、動的拡張 |
| CPU / 時間 | BSPのみ、ポーリング、例外診断 | PM timer / HPET手順とtimeout、FPU / SIMD状態、例外試験の拡張 |
| アプリ | 協調step、非同期I/O、版付きcontext | Rust APIとCヘッダー、handle・バッファ上限 |
| USB | xHCI、boot keyboard、BOT | リング/context仕様、状態機械、復旧手順 |
| FAT32 | 読出し専用、8.3名 | BPB制約、GPT検証、破損時の処理 |
| 有線LAN | 機種別初期化、lwIP NO_SYS、固定IPv4 | 各NICのregister / PHY仕様、PCI allowlist |
| 配布 | GPT + FAT32 ESP、同一USBイメージ | ツール版、再現ビルド、QEMU実行設定 |
| OSS | 独自コード・文書・設定はApache-2.0 | 依存ライセンス、配布時の表示 |

設計完了とは、上記の実装前項目を文書またはヘッダーで確定し、
各状態遷移の正常系・timeout・切断・不正入力時の挙動を説明できる状態をいう。
実機適合性の確定は設計完了とは別のリリース条件である。

### 作業順序と出口条件

1. ビルド仕様・起動情報・CPU遷移の詳細設計を確定する。
   出口: EFI生成、GOP表示、ExitBootServices後の自前スタックで動作。
2. メモリ、ページ属性、arena、時間源、アプリAPIを確定する。
   出口: 予約範囲の重複検査、arena境界試験、例外診断が通る。
3. xHCIとUSB列挙の状態機械を確定する。
   出口: QEMUと実機で直結キーボード入力、timeout時の診断ができる。
4. BOTとFAT32の制約・復旧を確定する。
   出口: 既知ファイルの長さとハッシュが一致し、破損イメージを拒否する。
5. NICごとの初期化とlwIP接続を確定する。
   出口: 3環境でARP / ICMP / UDP往復、リンク切断の診断ができる。
6. 同一イメージで統合試験し、公開条件を確認する。
   出口: 必須機能合格、既知制約、ライセンスと再現手順を文書化。

### 試験の設計

ホスト試験ではメモリ範囲算術、FAT / GPT解析、リングwrap、handle世代、
時間カウンタwrapを扱う。不正サイズ、循環、切断を含める。
QEMUではUEFI + xHCI + USB disk + USB keyboard + e1000e構成を版固定する。
タイムアウトや壊れたイメージを含め、期待する診断と停止・継続を検査する。

実機ではNUC5とNUC8を個別に試験する。同一イメージのSHA-256を確認し、
各機10回のcold boot、10回の再起動、30分の入力・ファイル読出し・UDP同時試験を行う。
内部SSDへ書込まない。Secure Bootは初期試験で無効とし、署名対応は別途扱う。
ファームウェア版、設定、PCI ID、USB機器、画面形式、RAM、相手機器を記録する。
合格判定は基本方針書の必須条件をすべて満たすこととする。
実機未入手・未試験の場合は0.1.0の全対象対応を宣言しない。

### 直近の次作業

port reset、Enable Slot、Address Device、Device Descriptor転送まで実装済み。
Configuration Descriptor、Set Configuration、Boot KeyboardのInterrupt IN診断まで実装済み。
単一キーボードの継続入力とAPI版2のFIFOを接続済み。USB BOTの容量・先頭 / 末尾セクタ読出し診断を追加済み。MBR / FAT32の短名ルートファイル読出しを追加済み。GPT検証と共通イメージ生成は実装済み。次はアプリ向けfile APIへ進める。
アプリのC ABI、ファイル・UDP APIも未完了。
PM timer非対応機向けのHPET fallbackも残る。
USB / NICのレジスタ仕様は一次資料に照合し、実機PCI診断情報を集める。
設計ブランチで文書を更新し、実装可能になった単位から実装へ進む。

### 合意した運用と残る確認

- 最初の検証アプリは診断アプリとし、`Hello Musha-OS!` を表示する。
  画面・入力・arena・ファイル・UDPの診断結果を一画面で確認する。
- NUC5と32GB USBは利用者が確保済み。NUC8は調達・試験状況未確認。実機試験前に機種・RAM・USB機器を記録する。
- GitHubへのpushは利用者から許可された。musha-ic-progの所有者を確認した結果、
  `hirofumi-iwasaki` 個人アカウント配下だった。同じ所有者での公開作成が承認され、
  https://github.com/hirofumi-iwasaki/musha-os を作成済み。mainとdesign/0.1.0の初回pushを完了した。

以下は担当者が詳細設計で確定でき、すべてを利用者に選択してもらう必要はない。
Rust版、UEFI crate、lwIP版、crate構成、CPU切替、メモリ属性、DMA配置、
APIの型、ドライバ状態機械、timeoutとキュー容量、QEMUとイメージ生成手順。
API / 容量 / CPU要件に変更が生じた場合は文書へ理由と検証条件を記録する。
