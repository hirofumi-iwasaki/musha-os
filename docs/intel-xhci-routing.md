# Intel shared-port routing and attach settling

## English

The NUC5 cold-boot failure on EFI `41a61ef...` is owner-reported. Routing to EHCI and late connection after reset are hypotheses; neither is proven fixed on hardware. Source reference: [Linux pci-quirks.c, usb_enable_intel_xhci_ports](https://github.com/torvalds/linux/blob/master/drivers/usb/host/pci-quirks.c) (consulted 2026-10-13). This is an independent implementation of register operations, not copied Linux source.

After ExitBootServices and successful xHCI firmware ownership handoff, before halt/reset/ring initialization, only vendor 8086 / class 0C0330 with an Intel EHCI companion can route ports. The Linux Sony 104D:90A8 exception is retained. Non-Intel controllers do not access these routing offsets. No EHCI driver is running in Musha-OS. A zero mask means no write for that pair; all-ones routing or mask reads are treated as unavailable. An unimplemented register pair reading zero is untouched. No BAR sizing, PCI relocation, EHCI register write, or device-ID spoofing is performed.

Read D0/D4/D8/DC. Set DC-selected bits in D8 first (SuperSpeed), then D4-selected bits in D0 (USB2), preserving all bits outside each mask by read-modify-write. Readback checks only the requested mask, with an immediate readback and final four-register snapshot. Mismatch is recorded, not silently considered a successful route and not followed by unbounded retries. Routing does not guarantee that a device is attached. Diagnostics journal includes `INTEL ROUTE BEFORE/AFTER`, writes/match, and each port's `ROUTE BEFORE/AFTER ... PORTSC`. Existing final port status lines are retained.

After port power setup, sample every 20 ms using the shared power epoch. Empty ports remain provisional for 150 ms, or 500 ms when routing actually wrote mask-selected bits. After grace, only pending attachment extends the shared wait, up to 1000 ms. Already-routed bits are not rewritten. SuperSpeed settlement shares this grace; it is not charged twice. The one-shot no-device rescan and unchanged safety deadlines are specified in [USB startup timing](usb-startup-timing.md). These margins still require NUC5 hardware validation. QEMU qemu-xhci is non-Intel and does not validate physical routing.

Host tests cover identity/companion/exception gates, zero/unavailable register pairs, independent masks, SuperSpeed-first ordering, readback mismatch and attach policy. QEMU cannot validate Intel electrical routing; the new candidate still needs owner cold-boot testing. The normal EFI also records the journal; the detailed diagnostic build displays its pages. ABI and file/media selection behavior are unchanged.

## 日本語

NUC5の失敗原因は未確定です。Intelの共有ポート切替と、リセット後の遅延接続を調べる変更です。ExitBootServicesと所有権取得の後、xHCI halt/resetの前に、Intel xHCIかつIntel EHCIが存在する場合だけ適用します。Linuxと同じSony除外条件も保持します。マスク0または読出し不能（全ビット1）は書き込まず、非Intelにはルーティングレジスターを読み書きしません。

D8のDC指定ビットを先に立て、次にD0のD4指定ビットを立てます。読出し・変更・書込みによりマスク外ビットを保持し、読戻しはマスク内だけを比較します。即時読戻しと前後4レジスター値、各PORTSCを診断記録に残します。接続待ちはPP時刻から20ms間隔で、通常150ms・実際に切替を書いた場合500msは空きを確定しません。猶予後は接続処理中の場合だけ共通上限1000msまで延長します。切替済みビットは再書込みせず、SuperSpeed待ちも同じ猶予を共有します。未検出時1回の再探索と安全期限は[起動時間記録](usb-startup-timing.md)を参照。猶予値の実機確認は未完了です。無期限待ちやPORTSCの状態ビット再書込みはありません。

QEMUではIntel固有の切替は実行されません。回帰合格は実機での問題解消を意味せず、新候補での冷間起動確認が必要です。ABIや媒体選択規則は変更しません。
