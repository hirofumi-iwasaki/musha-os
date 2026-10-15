# H18 MacBook Pro実機結果

利用者提供IMG_7516〜IMG_7526。全11ページを確認。記録日2026-10-10。

## 到達点

起動、runtime/arena/timer、診断セッション完了を確認。USB NO KBD/STORAGEは継続。00A0は26ポート走査に到達したが全CCS=0。0700はNOT BOOT OWNERのため復旧しない。boot owner候補7D00はPCI RECOVERYでRESOURCE UNKNOWN。

H17の最初の拒否peer 01:00.0 / BAR2 / DESCRIPTOR TYPEから、H18では00:1f.5 / BAR0 / BAR BASE MISMATCHへ移った。先の番号修正で旧拒否点を通過したことを示すが、全peer検査成功やUSB復旧成功ではない。

## 新しい拒否の根拠

IMG_7522〜7524:

- peer segment 0、bus 0、device 0x1f、function 5。
- config ID DWORD A3248086、class DWORD 0C800010、command/status 00000402、header 00000000。
- BAR0 FE010000（32bit memory、prefetch 0）。BAR1〜5は0。
- GetLocation status 0、GetBarAttributes status 0、NULL 0。
- BEFORE/AFTER/REPEATは全10項目status 0で一致。CF8/CFCも全10項目MATCH 1。
- 記述は構造正常、TYPE 0、GENERAL 0、SPECIFIC 0、GRANULARITY 0x20、MIN 0x81617000、MAX 0xFFF、TRANSLATION 0、LENGTH 0x1000。

設定基底0xFE010000と記述基底0x81617000が一致しないため、H18で追加した安全条件が復旧を拒否した。BAR0なので64bit上位枠による番号ずれでは説明できない。当該呼出し前後のconfig変動や読取り経路差も観測していない。

記述範囲[0x81617000, 0x81618000)は、0700の終了前BAR0基底0x81700000とも、復旧対象7D00の0x8F900000とも異なる。既存の他機器資源との関連、ファームウェアの記録とconfigの差、protocol/handle対応の問題は未確定。種別・長さが正常であることだけで、この記述を正しい資源として採用しない。

## 次の検査設計

復旧許可条件を維持し、Boot Services終了前の読取り専用証拠を増やす。

1. 同じhandle・protocolに対し、GetLocation、ID/class/header、全BARを早期と資源検査時に保存し、対応と時点の差を確認する。
2. 最初の不一致peerだけに限定し、論理資源番号0〜5のstatus・NULL・記述種別/base/lengthを固定上限で収集する。各非NULL戻り領域はfree_poolし、失敗を別資源の成功で置換しない。
3. 番号・基底・範囲から同一peerのconfigとの対応を確認。読み取りの途中でconfigが変わる場合も記録する。関連peerの一致は診断情報であり、不一致を許可へ変えない。
4. PCIIO経路とCF8/CFC経路の同一機器照合を維持。複数segmentや曖昧な対応は復旧拒否を維持する。

新規PCI設定書込み、BARサイズprobe、ファームウェア独自例外、MMIOの試し読みは追加しない。正常に初期化される他機種の経路を変えず、固定容量の保存領域と表示上限を設計する。単一・複数コントローラー回帰試験とRyzen実機確認が必要。

H19の実装・USB更新は今回行っていない。

## 調査後の対処案

追加診断案から、一次資料と範囲計算の検証を踏まえた[H19対処案](pci-h19-peer-resource-plan.md)へ更新。直接の拒否原因を特定。初回の判定変更は自動承認レビューに拒否されたが、その後に利用者がリスクを理解して明示承認し、H19条件付き復旧を実装・回帰検証した。H19の実機成功は未確認。
