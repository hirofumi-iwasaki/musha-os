# Initial application file API

## English

### Scope and ownership

API version 3 adds read-only `file_open / file_read / file_close` to the safe Rust context. The initial supported file is `/MUSHA.TXT`, up to 4096 bytes, copied from the first successfully verified FAT32 root file discovered in USB port order. This is a runtime-owned snapshot, independent of device DMA memory, USB shutdown, and caller buffers. Subsequent successful media do not replace it. No file writes, arbitrary-file loading, subdirectories, seek, C ABI, or external binary compatibility are provided.

`file_open(path)` accepts absolute ASCII root 8.3 names, with alphanumeric characters, `_` and `-`; matching is case-insensitive. Other valid filenames return NotFound once discovery has completed. Invalid paths return Invalid. Before a snapshot is ready, open returns Again while USB discovery is still running, even if an earlier medium was unsupported. After discovery, missing files return NotFound, unsupported media return Unsupported, and failed discovery returns Io. A successful snapshot remains readable if another device later fails.

### Handles and reads

Handles are scoped to their originating context and must not be passed to another context. At most four handles can be open. Each has an independent position and an opaque slot / generation identity. When capacity is exhausted, open returns NoMemory. `file_read(handle, out)` copies at most the caller buffer size, advances only that handle's position, and preserves the untouched suffix of the buffer. A zero return means EOF or a zero-length requested buffer; applications must use a nonempty buffer to detect EOF. The copied snapshot is immutable for the session.

`file_close` invalidates the handle. Read / close through a closed handle returns Invalid, including after slot reuse. Generation overflow retires the slot instead of wrapping its identity. Runtime invalidation clears the snapshot and invalidates active handles; new opens return the supplied error, for example Disconnected. CPU exceptions / panic retain the existing no-cleanup guarantee.

### Cooperative execution

The runtime copies data only after FAT32 validation succeeds. The application reads at most 64 bytes per step, computes length / FNV-1a hash, closes the file, and checks stale-handle rejection. It displays APP FILE READ OK on screens at least 508 pixels high. Missing / unsupported / failed files display a diagnostic marker while RAM tests continue; this is not a complete 0.1.0 feature pass.

Again currently represents pending boot-time discovery, rather than an asynchronous on-demand disk request. Reads from the snapshot are bounded CPU copies and do not wait for USB. USB enumeration / BOT reads still run sequentially during initialization, but all later storage is discovered before keyboard polling begins. The [cooperative runtime](cooperative-io.md) advances snapshot reads, input and QEMU NIC traffic together; traffic-triggered file rechecks verify length/hash/EOF/close/stale handles. General asynchronous device-backed reads remain unimplemented.

### Verification

2026-10-08: All 43 Rust host tests passed. Host tests cover source-buffer ownership, chunked reads, EOF, empty files / buffers, case-insensitive paths, malformed paths, missing files, pending discovery, media errors, four-handle limits, independent positions, closed / invalidated handles, slot reuse, and generation retirement. The diagnostic application closes unfinished handles during recoverable shutdown.
QEMU verified the actual GPT boot image's 16-byte file through the application API, and a fragmented GPT fixture's 1186-byte file. Both length / hash pairs matched runtime FAT32 reads; stale handles were rejected. Corrupt-file handling, timeout paths, and continuous keyboard termination are also included in integration checks. A normal hardware build was also booted in QEMU: the screen showed APP FILE READ OK, Esc termination, and RAM completion. The image SHA-256 remained `f20f4a2fca94ea14f38be34933e695a4d3d21c9620108b212c5c01bd57d5611c`. Hardware remains untested.

See [Rust application API](app-api.md), [FAT32 reader](fat32.md), and [shared USB image](usb-image.md).

## 日本語

### 範囲と所有権

API版3で、安全なRust contextへ読出し専用の`file_open / file_read / file_close`を追加した。初期対象は最大4096byteの`/MUSHA.TXT`。USBポート順で最初に読出し・検証に成功したFAT32ルートファイルをコピーする。ランタイム所有のsnapshotで、device DMA領域、USB停止、呼出し側bufferから独立する。後続媒体の成功で置き換えない。ファイル書込み、任意ファイルのロード、subdirectory、seek、C ABI、外部バイナリ互換は提供しない。

`file_open(path)`は絶対パスのASCIIルート8.3名に対応し、英数字、`_`、`-`を許可する。大文字・小文字を区別しない。他の有効なファイル名は探索完了後にNotFound。不正パスはInvalid。snapshot準備前は、前の媒体が未対応でもUSB探索中ならAgainを返す。探索後はファイルなしがNotFound、未対応媒体がUnsupported、探索失敗がIoとなる。取得済みsnapshotは別の機器が後で失敗しても読める。

### handleと読出し

handleは作成元contextだけで使い、別contextへ渡さない。同時に最大4つのhandleを開ける。各handleは独立した位置と、非公開のslot / generation識別子を持つ。上限ではopenがNoMemoryを返す。`file_read(handle, out)`は呼出し側buffer容量以下をコピーし、そのhandleの位置だけを進め、bufferの未使用末尾を保持する。戻り値0はEOFまたは長さ0のbuffer。EOF判定には空でないbufferを使う。snapshotはセッション中不変。

`file_close`でhandleを無効にする。閉鎖後のread / closeは、slot再利用後もInvalid。generationがoverflowするとslotを再利用不能にし、識別子を周回させない。ランタイムの無効化はsnapshotを消去し、使用中handleを失効させる。以後のopenはDisconnectedなど指定されたエラーを返す。CPU例外・panicで終了処理を保証しない既存の契約は継続する。

### 協調実行

FAT32の検証成功後にだけランタイムが内容をコピーする。アプリは1step最大64byteを読み、長さ・FNV-1a hashを計算し、closeして旧handleの拒否を確認する。高さ508pixel以上ならAPP FILE READ OKを表示する。ファイルなし・未対応・失敗では診断markerを記録し、RAM試験を継続する。これを0.1.0全機能合格とは扱わない。

現在のAgainは起動時探索の未完了を意味し、オンデマンド非同期ディスク要求ではない。snapshot読出しは上限付きCPUコピーでUSB待機を行わない。USB列挙・BOT読出しは初期化中の順次処理だが、キーボードpoll前に後続ストレージも探索する。[協調ランタイム](cooperative-io.md)でsnapshot read・入力・QEMU通信を同時に進め、通信によるfile再検査で長さ/hash/EOF/close/失効handleを確認する。一般の非同期device-backed読出しは未実装。

### 検証

2026-10-08: Rustホスト43試験に合格。ホスト試験は元bufferの所有権、分割read、EOF、空ファイル・buffer、大文字小文字、不正path、ファイルなし、探索中、媒体エラー、4handle上限、独立位置、閉鎖・失効handle、slot再利用、generation上限を扱う。診断アプリは回復可能な終了時に未完了handleを閉じる。
QEMUで実際のGPT起動イメージの16byteファイルと、断片化GPT fixtureの1186byteファイルをアプリAPIから読んだ。両方とも長さ・hashがランタイムFAT32読出しと一致し、旧handleを拒否した。破損ファイル、timeout、継続キー入力の終了も統合試験に含める。通常実機版もQEMUで起動し、APP FILE READ OK、Esc終了、RAM完了を目視確認。イメージSHA-256は`f20f4a2fca94ea14f38be34933e695a4d3d21c9620108b212c5c01bd57d5611c`で不変。実機は未検証。

[RustアプリAPI](app-api.md)、[FAT32読出し](fat32.md)、[共通USBイメージ](usb-image.md)を参照。
