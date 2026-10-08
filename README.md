# Musha-OS

## English

Musha-OS is a thin bare-metal execution environment based on x86-64 / UEFI.
It aims to make use of older PCs and provide applications with a display, input,
USB storage, wired networking, and a RAM region they can use freely.

The oldest hardware reference is the Intel NUC5i5RYH / RYK; the comparison machine
is the NUC8 (NUC8i5BEH). The goal is to boot the same USB image in QEMU and on both machines.

The implementation primarily uses Rust (`no_std`), with C for lwIP and assembly
for operations such as switching CPU execution state.

### Current status

Implementation has begun. Direct greeting output through GOP and the display of
`RUNTIME READY` after exiting UEFI Boot Services and switching to a dedicated stack
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
The normal build ends the diagnostic session with Esc.
USB BOT capacity queries and first / last sector reads have been verified in QEMU.
See [storage diagnostics](docs/usb-storage.md).
[FAT32 read diagnostics](docs/fat32.md) have been added; reading MUSHA.TXT from the root has been verified in QEMU.
The application file API and LAN communication are not yet implemented.
The first diagnostic application displays `Hello Musha-OS!` and is designed to
check the display, input, RAM, file, and UDP status.
See [development and build instructions](docs/development.md).

### Policy documents

- [Work plan before hardware arrival](docs/pre-hardware-work-plan.md)

- [Musha-OS baseline policy 0.1.0](docs/policy-v0.1.md)
- [0.1.0 architecture design](docs/design-0.1.0.md)
- [0.1.0 implementation plan and design completion criteria](docs/implementation-plan-0.1.0.md)

The first release number is `0.1.0`. Design and implementation are ongoing;
this version has not been released.

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

Musha-OSは、x86-64 / UEFIベースの薄いベアメタル実行環境です。
古いPCを活用し、アプリケーションに必要な画面、入力、USBストレージ、
有線ネットワークと自由に使えるRAM領域を提供することを目指します。

最古リファレンスはIntel NUC5i5RYH / RYK、比較機はNUC8（NUC8i5BEH）です。
QEMUと両実機で同一のUSBイメージを起動する構成を目標とします。

実装はRust（`no_std`）を主体とし、lwIPはC、CPU切替などはアセンブリを使用します。

### 現在の状態

実装を開始しました。GOPで挨拶を直接表示し、UEFI終了・専用スタックへの切替後に
`RUNTIME READY` を表示するところまでQEMUで確認済みです。
自前GDT / IDT / TSSとCPU例外診断を実装し、QEMUで#UD / #GP / #DFを確認済みです。
自前ページテーブルと予約メモリ管理、RAM arenaも実装し、
QEMUで64MiB arenaとページ保護を確認済みです。
PM timer、PCI列挙、xHCI所有権移行・リセット・DMA / ring診断とRustアプリのstep実行を追加しました。
QEMUでNo-Op 600回の完了、リング周回と停止を確認済みです。
USBポートをリセットし、USBメモリとキーボードのDevice Descriptor読出しを確認済みです。
[USB列挙の範囲と制約](docs/usb-enumeration.md)を参照。Boot Keyboardの押下・解放診断をQEMUで確認済みです。
[キーボード診断](docs/usb-keyboard.md)と[NUC5試験準備](docs/nuc5-bringup.md)を参照。
単一キーボードからアプリへの継続入力を追加しました。通常版はEscで診断を終了します。
USB BOTの容量取得と先頭・末尾セクタ読出しをQEMUで確認済みです。
[ストレージ診断](docs/usb-storage.md)を参照。[FAT32読出し診断](docs/fat32.md)を追加し、ルートのMUSHA.TXTをQEMUで確認済みです。
アプリ向けファイルAPIとLAN通信は未実装です。最初の診断アプリは `Hello Musha-OS!` と表示し、
画面・入力・RAM・ファイル・UDPの状態を確認する構成です。
[開発・ビルド手順](docs/development.md)を参照してください。

### 方針書

- [実機到着前の作業方針](docs/pre-hardware-work-plan.md)

- [Musha-OS 基本方針 0.1.0](docs/policy-v0.1.md)

- [0.1.0 アーキテクチャ設計](docs/design-0.1.0.md)
- [0.1.0 実装計画と設計完了条件](docs/implementation-plan-0.1.0.md)

最初のリリース番号は `0.1.0` です。現在は設計と実装を継続中で、リリース済みではありません。

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
