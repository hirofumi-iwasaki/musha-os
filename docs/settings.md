# SETTINGS files / 設定ファイル

Status: current

## English

This describes the current v0.4.0 development implementation, not completed physical or Bluetooth acceptance. See [startup configuration](startup-configuration.md), [stage3 evidence](stage3-0.4.0-progress.md) and [work plan](work-plan-0.4.0.md).

### Ownership and fixed capacity

`/SETTINGS/CONFIG.TXT` belongs to the user. The OS only reads it; it never pads, resizes, rewrites or deletes it. Boot configuration is read from the FAT32 boot device before the first display. CONFIG on exFAT is not read.

The internal writer currently permits only `/SETTINGS/PAIRING.DAT`. Other OS-owned records are future work, not an existing general write API. No application write syscall, delete, rename, mkdir or LFN write is provided. Existing same-named user files, unknown record versions and corrupt records are refused. Full magic/version/bounds/reserved-byte/CRC validation is required; filename alone does not establish ownership. A deliberately copied or forged valid record is indistinguishable from an OS-created record; this is format recognition, not authenticated origin.

The directory file size is always4096 bytes. The chain contains exactly ceil(4096 / cluster bytes) clusters:8 for512-byte clusters,1 for4096-byte clusters, and1 for8192-byte clusters. The last case allocates8192 bytes with4096 bytes of slack; allocation need not equal directory size. This is the minimum chain required by the file size, with no extra clusters. Every byte of every newly allocated cluster, including slack, is zeroed before publishing allocation metadata. Overwrite uses the existing chain without additional allocation.

Logical payload length is stored inside the record. Zero-length testing means payload length0, not a zero-byte FAT file. The current version1 pairing format has a64-byte header, at most8 bonds of40 bytes, explicit payload length0–320, a consumed-CONFIG hash flag/value and whole-file CRC32; unused bytes remain zero. Record corruption is refused before exposing parsed keys. CRC is not authentication.

### Pairing request consumption

The firmware reader hashes the exact raw CONFIG bytes with SHA-256 after bounded successful parsing and successful close; CRLF/LF, comments and whitespace are part of the hash. Only the32-byte digest survives the firmware file/buffer lifetime. `pair=on` with the same hash as a successfully consumed request is suppressed. Saving identical content again does not re-enable pairing: actual content change is required. A content change only enables a request when the parsed configuration still has `pair=on`.

Stage6 must record the hash only after successful pairing and successful durable record persistence. Pairing failure or persistence failure leaves the request unconsumed. The request/acknowledgement logic is currently pure host-tested code; actual SMP/Bluetooth success and the production save caller are not implemented. Normal startup currently performs no pairing-record write. SHA-256 provides strong collision resistance, not proof of ownership or secrecy of CONFIG.

### Failure limits

The writer preflights mirrored FATs, directory bounds, primary/backup FSInfo, capacity and cross-links. Writes use BOT WRITE(10), SYNCHRONIZE CACHE(10) and protection/sense checks. Recoverable metadata failures attempt rollback; persistent failure or removal during metadata publication can also defeat rollback and returns recovery-required status. There is no power-loss atomicity guarantee. Disposable-image fsck and unchanged-region checks cover the tested failure boundaries; they do not establish arbitrary-removal safety. Physical USB cold-boot/read-back, Windows chkdsk and actual Bluetooth remain unverified.

## 日本語

本書は現在のv0.4.0開発実装を説明し、実機・Bluetooth受入完了を意味しません。[起動設定](startup-configuration.md)、[工程3証拠](stage3-0.4.0-progress.md)、[作業計画](work-plan-0.4.0.md)を参照してください。

### 所有権と固定容量

`/SETTINGS/CONFIG.TXT` は利用者所有です。OSは読取りだけを行い、padding・resize・書換え・削除をしません。起動設定は初表示前にFAT32起動媒体から読みます。exFAT上のCONFIGは読みません。

内部writerが現在許可するのは `/SETTINGS/PAIRING.DAT` だけです。他のOS所有recordは将来計画で、一般書込みAPIではありません。アプリ書込みsyscall・削除・改名・mkdir・LFN書込みはありません。同名利用者file・未知版・破損recordを拒否します。magic/version/上限/予約byte/CRCの完全検証が必要で、名前だけでは所有性を判断しません。意図的にコピー・偽造された正常recordとOS作成recordの出自は区別できず、形式認識であって認証ではありません。

directory file sizeは常に4096bytesです。chainは厳密にceil(4096 / cluster bytes)個で、512byte clusterなら8個、4096byteなら1個、8192byteでも1個です。最後の場合は8192bytesを割当て、4096bytesがslackです。割当てbytesとdirectory sizeが等しい必要はありません。file sizeに必要な最小chainで余剰clusterがないという意味です。初回は割当てmetadata公開前にslackを含む全clusterの全byteをゼロ化します。上書きでは同じchainを使い、追加割当てしません。

論理payload有効長はrecord内部に保存します。0長試験はpayload有効長0で、FAT fileを0byteにする意味ではありません。現version1 pairing形式は64byte header、最大8個の40byte bond、有効長0–320、消費済みCONFIG hashのflag/value、file全体CRC32を持ち、未使用部はゼロです。破損recordは鍵の解析結果公開前に拒否します。CRCは認証ではありません。

### ペアリング要求の消費

firmware readerは上限付きparseとclose成功後、CONFIGの正確なraw bytesをSHA-256化します。CRLF/LF・コメント・空白もhashに含まれます。firmware file/bufferの寿命後に残るのは32byte digestだけです。成功時に消費したhashと同一なら `pair=on` を抑止します。同じ内容を保存し直すだけでは再有効化せず、内容変更が必要です。変更後もparse結果が `pair=on` の場合だけ要求が有効になります。

工程6ではpair成功かつrecordの永続保存成功後だけhashを記録します。pair失敗・保存失敗では要求を消費しません。現在の要求/acknowledgement処理はhost試験済み純粋関数で、実SMP/Bluetooth成功と本番保存callerは未実装です。通常起動はpairing recordを書きません。SHA-256は強い衝突耐性を持ちますが、所有性やCONFIGの秘密性を証明しません。

### 失敗時の限界

writerは両FAT・directory上限・primary/backup FSInfo・容量・cross-linkを事前検査します。BOT WRITE(10)・SYNCHRONIZE CACHE(10)・保護/sense検査を使います。回復可能metadata失敗ではrollbackを試みますが、永続故障やmetadata公開中の取外しではrollbackも失敗し、要回復状態を返します。停電原子性は保証しません。使い捨て像のfsck・変更外領域検査は試験した境界の証拠で、任意取外しの安全性を証明しません。実USB冷間起動/読戻し・Windows chkdsk・実Bluetoothは未確認です。

## Write preflight rules — English (owner decision 2026-10-18)

| Check | Rule |
|---|---|
| FAT layout | Two FATs, mirroring on (ExtFlags 0); refused otherwise |
| FAT integrity | Only the FAT sectors holding the record's clusters (existing chain or newly selected free clusters) are mirror-compared and audited for incoming references. No whole-FAT check, at save or at boot |
| Free clusters | Searched from the FSInfo next-free hint, wrapping once; the read budget (20,000 per step) ends the search as `Budget` |
| Primary FSInfo | Signature required; free count and next-free hint must be in range |
| Backup FSInfo | Missing or invalid signature: continue with the primary only and log a warning |
| `SETTINGS` entry | Directory bit 0x10 required; VOLUME_ID 0x08 refused; ARCHIVE, HIDDEN, SYSTEM and READ_ONLY ignored; a long name is accepted when the short name is `SETTINGS   ` |
| `PAIRING.DAT` entry | Short name only, no attribute bits, at most 4096 bytes, recognized OS record |
| FSInfo update | Free count decremented as a hint, or set unknown (0xFFFFFFFF) when it cannot cover the allocation |

The OS still writes only `SETTINGS/PAIRING.DAT`; these rules do not widen what is written.

## 書込み事前検査の規則 — 日本語（所有者決定 2026-10-18）

| 検査 | 規則 |
|---|---|
| FAT の構成 | FAT 2 本・ミラー有効（ExtFlags 0）。それ以外は拒否 |
| FAT の整合性 | 記録のクラスター（既存チェーンまたは新たに選んだ空き）を含む FAT セクターだけをミラー比較し、入参照を監査。保存時・起動時とも FAT 全体の検査は行わない |
| 空きクラスター | FSInfo の next-free ヒントから探し、1 周で打ち切る。読出し予算（各段階 20,000）で尽きたら `Budget` |
| 主 FSInfo | 署名必須。空き数と next-free ヒントは範囲内であること |
| バックアップ FSInfo | 欠落・署名不正なら主 FSInfo だけで続行し、警告を記録 |
| `SETTINGS` エントリー | ディレクトリービット 0x10 必須。VOLUME_ID 0x08 は拒否。ARCHIVE・HIDDEN・SYSTEM・READ_ONLY は無視。短い名前が `SETTINGS   ` なら長い名前付きも可 |
| `PAIRING.DAT` エントリー | 短い名前のみ、属性ビットなし、4096 バイト以下、OS が認識できる記録 |
| FSInfo の更新 | 空き数はヒントとして減算。確保分に足りなければ不明（0xFFFFFFFF） |

OS が書くのは引き続き `SETTINGS/PAIRING.DAT` だけで、これらの規則は書き込む対象を広げません。
