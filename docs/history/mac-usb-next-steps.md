# Mac USB diagnostics and built-in keyboard plan (H12)

H10実機ではメモリー初期化・PMタイマー・アプリ完了に到達した。C0はキーボード／ストレージ検出数0、C1/C2はPCI memory decoding無効で停止。内蔵キーボードの接続方式は画面だけでは確定できない。

## 今回の実装

1. 列挙前の全ルートポートについてPORTSC、生の速度ID、接続・有効・給電・リンク状態を記録する。Supported Protocolの欠落、未接続、列挙キュー投入を区別する。ログは既存の有限履歴と拡大ページで表示する。
2. 各コントローラーのPCI Command/Status、生のBAR0/1、UEFIが提供したBAR範囲、PM capabilityのPMCSRとD状態を読み取り専用で記録する。Capability走査は回数と循環を制限する。BARサイズ測定書込み、デコード有効化、電源状態変更は行わない。
3. 正常終了でもキーボード／BOTストレージが0ならUSB NO KBD/STORAGEと表示する。対応機器を認識した場合はUSB DEVICE READY。これは全USBクラスの動作保証ではない。

## FreeBSDを参考にした内蔵キーボード対応

FreeBSDのApple T2 BCEドライバーとBCE VHCI実装は通常のPCI xHCIとは別の経路を実装している。BCEには専用BAR、DMAキュー、メールボックス、割込み、ファームウェアイベント処理が必要であり、USB boot keyboard解析の修正だけでは追加できない。

- https://cgit.freebsd.org/src/commit/?id=6fd2ad9aa39db916bf2da9607653fb133f8fa078
- https://cgit.freebsd.org/src/commit/?id=9f90536c74b8172fc67cd977e5451f37a12462d5
- https://cgit.freebsd.org/src/commit/?id=ac5440ec248f9fa72646519743c89c3e61c07d5f

次段階では正確なMac機種とPCI一覧からBCE経路の存在を確認する。通常USB経路の診断と分離して、BCE検出→資源とDMA所有権設計→メールボックス／キュー→VHCI列挙→HID入力の順に設計・実装する。今回BCEドライバーを動作可能と扱わない。既存実装を移植する場合は各ファイルのライセンスと著作権表示を維持する。

## 実機で確認すること

H12のROOT履歴とPCI PMCSRを撮影する。C0のCCSが全て0なら外付けUSB経路／接続場所を切り分ける。CCSが1でNO PROTOCOLならSupported Protocol解析を調査する。C1/C2の資源・電源状態を確認するまではMEMORY DISABLEDのチェックを解除しない。外付け有線キーボードを併用すると、内蔵キーボード固有の経路と比較できる。

## H13: UEFI終了前後の比較

IMG_7463〜7465ではC0の全26ポートがCCS=0/PP=1、C1/C2はCommand=0・BAR0/1=0・D0だった。UEFIの資源記録とPCI BARの不一致があるため、BARの書戻しより先に変化時点を測定する。

H13は初期取得時EARLY、成功したExitBootServices呼出し直前PRE EBS、成功直後POST EBSのID・Command・Status・BAR0/1・PMCSRを保存する。終了直後の採取はランタイム移行とドライバー書込みより前。PMCSRのFFFFFFFFはcapabilityなし、FFFFFFFEは走査異常。最終メモリーマップ取得後の診断採取はPCI読み取りのみで、UEFIサービス・メモリー確保は呼ばない。

LoadedImageのDeviceHandleから起動デバイスパスを最大512バイト・64ノードまでコピーする。PCI/USBノードとその他ノードの生データを履歴へ出す。UEFI動作中に各対象xHCIのデバイスパスを照合し、一意のprefix一致があればBOOT XHCI BDFを記録。FFFFは不明、FFFEは複数候補。ファームウェアポインターは終了後に残さない。PCI bus番号を単に末尾PCIノードから推定しない。

実機ではEARLY/PRE EBS/POST EBSとBOOT XHCIのページを確認する。BARが既にEARLYで0なら資源記録／取得経路を調査し、PRE EBSとPOST EBSで変われば終了処理の影響を調査する。BARやmemory decodingを有効化する修正はこの測定後に検討する。

H13検証: QEMUの実GPT USBイメージ起動で3時点のID/Command/BARが一致し、BOOT XHCI BDF 0018が実際の接続先と一致。キーボード入力・Esc・FAT32ファイル読出し・DMA停止まで合格。旧64KiBスタック構成ではこのファイル読出し経路にdouble faultが発生し、128KiB構成で解消したため、ランタイムスタックを32ページへ増やした。既存の下端ガードページと範囲検証は維持する。原因箇所の厳密な最大スタック使用量測定は未実施。
