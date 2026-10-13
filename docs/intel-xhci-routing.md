# Intel shared-port routing and attach settling

## English

The NUC5 cold-boot failure on EFI `41a61ef...` is owner-reported. Routing to EHCI and late connection after reset are hypotheses; neither is proven fixed on hardware. Source reference: [Linux pci-quirks.c, usb_enable_intel_xhci_ports](https://github.com/torvalds/linux/blob/master/drivers/usb/host/pci-quirks.c) (consulted 2026-10-13). This is an independent implementation of register operations, not copied Linux source.

After ExitBootServices and successful xHCI firmware ownership handoff, before halt/reset/ring initialization, only vendor 8086 / class 0C0330 with an Intel EHCI companion can route ports. The Linux Sony 104D:90A8 exception is retained. Non-Intel controllers do not access these routing offsets. No EHCI driver is running in Musha-OS. A zero mask means no write for that pair; all-ones routing or mask reads are treated as unavailable. An unimplemented register pair reading zero is untouched. No BAR sizing, PCI relocation, EHCI register write, or device-ID spoofing is performed.

Read D0/D4/D8/DC. Write DC mask to D8 first (SuperSpeed), then D4 mask to D0 (USB2), with immediate readback and a final four-register snapshot. Mismatch is recorded, not silently considered a successful route and not followed by unbounded retries. Routing does not guarantee that a device is attached. Diagnostics journal includes `INTEL ROUTE BEFORE/AFTER`, writes/match, and each port's `ROUTE BEFORE/AFTER ... PORTSC`. Existing final port status lines are retained.

After port power setup, wait in 20 ms steps, at most 50 iterations per controller (one shared 1 s budget, not per port). Sample CCS and USB3 PLS 5/7/8; finish early only if all described ports have connected/settled. Empty ports keep the shared window open for delayed devices. Final enumeration uses freshly read PORTSC with existing reset/speed rules. No polling write replays change bits. Existing stalled-clock and MMIO error handling stays active. QEMU qemu-xhci is non-Intel, so only the bounded timing change applies; its output/pixel regression must still pass.

Host tests cover identity/companion/exception gates, zero/unavailable register pairs, independent masks, SuperSpeed-first ordering, readback mismatch and attach policy. QEMU cannot validate Intel electrical routing; the new candidate still needs owner cold-boot testing. The normal EFI also records the journal; the detailed diagnostic build displays its pages. ABI and file/media selection behavior are unchanged.

## 日本語

NUC5の失敗原因は未確定です。Intelの共有ポート切替と、リセット後の遅延接続を調べる変更です。ExitBootServicesと所有権取得の後、xHCI halt/resetの前に、Intel xHCIかつIntel EHCIが存在する場合だけ適用します。Linuxと同じSony除外条件も保持します。マスク0または読出し不能（全ビット1）は書き込まず、非Intelにはルーティングレジスターを読み書きしません。

D8へDCのマスクを先に書き、次にD0へD4のマスクを書きます。即時読戻しと前後4レジスター値、各PORTSCを診断記録に残します。接続待ちは全ポート共通で最大1秒・20ms間隔、CCS/PLSを読み取り、最終値で既存の列挙処理へ進みます。無期限待ちやPORTSCの状態ビット再書込みはありません。

QEMUではIntel固有の切替は実行されません。回帰合格は実機での問題解消を意味せず、新候補での冷間起動確認が必要です。ABIや媒体選択規則は変更しません。
