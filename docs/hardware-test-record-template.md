# Hardware test record template / 実機試験記録様式

Status: current (single boot log, 2026-10-16). Blank fields are not results.

## English

Use [boot-log](boot-log.md) with `log=devices` for new v0.4.0 runs. Capture sequential text and any available ring dump, not former panel pages/status colors. Keep original photos/videos immutable; record extracted-frame offsets separately from PM-epoch times. Published v0.3.0 [formatter trials](formatted-media-boot-verification.md) retain their own six-run viewer criterion. This blank template grants no physical write or acceptance.

| Field | Recorded value / evidence |
|---|---|
| Date/operator; machine/board/CPU/RAM/firmware | |
| EFI/source/features/SHA256; deployment provenance and read-back | |
| Media model/serial/capacity/sector/partition/filesystem | |
| CONFIG exact bytes/hash, requested/effective log and display; F8/8 result | |
| AUTORUN/sample filename, size/hash and permissions | |
| Cables/ports/adapters/hubs/input/display and device labels | |
| Cold/hot; off seconds; AC state; run ordinal | |
| GOP mode/stride/pixel format/grid/scale; framebuffer cache mapping | |
| Start/runtime/timer/ready lines and their PM timestamps | |
| PCI vendor:device/revision/BDF; subsystem/BAR/command/TxConfig if exposed | |
| USB VID:PID/class/controller/root/route/slot/speed; keyboard/mouse selection | |
| Storage format/size; README marker length/hash; file/read result | |
| LAN backend/reset/PHY/link/address/budget/lease time or exact unavailable reason | |
| UDP sent/returned payload, Mac IP/interface, OS counter before/after (if run) | |
| BT adapter/peer IDs, scan/pair/save/reconnect/input; exact failure if any | |
| App start/output/exit/fault/refusal; viewer launch/return ordinal | |
| Required failure or kernel exception: complete line, name/vector/error or n/a/RIP/CR2 | |
| Explicit device-stop confirmation; unconfirmed/quarantined state | |
| Original media path/hash, frame offsets, available ring dump, omitted evidence | |
| PASS/FAIL/BLOCKED for this bounded gate; UNKNOWN and owner review | |

Copy one form per run/configuration. Printed readiness does not certify optional devices or replace feature-specific acceptance. Do not infer cold conditions, network traffic, descriptor support or successful stop from a boot image alone. Transcribe exact event text rather than requiring obsolete `USB READ OK`/`NET STEP`/red-band wording. No new hardware photograph is supplied by this documentation change.

## 日本語

新規v0.4.0は[bootlog](boot-log.md)の`log=devices`を用い逐次textと取得できたringを採取。旧panel頁/色を条件にしません。原写真/videoを保持し動画offsetとPM時刻を分離。公開v0.3.0の[formatter](formatted-media-boot-verification.md)は専用6run/viewer条件のまま。本空様式で実書込み/受入を承認しません。

| 項目 | 実測値・証拠 |
|---|---|
| 日時/実施者、実機/board/CPU/RAM/firmware | |
| EFI/source/features/SHA256、配備来歴/読戻し | |
| 媒体型番/serial/容量/sector/partition/filesystem | |
| CONFIG原bytes/hash、要求/実log/display、F8/8認識 | |
| AUTORUN/sample名・size/hash・権限 | |
| cable/port/adapter/hub/input/display・device label | |
| 冷間/温間、off秒、AC状態、run番号 | |
| GOP/stride/pixel format/grid/scale、framebuffer cache mapping | |
| start/runtime/timer/ready行とPM時刻 | |
| PCI ID/revision/BDF、表示されたsubsystem/BAR/command/TxConfig | |
| USB ID/class/controller/root/route/slot/speed、keyboard/mouse選択 | |
| storage形式/容量、README長さ/hash、file/read結果 | |
| LAN backend/reset/PHY/link/address/budget/lease時間又は非対応理由 | |
| UDP送信/返信、Mac IP/interface、OS counter前後（実施時） | |
| BT adapter/peer、scan/pair/保存/再接続/入力又は正確な失敗 | |
| app開始/出力/終了/fault/拒否、viewer起動/復帰番号 | |
| 必須失敗/kernel例外の行全文、名前/vector/error又はn/a/RIP/CR2 | |
| 明示device停止確認、不確実/隔離状態 | |
| 原media path/hash、動画offset、ring dump、欠落証拠 | |
| 限定gateごとのPASS/FAIL/BLOCKED、UNKNOWN、所有者review | |

run/構成毎にcopy。readyで任意機器や機能受入を証明せず、起動だけで冷間/通信/descriptor/停止確認を推測しません。旧USB READ OK/NET STEP/赤帯を要求せず実event行を転記。今回文書変更で新実機写真は追加していません。
