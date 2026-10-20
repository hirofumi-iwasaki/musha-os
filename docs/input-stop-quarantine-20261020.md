# Input stop quarantine / 入力停止時の隔離

The `app-recovery-stop-failure` probe first runs the real xHCI application stop path. It then forces the recovery decision to treat stop as unconfirmed, so the app arena is quarantined before callback unwinding, scrubbing, or viewer restart. It does **not** simulate a physical controller refusing to halt.

The split-controller QEMU run at `/private/tmp/musha-five-offline-20261020/r2-split-stop-failure-current5/report.json` confirmed three distinct `APP_INPUT_QUIESCED BDF=` records after `APP_INPUT_LIVE`, followed by `APP_INPUT_STOPPED DMA_DISABLED` and `APP_RECOVERY_STOP_UNCONFIRMED QUARANTINED`. No `APP_ARENA_SCRUBBED` or `APP_RECOVERED` followed. Each BDF record is emitted only after controller halt and PCI bus-master disable are both confirmed. A normal split-controller return also passed at `/private/tmp/musha-five-offline-20261020/r2-split-current2/report.json`.

`app-recovery-stop-failure` probeは実際のxHCIアプリ停止処理を先に行い、その後、復旧判定だけを停止未確認として扱う。callbackの巻き戻し、arena消去、viewer再起動より前にarenaを隔離する。物理controllerの停止失敗を模擬した試験ではない。

分離した三つのcontrollerを用いたQEMU試験では、`APP_INPUT_LIVE`の後に異なる三つの`APP_INPUT_QUIESCED BDF=`、次に`APP_INPUT_STOPPED DMA_DISABLED`、最後に`APP_RECOVERY_STOP_UNCONFIRMED QUARANTINED`を確認した。その後の`APP_ARENA_SCRUBBED`と`APP_RECOVERED`はない。各BDF記録はcontroller haltとPCI bus master無効化の両方が確認された後にのみ出る。通常復帰も同じ分離構成で合格した。
