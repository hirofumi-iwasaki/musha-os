# Bluetooth HOGP discovery robustness and peer identity — English

Status: implemented locally (commits `55dd755`, `92788aa`, `0dc0ec7`); physical retry pending. IMG_7694 and IMG_7739 (`9db19de`, NUC5 + MM-BTUD46 + Xbox Series X|S model 1914) completed LE Secure Connections pairing, started encryption (`HCI CS 2019 status=00`) and handled the CID 5 parameter update (`HCI CS 2013 status=00`) twice. Discovery then stopped at `HOGP stage=services ... response=1b len=19` with `BT HOGP SERVICES REJECTED`: a Handle Value Notification (opcode, handle, 16-byte input report) arrived while Read By Group Type was pending and was parsed as the response.

## Unsolicited ATT PDUs

- Discovery skips any PDU that is neither the awaited response nor an Error Response. Skipped PDUs are not counted as discovery pages and are bounded at 256; an indication is answered with Handle Value Confirmation (0x1e).
- Before discovery (during SMP and encryption) and while CCCDs are being written, notifications are dropped. Indications are always confirmed and never delivered as input. Skipped notifications are not replayed after subscription.
- A peer ATT **request** is answered in every phase so the peer's 30-second transaction timer cannot disconnect: Exchange MTU keeps 23; Find Information, Find By Type Value, Read By Type and Read By Group Type get Attribute Not Found; other requests get Request Not Supported. This central has no GATT database.
- Devices log: `ATT ignored opcode=1b len=19 during stage=services` once, then a count (`HID subscribed; ATT PDUs skipped before input=N`); `ATT peer request opcode=xx answered opcode=yy`. Report contents are never logged.

## Later stages checked in advance

| Stage | Result |
|---|---|
| Report Map by Read Blob | Checked: Read (0x0a) then Read Blob (0x0c) at increasing offsets until a response shorter than 22 bytes or Invalid Offset / Attribute Not Long; 2048-byte cap. A ~320-byte independently authored Xbox-shaped map with a notification before every response reaches binding in a host test. |
| Several Report characteristics and CCCD | Fixed: only notifying Report characteristics whose Report Reference type is Input are considered; input reports neither decoder maps (AC Home, battery) are no longer a binding failure: they are not subscribed and their notifications are accepted without events. The gamepad parser preserves known Xbox BLE AC Home (0x0c:0x223) and Battery Strength (0x06:0x20) fields like the Share bit; other unknown fields still fail closed. CCCDs (0x2902) are written one by one with Write Request; a refused write is `BT CCCD WRITE REJECTED` with the ATT error code. Physical map pending. |
| Bond save | Logged: `bond saved record=N file=SETTINGS/PAIRING.DAT`, or `bond save failed reason=<error> transport=<ok or USB reason>`. A filesystem refusal stops only Bluetooth; a USB transport failure still ends the session (unchanged from `5d9443d`). Physical save pending. |
| Reconnect with a private address | Fixed. (a) The Pairing Request now asks the responder for IdKey (initiator distribution 0, responder 0x02); a response distributing anything else is refused. (b) Identity Information (IRK) then Identity Address Information are accepted after encryption, or just before the HCI Encryption Change is read; the bond keeps the IRK and a public or static identity (an invalid identity falls back to the pairing address). PAIRING.DAT version 2 (56-byte records) carries the IRK and is written only when a bond has one; version 1 stays readable. (c) A resolvable private advertiser (top bits 01) is resolved on the host with `ah(IRK, prand)`, as CSR8510 and ROM-mode 7265 have no LL Privacy resolving list. Xbox address type pending. |

Devices lines: `HID candidate ..:xx:yy addr-type=N (<class>[, stored bond via IRK])`, `SMP RX Identity Information code=08 length=17`, `SMP RX Identity Address Information code=09 length=8`, `bond identity addr-type=N (<class>) ..:xx:yy irk=yes|no`. Only the two low address octets are shown; IRK and LTK values are never logged.

## Limits

Host tests use synthetic bytes, the Core `ah` vector (cross-checked with OpenSSL AES-128) and an independently authored Xbox-shaped map, not a copied or measured descriptor. QEMU has no Bluetooth controller. The real Xbox Report Map, CCCD acceptance, bond save, reboot reconnect and the built-in 7265 remain unverified on hardware.

# 日本語: HOGP 探索の堅牢化と相手の identity

状態: ローカル実装済み（`55dd755`、`92788aa`、`0dc0ec7`）・実機再試験待ち。IMG_7694 と IMG_7739（`9db19de`、NUC5＋MM-BTUD46＋Xbox Series X|S model 1914）で、LE Secure Connections のペアリング、暗号化開始（`HCI CS 2019 status=00`）、CID 5 のパラメーター更新（`HCI CS 2013 status=00`）が 2 回成功しました。その後の探索が `HOGP stage=services ... response=1b len=19` と `BT HOGP SERVICES REJECTED` で止まりました。Read By Group Type の応答待ちに Handle Value Notification（opcode、handle、16 バイトの入力レポート）が届き、それを応答として解析していました。

## 要求していない ATT PDU

- 探索中は、待っている応答でも Error Response でもない PDU を読み飛ばします。探索のページには数えず、上限は 256 件です。Indication には Handle Value Confirmation（0x1e）を返します。
- 探索前（SMP・暗号化中）と CCCD の書込み中は通知を捨てます。Indication は常に確認応答し、入力としては渡しません。捨てた通知は購読後に再生しません。
- 相手からの ATT **要求**には、相手の 30 秒タイマーで切断されないよう、どの段階でも応答します。Exchange MTU は 23 のまま、Find Information・Find By Type Value・Read By Type・Read By Group Type には Attribute Not Found、その他の要求には Request Not Supported を返します。このセントラルは GATT データベースを持ちません。
- Devices ログ: `ATT ignored opcode=1b len=19 during stage=services` を 1 回、以後は件数（`HID subscribed; ATT PDUs skipped before input=N`）。`ATT peer request opcode=xx answered opcode=yy`。レポートの内容は出しません。

## その先の段階の事前点検

| 段階 | 結果 |
|---|---|
| Report Map の Read Blob | 確認済み。Read（0x0a）の後、22 バイト未満の応答か Invalid Offset／Attribute Not Long まで、オフセットを進めて Read Blob（0x0c）。上限 2048 バイト。独自に作成した約 320 バイトの Xbox 型 Report Map で、各応答の前に通知を挟んでも結合まで到達することを host 試験で確認。 |
| 複数の Report 特性と CCCD | 修正。Report Reference の type が Input で通知可能な Report 特性だけを対象にします。どの解析器も対応しない入力レポート（AC Home、電池）で結合全体を失敗させず、購読せず、その通知はイベント無しで受け付けます。ゲームパッド解析は、既知の Xbox BLE の AC Home（0x0c:0x223）と Battery Strength（0x06:0x20）を Share ビットと同様に位置だけ保持します。その他の未知フィールドは従来どおり拒否します。CCCD（0x2902）は Write Request で 1 つずつ書き、拒否されたら ATT エラーコード付きで `BT CCCD WRITE REJECTED`。実機の Report Map は未確認。 |
| ボンド保存 | ログ追加。`bond saved record=N file=SETTINGS/PAIRING.DAT`、失敗時は `bond save failed reason=<error> transport=<ok または USB の理由>`。ファイルシステムの拒否は Bluetooth だけを止め、USB 転送失敗は従来どおりセッションを終了（`5d9443d` の区分を維持）。実機の保存は未確認。 |
| private address からの再接続 | 修正。(a) Pairing Request で相手側の配布に IdKey を要求（initiator 0、responder 0x02）。要求外の配布を返す応答は拒否。(b) 暗号化後（または HCI の Encryption Change を読む直前）に Identity Information（IRK）→ Identity Address Information を受け付け、IRK と public／static の identity でボンドを確定。無効な identity ならペアリング時のアドレスを使用。PAIRING.DAT version 2（1 件 56 バイト）は IRK を含み、IRK を持つボンドがある時だけ書きます。version 1 も読めます。(c) CSR8510 と ROM モードの 7265 には LL Privacy の Resolving List が無いため、resolvable private（上位 2 ビット 01）の広告元はホストで `ah(IRK, prand)` を計算して照合します。Xbox のアドレス種別は未確認。 |

Devices 行: `HID candidate ..:xx:yy addr-type=N (<種別>[, stored bond via IRK])`、`SMP RX Identity Information code=08 length=17`、`SMP RX Identity Address Information code=09 length=8`、`bond identity addr-type=N (<種別>) ..:xx:yy irk=yes|no`。アドレスは下位 2 オクテットだけで、IRK・LTK の値は出しません。

## 限界

host 試験は合成バイト、Core 仕様の `ah` ベクター（OpenSSL の AES-128 で照合）、独自に作成した Xbox 型 Report Map を使います。複製や実測の記述子ではありません。QEMU には Bluetooth controller がありません。実 Xbox の Report Map、CCCD の受理、ボンド保存、再起動後の再接続、内蔵 7265 は実機で未確認です。

## Runtime stack overflow at the Report Reference read — English (2026-10-18)

IMG_7740 (`0e743cf`) passed every HOGP stage up to `ATT queue intent request=0a handle=0020`, then halted with `#DF (vector 8) ... rip 0x140093907 cr2 0x0`. A linker map from a rebuild of `0e743cf` (byte-identical, SHA-256 `f09b8df3…`) places `rip` at `__rust_probestack+0x17` (`test [rsp+8], rsp`): a frame-size probe touched the unmapped guard page. The #PF could not be pushed on the exhausted stack and became #DF. `cr2 0x0` was a reporting gap: CR2 was printed only for vector 14. The deep frames were `Reports::bind` 47,880 bytes, `Device::att` 26,952, USB enumeration 21,528 and the runtime loop 23,832, against 31 usable pages (124 KiB).

- Fix (`192096a`): the runtime stack is 64 pages (252 KiB usable). Discovery writes the bound reports into the caller's slot, so the 13 KB value is no longer moved through `action`/`gatt_action`. A host fixture of the IMG_7740 sequence reaches binding without errors. Devices logs `HOGP bound ...; stack never-touched N of M bytes` and the same after `bond saved`.
- Diagnostics (`5999cac`): #PF runs on its own IST stack, so an overflow is a #PF with CR2. CR2 is shown for #DF too. A kernel fault adds `cpu: rip image+0x... rsp 0x... stack used U of M bytes, never touched N` and `cpu: stack image addresses image+0x...` (at most eight image addresses above RSP; nothing outside the image is printed). The `fault-overflow` QEMU case forces a real probestack overflow and checks these lines.

## 日本語: Report Reference 読出し時のランタイムスタック溢れ（2026-10-18）

IMG_7740（`0e743cf`）は `ATT queue intent request=0a handle=0020` まで HOGP の全段階を通過した後、`#DF (vector 8) ... rip 0x140093907 cr2 0x0` で停止しました。`0e743cf` をバイト一致（SHA-256 `f09b8df3…`）で再ビルドしたリンカーマップでは、`rip` は `__rust_probestack+0x17`（`test [rsp+8], rsp`）です。フレーム確保前のページ確認が、マップされていないガードページに触れました。尽きたスタックへ #PF を積めずに #DF になっています。`cr2 0x0` は表示の欠落で、CR2 はベクター 14 の時しか出していませんでした。大きなフレームは `Reports::bind` 47,880 バイト、`Device::att` 26,952、USB 列挙 21,528、ランタイムのループ 23,832 で、使用可能な 31 ページ（124 KiB）を超えました。

- 修正（`192096a`）: ランタイムスタックを 64 ページ（使用可能 252 KiB）にしました。探索は結合したレポートを呼出し側の領域へ直接書き、13 KB の値を `action`／`gatt_action` 経由で移しません。IMG_7740 の手順を模した host fixture はエラー無しで結合に到達します。Devices に `HOGP bound ...; stack never-touched N of M bytes` を出し、`bond saved` の後にも同様に出します。
- 診断（`5999cac`）: #PF を専用の IST スタックで動かし、溢れは CR2 付きの #PF として報告します。#DF でも CR2 を表示します。カーネル側の例外では `cpu: rip image+0x... rsp 0x... stack used U of M bytes, never touched N` と `cpu: stack image addresses image+0x...`（RSP より上の像内アドレスを最大 8 個、像外の値は出さない）を追加します。QEMU の `fault-overflow` ケースは実際に probestack で溢れさせ、これらの行を確認します。
