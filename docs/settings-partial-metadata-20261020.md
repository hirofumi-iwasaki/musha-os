# FAT32 metadata partial transfer test (2026-10-20)

The `settings-write-partial-metadata` development feature targets the first FSInfo sector selected by `settings::preflight` on a disposable 4096-byte-sector FAT32 image. The first data sector and both FAT patches complete before the fault. During the FSInfo WRITE(10), xHCI sends one complete 1024-byte bulk OUT packet of the requested 4096 bytes. The probe waits for that packet's completion, cancels the BOT transaction, keeps its buffers reserved, and leaves the controller to stop and disable DMA. This is a transport DATA-phase interruption, not a simulated block-device error.

The QEMU run in `/private/tmp/musha-five-offline-20261020/fat-partial-qemu-verified` recorded FSInfo LBA `1`, `BYTES=0x400/0x1000`, `Err(RecoveryRequired)`, and then `XHCI_QUIESCED DMA_DISABLED`. The writer attempted rollback, but BOT remained occupied without a CSW, so rollback could not start. No later data transfer was submitted before controller shutdown. The test verifies these events and stores before/after copies and SHA-256 hashes for both FAT sectors, both FSInfo sectors, and the SETTINGS directory sector in `result.json`.

Only sectors `32` and `96` (the two FAT copies) and `163` (new record data) changed. FSInfo sectors `1` and `7` and directory sector `161` remained byte-identical. This QEMU USB-storage model did not commit the incomplete FSInfo sector write to the image; that observation does not prove that other devices cannot commit part of a sector. `fsck_msdos -n` returned `0` before and `206` after, reporting one orphan cluster and an FSInfo free-count mismatch (`65522` versus `65521`). The result is explicitly classified as needing recovery, not a clean filesystem. The final image SHA-256 is `9f588344b39c3b8c4316ab3efba75222058ffb68a22d002113a5ed4a747da5f8`.

The worktree host `musha-fs` suite passed 74 tests (including pre-existing, unstaged filesystem tests). Its recoverable-error test fails each write in turn and verifies that all metadata returns to its prior bytes; its persistent-failure test requires `RecoveryRequired`. A normal `settings-write-probe` QEMU run on the same 4096-byte geometry also passed, changed six expected sectors, and returned a clean fsck status of `0`. The QEMU partial-transfer result exercises the distinct case where transport failure prevents rollback from starting.

## 日本語

開発用feature `settings-write-partial-metadata` は、破棄可能な4096バイト/sectorのFAT32画像で、`settings::preflight` が選んだ最初のFSInfo sectorを対象にする。データsectorとFATの両面を先に書き終え、FSInfoのWRITE(10)では4096バイト要求のうち1024バイトのbulk OUT packetを1個完了させる。完了を確認してBOT処理を中断し、bufferを保持したままcontrollerを停止してDMAを無効化する。これはblock deviceの人工的なI/O errorではなく、USB DATA phaseでの部分転送である。

QEMUの記録にはFSInfo LBA `1`、`BYTES=0x400/0x1000`、`Err(RecoveryRequired)`、その後の`XHCI_QUIESCED DMA_DISABLED`がある。CSWのないBOT処理が占有状態のためrollbackの書き込みは開始できず、停止まで追加のDATA転送もない。検証器はこの順序を確認し、両FAT sector、両FSInfo sector、SETTINGS directory sectorの前後の実データとSHA-256を保存する。

媒体上で変化したのはFAT両面のsector `32`、`96`と新しいrecord dataのsector `163`だけだった。FSInfoの`1`、`7`とdirectoryの`161`は同一バイトである。今回のQEMU USB storageは未完了のFSInfo書き込みを画像に反映しなかったが、他の機器でも部分書き込みが起きないことは意味しない。`fsck_msdos -n` は実行前`0`、実行後`206`で、孤立clusterが1個とFSInfo空き数の不一致（`65522`対`65521`）を報告した。この結果は修復が必要な状態と分類する。

作業中worktreeのhost `musha-fs` suiteは74件合格した（stageしていない既存のfilesystem testも含む）。回復可能なerrorのtestは各書き込み位置で失敗させ、metadataが元のバイト列に戻ることを確認する。永続的な失敗では`RecoveryRequired`を要求する。同じ4096バイトgeometryでの通常の`settings-write-probe` QEMU試験も合格し、想定する6 sectorだけが変化してfsckのstatusは`0`だった。今回の部分転送試験は、転送中断によってrollback自体を開始できない別の経路を確認した。
