# Musha-OS v0.1.0

## English

First public development release for MI68. A thin Rust no_std x86-64/UEFI bare-metal
runtime, with C lwIP and minimal assembly. The diagnostic application is statically linked.

- Own post-UEFI runtime, GOP display, memory protection, RAM arena and exception diagnostics.
- ACPI PM timer, PCI discovery, sequential multi-controller xHCI enumeration and bounded USB hub discovery.
- One selected USB Boot Keyboard, read-only USB BOT/GPT/FAT32, and a maximum 4KiB file snapshot API.
- QEMU e1000e/82574 ARP, ICMP and UDP diagnostics with cooperative input/file/network processing.
- H19 conditional boot-controller PCI recovery: MacBook Pro 2018 storage and external Lenovo keyboard demonstrated together.

Physical evidence: SanDisk 0781:55A9, Lenovo 17EF:6009, A / Shift+A / Esc, and
16-byte MUSHA.TXT with hash 9A42A948C590F507. See [validation](release-validation-0.1.0.md)
and [H19 results](pci-h19-hardware-results.md). Ryzen H4 results are historical,
not a claim of H19 revalidation. [Final scope](release-scope-0.1.0.md) replaces the original full NUC acceptance plan.

Known limits: Mac internal keyboard not validated; one selected keyboard only;
NUC5/NUC8 and I218-V/I219-V networking not validated/implemented; repeated-boot and
long-duration stability not established. USB3 hubs/high-speed TT paths are unverified.
No Legacy BIOS/CSM/EHCI, Wi-Fi, Bluetooth, audio, internal AHCI/NVMe, GPU acceleration,
SMP, POSIX, external binary loader or general file writes. No performance guarantees.

Download `musha-os-0.1.0.tar.gz` and verify `SHA256SUMS`. The bundle contains the
normal EFI, MUSHA.TXT, a QEMU-only 64MiB image, generator, documentation, licenses
and build provenance. Copy EFI/BOOT/BOOTX64.EFI and MUSHA.TXT to a dedicated FAT32
USB using [the installation instructions](usb-install-macos.md). Do not raw-copy
the 64MiB image onto a larger physical USB. Disable Secure Boot for initial testing.
Esc ends the diagnostic session; power off and remove the USB to return to normal boot.

Original work is Apache-2.0; dependencies retain their own licenses. See LICENSE,
NOTICE and bundled third-party notices. APIs and supported configurations remain experimental.

## 日本語

MI68に向けた最初の開発版です。UEFI終了後に独自ランタイムで画面・メモリ・USBを扱います。
MacBook Pro 2018でSanDisk USBのFAT32/MUSHA.TXT読出しと、外付けLenovoの
A・Shift+A・Esc入力を同じ実行で確認しました。H19ではUEFI終了時に失われた
起動元USBコントローラーのPCI設定を、資源・ブリッジ・他機器の検証を条件に復旧します。

QEMUでは入力・USB読出し・e1000e/82574のARP/ICMP/UDP診断を提供します。
内蔵キーボード、NUC5/NUC8、I218/I219通信、全ポート、複数キーボードの同時入力、
反復起動・長時間安定性は対応済みとしません。Ryzen H4の実機記録は過去版の結果です。
[公開範囲](release-scope-0.1.0.md)と[検証記録](release-validation-0.1.0.md)を参照してください。

配布アーカイブとSHA256SUMSをダウンロードして検証してください。通常版EFI、
MUSHA.TXT、QEMU専用64MiBイメージ、生成ツール、文書、ライセンス、生成元情報を同梱。
実USBは[導入手順](usb-install-macos.md)に従い専用FAT32へファイルを配置します。
64MiBイメージを大容量USBへそのまま書き込まないでください。Escで診断終了後、
電源を切ってUSBを外せば通常の起動媒体へ戻せます。
