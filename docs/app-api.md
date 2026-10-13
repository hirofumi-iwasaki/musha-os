# Initial Rust application API contract

## English

`musha-api` provides a safe API for Rust applications statically linked into a single EFI.
The runtime calls `init`, `step`, and `shutdown` on the `Application` trait, plus bounded `files` callbacks while the mounted file service is available. `files_closed` invalidates saved stream IDs after transport shutdown (except failed apps or unconfirmed DMA shutdown).
The API version is 5. Rust types carry no external binary-compatibility guarantee.
The [snapshot API](file-api.md) remains available. The [stream API](stream-file-api.md) now supports root-file selection and read-only FAT32/exFAT requests backed by USB. Experimental [C/Rust SDK declarations](sdk.md) now exist; the external syscall runtime, application UDP handles and file writes remain unimplemented.

### Ownership and execution

Context holds the arena's sole mutable slice. Applications borrow it through `arena()`;
the runtime retains no mutable reference to the same region as the context.
It provides `arena_bytes()`, `version()`, and `now_ms()`.
Do not expose raw framebuffer addresses to applications; use drawing operations within the context.
Context construction is an unsafe runtime boundary guaranteeing display lifetime,
exclusive drawing access, and no overlap with the arena.
Applications are trusted because isolation and preemption are absent.

After successful init, call step repeatedly. Continue advances to the next iteration; Complete ends normally.
If a step or time retrieval fails, call shutdown once and display diagnostics.
Do not call shutdown after init failure; init is responsible for partial-initialization cleanup.
Cleanup is not guaranteed on panic / CPU exceptions.
While a Boot Keyboard is connected, call step between nonblocking USB polls.
Explicit diagnostic builds continue RAM diagnostics without input; normal viewer builds skip that application-specific scan. After the input session ends, call shutdown and halt.
The initial [cooperative runtime](cooperative-io.md) now services the first keyboard, [QEMU networking](network.md), and application snapshot-file reads together. USB/BOT boot discovery is still synchronous and completes before this session. The mounted file service also advances bounded metadata/data requests in this loop, including when storage and input use separate xHCI controllers. Multiple active keyboards and application UDP handles remain pending.

now_ms is a monotonic PM-timer millisecond sample in the runtime-owned clock epoch, updated immediately before every step. Applications measure durations by differences within this session.
The runtime-only context advance operation rejects backward time.
Keep calls closer together than one PM timer wrap. Steps must not block.
Diagnose steps exceeding the 1ms budget without forced interruption.
Currently integer milliseconds report overruns at 2ms or more; strict 1ms detection is not provided.

### Drawing

pixel and rectangle convert RGB colors to the display format and return Invalid for out-of-bounds access or arithmetic overflow.
rectangle validates the entire range before writing. text uses the existing fixed font with clipping.
text_at uses pixel coordinates and scale 1..4, rejects invalid origins/scales and clips each glyph pixel.
Drawing is synchronous; applications must split large drawing operations across steps.
The current Error is a Rust enum with Invalid / Unsupported / Io / Again / NotFound / NoMemory / Disconnected, distinct from the future C integer-status contract.

The normal v0.2.0 development build uses the [file viewer](file-viewer.md); the following describes retained diagnostic builds.

### Diagnostic application and tests

Display Hello Musha-OS! during init.
Write and read back 64 pages per phase to check both ends of every RAM page.
A 64MiB RAM test requires 512 steps.
After RAM completion, continue processing keys with Continue while the input session is active.
The normal build ends on an Esc press.
Display APP COMPLETE during shutdown only after normal completion.
QEMU checks lifecycle success markers.
Host tests checked backward time, drawing boundaries / overflow / padding, and guard preservation.

### Key input

`next_key()` returns `KeyEvent { usage, pressed, timestamp_ms }` from a FIFO.
usage uses USB HID Boot Keyboard values, with modifiers E0–E7; no character or layout conversion is performed.
The timestamp is when the report is passed to the application, not the physical press time.
Capacity is 64. Overflow preserves queued events and coalesces new events into
latest held-key state. After the FIFO drains, releases then presses reconcile
consumer state; each phase emits modifiers first. Recovery timestamps use current
context time. Drain until None. Intermediate taps, text and original timing are
not recoverable. `lost_key_events()` counts coalesced events cumulatively with
saturation; applications can cancel history-dependent operations when it changes.
See [recovery and validation](input-prehardware-validation.md).
`push_key()`, `set_input_active()`, and `advance()` are for the statically linked runtime.
The diagnostic application can access `input_active()` and `screen_size()`.
The runtime validates reports before enqueuing events; the application consumes them on the next step.
Host tests verified FIFO order, wraparound, overflow, and timestamps.

---

## 日本語

**RustアプリAPIの初期契約**

`musha-api`は単一EFIへ静的リンクするRustアプリ向けの安全なAPI。
`Application` traitの`init`、`step`、`shutdown`に加え、媒体利用中は`files`を呼ぶ。
`files_closed`では保存した逐次APIのID/handleを破棄する。失敗済みアプリ、DMA停止を確認できない場合は呼ばない。
API版は5。Rust型には外部バイナリ互換性を約束しない。
既存の[snapshot API](file-api.md)を維持し、USBに接続した[FAT32/exFAT逐次API](stream-file-api.md)を実装済み。C ABI、Cヘッダー、アプリUDP handle、ファイル書込みは未実装。

### 所有権と実行

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
初期の[協調ランタイム](cooperative-io.md)で、最初のキーボード、[QEMU通信](network.md)、アプリsnapshot file readを同時に進める。起動時のUSB/BOT探索は同期処理としてこのセッション前に完了する。同じループでファイルのmetadata/data要求を分割実行し、入力とstorageが別xHCI controllerでも進める。複数キーボードとアプリUDP handleは未実装。

now_msはランタイム所有clockのepochにおけるPM timerの単調なミリ秒値で、毎step直前に更新する。セッション内の差分で経過時間を測る。
contextのadvanceはランタイム用で、時刻の巻戻しを拒否する。
呼出し間隔はPM timerの一周より短く保つ。stepのブロックは禁止する。
1ms予算を超えたstepは診断するが、強制中断はしない。
現在は整数msで2ms以上を超過として報告するため、厳密な1ms判定は未提供。

### 描画

pixelとrectangleはRGB色を画面形式へ変換し、境界外や算術overflowをInvalidとして返す。
rectangleは全範囲を検証してから書き込む。textは既存の固定フォントでclipして描画する。
text_atはピクセル座標と倍率1〜4を使用し、無効な原点・倍率を拒否し、画素ごとにclipする。
描画は同期処理なので、大きな描画はアプリがstepへ分割する。
今のErrorはInvalid / Unsupported / Io / Again / NotFound / NoMemory / DisconnectedのRust enum。将来のC整数status契約とは区別する。

v0.2.0開発通常版は[ファイルビューアー](file-viewer.md)を使用する。以下は維持している診断版の説明。

### 診断アプリと試験

initでHello Musha-OS!を表示する。64ページずつ書込むphaseと読戻すphaseで
RAM全ページの両端を検証する。64MiBではRAM試験に512step必要。
RAM完了後も入力セッション中はContinueし、キーを処理する。診断版はEsc押下で終了する。
shutdown時のAPP COMPLETE表示は正常完了時だけ行う。
QEMUはライフサイクル成功マーカーを検査する。
ホストでは時刻巻戻し、描画の境界・overflow・paddingとguard保存を検査した。

### キー入力

`next_key()` はFIFOから `KeyEvent { usage, pressed, timestamp_ms }` を返す。
usageはUSB HID Boot Keyboardの値で、modifierはE0〜E7。文字や配列へ変換しない。
timestampは報告をアプリへ渡した時刻で、物理的な押下時刻ではない。
容量64。溢れたら既存FIFOを維持し、新規イベントは最新の押下状態に集約する。
FIFO読出し後に解放→押下の順（それぞれ修飾キー優先）で状態差を補う。
復元イベントは現在のcontext時刻を用いる。Noneまで読み出すこと。
途中で失われたタップ・文字・元の時刻は復元できない。
`lost_key_events()` は集約した件数を飽和加算し、アプリは変化を検出して
入力履歴に依存する操作を取り消せる。[復元契約と検証](input-prehardware-validation.md)を参照。
`push_key()`、`set_input_active()`、`advance()` は静的リンクランタイム用。
`input_active()` と `screen_size()` は診断アプリから参照できる。
ランタイムは報告を検査してからイベントを入れ、次のstepでアプリが消費する。
ホスト試験でFIFO順序、wrap、overflow、時刻を確認した。
