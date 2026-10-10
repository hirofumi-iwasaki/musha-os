# v0.1.0 release scope / 公開範囲

Decision: 2026-10-10, approved by the project owner for the MI68 announcement.
This is an initial development release, limited to demonstrated configurations.
This decision supersedes the original v0.1.0 acceptance requirements in
policy-v0.1.md and implementation-plan-0.1.0.md; their broader targets remain a roadmap.

## Accepted scope

- x86-64 UEFI boot, GOP display, protected runtime memory, ACPI PM timer and PCI discovery.
- Own xHCI, one selected external USB Boot Keyboard, read-only BOT storage and GPT/FAT32 MUSHA.TXT snapshot reads.
- Conditional restoration of the boot-owner PCI controller after ExitBootServices, with resource/bridge/peer validation and DMA disabled during restoration.
- QEMU e1000e/82574 ARP, ICMP and UDP diagnostics; physical NUC networking is not included.
- Normal EFI built from clean source without debug, fault-injection or PHY-probe features; packaging, host and QEMU regressions must pass.
- MacBook Pro 2018 H19: SanDisk 0781:55A9 storage/FAT32/file reads and Lenovo 17EF:6009 A, Shift+A, Esc input in the same session. Record source/binary correspondence in release-validation-0.1.0.md.

## Deferred criteria and limitations

NUC5/NUC8 acceptance, I218-V/I219-V networking, ten cold boots/ten restarts,
30-minute integrated stability and universal port/hub compatibility remain unverified.
Ryzen H4 results are historical evidence; H19 has not been re-tested there.
The Mac internal keyboard is not validated. Only one selected keyboard is polled;
simultaneous multi-keyboard input is not supported. USB3 hubs and high-speed TT paths
are not validated. No general-purpose filesystem writes or internal SSD support.

Physical tests used files copied to a dedicated FAT32 USB, rather than the same
64MiB raw image used by QEMU. The shared deliverable is the normal EFI plus MUSHA.TXT.
The 64MiB image is for QEMU only; physical media require the file-copy procedure
or regeneration for the exact device capacity. See usb-install-macos.md.

## 日本語

2026-10-10、所有者の指示により、MI68向けの初期開発版としてv0.1.0の範囲を確定。
当初のNUC5/NUC8共通イメージ・有線LAN・反復起動・長時間試験をすべて満たす版から、
実証済み機能と構成に限定した版へ変更した。理由は、MacBook Pro 2018でH19の
USB読出しと外付けLenovo入力が成立し、現時点の成果を公開するため。
旧方針書・実装計画の公開条件は本書を優先し、未達項目は将来の検証課題として保持する。
内蔵キーボード、未検証NUC、全ポート、複数キーボード、長時間安定性への対応を宣言しない。
配布物は通常EFI・MUSHA.TXT・QEMU専用64MiBイメージ・生成ツール・文書・ライセンス・チェックサム。
ホスト試験、QEMU回帰、クリーンビルド、実機H19との対応確認を公開前に記録する。
