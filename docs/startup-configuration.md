# Startup configuration / 起動設定

## English

The v0.4.0 development implementation reads `/SETTINGS/CONFIG.TXT` on the FAT32 boot device through LoadedImage → BlockIO geometry validation → SimpleFileSystem, before GOP selection or the first OS display. Other attached data volumes do not supply CONFIG. exFAT is read-only data media. File, directory and volume handles are closed before GOP and final memory-map acquisition; only copied settings survive ExitBootServices. No firmware call is added between the final map and ExitBootServices.

Create this optional file yourself; distribution archives keep SETTINGS empty. Up to 4096 ASCII bytes are accepted. Use LF or CRLF, `key=value`, whitespace and `#` comments (including inline comments). Unknown keys are ignored; repeated known keys use the last value. Missing directory/file, read/close error, oversized/non-ASCII input or any malformed known value uses all defaults. A bare line without `=` is malformed. A firmware close error is recorded by the development debug probe and uses defaults; every acquired handle receives a close attempt.

```text
# Defaults are log=ready and display=max
log=devices
display=1024x768
pair=on
```

| Key | Values / behavior |
|---|---|
| log | quiet, ready (default), phases, devices |
| display | max (default), or positive decimal WxH; unavailable/unsafe modes fall back to max and phases/devices reports the fallback |
| pair | on; retained for stage6. Bluetooth and pairing writes are not implemented in stage2. |

Firmware F8 or digit8 forces devices after CONFIG is read. Press/hold it during firmware boot; recognition depends on the firmware input queue. Both paths have QEMU injection tests, which do not prove every physical firmware's key handling.

Log control covers OS startup only. quiet makes no ordinary startup writes or background repaint; required failures still appear. ready adds only `Musha-OS ready.` after required discovery. phases lists UEFI, EBS, runtime, timer, USB host, boot media, keyboard, LAN, Bluetooth, application/viewer and ready. devices retains detailed diagnostics, shows the latest phase in a bottom status line and one compact PM-epoch summary. Unsupported LAN/Bluetooth is reported as unavailable, not fabricated success or an unconditional fatal error. The clock starts when the OS PM epoch exists; firmware entry time remains unknown.

Application logs and viewer UI remain visible at every level. Their existing 80×24 console and viewer grid are unchanged. The ready footer uses only space below both grids and is restored after viewer frames/application console rendering; it never reserves a row inside application output. Required media or viewer keyboard failure does not claim ready. This is software development validation, not physical acceptance; use the [new record template](hardware-test-record-template.md) with devices for new hardware evidence.

Owner addition2026-10-15: CONFIG is user-owned and the OS never pads, resizes or rewrites it. Stage3 retains SHA-256 of the exact raw bytes after successful parse/close. Stage6 will persist that hash only after successful pairing and durable record save; an identical consumed hash suppresses `pair=on`. Changing content rearms it; saving identical content does not. Actual Bluetooth/pairing remains unimplemented.

## 日本語

v0.4.0 開発版は FAT32 起動デバイスの `/SETTINGS/CONFIG.TXT` を LoadedImage → BlockIO 幾何確認 → SimpleFileSystem の順に読み、GOP 選択と OS 初表示より前に設定を確定する。他のデータ媒体からは読まない。exFAT は読出し専用データ媒体。file／directory／volume handle は GOP と最終 memory map 採取前に close し、ExitBootServices 後はコピーした値だけを保持する。最終 map／ExitBootServices 間に firmware 呼出しを増やさない。

任意ファイルは利用者が作成する。配布 archive の SETTINGS は空のまま。上限 4096 ASCII byte、LF／CRLF、`key=value`、空白、`#` comment（行途中も可）に対応する。未知 key は無視し、既知 key の重複は最後を使う。directory／file 欠落、read／close error、上限超過、非 ASCII、既知値の不正はいずれも設定全体を既定値に戻す。`=` のない非空行も不正。close error は開発 debug probe に記録し既定値を使い、取得した各 handle に close を試みる。

```text
# defaults: log=ready, display=max
log=devices
display=1024x768
pair=on
```

| key | 値／動作 |
|---|---|
| log | quiet、ready（既定）、phases、devices |
| display | max（既定）または正の10進数 WxH。利用不可・安全条件未達の mode は max に戻し phases／devices で報告 |
| pair | on を工程6用に保持。工程2では Bluetooth・pairing 書込みは未実装 |

firmware F8 または数字8は CONFIG 読出し後に devices を強制する。firmware 起動中に押す／保持する。認識は firmware の入力 queue に依存する。両経路の QEMU 注入試験は実機全 firmware の認識を保証しない。

log は OS 起動出力だけを制御する。quiet は通常の起動描画・背景再塗りを行わず、必須失敗は表示する。ready は必須探索後の `Musha-OS ready.` だけ。phases は UEFI／EBS／runtime／timer／USB host／boot media／keyboard／LAN／Bluetooth／applicationまたはviewer／ready を表示する。devices は詳細診断に加え下部専用行に最新 phase、PM-epoch 要約を表示する。未対応 LAN／Bluetooth は未対応とし、成功を装ったり毎回無条件 fatal にしたりしない。OS の PM epoch 成立前の firmware 開始時刻は不明のまま。

アプリの log と viewer UI は全 level で表示し、既存80×24 console と viewer grid を変えない。ready footer は両 grid より下の余白だけを使い、viewer frame／app console 描画後に復元する。アプリ出力内の行を予約しない。必須媒体や viewer keyboard の失敗では ready を宣言しない。これは開発時のソフトウェア検証で実機受入ではない。新しい実機証拠は devices で[記録様式](hardware-test-record-template.md)を使う。

所有者追加判断2026-10-15：CONFIGは利用者所有で、OSはpadding・resize・書換えを一切行わない。工程3はparse/close成功後、正確なraw bytesのSHA-256のみ保持する。工程6でpair成功とrecord永続化成功後にだけhashを保存し、同じ消費済みhashの `pair=on` を抑止する。内容変更で再有効になり、同一内容を保存し直すだけでは再有効にならない。実Bluetooth/pairingは未実装。
