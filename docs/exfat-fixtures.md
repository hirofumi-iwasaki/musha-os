# exFAT試験用イメージ

2026-10-11。工程2の初期データ。[Musha-OS解析器](exfat-reader.md)のホスト試験でも使用する。
`tools/make-exfat-fixture.py`は新規の通常ファイルだけを作る。既存ファイルやdevice nodeを開かず、
フォーマット・USB更新・自動mountを行わない。生成物は試験専用で起動用ではない。

```sh
python3 tools/make-exfat-fixture.py out/exfat-normal.img > out/exfat-normal.json
python3 tools/make-exfat-fixture.py out/exfat-cycle.img --case chain-cycle > out/exfat-cycle.json
python3 tools/test-exfat-fixture.py
```

stdoutのmanifestには名前、長さ、ValidDataLength、cluster配置、期待ファイルSHA-256、
イメージSHA-256、将来の解析器の期待結果を記録する。出力先は新しいパスを使う。
Corruptの期待は将来のMusha-OS試験用であり、外部検査器の全挙動を規定するものではない。

## 初期セット

既定は512byteセクター（`--sector-size 4096`も選択可）、1セクター/cluster、4096cluster、FAT1本のsuperfloppy。
rootはcluster 2→600、up-caseは非圧縮128KiB。独自のup-caseはASCII小文字だけを大文字化し、
それ以外はidentity mappingとする。一般的なformatterが作る推奨tableの代わりにはならない。

次の内容表と固定SHA-256は既定の512byte版。4096byte版は断片化ファイルが8451byteになる。

| ファイル | 期待内容 | 配置 |
|---|---|---|
| DIAGNOSE.TXT | 16byteの既存挨拶 | 連続 |
| FRAGMENT.BIN | 1283byteの既知byte列 | 270→272→271 |
| Long name for exFAT test.txt | 15byte | 複数name entry |
| EMPTY.TXT | 0byte | clusterなし |
| ZEROTAIL.BIN | validの5byteと695byteのゼロ | 連続。entry setはroot cluster境界を跨ぐ |

`normal`は構造・内容の独立検査用。`valid-tail`は未初期化領域を0xA5で埋め、
読出し時にValidDataLength以降をゼロとして返すことを検証する正常構造の特殊ケース。
`boot-checksum`はmainのserialだけを変更（backupは正常）、`entry-checksum`はfile setのchecksumを変更、
`chain-cycle`は270→272→270へ変更、`bitmap-free`は使用中の260を未割当にする。
破損ケースはnormalから対応箇所だけを変更し、その差分範囲も自動試験する。

## 独立検証結果

macOSの仮想ディスクとしてnormalを読出し専用で接続し、`fsck_exfat -n`で
main/backup boot、up-case、file hierarchy、bitmapが正常と確認した。
macOS側の読出しで5ファイルすべての長さとSHA-256がmanifestと一致した。
検査前後のイメージSHA-256は以下で不変。仮想ディスクは検査後に取り外した。

```text
a00ebb2b662767417172ab188f58c09f0fdd8d82a22e878e6349b92803b16501
```

通常ファイルを直接fsckへ渡すとblock ioctlが利用できず失敗したため、仮想ディスク経由で検査した。
独立検査を再実行する場合は、生成したイメージを読出し専用・mountなしで接続し、
その操作が返した仮想deviceに対して検査する。device番号を固定しない。

最初の未初期化領域0xA5入りイメージは構造検査には通ったが、macOS側のZEROTAIL読出しに0xA5が残り、
期待ハッシュと不一致だった。このためnormalとvalid-tailを分離した。
valid-tailの期待値は [Microsoft仕様7.6.5](https://learn.microsoft.com/en-us/windows/win32/fileio/exfat-specification#765-validdatalength-field)
のゼロ返却規則に基づく。macOSの当該結果を期待値に採用せず、Musha-OSで明示的に検証する。
この観測だけで原因や他のmacOS構成の挙動は断定しない。

自動試験7件は再現性、既知内容、配置、未初期化領域、破損範囲、境界跨ぎ、
既存ファイル/symlink上書き拒否、不正caseで未作成を確認する。
独立検査済みnormalのSHA-256を固定し、生成器変更時の差異を検出する。
CIではこの7件を実行する。仮想ディスクを必要とする外部検査は今回の手動検証記録であり、CIには含めない。

## 次の追加

[初期設計](exfat-design.md)に従い、Rust側の解析器試験ではGPT/MBR、非ASCII/255文字、短いFATチェーン、範囲外参照、
128/129件を追加済み。これらはPython生成器のCLIオプションではなく、試験内で合成する。
合成fixtureとMusha-OS解析器の内容比較は実施済み。工程7でmacOS標準formatter由来の64MiB像も検証した。詳細は [公開候補検証](release-validation-0.2.0.md)。
