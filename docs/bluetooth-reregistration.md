# Bluetooth re-registration and saved-record recovery

Normal startup uses validated stored SC keys. A missing/rejected peer key does not trigger automatic fresh pairing. Keys are never logged.

For re-registration: turn the pad off, boot the viewer, press P, then put the pad into fast-blinking pairing mode before it reconnects. P now interrupts stored-key scan/connection/encryption/HID discovery through the existing confirmed-stop restart. Repeated P during fresh pairing does not reset it. A stopped/isolated adapter still requires an OS restart; no automatic re-enable was added. Expected-adapter and entropy refusals remain.

Successful fresh pairing replaces only the same identity, keeping other peers and rejecting weaker keys. Keys and new own-address metadata replace the old entry together. Save failure does not report success or publish a new consumed CONFIG hash; partial media changes remain possible.

There is no in-app Forget button or per-peer persistent removal API. Prefer P recovery. If intentionally clearing all host registrations is necessary, shut Musha-OS down, mount the dedicated USB on the Mac and first privately back up the entire SETTINGS directory. PAIRING.DAT contains secret keys: do not upload or print its contents. After checking the backup, manually remove only SETTINGS/PAIRING.DAT, preserving CONFIG and all other files, then safely eject. This loses ALL saved peers and the consumed pair=on CONFIG fingerprint. Existing pair=on can therefore rearm pairing; review it first. It does not erase the pad's own bond, so fast-blinking pairing mode remains necessary. No automatic deletion occurs. Current evidence does not establish filesystem corruption; check any reported inconsistency before editing.

Future selective Forget proposal (not implemented): explicitly identify one peer, preserve other records and CONFIG consumption, report success only after confirmed persistence, and stop on ambiguous persistence. A separate approved UI/tests are needed.

Common session end now retains ok/error, first fixed stop trigger and exact static reason in viewer metadata. Unattributed normal completion is explicitly labelled. Endpoint/DMA behavior is unchanged.

## 日本語

通常起動は保存SC鍵を使います。鍵の不一致から自動で新規登録には切り替えません。

再登録は、パッドを切ってviewerを起動し、Pを押してから、再接続する前にパッドを高速点滅のペアリングモードにします。今回の修正で保存鍵のscan・接続・暗号化・HID探索の途中でもPを受け付け、停止確認済みの再開始経路を通って新規登録します。新規登録中のP連打では処理をリセットしません。停止・隔離済みのアダプターはOSの再起動が必要です。

新規登録は同じidentityの鍵とホスト側アドレスをまとめて置換し、別の機器の登録は保持します。認証強度を下げる置換は拒否します。保存が失敗した場合は成功やCONFIG消費済みとせず、媒体への部分書込みの可能性を残します。

現在、画面内の登録解除や機器単位の永続削除機能はありません。まずPによる再登録を優先してください。全登録を意図して消す場合は、OS終了後に専用USBをMacへ接続し、SETTINGS全体を非公開の場所へバックアップします。PAIRING.DATには秘密鍵があるため、公開・添付・内容表示をしないでください。バックアップ確認後、手動でSETTINGS/PAIRING.DATだけを削除し、CONFIGと他のファイルを保持して安全に取り外します。全機器の保存鍵とpair=onの消費済み識別情報が失われ、pair=onが残ると登録が再開し得るため、先に確認してください。パッド側の鍵は消えないのでペアリングモードは必要です。今回、実データは削除しません。不整合の報告がある場合は編集前に確認し、今回の映像だけで破損とは判断しません。

将来の選択解除は対象の明示、他の登録とCONFIG消費状態の保持、永続保存確認後だけ成功、保存状態不明なら停止、を条件とする未実装案です。別途承認済みUIと試験が必要です。

終了時はok/error・停止契機・理由をviewerに残し、正常終了でも契機未記録なら不明と表示します。endpointやDMAの扱いは変えません。
