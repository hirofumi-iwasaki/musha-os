# RustアプリAPIの初期契約

`musha-api`は単一EFIへ静的リンクするRustアプリ向けの安全なAPI。
`Application` traitの`init`、`step`、`shutdown`をランタイムが呼ぶ。
API版は2。Rust型には外部バイナリ互換性を約束しない。
C ABI、Cヘッダー、ファイル・UDP・handle APIは未実装で、別途追加する。

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
Boot Keyboard接続中は非ブロッキングUSB pollの合間にstepを呼ぶ。
無入力中もRAM診断を進める。入力セッション終了後にshutdownし、停止する。
複数デバイスの汎用イベントループとNIC pollはまだ提供していない。

now_msは最初のstep開始を基点とするPM timerの単調経過値で、毎step直前に更新する。
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
RAM全ページの両端を検証する。64MiBではRAM試験に512step必要。
RAM完了後も入力セッション中はContinueし、キーを処理する。通常版はEsc押下で終了する。
shutdown時のAPP COMPLETE表示は正常完了時だけ行う。
QEMUはライフサイクル成功マーカーを検査する。
ホストでは時刻巻戻し、描画の境界・overflow・paddingとguard保存を検査した。

## キー入力

`next_key()` はFIFOから `KeyEvent { usage, pressed, timestamp_ms }` を返す。
usageはUSB HID Boot Keyboardの値で、modifierはE0〜E7。文字や配列へ変換しない。
timestampは報告をアプリへ渡した時刻で、物理的な押下時刻ではない。
容量64。満杯なら新規イベントを捨て、既存FIFOを維持する。
`lost_key_events()` は飽和する累積損失数を返す。損失後の押下状態は保証しないため、
アプリはカウンタ変化を監視して保持状態を破棄できる。
`push_key()`、`set_input_active()`、`advance()` は静的リンクランタイム用。
`input_active()` と `screen_size()` は診断アプリから参照できる。
ランタイムは報告を検査してからイベントを入れ、次のstepでアプリが消費する。
ホスト試験でFIFO順序、wrap、overflow、時刻を確認した。
