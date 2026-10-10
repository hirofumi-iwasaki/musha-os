# H3 hardware USB diagnostics plan

## English

Preserve existing dirty work and transfer validation. Ryzen H2 evidence: storage/FAT32/MUSHA.TXT read succeeded on BDF 0801 after removing the wireless dongle; keyboard remains unmatched; BDF 0F03 fails during Device Descriptor transfer. A second Boot Keyboard candidate is currently rejected by the parser, but the actual dongle descriptor has not been captured. Do not infer hub enumeration success from file-read success alone.

1. Retain bounded per-controller panel snapshots and a persistent inventory across scans. Explicitly report truncation. On wide GOP screens show the persistent inventory beside the existing panel.
2. Capture failed event raw words, expected TRB/slot/endpoint, completion code and residual; record the last control request and route before descriptor reads. Keep strict completion checks and DMA shutdown.
3. Add configuration interface/endpoint observations and unsupported reasons. Next implement readable pages without requiring a working keyboard, including smaller GOP layouts, hub port/reset outcomes and descriptor capture. No USB log writes.
4. Once descriptors establish the cause, select one validated Boot Keyboard interface while validating the full configuration; add composite keyboard fixtures. Do not loosen parsing blindly. Re-test normal build, USB2 hubs, split controllers and injected failure shutdown before USB deployment.

Initial H3 implementation starts with persistent inventory, controller snapshots, configuration summary and transfer failure details. Paging, exhaustive port histories and keyboard parser changes remain separate follow-up work. Hardware validation remains pending.

## 日本語

既存の未コミット変更と転送の検証条件を維持する。H2実機では無線ドングルを外した後、0801でUSB/FAT32/MUSHA.TXT読出しが成功した。有線キーボードは未検出、0F03ではDevice Descriptor取得時のエラーが残る。解析器は2個目のBoot Keyboard候補を拒否するが、実物の記述子はまだ取得できていない。ファイル読出しだけでハブ列挙の成功を断定しない。

1. コントローラー別の固定容量スナップショットと、調査をまたいで残る機器一覧を保存する。上限超過を明示する。広いGOP画面では一覧を隣に表示する。
2. 失敗イベントの生の4ワード、期待TRB/Slot/Endpoint、完了コード、残り長、最後の制御要求と機器経路を保存する。転送検証とDMA停止は変更しない。
3. インターフェース／エンドポイントと対象外理由を追加する。その後、キーボード不要のページ表示、小さい画面、ハブのポート／リセット履歴、記述子取得を整える。USBへのログ書込みは追加しない。
4. 記述子で原因を確認後、全構成を検証しつつBoot Keyboardを1つ選ぶ解析に改善する。複合機器テスト、通常ビルド、USB2ハブ、複数コントローラー、故障時停止を検証して実機へ進める。

初回H3改修は持続する機器一覧、コントローラー別スナップショット、構成概要、転送エラー詳細から着手する。ページ表示、全ポート履歴、キーボード解析変更は後続作業とし、実機検証は未実施。

## H4 implementation / H4実装

English: Normal builds rotate retained diagnostic pages every 8 seconds after the USB/application session ends. Pages contain final status, persistent inventory, each controller snapshot and chronological USB/hub observations. A working keyboard is unnecessary. Full pages require GOP at least 408×444; smaller screens retain compact status. Rotation requires the existing ACPI timer; timer failure stops rotation safely. CPU polling is used with interrupts disabled; this is a hardware diagnosis mode, not a power-saving idle implementation. QEMU debug builds keep their original final stop to preserve smoke-test timing.

The journal is bounded at 255 observations plus an explicit truncation marker; consecutive identical observations are coalesced. It records controller/stage changes, hub port status/change bits and set/clear feature requests. It does not yet capture all descriptor bytes or implement hot-plug recovery. Keyboard configuration parsing selects the first valid Boot Keyboard interface while validating subsequent candidates; duplicate interrupt-IN endpoints in one candidate still fail. Multiple devices/input streams are not enabled.

日本語：通常版はUSB／アプリの処理終了後、8秒間隔で診断ページを自動切替する。最終結果、持続する機器一覧、コントローラー別保存画面、USB／ハブ処理履歴を表示し、キーボードを必要としない。全ページ表示はGOP 408×444以上を対象とし、小さい画面は従来の簡易表示を維持する。既存ACPIタイマーが必要で、タイマー異常時は切替を停止する。割込み無効のCPUポーリングによる診断用途で、省電力待機ではない。QEMU debug版は既存の終了動作を維持する。

履歴は255件＋上限表示の固定容量。連続して同一の観測はまとめる。コントローラー／段階、ハブポートの状態・変更ビット、featureのset/clear要求を残す。記述子の全バイト保存やhot-plug復旧は未実装。Boot Keyboard解析は後続候補も検証し、最初の有効インターフェースを選ぶ。同一候補内の複数interrupt-INは引き続き拒否する。複数機器の同時入力には対応しない。
