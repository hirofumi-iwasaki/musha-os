# exFATと逐次ファイル読出しの初期設計

Status: record (2026-10-11 to 2026-10-15)

2026-10-11。v0.2.0の実装に向けた契約案であり、対応済み機能の一覧ではない。
[作業順](../work-plan-0.2.0.md)の工程1を具体化する。実装状況は [解析器](../exfat-reader.md) を参照。既存snapshot APIは工程4まで変更しない。

## 対応範囲と初期上限

| 項目 | 初期設計 |
|---|---|
| 形式 | exFAT revision 1.00、FAT 1本。TexFATはUnsupported |
| セクター | 512 / 4096 byte。媒体の論理セクターと一致必須 |
| クラスター | 最大32MiB。全体をバッファーにせずセクター単位で処理 |
| 配置 | superfloppy、MBR primary type 07、GPT Microsoft Basic Data。起動ESPはFAT32のまま |
| ファイル | ルート直下、最大64MiB。超過は一覧にサイズと「上限超過」を表示しopenを拒否 |
| 名前 | 最大255 UTF-16 code unit。元の名前を保持し、描けない文字はエスケープ表示 |
| 一覧 | 最大128ファイル、ディレクトリー走査最大1MiB。打切りを明示し完全な一覧と偽らない |
| 転送 | 一度に最大4096byte、媒体ごとの要求は1個。初期は閲覧用handle 1個 |
| 長期保持メモリー | 一覧128×名前510byteと属性、up-case展開128KiB、セクターバッファー等を含め256KiB以内を目標。スタックに配置しない |

上限はMusha-OSの選択でありexFAT形式自体の限界ではない。LBA・サイズ計算はu64とchecked演算を使う。
64MiB上限でも4KiB snapshotから十分拡張でき、巨大ファイル対応を公開条件へ追加せずに済む。
FAT32はまずルート短名を同じ一覧・読出し契約に接続する。FAT32 LFNは追加目標とする。

## 検証と破損時の扱い

Microsoftの [exFAT仕様](https://learn.microsoft.com/en-us/windows/win32/fileio/exfat-specification)
のboot region、FAT、bitmap、up-case table、file/stream/name entryを基準に実装する。
main bootのchecksum・幾何情報を検証する。初版はmainが不正ならCorruptとし、backupへの自動復旧は行わない。
backupを利用する場合も独立検証を必須とする。dirty/media failure状態は初版では読出しを拒否する。

entry set checksum、secondary数、名前長・hash、up-case checksumと展開数、
FirstCluster、DataLength、ValidDataLength、領域範囲を検証する。
未知のcritical entryはUnsupported、benign entryは仕様に沿って読み飛ばす。
NoFatChainは連続領域を使い、それ以外はFATをたどる。アクセスするclusterのbitmapを確認する。
ValidDataLength以降DataLengthまでをゼロとして返し、未初期化ディスク内容を表示しない。
走査には独立の回数上限と循環検出を設け、短いチェーンや不正clusterはCorruptとする。

名前を省略・代替表示した文字列でopenしない。一覧から得た非公開ID（媒体世代とentry位置）で選ぶ。
名前hashだけで一致を決めない。元UTF-16とup-case tableを用いる検証は表示フォントと独立する。
不正なUTF-16は名前の検証エラーとする。名前の同一視規則と選択IDの寿命は工程4の試験で固定する。

## APIと協調処理

既存 `/DIAGNOSE.TXT` snapshotと新しい媒体上のファイルを別の所有権で扱う。
新APIは概念上 `list_begin/list_next/open_id/read/close`。実際の型とAPI版更新は工程4で確定する。
一覧とreadはPending（Again）を返せる。Againで位置・利用者bufferを変更せず、同じ要求を重複発行しない。
完了後にだけ利用者bufferへコピーし位置を進める。空でないreadの0byteがEOF。
close/cancel後の遅い完了を世代で除外するが、DMAが停止・完了するまでは転送bufferを再利用しない。
媒体切断で関連IDとhandleを失効し、再接続は別世代。世代overflow時は再利用を止める。

runtime所有の固定bufferとpoll方式を使い、アプリへの生ポインターは保持しない。
1サービス呼出しで完了観測1回、または最大1セクターの解析を進めて入力・通信へ戻る。
初期値はUSB要求1個2秒、UI操作全体10秒、時計停止対策は各要求最大100万pollとし、
QEMUで遅延・入力応答を測定して公開前に調整する。これらはまだ実測保証ではない。
BOTのCBW/data/CSW、timeout、キャンセル、controller共有、DMA停止を工程5で明示的な状態へ分ける。

Corrupt / Unsupported / TooLarge / Limit / Io / Disconnected / Invalid / Againを区別する。
部分表示後に障害が起きた場合は「途中まで」と表示し、ファイル全体の成功扱いにはしない。

## 試験計画

初期fixtureは512byteのsuperfloppyを使い、連続・断片化・長名・空ファイル・
ValidDataLengthのゼロ埋めと、boot checksum・entry checksum・循環・bitmap欠落を含める。
正常像はmacOS `fsck_exfat -n`でも検査する。fixtureの生成成功はMusha-OSの読出し成功を意味しない。
追加予定は4096byte、圧縮up-case、255文字、entryのセクター跨ぎ、GPT/MBR、
巨大サイズ、領域overflow、早期EOF、root循環、unknown entry、切断・timeout・上限到達。
生成manifestの期待内容と読出し結果を比較し、正常像のSHA-256が試験前後で変わらないことを確認する。
