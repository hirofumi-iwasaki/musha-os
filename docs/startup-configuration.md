# Startup configuration / 起動設定

Status: current

## English

The v0.4.0 development implementation reads `/SETTINGS/CONFIG.TXT` on the FAT32 boot device through LoadedImage → BlockIO geometry validation → SimpleFileSystem, before GOP selection or the first OS display. Other attached data volumes do not supply CONFIG. exFAT is read-only data media. File, directory and volume handles are closed before GOP and final memory-map acquisition; only copied settings survive ExitBootServices. No firmware call is added between the final map and ExitBootServices.

Create this optional file yourself; distribution archives keep SETTINGS empty. Up to 4096 ASCII bytes are accepted. Use LF or CRLF, `key=value`, whitespace and `#` comments (including inline comments). Unknown keys are ignored; repeated known keys use the last value. Missing directory/file, read/close error, oversized/non-ASCII input or any malformed known value uses all defaults. A bare line without `=` is malformed. A firmware close error is recorded by the development debug probe and uses defaults; every acquired handle receives a close attempt.

```text
# Defaults: log=ready, display=max, lan.echo=off
log=devices
display=1024x768
pair=on
```

| Key | Values / behavior |
|---|---|
| log | quiet, ready (default), phases, devices, trace |
| display | max (default), or positive decimal WxH; unavailable/unsafe modes fall back to max and phases/devices/trace reports the fallback |
| lan.echo | exact lowercase on/off; default off. on enables UDP12345 echo; invalid value resets the whole CONFIG to defaults. DHCP is independent; no driver admission changes |
| pair | on; retained for stage6. Bluetooth and pairing writes are not implemented in stage2. |

For a build implementing the 2026-10-16 UDP-echo decision, omit `lan.echo` for default-off or use `lan.echo=on` only for an intended echo test. Record exact CONFIG bytes/hash; the choice is retained through session reinitialization. With link up, phases reports `lan: link ready; echo disabled; DHCP pending` (enabled when opted in). After a lease, off reports `lan: DHCP a.b.c.d; echo disabled`; on retains `DHCP a.b.c.d UDP echo port 12345 ready`. These are intended effective-state lines, not new physical results. This user-owned file is edited by the host, never rewritten by Musha-OS. An older EFI may ignore the key and retains its old service behavior; verify source/features/hash before relying on the switch.

Firmware F8 or digit8 forces trace after CONFIG is read. Press/hold it during firmware boot; recognition depends on the firmware input queue. Both paths have QEMU injection tests, which do not prove every physical firmware's key handling.

The display contract is [one sequential boot log](boot-log.md). `quiet` hides ordinary OS events; required failures still print. `ready` prints `Musha-OS 0.4.0 ready` when required USB-host, boot-media and keyboard discovery finishes, before `app: starting <name>`. App start/exit/fault/rejection and raw app bytes remain visible at ready level; optional LAN/BT availability may be reported before or after readiness. `phases` starts with `Musha-OS 0.4.0 starting` and gives concise completion/optional-failure lines: `core: runtime ready`, `display: GOP <W>x<H>...`, timer, `usb: host ready`, storage, keyboard and LAN. LAN reports actual link/DHCP-pending state or an unavailable reason, not an extra certification line. `devices` adds one timestamped identity/decision line per device, not every transfer or polling stage. `trace` adds the detailed transfer/stage/timing records, ranges and diagnostic inventory. Devices and trace use PM-epoch `[sss.mmm]` prefixes. Product name/case in the starting/ready lines is retained; VERSION comes from the running build. A ready line is not hardware acceptance. No separate panel, rotating pages, PM footer or status color remains. The epoch is an OS observation boundary, not measured firmware power-on time.

All levels retain the same 64 KiB whole-record event ring including trace detail; level filtering changes what is painted, not what is retained. F8/8 firmware override requests the ring's trace view through the same formatter; actual recognition depends on firmware. Serial mirroring is deferred to v0.5.0. Application log bytes append directly to the same full-frame white/black console at every level; there is no80x24 app area. The viewer owns its existing layout; launching from it clears the console to black, while returning repaints the viewer. Startup itself does not clear the framebuffer. Required failures print `ERROR: <subsystem>: <reason>` and `Musha-OS halted` at every level. Optional LAN/BT failure records a reason without fabricating success or unconditionally halting. Use [hardware records](hardware-test-record-template.md) for exact-build evidence; local validation is separate from physical acceptance.

Owner addition2026-10-15: CONFIG is user-owned and the OS never pads, resizes or rewrites it. Stage3 retains SHA-256 of the exact raw bytes after successful parse/close. Stage6 will persist that hash only after successful pairing and durable record save; an identical consumed hash suppresses `pair=on`. Changing content rearms it; saving identical content does not. Actual Bluetooth/pairing remains unimplemented.

## 日本語

v0.4.0 開発版は FAT32 起動デバイスの `/SETTINGS/CONFIG.TXT` を LoadedImage → BlockIO 幾何確認 → SimpleFileSystem の順に読み、GOP 選択と OS 初表示より前に設定を確定する。他のデータ媒体からは読まない。exFAT は読出し専用データ媒体。file／directory／volume handle は GOP と最終 memory map 採取前に close し、ExitBootServices 後はコピーした値だけを保持する。最終 map／ExitBootServices 間に firmware 呼出しを増やさない。

任意ファイルは利用者が作成する。配布 archive の SETTINGS は空のまま。上限 4096 ASCII byte、LF／CRLF、`key=value`、空白、`#` comment（行途中も可）に対応する。未知 key は無視し、既知 key の重複は最後を使う。directory／file 欠落、read／close error、上限超過、非 ASCII、既知値の不正はいずれも設定全体を既定値に戻す。`=` のない非空行も不正。close error は開発 debug probe に記録し既定値を使い、取得した各 handle に close を試みる。

```text
# defaults: log=ready, display=max, lan.echo=off
log=devices
display=1024x768
pair=on
```

| key | 値／動作 |
|---|---|
| log | quiet、ready（既定）、phases、devices、trace |
| display | max（既定）または正の10進数 WxH。利用不可・安全条件未達の mode は max に戻し phases／devices／trace で報告 |
| lan.echo | 小文字on/offのみ、既定off。onでUDP12345 echo有効。不正値はCONFIG全体を既定へ戻す。DHCPは独立、driver gate不変 |
| pair | on を工程6用に保持。工程2では Bluetooth・pairing 書込みは未実装 |

2026-10-16採用のecho設定を実装したbuildでは、lan.echo省略で既定off、echo試験時だけ`lan.echo=on`を使いCONFIG原bytes/hashを記録し、session再初期化にも設定を保持。link時は`lan: link ready; echo disabled; DHCP pending`（有効時enabled）、lease後offは`lan: DHCP a.b.c.d; echo disabled`、onは`DHCP a.b.c.d UDP echo port 12345 ready`を表示する契約です。新しい実機結果ではありません。この利用者所有fileはhostで編集しMusha-OSから書換えません。旧EFIはkeyを無視して旧service動作を保つ場合があるため、source/features/hashを確認します。

firmware F8 または数字8は CONFIG 読出し後に trace を強制する。firmware 起動中に押す／保持する。認識は firmware の入力 queue に依存する。両経路の QEMU 注入試験は実機全 firmware の認識を保証しない。

表示は[単一逐次bootlog](boot-log.md)。quietは通常OS行を隠しても必須失敗を表示。readyは必須USB host・起動媒体・keyboardの探索完了時、`app: starting <name>`より前に`Musha-OS 0.4.0 ready`を表示。readyでもapp開始/終了/fault/拒否・生byte出力を表示し、任意LAN/BTの報告はready前後どちらでも可。phasesは`Musha-OS 0.4.0 starting`から始め、`core: runtime ready`・`display: GOP <W>x<H>...`・timer・`usb: host ready`・storage・keyboard・LANの完了/任意失敗を簡潔に表示します。LANは実link/DHCP待機又は利用不可理由を記し、認証状態の補足行は加えません。devicesは各deviceの識別/判断を時刻付き1行にまとめ、transferやpolling段階は列挙しません。traceは詳細transfer/stage/timing・範囲・診断一覧を追加。devicesとtraceにPM起点`[sss.mmm]`を付けます。開始/ready行の製品名・大小文字を保持し、版は実VERSIONを使用。ready表示は実機受入ではありません。別panel/巡回page/PM footer/色付きstatusは廃止。epochはOS観測境界で、firmware電源投入時刻の実測ではありません。

全levelでtrace詳細まで64 KiBのrecord ringに保持し、levelは画面filterだけです。F8/8 firmware overrideは同formatterのtrace viewを要求し認識はfirmware依存。serial mirrorはv0.5.0。app logは全levelで同じ全画面白黒consoleへ追記し80x24専用枠は使いません。viewerは従来layoutを所有し、そこからのapp起動はconsoleを黒clear、復帰でviewer再描画。起動自体でframebuffer全消去しません。必須失敗は全levelで`ERROR: <subsystem>: <reason>`と`Musha-OS halted`、任意LAN/BTは理由付きで成功を装ったり無条件haltしません。正確なbuildは[実機様式](hardware-test-record-template.md)へ記録しlocalと実機を区別します。

所有者追加判断2026-10-15：CONFIGは利用者所有で、OSはpadding・resize・書換えを一切行わない。工程3はparse/close成功後、正確なraw bytesのSHA-256のみ保持する。工程6でpair成功とrecord永続化成功後にだけhashを保存し、同じ消費済みhashの `pair=on` を抑止する。内容変更で再有効になり、同一内容を保存し直すだけでは再有効にならない。実Bluetooth/pairingは未実装。
