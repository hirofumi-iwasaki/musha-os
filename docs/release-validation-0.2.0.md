# v0.2.0 工程7・公開候補の検証記録

2026-10-12。対象ブランチ release/0.2.0。
工程7のローカル統合回帰と候補準備を進め、実機受入・公開承認は別の完了条件として扱う。

## 標準exFATとの互換性

macOSのhdiutil create（64MiB、ExFAT、layout NONE）で新規仮想媒体を作り、
MUSHA.TXT、FRAGMENT.BIN、長いASCII名、EMPTY.TXT、ZEROTAIL.BINを標準ファイル操作で配置した。
macOSが各ファイルにAppleDouble補助ファイルを追加したため、一覧は10項目。
補助ファイルを含む実際の一覧順で、目的の5ファイルを選んで内容を照合した。
この標準像のFRAGMENT.BINが物理的に断片化しているとは主張しない。断片化は合成像で別に確認する。

QEMUの別controller入力構成で全ページ・EOF・空ファイル・一覧復帰が合格。
fsck_exfat -nも終了値0でmain/backup boot、up-case、hierarchy、bitmapを正常と確認。
検査用接続は解除済み。物理ディスクは変更していない。

標準像SHA-256: 007827156e460d81974854317cebaa825c46bc31ae9daf3c419afb13cb186e32。
標準像はタイムスタンプ等を含む手動生成の検証物で、再生成時の同一ハッシュは要求しない。
再実行はsmoke-viewer.pyの --apple-double-files を用い、通常の合成像の試験とは分ける。

## 実機と公開

[実機受入試験](hardware-acceptance-0.2.0.md) は全て未実施。
MacBook ProのPCI復旧を含む通常候補の動作を、v0.1.0の結果だけで合格にしない。
[公開候補ノート](release-notes-0.2.0-draft.md) と候補manifestには未公開・実機未確認を明記する。
物理USB更新、mainへのマージ、タグ、GitHub Release公開は今回実施していない。

## 統合回帰（ローカル合格）

CIの実行ブロック14グループをローカルで順に実行し、全て終了値0。
ホスト224件、Pythonのfixture／USB生成／パッケージ試験、外部コードの固定SHA照合、
HID mutation 100,000回×2 seed、UEFIのライブラリ検査を含む。
別途HIDのUEFIメモリー計測も合格した。CIサーバーでの実行は今回行っていない。

QEMU 11.1.2でUSB列挙、キー継続入力、無入力、keyboard切断、link-down、
ARP/ICMPと257 UDP echo、複数controller、5段hub、CPU例外と保護page、
xHCI／descriptor／BOT／NIC timeout、アプリ失敗、破損GPT/FAT32を確認。
さらにFAT32/exFATの共通file API、実行中取消し、別controllerの保持、
媒体切断とDMA停止、工程6の閲覧7構成を回帰した。
標準formatter像の確認は上記とは別の追加試験。

再実行元は .github/workflows/release-candidate.yml。
ローカルの各グループ結果は out/phase7/00.log〜13.log、進行記録はprogress.log、
標準像はstandard-viewer-final.logとstandard-fsck.logに保存。
通常候補はこの記録を含むクリーンなコミットから、隔離target directoryで生成する。

## 候補検査の方法

tools/prepare-release.py --version 0.2.0 はfeature指定なしのEFIを新規ビルドし、
manifestに完全commit、working tree、ツール版、Cargo.lock hash、featureとRUSTFLAGSを保存する。
同梱ファイルとアーカイブのSHA256SUMSを作る。タグ・外部公開・物理媒体書込みは行わない。

tools/smoke-normal-viewer.py はdebug出力に依存せず、工程6で検証した同一fixtureの画面と
候補の一覧→開く→一覧の画像を画素単位で比較する。Q終了画面も保存する。
内部DMA停止の証拠は診断版の回帰試験が担当し、この通常版試験とは区別する。
候補の生成commitと最終SHAは候補manifest／SHA256SUMSを正とする。
