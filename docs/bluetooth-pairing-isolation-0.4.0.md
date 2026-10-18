# Bluetooth pairing-phase isolation — English

Status: implemented locally; physical retry pending. IMG_7692 (normal `e64a092`, CSR8510 dongle) reached Reset/Version/LE Rand, `ready` and the viewer, then lost the whole USB session 2–3 s after an Xbox Series X|S controller entered pairing mode. No reason was shown, so the failure bypassed the Bluetooth isolation. The exact reason is not recorded; this change makes it visible and confines every Bluetooth-slot failure found in the code.

## Paths that ended the session

- **Unclaimed Transfer Events of the Bluetooth slot.** `event_dispatch` (`boot/src/xhci.rs`) returned `BAD TRANSFER` for any Transfer Event that no poller owned, and left it at the dequeue pointer. An HCI command STALL on EP0, a Babble or transaction error on the event endpoint (dci 3), or any ACL error (dci 4/5) therefore ended the session, whichever poller (keyboard, storage, Bluetooth) saw it first. Scanning is the first high-rate use of these endpoints.
- **Event TD length.** The live event TD was 257 bytes with a 16-byte interrupt max packet. A device need not send a ZLP after an event that fills whole packets, so consecutive 16- or 32-byte advertising events (2 or 18 AD bytes) can fill 256 bytes of whole packets; the next packet then exceeds the 1 remaining byte (Babble, cc=3).
- **Pairing-record save.** `pairing_step` (storage) returned `BT RECORD PERSISTENCE UNKNOWN` outside the Bluetooth poller, even for a filesystem refusal with healthy storage.
- **Bluetooth input overflow.** `drain_keys` / `drain_gamepad` errors became `APP INPUT ERROR`.
- **String-prefix classification.** `local_failure` treated only `BT ...`, `TRANSFER TIMEOUT` and `SHORT CONTROL WRITE` as local; for example `SHORT CONTROL READ`, `TRANSFER RANGE` or `DMA FULL` on the Bluetooth slot ended the session.
- **Refused Disable Slot.** A consumed, correlated `COMMAND REJECTED` for Disable Slot ended the session.

Checked and unchanged: multi-report advertising events are validated as a whole and a length mismatch is a local `BT ADVERTISING`; the 255-TRB event/ACL rings republish the Link TRB with the producer cycle exactly like the HID ring proven on hardware; LE Create Connection is accepted by Command Status and completed by LE Connection Complete; only Core 4.0 commands are sent (`0c03 0c01 2001 200b 200c 1009 2002 1005 200d 2019 2018 1001`), with no Data Length, Remote Features or Resolving List command; an LE buffer size of zero falls back to Read Buffer Size; the first ACL fragment uses PB=10b (recorded in the log, not changed).

## Changes

- The xHCI mailbox owns the Bluetooth slot from preparation. Any otherwise-unclaimed Transfer Event of that slot (any DCI) is consumed and kept as a Bluetooth fault; a waiting Bluetooth TD returns `BT TRANSFER FAULT` immediately. Only completion metadata is logged, never buffer contents. Correlated Disable Slot then disowns the slot.
- Event TDs are the largest multiple of the interrupt max packet not above 257 bytes (256 for CSR8510); the stream reassembler joins events split across TDs.
- `local_failure` is the complement of an explicit `CONTROLLER_SAFETY` list (`boot/src/xhci/usb/bluetooth/isolation.rs`): DMA not shown stopped or retirement unrecorded, an unanswered or ambiguous command ring, an inconsistent controller/event ring (including an unclaimed event of a non-Bluetooth slot), an inconsistent shared clock, and the deliberate re-pair restart. A host test classifies every error literal of the shared xHCI and clock sources.
- A refused Disable Slot quarantines the slot: the DCBAA context stays valid, DMA stays reserved, nothing is re-armed and its retired TDs are still absorbed. Timeout or unconsumed completion still stops the controller.
- A pairing-record filesystem refusal stops only Bluetooth (`BT BOND SAVE UNKNOWN`); a USB transport failure during the save still ends the session. Input overflow drops the batch and isolates Bluetooth (`BT INPUT OVERFLOW`); held keys and axes are released by the existing stop path.
- A session that ends shows `Device session ended: <reason>. Restart to try again.` and logs `usb: session ended: <reason>` at Ready. A non-local Bluetooth reason first logs `bluetooth: non-local failure: <reason>`.

## Devices log for the next retry

Per-TD `event/ACL queued` / `TransferEvent` lines are limited to six per pipe. The pairing stages appear once each, in this order:

1. `LE scan enabled: passive 10ms/10ms, duplicates filtered; ...` (with the scan CC lines).
2. `advertising window events=E reports=R hid-candidates=H hid-unsupported=U last=..:xx:yy` about once per second; only the low two address octets.
3. `HID advertisement ..:xx:yy ignored: appearance=XXXX ...` once, if a HID advertiser has an appearance other than 03c1/03c4 or none.
4. `HID candidate ..:xx:yy address-type=T method=...; connecting`.
5. `connect stage=<name> phase=N` with `HCI TX intent` and `HCI CC/CS` for set event mask, LE set event mask, read BD_ADDR, buffer size, scan disable, create connection; `ACL buffer bytes=B packets=P source=LE|shared BR/EDR`.
6. `LE create connection accepted (CS 00); awaiting LE Connection Complete`, then `LE Connection Complete status= handle= role= peer=..:xx:yy type= interval=Nx1.25ms latency= timeout=Nx10ms`.
7. `ACL first TX dci=4 handle= PB=02 length=`, `ACL first RX dci=5 length=`, `L2CAP first TX/RX cid=0006 (SMP)` / `cid=0004 (ATT)`.
8. `SMP TX/RX <PDU name> code=XX length=N` for Pairing Request/Response, Public Key, Confirm, Random, DHKey Check (codes and lengths only); `SMP .. Pairing Failed reason=XX`; `Encryption Change status= handle= enabled=`.
9. On failure: `transfer fault slot= dci= cc= (...)`, then `bluetooth: unavailable: <reason>` (isolated) or `bluetooth: non-local failure: <reason>` and `usb: session ended: <reason>` (session stop). `Disconnection Complete ... reason=XX` and `Hardware Error code=XX` are also logged.

Keys, nonces, confirm/random values, DHKey checks, passkeys and full addresses are never logged.

## Limits

Host tests run the production classification, mailbox fault capture, command-reply correlation, TD sizing and advertising/ACL/SMP framing on synthetic bytes. QEMU has no Bluetooth controller; its three Bluetooth-absent configurations show only that existing USB services are unaffected. Physical CSR8510/Intel 7265 scanning, pairing, HOGP and the actual IMG_7692 reason remain unverified.

# 日本語: Bluetooth ペアリング段階の分離

状態: ローカル実装済み・実機再試験待ち。IMG_7692（通常版 `e64a092`、CSR8510 ドングル）は Reset/Version/LE Rand、`ready`、ビューアーまで到達した後、Xbox Series X|S コントローラーをペアリングモードにして 2〜3 秒後に USB セッション全体を失いました。理由が表示されなかったため、Bluetooth の分離をすり抜けた失敗です。正確な理由は記録されていません。本変更は理由を表示し、コード上で見つかった Bluetooth slot の失敗をすべて局所化します。

## セッションを終わらせていた経路

- **Bluetooth slot の未所有 Transfer Event。** `event_dispatch`（`boot/src/xhci.rs`）は、どの poller も所有しない Transfer Event を `BAD TRANSFER` として返し、dequeue 位置に残していました。EP0 での HCI 命令 STALL、event endpoint（dci 3）の Babble・transaction error、ACL（dci 4/5）の異常は、最初に見た poller（キーボード・ストレージ・Bluetooth）にかかわらずセッションを終了させます。スキャンはこれらの endpoint を初めて高頻度で使う段階です。
- **event TD 長。** 通常経路の event TD は 257 バイト、割込み max packet は 16 バイトでした。パケット境界で終わるイベントの後に ZLP を送る義務はないため、16 または 32 バイト（AD 2 または 18 バイト）の Advertising イベントが続くと 256 バイトが満パケットで埋まり、次のパケットが残り 1 バイトを超えて Babble（cc=3）になります。
- **ペアリング記録の保存。** ストレージ側の `pairing_step` は、ストレージが健全なファイルシステム拒否でも `BT RECORD PERSISTENCE UNKNOWN` を Bluetooth poller の外で返していました。
- **Bluetooth 入力のあふれ。** `drain_keys` / `drain_gamepad` のエラーが `APP INPUT ERROR` になっていました。
- **前方一致による判定。** `local_failure` は `BT ...`・`TRANSFER TIMEOUT`・`SHORT CONTROL WRITE` だけを局所とし、例えば Bluetooth slot の `SHORT CONTROL READ`・`TRANSFER RANGE`・`DMA FULL` でセッションが終わっていました。
- **Disable Slot の拒否。** 消費済みで相関の取れた Disable Slot の `COMMAND REJECTED` でセッションが終わっていました。

点検して変更なし: 複数レポートの Advertising イベントは全体を検証し、長さ不一致は局所の `BT ADVERTISING`。255 TRB の event/ACL リングは実機で実証済みの HID リングと同じく producer cycle で Link TRB を再発行。LE Create Connection は Command Status で受理され LE Connection Complete で完了。送信命令は Core 4.0 のみ（`0c03 0c01 2001 200b 200c 1009 2002 1005 200d 2019 2018 1001`）で、Data Length・Remote Features・Resolving List 系は送りません。LE バッファーサイズ 0 は Read Buffer Size へ切替。ACL 最初の断片は PB=10b（ログに記録、変更なし）。

## 変更点

- xHCI mailbox が準備段階から Bluetooth slot を所有します。その slot の未所有 Transfer Event（全 DCI）は消費して Bluetooth の fault として保持し、待機中の Bluetooth TD は即座に `BT TRANSFER FAULT` を返します。ログは完了メタデータのみで、バッファー内容は出しません。相関した Disable Slot 完了後に所有を解除します。
- event TD は 257 バイト以下で割込み max packet の最大の倍数（CSR8510 では 256）。TD をまたぐイベントはストリーム再構成器が結合します。
- `local_failure` は明示した `CONTROLLER_SAFETY` 一覧（`boot/src/xhci/usb/bluetooth/isolation.rs`）の補集合です。DMA 停止や退役記録を示せない、command ring の無応答・曖昧応答、controller/event ring の不整合（Bluetooth 以外の slot の未所有イベントを含む）、共有時計の不整合、意図的な再ペアリング再開だけがセッションを終わらせます。共有 xHCI と時計のソース中のエラー文字列はすべて host 試験で分類しています。
- Disable Slot が拒否された場合は slot を隔離保持します。DCBAA の context は有効なまま、DMA は予約したまま、再 arm せず、退役 TD の完了は引き続き吸収します。timeout や未消費の完了は従来どおり controller を停止します。
- ペアリング記録のファイルシステム拒否は Bluetooth だけを止めます（`BT BOND SAVE UNKNOWN`）。保存中の USB 転送失敗は従来どおりセッションを終了します。入力あふれはそのバッチを捨てて Bluetooth を分離し（`BT INPUT OVERFLOW`）、押下中のキーと軸は既存の停止処理で解放します。
- セッション終了時はビューアーに `Device session ended: <reason>. Restart to try again.`、Ready レベルに `usb: session ended: <reason>` を出します。分離できない Bluetooth の理由は先に `bluetooth: non-local failure: <reason>` を出します。

## 次回再試験の devices ログ

TD ごとの `event/ACL queued` / `TransferEvent` は pipe ごとに 6 件までです。ペアリング段階は次の順に各 1 回出ます。

1. `LE scan enabled: passive 10ms/10ms, duplicates filtered; ...`（スキャンの CC 行とともに）。
2. `advertising window events=E reports=R hid-candidates=H hid-unsupported=U last=..:xx:yy` をほぼ 1 秒ごと。アドレスは下位 2 オクテットだけ。
3. HID 広告の appearance が 03c1/03c4 以外か欠落なら `HID advertisement ..:xx:yy ignored: appearance=XXXX ...` を 1 回。
4. `HID candidate ..:xx:yy address-type=T method=...; connecting`。
5. `connect stage=<名前> phase=N` と `HCI TX intent`・`HCI CC/CS`（set event mask、LE set event mask、read BD_ADDR、buffer size、scan disable、create connection）、`ACL buffer bytes=B packets=P source=LE|shared BR/EDR`。
6. `LE create connection accepted (CS 00); awaiting LE Connection Complete`、続いて `LE Connection Complete status= handle= role= peer=..:xx:yy type= interval=Nx1.25ms latency= timeout=Nx10ms`。
7. `ACL first TX dci=4 handle= PB=02 length=`、`ACL first RX dci=5 length=`、`L2CAP first TX/RX cid=0006 (SMP)` / `cid=0004 (ATT)`。
8. `SMP TX/RX <PDU 名> code=XX length=N`（Pairing Request/Response、Public Key、Confirm、Random、DHKey Check。コードと長さのみ）、`SMP .. Pairing Failed reason=XX`、`Encryption Change status= handle= enabled=`。
9. 失敗時: `transfer fault slot= dci= cc= (...)` の後、分離なら `bluetooth: unavailable: <reason>`、セッション停止なら `bluetooth: non-local failure: <reason>` と `usb: session ended: <reason>`。`Disconnection Complete ... reason=XX`・`Hardware Error code=XX` も出ます。

鍵・nonce・confirm/random 値・DHKey check・passkey・完全なアドレスは出しません。

## 限界

host 試験は本番の判定関数、mailbox の fault 捕捉、命令応答の相関、TD 長、Advertising/ACL/SMP のフレーミングを合成バイトで実行します。QEMU には Bluetooth controller が無く、Bluetooth 不在 3 構成は既存 USB 機能に影響が無いことだけを示します。実 CSR8510/Intel 7265 のスキャン・ペアリング・HOGP と、IMG_7692 の実際の理由は未確認です。

## LE signaling channel (CID 5) — English (2026-10-17)

IMG_7693 (`5d9443d`) kept the USB session and showed `Bluetooth unavailable: BT ACL REASSEMBLY` right after pairing started. `Reassembly` accepted only CID 4 (ATT) and 6 (SMP). HID peripherals commonly send an L2CAP Connection Parameter Update Request on CID 5 just after connecting; with a CSR8510 whose LE features lack the Connection Parameters Request procedure, this is the peripheral's only way to negotiate. The first ACL is consistent with that request, but the photographed lines did not show its CID.

- CID 5 is reassembled. A Connection Parameter Update Request (0x12) is range-checked (interval 6–3200 with min ≤ max, latency ≤ 499, timeout 10–3200 and longer than (1 + latency) × max × 2). If valid, the host answers accepted (0x13, result 0) and then issues HCI LE Connection Update (0x2013), which completes with Command Status and LE Connection Update Complete. An out-of-range request is answered rejected (result 1), and the connection is kept.
- Other requests get Command Reject (not understood). Responses, Command Reject and indications are never answered. A repeated identifier is answered only once. A malformed envelope or identifier 0 stops Bluetooth only (`BT L2CAP SIGNALING`).
- A refused LE Connection Update (non-zero Command Status) is logged and the connection is kept.
- The LE event mask now also enables LE Connection Update Complete (0x17 instead of 0x13).
- Up to two short SDUs (signaling, SMP, ATT; at most 72 bytes) are deferred in order while an L2CAP frame is in flight, so a signaling reply cannot cause `BT TX QUEUE FULL`.

New Devices lines: `L2CAP first RX cid=0005 (LE signaling)`, `LE signaling Connection Parameter Update Request id= interval=a-bx1.25ms latency= timeout= accepted|rejected`, `connect stage=LE connection update`, `LE connection update accepted (CS 00)` or `refused CS status=`, and `connection update status= handle= interval= latency= timeout=`. Parameter values are synthetic host fixtures; no Xbox trace was measured.

## 日本語: LE シグナリングチャネル（CID 5、2026-10-17）

IMG_7693（`5d9443d`）では USB セッションを維持したまま、ペアリング開始直後に `Bluetooth unavailable: BT ACL REASSEMBLY` が出ました。`Reassembly` は CID 4（ATT）と 6（SMP）しか受理していませんでした。HID 周辺機器は接続直後に CID 5 で L2CAP Connection Parameter Update Request を送るのが普通です。CSR8510 の LE features に Connection Parameters Request 手続きが無いこの構成では、周辺機器が交渉できる手段はこれだけです。最初の ACL がこの要求だったと考えると整合しますが、撮影した行からは CID を確認できていません。

- CID 5 を再構成します。Connection Parameter Update Request（0x12）は値域を検査します（interval 6〜3200 かつ min ≤ max、latency ≤ 499、timeout 10〜3200 かつ (1 + latency) × max × 2 より長い）。値域内なら accepted（0x13、result 0）で応答し、続けて HCI LE Connection Update（0x2013）を送ります。これは Command Status と LE Connection Update Complete で完了します。値域外は rejected（result 1）で応答し、接続は維持します。
- その他の要求には Command Reject（not understood）を返します。応答・Command Reject・indication には応答しません。同じ identifier の再送には 1 回だけ応答します。外枠の不正や identifier 0 は Bluetooth だけを止めます（`BT L2CAP SIGNALING`）。
- LE Connection Update が拒否された場合（Command Status が非 0）は記録だけして接続を維持します。
- LE event mask に LE Connection Update Complete を追加しました（0x13 → 0x17）。
- L2CAP フレームの送信中は、短い SDU（シグナリング・SMP・ATT、72 バイト以下）を 2 件まで順序どおりに保留します。シグナリング応答で `BT TX QUEUE FULL` にはなりません。

追加した Devices 行: `L2CAP first RX cid=0005 (LE signaling)`、`LE signaling Connection Parameter Update Request id= interval=a-bx1.25ms latency= timeout= accepted|rejected`、`connect stage=LE connection update`、`LE connection update accepted (CS 00)` または `refused CS status=`、`connection update status= handle= interval= latency= timeout=`。パラメーター値は合成の host fixture で、Xbox の実測ではありません。

## Silent bonded peers — English (2026-10-18)

A connection whose peer sends no Pairing Response within 5 s, or sends SMP Security Request while this host has no bond for it, is given up: HCI Disconnect (0x0406, reason 0x13) is sent, Command Status and Disconnection Complete must arrive within 1 s (otherwise only Bluetooth stops), per-connection state is released, scanning is re-enabled with the pairing request kept, and the address is deferred for 10 s. At most three attempts are made; after that the SMP timeout stops Bluetooth with `BT PAIRING NO RESPONSE` and the guidance stays on screen. Devices lines: `... disconnecting ..:xx:yy and deferring it for 10000ms`, `disconnect accepted (CS 00)`, `Disconnection Complete ...; rescanning`, `LE scan re-enabled; pairing request kept`. All HCI commands remain Bluetooth 4.0 (`0c03 0c01 2001 200b 200c 1001 2018 1009 2002 1005 200d 2019 2013 0406`), suitable for the CSR8510 and the ROM-mode Intel 7265.

日本語: Pairing Request に 5 秒応答しない相手、またはホストがボンドを持たないのに SMP Security Request を送ってきた相手との接続は諦めます。HCI Disconnect（0x0406、reason 0x13）を送り、Command Status と Disconnection Complete を 1 秒以内に待ちます（超えたら Bluetooth だけを停止）。接続ごとの状態を解放し、ペアリング要求を保ったままスキャンを再開し、そのアドレスを 10 秒間除外します。試行は最大 3 回で、それ以降は SMP タイムアウトで `BT PAIRING NO RESPONSE` として Bluetooth を止め、案内は画面に残します。Devices 行：`... disconnecting ..:xx:yy and deferring it for 10000ms`、`disconnect accepted (CS 00)`、`Disconnection Complete ...; rescanning`、`LE scan re-enabled; pairing request kept`。HCI 命令はすべて Bluetooth 4.0 の範囲（`0c03 0c01 2001 200b 200c 1001 2018 1009 2002 1005 200d 2019 2013 0406`）で、CSR8510 と ROM モードの Intel 7265 に適合します。
