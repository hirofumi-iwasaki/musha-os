# exFAT読出し専用解析器

2026-10-11。`musha-fs::exfat`にボリューム単体の解析器を追加した。
USB・アプリAPI・画面にはまだ組み込んでいない。
[作業順](work-plan-0.2.0.md)の工程3の中核であり、[初期設計](exfat-design.md)全体の実装完了ではない。

## 利用契約

`Workspace`を呼出し側が用意し、`FileSystem::mount`へ媒体の総セクター数、論理セクターサイズ、
選択したボリュームの開始位置と長さ、読出しcallbackを渡す。
自動パーティション選択は未実装。superfloppyは開始0、既知のpartitionはその範囲を渡す。
開始位置と長さのoverflow、媒体外・ボリューム外参照を拒否する。

callbackは要求された1セクターを全部読み、失敗時はErrorを返す。書込みcallbackは存在しない。
mountから最後のreadまで媒体は不変でなければならない。
この同期APIのcallbackに対してAgainや非同期DMAを直接接続しない。USB協調処理は後続工程で実装する。

`entries()`は検証済みのルートファイル一覧を返す。名前は元のUTF-16で、表示用短縮名に変換しない。
`read_at(index, offset, out)`で最大4096byteを読む。返却数以降のbufferは変更せず、失敗時はbuffer全体を変更しない。
空bufferまたはEOF以降では0を返す。64MiBを超えるファイルは一覧に残し、readをTooLargeで拒否する。
indexはそのmountだけに有効。アプリへ渡す世代付きhandleではない。

## 実装済みの検証

- revision 1.00、FAT1本、512/4096byteセクター。最大32MiB clusterをセクター単位で処理。
- main bootの署名・checksum・範囲・サイズを検査。dirty/media failureやactive FAT不整合を拒否。
  mainが破損していればbackupから自動復旧しない。
- bitmapとup-caseが各1個あること、長さ・FATチェーン・使用clusterの割当を確認。
- 非圧縮・圧縮up-caseのchecksum、展開数65536、必須ASCII変換を検証。
- file/stream/name entry順序、secondary数、set checksum、名前hash、禁止文字、UTF-16を検証。
  ファイル同士の大文字小文字を無視した重複名を拒否する。
- NoFatChainによる連続配置とFATによる断片化。短い・長すぎる・循環・無効なチェーンを拒否。
- ValidDataLengthより後ろをゼロで返す。ディスクの未初期化byteを利用者bufferへコピーしない。
- 未知のcritical entryはUnsupported。未知のbenign secondaryは必要なname entryの後で読み飛ばす。

全媒体の整合性検査器ではない。例えば未使用ファイル間のcross-linkや孤立した割当を全走査して検出しない。
ファイルの割当チェーン検査はread時に行うため、一覧成功は全ファイル内容の正常性を保証しない。
subdirectoryは一覧対象外。ファイル属性・タイムスタンプのUI表示は未実装。

## 上限とメモリー

root走査は1MiB、ファイル一覧は128件。超過時はLimitでmountを失敗させる。
部分一覧と打切り表示を返す契約は工程4で整える。
mountと各readにそれぞれ最大100万セクターcallbackの上限を設け、循環検出も併用する。
上限は処理量であり、callback内部で停止しない時間保証ではない。

heap allocationやunsafeは使わない。Workspaceは256KiB未満をホスト試験で確認する。
初期化時も小さなスタックにWorkspaceを一時生成しないよう、runtime統合時には配置方法を確認する。
一時bufferは4096byte単位。今回、UEFI最終リンク後のスタック最大使用量は測定していない。
各readでチェーンを再検証する方式なので、細かいreadを長いファイルで繰り返す性能改善は後続課題。

## 検証

新規16試験で、[独立検査済みfixture](exfat-fixtures.md)の5ファイルを読み、
1/7/511/512/513/4096byte単位で元の内容と比較した。
セクター境界・root cluster境界・EOF・空ファイル・4096byteセクター・非ゼロ開始範囲、
圧縮up-case、破損checksum、bitmap欠落、循環/短い/長いチェーン、範囲外・overflow、
名前・ValidDataLength異常、I/O失敗時のbuffer不変、root走査上限、ファイルサイズ上限を含む。

既存FAT32の9試験を含む全ホスト182試験が合格。fixture生成器の7試験も合格。統合試験はPython3で再現可能なfixtureを生成し、外部ディスクへ書き込まない。
UEFI no_stdのcheckと通常版buildも成功した。
USB経由のexFAT読出し、QEMU画面操作、実機確認は未実施。

```sh
cargo test --locked -p musha-fs
cargo check --locked --target x86_64-unknown-uefi --lib -p musha-fs
python3 tools/test-exfat-fixture.py
```

次はGPT/MBRの選択と追加境界試験を整え、工程4の共通ファイルAPIへ進む。
