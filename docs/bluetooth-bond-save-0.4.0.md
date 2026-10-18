# Bluetooth bond save on large FAT32 media — English

Status: implemented locally (commits `2dbff44`, `1fc4f88`); physical retry pending. IMG_7741 (`015cae1`) bound HOGP (`HOGP bound input-reports=1 unmapped=0; stack never-touched 127664 of 250048 bytes`), then 8.5 s later logged `bond save failed reason=PersistenceUnknown transport=ok` and isolated Bluetooth with `BT BOND SAVE UNKNOWN`. Storage and keyboard stayed live.

## Cause

The medium has 60,123,103 sectors of 512 bytes and 16 KiB clusters (macOS-formatted, about 1.88 million clusters), so one FAT is about 14,700 sectors. The SETTINGS preflight compared both mirrors of the whole FAT and counted free clusters, about 29,400 single-sector reads against a 20,000-read budget. The budget ran out (`Error::Limit`), and the bond-save path folded every refusal into `PersistenceUnknown`. The 8.5 s matches roughly 20,000 BOT reads. A host test with this geometry shows that the whole-FAT reads exceed the budget. The exact failing step on the device is not yet captured: the new log line will show it.

## Changes

- Every refusal is reported as `bond save failed reason=<kind> step=<old-preflight|read-old|request|preflight|audit|verify-record|write> detail=<check> value=0x.. reads old=N preflight=N audit=N elapsed=Nms transport=..`. A Layout refusal names its check, for example `SETTINGS attribute` with the attribute value, `SETTINGS has a long name`, `FAT count`, `FAT ExtFlags (mirroring off)`, `FSInfo signature (0 primary, 1 backup)`, `FSInfo free count`, `FAT mirrors differ at FAT sector`, or an audit limit. An exhausted read budget is `Budget`. The viewer shows the kind, for example `Bluetooth unavailable: BT BOND SAVE LAYOUT`.
- Preflight reads only the FAT sectors it needs. For an existing record, it reads the record's chain sectors and compares both mirrors of each. For a new record, it searches for free clusters from the FSInfo next-free hint, wrapping once. The audit checks incoming FAT references only within the FAT sectors holding the selected clusters; the directory audit is unchanged. On the IMG_7741 geometry, a create or an overwrite takes fewer than 32 reads per step in a host test.
- The FSInfo free count is no longer verified against a full count. The writer still decrements it as a hint, and writes unknown (0xFFFFFFFF) when it cannot cover the allocation; this is a valid FAT32 value.
- BOT reads stay single-sector. With the bounded scan the reads drop from about 29,400 to tens, so multi-sector READ(10) is not needed now. It would also change the transport shared with the file viewer.

## Owner decisions

1. **Whole-FAT integrity check.** The former preflight rejected mirror differences and free-count mismatches anywhere in the FAT, and the audit rejected shared chain tails anywhere. Now only the sectors touched by the save are checked. A full check costs about 29,400 reads (about 12 s at the observed rate) on this medium for one 4 KiB file. Recommendation: keep the bounded check. If a full check is wanted, run it once per boot in the background, not per save.
2. **SETTINGS attribute.** Accept the directory when bit 0x10 is set, ignoring ARCHIVE, HIDDEN and SYSTEM; today only exactly 0x10 (within 0x3f) is accepted. macOS can set ARCHIVE. The OS still writes only its own file inside it. Not implemented; waiting for approval.
3. **SETTINGS with a long name.** Accept a long-name entry whose short name is exactly `SETTINGS   `; today any long name refuses. macOS may create a long name for a folder that it shows as "SETTINGS". Not implemented.
4. **Missing backup FSInfo.** Fall back to the primary FSInfo with a warning instead of a Layout refusal. Not implemented.

## Media metadata requested from the owner (read-only)

To reproduce the exact check on the host, the owner can provide (no user data or keys):

- `diskutil info /Volumes/MUSHA-OS`
- `ls -lO@ /Volumes/MUSHA-OS /Volumes/MUSHA-OS/SETTINGS`
- After unmounting (`diskutil unmount /Volumes/MUSHA-OS`), with N from `diskutil list`:
  - `sudo dd if=/dev/rdiskN bs=512 count=1`: the protective MBR.
  - `sudo dd if=/dev/rdiskNs1 bs=512 count=16`: the boot sector, FSInfo and backup.
  - `sudo dd if=/dev/rdiskNs1 bs=512 skip=<reserved> count=8`: the first FAT sectors.
  - The first cluster of the root directory and of `SETTINGS`.

Until then, the next physical run's `detail=` line identifies the check.

# 日本語: 大容量 FAT32 媒体での Bluetooth ボンド保存

状態: ローカル実装済み（`2dbff44`、`1fc4f88`）・実機再試験待ち。IMG_7741（`015cae1`）では HOGP の結合（`HOGP bound input-reports=1 unmapped=0; stack never-touched 127664 of 250048 bytes`）まで成功しました。その 8.5 秒後に `bond save failed reason=PersistenceUnknown transport=ok` となり、`BT BOND SAVE UNKNOWN` で Bluetooth を分離しました。ストレージとキーボードは維持されました。

## 原因

媒体は 512 バイト × 60,123,103 セクター、クラスター 16 KiB（macOS でフォーマット、約 188 万クラスター）で、FAT 1 本は約 14,700 セクターです。SETTINGS の preflight は FAT 全体を両ミラーで比べて空き数を数えるため、1 セクターずつ約 29,400 回読みます。予算は 20,000 回なので途中で尽き（`Error::Limit`）、ボンド保存の経路はあらゆる拒否を `PersistenceUnknown` に畳んでいました。8.5 秒は約 20,000 回の BOT 読出しと合います。この幾何の host 試験で、全 FAT の読出しが予算を超えることを確認しました。実機でどの段階が拒否したかはまだ記録されていません。新しいログ行で分かります。

## 変更点

- 拒否はすべて `bond save failed reason=<種別> step=<old-preflight|read-old|request|preflight|audit|verify-record|write> detail=<検査> value=0x.. reads old=N preflight=N audit=N elapsed=Nms transport=..` で出します。Layout は検査名を示します（例：属性値付きの `SETTINGS attribute`、`SETTINGS has a long name`、`FAT count`、`FAT ExtFlags (mirroring off)`、`FSInfo signature (0 primary, 1 backup)`、`FSInfo free count`、`FAT mirrors differ at FAT sector`、監査の上限）。読出し予算の枯渇は `Budget` です。ビューアーには `Bluetooth unavailable: BT BOND SAVE LAYOUT` のように種別を出します。
- preflight は必要な FAT セクターだけを読みます。既存の記録ではそのチェーンのセクターを読み、両ミラーを比べます。新規の記録では FSInfo の next-free ヒントから空きクラスターを探し、1 周で打ち切ります。監査は、選んだクラスターを含む FAT セクターの範囲で入参照を確認します。ディレクトリー監査は従来どおりです。IMG_7741 の幾何の host 試験では、作成・上書きとも各段階 32 回未満の読出しで済みます。
- FSInfo の空き数は全数と照合しなくなりました。writer は引き続きヒントとして減算し、確保分に足りない場合は不明（0xFFFFFFFF、FAT32 で有効な値）を書きます。
- BOT の読出しは 1 セクター単位のままです。走査を限定したことで読出しは約 29,400 回から数十回に減ったため、複数セクターの READ(10) は今は不要です。ファイルビューアーと共有する転送路を変えることにもなります。

## 所有者判断

1. **FAT 全体の整合性検査。** 従来は FAT のどこかでミラー差・空き数の不一致があれば拒否し、監査も共有チェーン末尾を FAT 全体で拒否していました。現在は保存で触れるセクターだけを検査します。全体検査は、4 KiB のファイル 1 本の保存に対してこの媒体で約 29,400 回（観測した速度で約 12 秒）かかります。推奨は限定検査の維持です。全体検査が必要なら、保存ごとではなく起動ごとに 1 回、裏で行う形にします。
2. **SETTINGS の属性。** 0x10 ビットが立っていれば ARCHIVE・HIDDEN・SYSTEM は無視して受け付けます。現在は 0x3f の範囲でちょうど 0x10 のみ受け付けています。macOS が ARCHIVE を付けることがあります。OS がその中で書くのは自分のファイル 1 本だけという方針は変わりません。承認待ちで未実装です。
3. **長い名前付きの SETTINGS。** 短い名前がちょうど `SETTINGS   ` なら受け付けます。現在は長い名前があると拒否しています。macOS は「SETTINGS」と表示するフォルダーに長い名前を作ることがあります。未実装です。
4. **バックアップ FSInfo が無い場合。** Layout で拒否せず、主 FSInfo だけで続け、警告を出します。未実装です。

## 所有者へ依頼する媒体のメタデータ（読出しのみ）

host で同じ検査を再現するため、次の提供をお願いします（利用者データや鍵は含みません）。

- `diskutil info /Volumes/MUSHA-OS`
- `ls -lO@ /Volumes/MUSHA-OS /Volumes/MUSHA-OS/SETTINGS`
- アンマウント（`diskutil unmount /Volumes/MUSHA-OS`）の後、`diskutil list` で確認した N を使って：
  - `sudo dd if=/dev/rdiskN bs=512 count=1`：保護 MBR
  - `sudo dd if=/dev/rdiskNs1 bs=512 count=16`：ブートセクター・FSInfo・バックアップ
  - `sudo dd if=/dev/rdiskNs1 bs=512 skip=<reserved> count=8`：FAT の先頭
  - ルートディレクトリーと `SETTINGS` の先頭クラスター

それまでは、次回の実機の `detail=` 行で拒否した検査が分かります。

## Decisions — English (2026-10-18)

The owner approved all four items above: no whole-FAT check (the bounded check is the method, also not at boot); SETTINGS accepted when bit 0x10 is set (VOLUME_ID refused, other bits ignored); long-name SETTINGS accepted when the short name matches; missing or invalid backup FSInfo tolerated with a warning (an invalid primary is refused). Implemented in the SETTINGS writer; rules in [SETTINGS files](settings.md).

日本語: 上記 4 項目はすべて所有者が承認しました。FAT 全体の検査は行わない（限定検査を正式な方式とし、起動時にも行わない）、0x10 ビットがあれば SETTINGS を受け付ける（VOLUME_ID は拒否、他のビットは無視）、短い名前が一致すれば長い名前付きの SETTINGS も受け付ける、バックアップ FSInfo の欠落・不正は警告付きで許容（主 FSInfo の不正は拒否）。SETTINGS writer に実装し、規則は[SETTINGS ファイル](settings.md)にあります。
