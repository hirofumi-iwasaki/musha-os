# Musha-OS v0.1.0 release notes (draft)

## English

Unreleased. Do not create a tag or GitHub Release yet. Finalize this document
only after release scope and acceptance criteria are confirmed and candidate
testing is complete.

### Overview

A thin x86-64/UEFI bare-metal environment, primarily Rust no_std. After exiting
UEFI Boot Services, its own runtime handles display, memory and devices. lwIP
uses C, and CPU transitions use minimal assembly. The diagnostic app is statically
linked; a general desktop OS or external binary loader is not provided.

### Implemented functions (not a claim of hardware validation)

- UEFI boot, direct GOP rendering, dedicated stack, CPU tables and page tables.
- Exception diagnostics, page protection and a RAM arena excluding reserved memory.
- ACPI PM timer, PCI enumeration, own xHCI and continuous input from one direct Boot Keyboard.
- Read-only USB BOT storage, GPT/FAT32 and root MUSHA.TXT reads.
- A read-only snapshot file API limited to 4KiB.
- QEMU e1000e/82574 and lwIP static IPv4 ARP, ICMP and UDP echo diagnostics.
- Cooperative input, snapshot file reads and QEMU networking on one boot CPU.
- A persistent normal-build hardware panel; Esc ends the session.

### Validation (complete before release)

| Item | Result |
|---|---|
| Release commit/tag | Not frozen / not created |
| EFI/image SHA-256 | Record candidate manifest.json and SHA256SUMS |
| Clean Mac checkout build | See mi68-preparation-results.md |
| QEMU regression | See mi68-preparation-results.md; diagnostic and normal builds are tested separately |
| NUC5 firmware/RAM/USB configuration | Not verified |
| NUC5 input/file/RAM/repeated boot | NOT RUN |
| NUC5 I218-V traffic | Not implemented / NOT RUN |
| NUC8 I219-V and other physical tests | Not implemented / NOT RUN |
| All current policy criteria | Not met |

### Limitations

The target is a verified x86-64/UEFI/ACPI/PCI/xHCI/GOP configuration. Legacy BIOS,
CSM or EHCI-dependent systems and all older PCs are not supported. Disable Secure
Boot for initial testing and connect USB directly. GOP supports 32-bit RGB/BGR;
no HPET fallback exists.

NUC I218-V/I219-V networking, an application UDP API, general asynchronous media
access, arbitrary file writes, a C ABI and external binary compatibility are not
provided. TCP, DHCP, IPv6, Wi-Fi, Bluetooth, audio, AHCI/NVMe, GPU acceleration,
SMP and POSIX are outside scope. No performance or real-time guarantees have been
validated; application steps must return promptly.

NET TEST OK alone does not prove communication. Use real transmitted/received
packet tests; never describe unsupported NUC LAN as successful networking.

### Distribution, boot and recovery

Prepare a normal EFI, QEMU 64MiB image, generator, licenses, manifest and checksums
with tools/prepare-release.py. Generation is not publication. The bundle includes
root MUSHA.TXT for FAT32 file-copy boot. See test-usb-build.md and usb-install-macos.md.
Do not raw-copy the 64MiB image onto a 32GB USB; regenerate for its exact capacity.
Record the last displayed line, errors and connections if boot fails. After the
session stops, power off, remove the USB and select the normal boot medium.

### License

Original code, documents and configuration are Apache-2.0 unless separately
specified. lwIP and the BSD-derived I218 port retain their own terms. Consult
LICENSE, NOTICE, third-party documentation and bundled notices. Do not label all
code uniformly Apache-2.0.

## 日本語


状態: 未公開。タグとGitHub Releaseはまだ作成しない。
公開範囲・合格条件の確定と、候補commitの試験完了後に本書を確定する。

## 概要

Rust（no_std）を主体にした、x86-64 / UEFI向けの薄いベアメタル実行環境。
UEFI Boot Servicesを終了した後、独自ランタイムが画面・メモリ・機器を扱う。
lwIPはC、CPU状態切替は最小限のアセンブリを使用する。
検証アプリは静的リンクされる。一般的なデスクトップOSや外部バイナリローダーではない。

## 実装済みの機能（実機対応の宣言とは別）

- UEFI起動、GOPへの直接描画、専用スタックと独自CPUテーブル・ページテーブル。
- CPU例外診断、ページ保護、予約領域を除外したRAM arena。
- ACPI PM timer、PCI列挙、独自xHCI、直結USB Boot Keyboard 1台の継続入力。
- USB BOTストレージの読出し、GPT/FAT32、ルートのMUSHA.TXT読出し。
- 最大4KiBの読出し専用snapshot file API。
- QEMU e1000e/82574とlwIPによる固定IPv4、ARP、ICMP、UDP echo診断。
- 単一起動CPUの協調stepによる入力・snapshot file read・QEMU通信の同時進行。
- 通常版の実機診断パネル。Escで診断を終了する。

## 検証結果（公開前に埋める）

| 項目 | 結果 |
|---|---|
| リリースcommit / tag | 未確定 / 未作成 |
| 配布EFI / image SHA-256 | 候補生成後、manifest.jsonとSHA256SUMSを転記 |
| Mac新規checkoutからのbuild | 未実施 |
| QEMU候補の回帰試験 | 今回の結果はmi68-preparation-results.mdを参照 |
| NUC5 / firmware / RAM / USB構成 | 未確認 |
| NUC5入力・file・RAM・反復起動 | 未実施 |
| NUC5 I218-V通信 | 未実装・未実施 |
| NUC8 I219-V通信とその他実機試験 | 未実装・未実施 |
| 現行policyの全合格条件 | 未達 |

## 主な制約

対象はx86-64、UEFI、ACPI、PCI/PCIe、xHCI、GOPを備える検証済み構成。
Legacy BIOS/CSM/EHCI前提の構成、全ての古いPCへの対応は提供しない。
初期試験はSecure Bootを無効にする。USBはまずハブを使わず直結する。
GOPは32bit RGB/BGRのみ。HPET fallbackは未実装。

NUCのI218-V/I219-V通信、アプリ用UDP API、一般的な非同期媒体アクセス、
任意ファイルの書込み、C ABI、外部バイナリ互換性は未提供。
TCP、DHCP、IPv6、Wi-Fi、Bluetooth、音声、AHCI/NVMe、GPU加速、SMP、POSIXは対象外。
性能比較・リアルタイム保証は未実施。アプリのstepは速やかに返る必要がある。

`NET TEST OK`だけでは実通信の成功ではない。送受信packet試験で判断する。
NUCのLAN未対応表示を正常通信と説明しない。

## 配布・起動

`tools/prepare-release.py` で通常版EFI、QEMU向け64MiBイメージ、生成ツール、
ライセンス、manifestとSHA256SUMSをまとめる。生成だけで公開完了とはしない。
64MiB imageを32GB USBへそのままrawコピーしない。実媒体と正確に同じ容量の
イメージ生成と書込みは `usb-install-macos.md` を参照。

失敗した場合は最後の表示行、エラー、接続構成を撮影して保存する。
診断終了後は電源を切り、USBを抜いて通常の起動媒体を選び直す。

## ライセンス

独自コード・文書・設定はApache License 2.0（個別指定を除く）。
lwIPとBSD由来のI218移植は元の条件を保持する。LICENSE、NOTICE、third-party文書、
同梱した第三者ライセンスを参照。全コードを一律Apache-2.0とは表示しない。
