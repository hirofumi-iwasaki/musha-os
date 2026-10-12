# ファイル一覧・内容表示（工程6）

2026-10-12。`release/0.2.0` の通常版に組み込む読出し専用ビューアー。
工程6の実装とホスト／QEMU検証を完了。実機確認・公開判断は工程7で行う。

## 操作

起動時に最初に利用可能になったFAT32/exFAT媒体のルートファイルを表示する。
起動用FAT32とexFATデータ媒体の構成・選択制約は [共通ファイルAPI](stream-file-api.md) に従う。
外付けUSB Boot Keyboardが必要。内蔵キーボードの対応は追加していない。

| キー | 一覧 | 内容 |
|---|---|---|
| 上／下 | 前／次のファイル | 前／次の256バイト |
| Page Up／Page Down | 10項目移動 | 前／次の256バイト |
| Home | 最初の項目 | ファイル先頭 |
| End | 最後の項目 | 操作なし |
| Enter | 選択ファイルを開く | 操作なし |
| 左／右 | 選択名の前／次の部分 | 同左 |
| Esc／Backspace | 操作なし | 一覧へ戻る。読出し中は取消し |
| Q | セッション終了 | 同左 |

一回の押下につき一操作。キーを押し続けた際の自動リピートはない。
内容は16進数とASCIIの併記。位置は画面上部で10進数、各行で16進数を表示する。
`Bytes start..end` のendは範囲に含めない。末尾はEOF、空ファイルは専用表示。
読出し中はページを空にして待機表示する。失敗時に前の内容を残さない。

## 名前と上限

- ルート直下の最大128ファイル、一覧10件／ページ。サブディレクトリーは対象外。
- 一覧は名前47文字までと省略記号 `~`、一覧番号とサイズを表示する。
- 選択名は完全なUTF-16名をエスケープしたものを72文字ずつ表示し、左右で全体を確認できる。
- ASCIIの表示可能文字を描画し、それ以外のUTF-16コード単位は `\uXXXX`。
  サロゲートもそれぞれ表示し、バックスラッシュ自体も `\u005C` とする。
  表示が省略されても選択はAPIのIDで行い、表示文字列を検索キーにしない。
- ファイル上限64MiB、1ページ256バイト。ファイル全体をメモリーへ取り込まない。
- 1ページの取得は開始から10秒まで。期限超過はエラー表示し、要求を取消す。
  USB側のタイムアウトが先に発生した場合は媒体セッションを終了する。
- 後方移動はclose／再openし、先頭から256バイトずつ読み飛ばす。
  直接seekは未実装のため、大きなオフセットへの再読出しは10秒制限に達する場合がある。
- 媒体切断・入力終了時は保存IDと内容を無効化し、再起動案内へ移る。再接続による自動復帰はしない。
- 640×480以上を表示確認の基準とする。小さい画面は境界外を書かずclipするが全項目の表示は保証しない。

## 実装

`musha-viewer` は固定サイズの状態機械。画面操作から直接USBを呼ばず、
`Application::files` 内で一覧取得・open/read/closeを行う。
取消し済みの実行中要求は工程5のAPIがDMA完了まで保持し、新しいopenはAgainで待つ。
1回の描画は8192画素以下の背景消去、または80文字以下の1行へ分割する。
API `text_at` はピクセル座標・倍率1〜4を使用し、画面外をclipする。
Viewer本体は16KiB未満、読出し用一時領域は256バイト。動的な全ファイル割当はない。

通常版はファイルビューアーを表示し、USB探索後の従来の診断パネル描画を抑止する。
起動時の探索、RAM検査、通信と終了時のDMA停止は継続する。
既存の `qemu-debug` 試験と `t2-diagnostics` は従来の診断画面・Esc終了を維持する。
`file-viewer-probe` は通常ビューアーにQEMU検証用の表示完了ログと無期限入力を付ける。
ログのページハッシュはFNV-1aで内容一致を検査するもので、暗号学的検証ではない。

## ローカル検証

- ホスト: 前後移動、最終短ページ、EOF、空ファイル、128項目の移動、
  同じ省略名でも別IDの選択、Unicode代替表示、名前スクロール、64MiB超の拒否、
  取消し後の完了待ち、タイムアウト、破損、切断による内容無効化。
- 描画: 1×1、640×480、1280×800の境界・stride padding・guard保存、倍率拒否、表示可能ASCII。
- QEMU: FAT32、およびexFATの5ファイルを一覧から選択。
  断片化ファイルの最終ページ、長い名前、空ファイル、ValidDataLength後のゼロ補完を元データと比較。
- 同じxHCI／別xHCIのキーボード構成で操作。読出し待ちのEsc取消しと再open、
  USB切断、故障注入timeoutで古い内容を表示しないこととDMA停止を確認。
- 対応媒体を取得できない場合の案内表示も確認。
- 一覧・16進表示・空ファイルのQEMU画面を目視確認。USBデータイメージはreadonlyで接続し、前後のSHA-256一致を検査。
- 既存の協調I/O試験でもFAT32／exFAT、入力、UDP通信を回帰確認する。

再実行例（生成先は新しいファイル名を使う）:

```sh
cargo test --locked --workspace --exclude musha-boot
cargo build --locked --release --target x86_64-unknown-uefi -p musha-boot --features file-viewer-probe
python3 tools/make-usb-image.py out/viewer-boot.img --efi target/x86_64-unknown-uefi/release/musha-boot.efi
python3 tools/make-exfat-fixture.py out/viewer-data.img
python3 tools/smoke-viewer.py --firmware-dir "$MUSHA_FIRMWARE_DIR" --boot-image out/viewer-boot.img --data-image out/viewer-data.img --split-controller
```

取消し／切断は `file-viewer-probe,live-file-probe` でビルドし `--case cancel`／`disconnect`。
timeoutは `file-viewer-probe,file-app-timeout` と `--case timeout`。
FAT32は `--format fat32`、データにもmake-usb-image.pyの像を指定する。
`--case no-medium` はboot-checksum破損fixtureをデータに指定する。
これらは `.github/workflows/release-candidate.yml` に登録する。CI上の実行結果は別途確認する。

工程7では標準フォーマッター由来の媒体、通常版の実機表示・キー操作、
MacBook ProのPCI復旧との組合せを確認する。今回、物理USBは更新していない。

最終確認: ホスト224件合格。QEMU閲覧7構成（FAT32、exFAT同一／別controller、取消し、切断、timeout、媒体利用不可）合格。既存の協調I/O回帰2構成で各257 UDP echoを含め合格。
通常版（feature指定なし）でもQEMUで一覧→内容→一覧の画面が検証版と画素単位で一致し、debugログを出さないことを確認した。
通常EFIのSHA-256: `7e10b39a5e6e36326bb85a32347636eb05363d4ae2e588688ee585267cf7c685`。
通常版の物理MacBook Proでの確認は未実施。
