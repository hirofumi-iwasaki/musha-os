# RustアプリAPIの初期契約

`musha-api`は単一EFIへ静的リンクするRustアプリ向けの安全なAPI。
`Application` traitの`init`、`step`、`shutdown`をランタイムが呼ぶ。
API版は1。Rust型には外部バイナリ互換性を約束しない。
C ABI、Cヘッダー、入力・ファイル・UDP・handle APIは未実装で、別途追加する。

## 所有権と実行

Contextはarenaの唯一の可変sliceを保持する。アプリは`arena()`を通して借用し、
contextと同じ領域の可変参照をランタイムが保持しない。
`arena_bytes()`、`version()`、`now_ms()`を提供する。
framebufferの生アドレスはアプリへ公開せず、context内の描画操作を使う。
context構築はランタイムのunsafe境界で、画面の寿命と排他的描画、arenaとの非重複を保証する。
隔離・プリエンプトはないため、信頼するアプリを対象とする。

initが成功したらstepを繰り返す。Continueは次周回、Completeは正常終了。
stepまたは時間取得が失敗した場合もshutdownを一度呼び、診断を表示する。
init失敗時はshutdownを呼ばず、部分初期化の後始末はinitの責任とする。
panic / CPU例外では終了処理を保証しない。
診断アプリ終了後は停止する。まだ汎用イベントループやUSB / NIC pollを提供していない。

now_msはアプリループ開始を基点とするPM timerの単調経過値で、毎step直前に更新する。
contextのadvanceはランタイム用で、時刻の巻戻しを拒否する。
呼出し間隔はPM timerの一周より短く保つ。stepのブロックは禁止する。
1ms予算を超えたstepは診断するが、強制中断はしない。
現在は整数msで2ms以上を超過として報告するため、厳密な1ms判定は未提供。

## 描画

pixelとrectangleはRGB色を画面形式へ変換し、境界外や算術overflowをInvalidとして返す。
rectangleは全範囲を検証してから書き込む。textは既存の固定フォントでclipして描画する。
描画は同期処理なので、大きな描画はアプリがstepへ分割する。
今のErrorはInvalid / Unsupported / IoのRust enum。将来のC整数status契約とは区別する。

## 診断アプリと試験

initでHello Musha-OS!を表示する。64ページずつ書込むphaseと読戻すphaseで
RAM全ページの両端を検証する。64MiBでは合計512stepで完了する。
shutdown時のAPP COMPLETE表示は正常完了時だけ行う。
QEMUはライフサイクル成功マーカーを検査する。
ホストでは時刻巻戻し、描画の境界・overflow・paddingとguard保存を検査した。
