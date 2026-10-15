# Hardware test record template / 実機試験記録様式

Status: current

## English

Copy this document into a new result file for each configuration. Default values are UNKNOWN / NOT RUN. Fill observations without converting unknown or unsupported features into passes. Do not change the template itself to record a particular machine.

For new v0.4.0 acceptance records, use `log=devices` (or firmware F8 / digit8 override), record CONFIG bytes and the override, and photograph the PM-epoch summary. This requirement does not change older records. See [startup configuration](startup-configuration.md).

| Field | Observation |
|---|---|
| Date / operator / record ID | UNKNOWN |
| Machine / board / CPU | UNKNOWN |
| RAM modules / capacity | UNKNOWN |
| UEFI version / settings / Secure Boot state | UNKNOWN |
| Display / connector / cable / GOP resolution | UNKNOWN |
| USB drive model / exact capacity bytes / logical sector bytes | UNKNOWN |
| Keyboard model / USB Boot Keyboard capability | UNKNOWN |
| USB connections: drive port / keyboard port / hubs | UNKNOWN |
| Ethernet controller PCI ID / cable / peer | UNKNOWN |
| Git branch / full commit / working tree changes | UNKNOWN |
| Build type / enabled features / toolchain | UNKNOWN |
| CONFIG contents / log level / F8 or digit8 override / requested and actual GOP | UNKNOWN |
| PM-epoch core / ready / current ms / timing record count (devices) | UNKNOWN |
| BOOTX64.EFI size / SHA-256 | UNKNOWN |
| Complete USB image size / SHA-256 / GPT backup location | UNKNOWN |
| Image creation command / physical USB preparation method | UNKNOWN |
| Photo / log filenames | UNKNOWN |

| Check | Result (NOT RUN / PASS / FAIL / UNSUPPORTED) | Exact observation |
|---|---|---|
| devices startup / `Musha-OS ready.` / required phase results | NOT RUN | UNKNOWN |
| CPU tables / paging / arena size | NOT RUN | UNKNOWN |
| Timer / PCI IDs / BDF | NOT RUN | UNKNOWN |
| xHCI ownership / reset / command rings | NOT RUN | UNKNOWN |
| USB port / VID:PID / speed / descriptor count | NOT RUN | UNKNOWN |
| Storage blocks / sector bytes / total capacity | NOT RUN | UNKNOWN |
| README.TXT build length / FNV-1a / startup marker result | NOT RUN | UNKNOWN |
| Keyboard A / Shift+A press and release / displayed key code | NOT RUN | UNKNOWN |
| Idle input with ongoing application work | NOT RUN | UNKNOWN |
| Esc / APP COMPLETE / USB stop status | NOT RUN | UNKNOWN |
| Keyboard disconnect / exact failure / remaining progress | NOT RUN | UNKNOWN |
| LAN driver status / link | NOT RUN | UNKNOWN |
| ARP / ICMP / UDP communication with identified peer | NOT RUN | UNKNOWN |
| NET stop status / counters | NOT RUN | UNKNOWN |
| Cold boot repeat / elapsed time / stability | NOT RUN | UNKNOWN |

For current NUC5/NUC8 builds, LAN driver UNSUPPORTED is expected; ARP/ICMP/UDP remain NOT RUN until their NIC initialization is implemented. SESSION COMPLETE alone is not a pass. Record all error rows, the last stage, and whether shutdown was actually verified. Photograph the screen before and after Esc. Record expected file contents independently; the displayed FNV-1a is not the complete image SHA-256.

Failure details: UNKNOWN. Reproduction steps and recovery: UNKNOWN. Final result and remaining issues: NOT RUN.

## 日本語

構成ごとに本書を別の結果ファイルへコピーする。初期値はUNKNOWN（不明）／NOT RUN（未実施）。不明や未対応を合格に読み替えず、観測結果を記入する。個体の結果をこの様式原本へ直接記入しない。

v0.4.0 の新規受入記録は `log=devices`（または firmware F8／数字8）で採取し、CONFIG の内容・override と PM-epoch 要約の写真を残す。過去の記録を変更する要件ではない。[起動設定](startup-configuration.md)を参照。

| 項目 | 観測内容 |
|---|---|
| 日時／試験者／記録ID | 不明 |
| 機種／基板／CPU | 不明 |
| RAMモジュール／容量 | 不明 |
| UEFI版／設定／Secure Boot状態 | 不明 |
| display／端子／ケーブル／GOP解像度 | 不明 |
| USB型番／正確な容量byte／logical sector byte | 不明 |
| keyboard型番／USB Boot Keyboard対応 | 不明 |
| USB接続: drive port／keyboard port／hub | 不明 |
| Ethernet PCI ID／ケーブル／通信相手 | 不明 |
| Git branch／完全commit／未commit変更 | 不明 |
| build種別／feature／toolchain | 不明 |
| CONFIG内容／log／F8または8／要求と実際のGOP | 不明 |
| devices PM-epoch core／ready／現在ms／timing件数 | 不明 |
| BOOTX64.EFIサイズ／SHA-256 | 不明 |
| USB image全体サイズ／SHA-256／GPT副header位置 | 不明 |
| image生成コマンド／実物USBの準備方法 | 不明 |
| 写真／logファイル名 | 不明 |

| 確認 | 結果（未実施／合格／失敗／未対応） | 正確な観測内容 |
|---|---|---|
| devices起動／`Musha-OS ready.`／必須段階の結果 | 未実施 | 不明 |
| CPU tables／paging／arena容量 | 未実施 | 不明 |
| timer／PCI ID／BDF | 未実施 | 不明 |
| xHCI ownership／reset／command rings | 未実施 | 不明 |
| USB port／VID:PID／速度／descriptor数 | 未実施 | 不明 |
| storage block数／sector byte／総容量 | 未実施 | 不明 |
| README.TXTビルド長さ／FNV-1a／起動マーカー結果 | 未実施 | 不明 |
| A／Shift+Aの押下・解放／key code | 未実施 | 不明 |
| 無入力中のアプリ処理継続 | 未実施 | 不明 |
| Esc／APP COMPLETE／USB停止状態 | 未実施 | 不明 |
| keyboard切断／エラー全文／残る処理の進行 | 未実施 | 不明 |
| LAN driver状態／link | 未実施 | 不明 |
| 相手を特定したARP／ICMP／UDP通信 | 未実施 | 不明 |
| NET停止状態／counter | 未実施 | 不明 |
| cold boot再試行／所要時間／安定性 | 未実施 | 不明 |

現行NUC5／NUC8版ではLAN driver未対応が想定結果。NIC初期化の実装まではARP／ICMP／UDPは未実施とする。SESSION COMPLETEだけでは合格ではない。全エラー行、最後の段階、停止を実際に検証できたかを記録する。Escの前後で画面を撮影する。期待するファイル内容は独立して記録する。表示されるFNV-1aはimage全体のSHA-256ではない。

失敗詳細: 不明。再現手順と復旧方法: 不明。総合結果と残る課題: 未実施。
