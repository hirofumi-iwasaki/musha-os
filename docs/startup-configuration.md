# Startup configuration / 起動設定

Status: current

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

The display contract is [one sequential boot log](boot-log.md). `quiet` hides ordinary OS events; required failures still print. `ready` prints `Musha-OS 0.4.0 ready` after required discovery. `phases` appends subsystem completion/optional-failure events. `devices` adds per-device decisions and PM-epoch `[sss.mmm]` timestamps, including PCI/USB identity, media/read results, ranges, link/address and reasons. Use the running VERSION in the ready line; the literal0.4.0 example is not a new acceptance result. No separate panel, rotating pages, PM footer or status color remains. The timer epoch is an OS observation boundary, not measured firmware power-on time.

All levels retain the same 64 KiB devices-detail whole-record event ring; level filtering changes what is painted, not what is retained. F8/8 firmware override requests the ring's devices view through the same formatter; actual recognition depends on firmware. Serial mirroring is deferred to v0.5.0. Application log bytes append directly to the same full-frame white/black console at every level; there is no80x24 app area. The viewer owns its existing layout; launching from it clears the console to black, while returning repaints the viewer. Startup itself does not clear the framebuffer. Required failures print `ERROR: <subsystem>: <reason>` and `Musha-OS halted` at every level. Optional LAN/BT failure records a reason without fabricating success or unconditionally halting. Use [hardware records](hardware-test-record-template.md) for exact-build evidence; local validation is separate from physical acceptance.

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

表示は[単一逐次bootlog](boot-log.md)。quietは通常OS行を隠しても必須失敗を表示。readyは必須探索後に`Musha-OS 0.4.0 ready`、phasesはsubsystem完了/任意失敗、devicesは各device判断・PM起点`[sss.mmm]`とPCI/USB/媒体/read/範囲/link/address/理由を追加。ready版文字列は実VERSIONを使い、例の0.4.0を新受入結果にしません。別panel/巡回page/PM footer/色付きstatusは廃止。PM epochはOS観測境界でfirmware電源投入時刻の実測ではありません。

全levelで64 KiBのdevices詳細record ringを保持し、levelは画面filterだけです。F8/8 firmware overrideは同formatterのdevices viewを要求し認識はfirmware依存。serial mirrorはv0.5.0。app logは全levelで同じ全画面白黒consoleへ追記し80x24専用枠は使いません。viewerは従来layoutを所有し、そこからのapp起動はconsoleを黒clear、復帰でviewer再描画。起動自体でframebuffer全消去しません。必須失敗は全levelで`ERROR: <subsystem>: <reason>`と`Musha-OS halted`、任意LAN/BTは理由付きで成功を装ったり無条件haltしません。正確なbuildは[実機様式](hardware-test-record-template.md)へ記録しlocalと実機を区別します。

所有者追加判断2026-10-15：CONFIGは利用者所有で、OSはpadding・resize・書換えを一切行わない。工程3はparse/close成功後、正確なraw bytesのSHA-256のみ保持する。工程6でpair成功とrecord永続化成功後にだけhashを保存し、同じ消費済みhashの `pair=on` を抑止する。内容変更で再有効になり、同一内容を保存し直すだけでは再有効にならない。実Bluetooth/pairingは未実装。
