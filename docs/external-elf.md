# External ELF loading and memory lifetime

Status: current

## English

The development normal build now loads external ELF and displays application output. [Application usage](external-app.md), [scope](release-scope-0.3.0.md), [2b results](handoff/0.3.0-step2b-results.md) and [2c results](handoff/0.3.0-step2c-results.md) distinguish implementation, local tests and outstanding release gates. ABI candidate r1 is not frozen. Nothing here changes published v0.2.0 assets.

The diagnostic heading reads the repository `VERSION` file and the active test name
(`EXTERNAL ELF` for the normal build), rather than a hard-coded historical release.
External application text uses scale 1 below height 2160, scale 2 below 2880,
and scale 3 at 2880 or above. The default QEMU GOP height is 2048, so its
existing scale-1 pixels are preserved. Diagnostic text remains scale 6. The bounded
80×24 console reduces visible columns/rows to fit the framebuffer and wraps/scrolls.

### Selection, state and lifetime

The existing file service selects the data medium. Its root catalogue is checked for ASCII case-insensitive `AUTORUN.ELF`; absence starts the unchanged viewer. Size is 64 bytes–16 MiB; reads are at most 4096 bytes with one outstanding request and a 5000 ms deadline. The viewer is initialized only after the absent/no-service decision. Matching files instead show `Loading AUTORUN.ELF ...` and external OS_READY; APP_ENTERING precedes the Ring 3 transition and the first query records APP_RUNNING.

Both filesystem parsers reject duplicate names before catalogue publication. A bounded name prefix in the internal `DuplicateName` error preserves this refusal and lets the selected boot medium display an ambiguous-AUTORUN.ELF reason. Discovery does not claim an application or render this error from an unselected medium. Other duplicate names remain filesystem failures. No duplicate file is opened and no scan beyond the existing medium selection is introduced.

Staging is permanent CPU-owned memory, never a USB DMA target. Stream errors, short reads, removal, replacement, interruption and deadline expiry never publish partial data. Closing cancels pending work while retaining its slot until DMA completion/quiescence. Even completed bytes are invalidated if device session completion fails. USB and network owners stop before runtime returns and before arena reuse. USB stop-unconfirmed does not enter the mapper; failed network quiescence halts in its owner.

ELF/NOTE parsing remains **after device shutdown**, followed by the C2 mapper's independent validation. This keeps one immutable-image handoff and one cleanup path for both malformed and executable images. Early parsing would not avoid the required shutdown, so it is not added. All C2 permissions, owned-alias copying and checked return frames remain in force. All `ring3::failed` refusals are drawn before stopping. Exit and user fault stop permanently; C3 restoration is not in normal builds.

### 16 MiB staging / 32 MiB loaded-image bound

The normal EFI's permitted **resident ImageSize** increases from 16 to 32 MiB. It is a maximum accepted loaded-image size, not a physical base address, an arena size, a DMA allocation or a reservation of the entire 32–256 MiB window.

The final inspected normal EFI is 458,752 bytes on disk and 17,596,416 bytes resident (`out/external-elf-2c-final/normal-inspection.json`). Its writable PE section has 17,238,376 virtual bytes but only 117,760 file bytes. The 16 MiB staging array resides in the zero-fill tail (`.bss` in `normal.map`), not in a 16 MiB on-disk payload. UEFI chooses the physical image placement and supplies LoadedImage.ImageBase/ImageSize; no allocation service is called after ExitBootServices.

`boot/src/memory.rs` reserves the **whole loaded image**, plus stack, memory-map storage, BootInfo, emergency stacks, page tables, framebuffer, USB DMA and network DMA. Reservations must be pairwise disjoint. The arena selector uses only non-runtime WB conventional RAM and subtracts those reservations. Its result is then restricted to physical `[0x02000000, 0x10000000)` (32–256 MiB), the existing C1/C2 backing window. This restriction is a subset operation and cannot reintroduce an overlap. It rejects insufficient space instead of using firmware/image/DMA memory.

An explicit normal-build check verifies that staging is wholly inside the image reservation and disjoint from arena and all other reservations. C2 additionally refuses already-mapped candidate virtual pages: its low image/start/arena/stack addresses cannot overwrite an identity-mapped UEFI image or another kernel allocation. UEFI placement inside the *broad* 32–256 MiB window is allowed; the actual selected arena still excludes the image. Claiming that the entire window is unused would be incorrect.

An observed QEMU placement (diagnostic build) is image `[0x0C764000,0x0D831000)`, staging `[0x0C7DBD38,0x0D7DBD38)`, arena `[0x02000000,0x05780000)`, USB DMA `[0x0C564000,0x0C664000)`, network DMA `[0x0D84E000,0x0D85E000)`. The harness validates recorded ranges; these are observations, not fixed physical addresses. Normal viewer skips repetitive diagnostic RAM testing; diagnostic configurations test only the reserved-excluding arena. External staging is outside that slice, and its transition reuses the arena only after device owners stop.

### Read-time observations

One padded FAT32 run at each required size in `out/external-elf-2b-accepted/report.json`:

| File size | Guest elapsed time | Outcome |
|---|---:|---|
| 16 KiB | 24 ms | Complete |
| 1 MiB | 5000 ms | Deadline; incomplete |
| 16 MiB | 5000 ms | Deadline; incomplete |

These are elapsed-to-completion **or deadline-censored** times; the full read time of 1 MiB/16 MiB is unknown, not reported as five seconds. The 13,312-byte SDK Hello completed in 30 ms in FAT32, exFAT and split-controller tests. The largest tested successful size is **16 KiB**, not a universal practical upper bound for all media. The five-second deadline is retained; 16 MiB is only the admission/staging ceiling. Chain validation and metadata traversal on every bounded request make linear extrapolation inappropriate. Hardware throughput remains unmeasured. Subsequent 16 MiB runs sometimes hit the lower USB transfer timeout before the file-reader deadline, including an isolated run; these failed observations are retained and are not successful reads. Timing cases are explicit `--case timing-16384`, `timing-1048576`, `timing-16777216` observations, separate from the default functional suite.

### Features and evidence

`external-elf-probe` now enables only `qemu-debug`, selecting the same one-shot external path and viewer fallback as normal. Build-time cfgs choose external execution by default, the fixed payload only for C1, and embedded ELF only for C2/C3. External and embedded explicit features remain mutually exclusive. Normal linking excludes fixed assembly payload symbols, embedded SDK ELF and lifecycle code; see `normal.map` and `normal-inspection.json`. Debug writes remain gated by `qemu-debug`.

`tools/smoke-external-elf.py --case <name>` records commands, hashes, screenshots, decoded screen text, read timings and memory ranges. `--normal` observes screens without debug; `--efi <path>` tests an exact already-built EFI. The absent case always uses a feature-free EFI and compares list/open/back to `out/v3-normal-final-pixels`, with zero debug bytes and unchanged virtual images. Full evidence and failed/retried runs are listed in the 2c report. All tests use local regular-file virtual images, never physical USB.

Historical 2a evidence remains `out/external-elf-accepted/report.json` (12 cases), `out/external-elf-final/host-tests.log` (238 tests) and its normal pixel result. Owner-approved 2a was committed/pushed as `506d112`; 2b/2c are separate uncommitted candidates awaiting approval. Hardware cold boots, SDK distribution/relocation gates and publication remain outstanding. C3 OS return, Rust guests, practical user I/O and public ABI compatibility are not claimed.

## 日本語

診断見出しはリポジトリの `VERSION` と現在の試験名（通常版は `EXTERNAL ELF`）から生成します。外部アプリの文字はGOP高さ2160未満で1倍、2880未満で2倍、それ以上で3倍です。既定QEMUの高さ2048では1倍を維持し、診断ログは6倍のままです。最大80×24セルの範囲で画面寸法に合わせて折返し・スクロールします。


開発版の通常構成へ外部ELF読込み・画面出力を統合しました。[利用方法](external-app.md)、[2b記録](handoff/0.3.0-step2b-results.md)、[2c記録](handoff/0.3.0-step2c-results.md)で実装・QEMU・実機未検証を区別しています。ABIは実験候補r1、公開済みv0.2.0配布物は不変です。

既存サービスが選択した媒体のルートの`AUTORUN.ELF`を、64バイト～16MiB・1要求4096バイト・期限5000msで読みます。未検出なら従来ビューアー、検出後は読込み案内と外部OS_READYを表示・記録し、APP_ENTERING、最初のqueryでAPP_RUNNINGへ進みます。FSで一覧公開前に拒否される重複名も、選択された媒体に限って理由を画面へ伝えます。

stagingはCPU専用で、arena／DMAとは別です。部分読込み、短い応答、切断、媒体交換、中断、期限超過は実行へ進めません。取消しではDMA停止までスロットを保持し、USB／ネットワーク停止後にのみarenaを再利用します。ELF検査は従来どおり機器停止後です。早期検査でも停止は必要なため、検査と終了処理を二重化せず、不変な完成画像を一度渡す方式を維持しました。拒否・終了・例外は表示して停止し、通常版にOS復帰は入れません。

EFIの常駐メモリ上限32MiBは配置アドレスではありません。通常版の実測はディスク458,752バイト、常駐17,596,416バイトです。16MiB配列はPEのゼロ埋め領域で、ディスクへ同量を格納しません。UEFIが実際の配置を決め、ロード済みイメージ全体を予約します。arenaは予約領域を除いた通常RAMだけから選び、物理32～256MiB窓内へ制限します。UEFIイメージがこの広い窓に入ることはありますが、選択したarenaとは重なりません。stagingのイメージ内包含とarena／DMA等との非重複を通常版でも検査し、C2は仮想ページの既存マッピングも拒否します。診断RAM検査もこのarenaだけを使用します。具体的な観測範囲と根拠は英語欄に記載しました。

必要な3サイズの1回ずつの測定は、16KiBが24msで完読、1MiB／16MiBは5000msで未完読です。後者の全量読出し時間は不明であり、5秒とは報告しません。Hello 13,312バイトは30msでした。期限内に完読を確認した最大試験サイズは16KiBで、すべての媒体に保証する上限ではありません。5秒期限は維持し、16MiBを受付上限として区別します。実機速度は未検証です。後続の16MiB試験では単独実行でも下位USB転送期限が先に発火する場合があり、不合格の観測として保存しています。測定ケースは`--case timing-16384`等で個別実行し、既定の機能試験と分けます。

`external-elf-probe`はdebug記録のみを有効にします。通常版では固定asm・埋込みELF・C3をリンクしません。`--normal --efi`で同一EFIの画面試験、不在ケースで既存参照との画面一致・debug 0バイト・仮想画像不変を検証します。2aは承認済み`506d112`としてpush済み、2b／2cは未コミット候補です。実機・SDK配布・公開は残件で、Rust guest、実用I/O、ABI固定は扱いません。
