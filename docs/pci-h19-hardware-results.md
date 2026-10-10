# H19 MacBook Pro 2018 実機結果

記録日: 2026-10-10（日本時間）。利用者が機種をMacBook Pro 2018と明示。IMG_7527〜IMG_7536、診断1/10〜10/10を確認。

## 初回実行の結論（外付け未接続）

起動担当USB制御器7D00の条件付きPCI復旧が成功し、OS独自USBドライバによるSanDiskの容量取得・セクタ読出し・FAT32・MUSHA.TXTのアプリ読出しまで実機で成功した。H18で止まっていた資源検査を通過したことを確認できる。キーボードは全コントローラーでK0であり、入力成功は未確認。USB全機器対応・全ポート動作・繰返し安定性の確認とは区別する。

## 初回実行の写真の証拠

- IMG_7533（7/10）: PEER BOUND 00:1F.5 BAR 0。LIVE FE010000 / MAX BYTES 10000、FW 81617000 / BYTES 1000。親ブリッジ78:02.0、7A:00.0、00:01.2の記録に続き、`PCI RECOVERED 7D00 BAR 8F900000 CMD 0002 DMA OFF`。復旧後のCAPABILITY、OWNERSHIP、HALT、RESET、RINGS / COMMANDSへ進み、root 4 USB3でCCS 1、PED 1、SPEED 4、PLS 0となって列挙を開始。
- IMG_7534（8/10）: DEVICE DESCRIPTOR、CONFIGURATION、STORAGE BOT / CAPACITY、STORAGE SECTOR READ、FAT32 FILE READを通過し、STOPPED DMA DISABLEDへ到達。
- IMG_7528、7531（2/10、5/10）: C2 / root 04 / route 00000 / slot 01、SanDisk 0781:55A9、SUPER、BOT STORAGE 1、BOOT KBD 0。記述1件。MUSHA.TXTの長さ0x10（16 byte）、ハッシュ9A42A948C590F507。媒体ブロック数0x3957000、セクタ512 byte、容量0x72AE00000（30,784,094,208 byte）。
- 全ページ左側: USB DEVICE READY、USB READ OK、FAT32 READ OK、APP FILE READ OK。
- IMG_7527（1/10）: 現在の詳細表示はC1だが、ファイル結果は`CACHED FILE RESULT FROM CONTROLLER C2`。C1のMEMORY DISABLEDはC2の成功を取り消すものではない。
- IMG_7536（10/10）: C1 / 0700は`PCI RECOVERY SKIP NOT BOOT OWNER`。現行設計が起動担当のみを復旧するため、C1は復旧していない。
- IMG_7529、7530、7531: C0 / 00A0 = K0 S0、C1 / 0700 = MEMORY DISABLED、C2 / 7D00 = K0 S1。C0の26ポートは写真の走査結果でCCS 0。

## 原因と対処の評価

実測で、Boot Services終了時の7D00のBAR/CMD消失、および復旧前のpeer資源不一致による拒否という二段階の問題があった。H19の範囲検査と条件付き復元により、起動担当制御器についてファイル読出しまで回復した。資源不一致が発生するファームウェア内部の理由までは断定しない。

画面のAPP COMPLETEとSTOPPED DMA DISABLEDは、今回C2では読出し処理を終えた後の停止記録。フリーズや読出し失敗の証拠として扱わない。

## 初回実行の入力機器の接続条件（利用者の追加回答）

今回の実行はMacBook Pro 2018本体のキーボードのみ。外付けLenovo有線キーボードは接続していなかった。したがってK0を外付けUSBキーボードの検出失敗と扱わない。0700配下にLenovoが接続されていたという仮説は今回には該当しない。

内蔵キーボードは今回のUSB走査では検出されておらず、その入力経路と必要な対応は別途調査が必要。写真だけでは内蔵キーボードの接続方式を確定しない。USBストレージ復旧の成功と内蔵キーボード対応を区別する。

初回時点で計画した次の確認はH19を変更せず、外付けLenovo有線を接続した状態でのKEYBOARD READY、Shift+A、Esc。Razerドングルは外して単独で確認する。失敗した場合は接続ポート・ハブ構成と診断表示をもとに切り分ける。起動担当外0700の復旧許可を、この未接続という結果だけで拡大しない。

初回時点では外付け入力も未確認だった。以下の追加実行で外付け入力の成功を確認した。

## 追加実行：外付けLenovo入力とストレージ読出し成功（最新）

2026-10-10、IMG_7540〜IMG_7552の13枚で診断全12ページを確認（1/12は2枚）。同じMacBook Pro 2018、H19。利用者がLenovoキーボードでA、Shift+A、Escの動作を明示報告。

- IMG_7540、7542、7552: C0 / BDF00A0 / Intel 8086:A36D / root02 / route00000 / slot01にLenovo 17EF:6009をLow Speedで認識。BOOT KBD 1、INPUT DONE K1 S0。終了後のKEY CODE 29、APP COMPLETE、STATE SESSION COMPLETEを確認。A・Shift+Aは利用者の操作報告、Escは操作報告と画面の最終キー・終了状態の双方が根拠。
- IMG_7544: C2 / BDF7D00 / root03 / slot01でSanDisk 0781:55A9をSuperSpeed認識。K0 S1。USB READ OK、FAT32 READ OK、APP FILE READ OK。MUSHA.TXT 16 byte、ハッシュ9A42A948C590F507。初回root04から今回はroot03へ変化しているため、同じ物理接続構成の反復安定性試験とは扱わない。
- IMG_7546: PEER BOUNDに続いてPCI RECOVERED 7D00 BAR 8F900000 CMD 0002 DMA OFFを再確認。
- IMG_7549、7551: C0の列挙・KEYBOARD POLLINGを経てSTOPPED DMA DISABLED。C1 / 0700は引き続きNOT BOOT OWNERで復旧対象外。

今回の構成では、C2側でファイルを読み出し、別コントローラーC0側でキーボード入力とEsc終了まで成功。0700を復旧対象へ広げる必要は、この成功構成にはない。キーボード一覧の同一VID/PID重複だけで、物理キーボードが2台あったとは判断しない。

今回の対処目標であるMacBook Pro 2018でのUSBファイル読出しと外付けLenovo入力・Esc終了は達成した。内蔵キーボード対応、全ポート互換性、同一構成での反復・長時間安定性、複数キーボード同時監視、Ryzen H19実機回帰は別の残件。コード・USB媒体の追加変更は行っていない。
