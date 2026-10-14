# v0.4.0 release scope — draft

## English

Status: **draft, not decided.** Collected from owner decisions and deferred items recorded during v0.3.0 (2026-10-14). The owner approves the final scope after v0.3.0 is published. Nothing here is implemented in v0.3.0.

### Direction

v0.3.0 proved that a user-built C program runs at Ring 3 from a USB drive. v0.4.0 makes the drive and the session usable as a **small application environment**: a clean drive layout, returning to the viewer after an application ends, and launching applications from the viewer. The ABI stays experimental; no practical application I/O (stage D) is promised here.

### Candidate goals (owner decisions already taken are marked ✔)

| Item | Scope | Completion condition |
|---|---|---|
| USB root layout ✔ | Root contains only `EFI/`, `DOCS/`, `README.TXT` and the user's `AUTORUN.ELF`. `DOCS/` holds `LICENSE`, `NOTICE` and `third_party/`. | Fresh image, update ZIP and `update-usb.py` all produce this layout; old root files are migrated as obsolete entries |
| README.TXT as the startup marker ✔ | `DIAGNOSE.TXT` is removed. The startup read check reads `/README.TXT`; its expected length and hash are computed at build time from the generated text (under 4 KiB, ASCII). | FAT32/exFAT read diagnostics pass with the generated README in QEMU and on hardware |
| Stateless update tool ✔ | The on-drive copy of `update-manifest.json` is removed. The package manifest carries file hashes, the previous-version hashes and an obsolete-file list; the tool hashes the managed files on the drive and compares. | Update, rename migration and user-modified-file detection pass in virtual-image and directory tests; no state file is written to the drive |
| Return to the viewer after exit | Promote the C3 continuation path (rollback, arena scrub, device restart) into the normal build: after `exit` or a user fault, devices are re-initialized and the viewer starts without reboot. | QEMU: exit 0, nonzero exit and a faulting ELF each return to the viewer; DMA stop-before-reuse rules verified; hardware cold boot and repeated launches |
| Launch from the viewer | Selecting an `.ELF` in the viewer and pressing Enter runs it (same validation as `AUTORUN.ELF`). `AUTORUN.ELF` keeps the automatic start for keyboard-less use. | Both paths covered by QEMU; one application at a time; no scheduler |
| Mac storage isolation | A/B with the v0.2.0 EFI under identical connections, diagnostic captures of the 7D00 recovery path, and a fix or a documented limitation. | Either FAT32/exFAT reading on MacBook Pro 2018 with the external keyboard, or a recorded root cause |
| Startup timing instrumentation | Keep the PM-epoch stage timers in the normal build and show a one-line summary on the diagnostic panel. | Ryzen and NUC5 numbers recorded per release |
| Documentation structure | Stage 2 of the documentation cleanup: `docs/README.md` index, `Status:` lines, `docs/history/`, merges, bilingual user-facing pages. | Link check passes; README.md links only to the index and user pages |

### Explicitly outside v0.4.0

Rust guest execution and a Rust SDK; new syscalls beyond `query`/`log`/`exit`; application input, file, display-command or network APIs; preemption or CPU budgets; multiple concurrent applications; ABI compatibility freeze; physical NIC networking; internal Mac keyboard (T2).

### Naming rule (owner decision)

No file or directory on the drive may suggest that it is "Musha-OS itself" other than `EFI/BOOT/BOOTX64.EFI`. Names describe the role (`AUTORUN`, `DOCS`, `README`), not the product.

## 日本語

状態：**草案、未決定。** v0.3.0 の作業中に記録した所有者決定と先送り項目を集めたもの（2026-10-14）。最終的な範囲は v0.3.0 公開後に所有者が承認する。ここに書かれたものは v0.3.0 には実装されていない。

### 方向

v0.3.0 で「利用者が作った C プログラムが USB から Ring 3 で動く」ことを示した。v0.4.0 は USB とセッションを**小さなアプリ実行環境**として使えるようにする：整理された USB 構成、アプリ終了後のビューアーへの復帰、ビューアーからのアプリ起動。ABI は実験版のままで、実用アプリ I/O（工程 D）は約束しない。

### 候補目標（決定済みの所有者判断は ✔）

| 項目 | 範囲 | 完了条件 |
|---|---|---|
| USB ルート構成 ✔ | ルートは `EFI/`、`DOCS/`、`README.TXT`、利用者の `AUTORUN.ELF` だけ。`DOCS/` に `LICENSE`・`NOTICE`・`third_party/`。 | 初回イメージ・更新 ZIP・`update-usb.py` がこの構成を生成し、旧ルートファイルは廃止項目として移行される |
| README.TXT を起動マーカーに ✔ | `DIAGNOSE.TXT` を廃止。起動時の読出し確認は `/README.TXT` を読み、期待する長さとハッシュを生成テキストからビルド時に計算する（4 KiB 未満、ASCII）。 | 生成 README で FAT32/exFAT 読出し診断が QEMU と実機で合格 |
| 更新ツールのステートレス化 ✔ | USB 上の `update-manifest.json` を廃止。配布 ZIP の manifest にハッシュ・前版ハッシュ・廃止ファイル一覧を持たせ、ツールは USB 上の管理ファイルをハッシュして突き合わせる。 | 更新・改名移行・利用者変更の検出が仮想イメージとディレクトリー試験で合格、USB に状態ファイルを書かない |
| 終了後のビューアー復帰 | C3 の継続経路（ロールバック・arena 消去・機器再初期化）を通常版へ昇格し、`exit` やユーザー例外の後に再起動なしでビューアーへ戻る。 | QEMU で終了 0・非 0・障害 ELF の各ケースがビューアーに戻る。DMA 停止前再利用禁止を検証。実機で冷間起動と反復起動 |
| ビューアーからの起動 | 一覧で `.ELF` を選んで Enter で実行（`AUTORUN.ELF` と同じ検査）。`AUTORUN.ELF` の自動起動はキーボード無し用途のため維持。 | 両経路を QEMU で確認。同時 1 アプリ、スケジューラーなし |
| Mac ストレージの切り分け | 同一接続での v0.2.0 EFI との A/B、7D00 復旧経路の診断採取、修正または制約の文書化。 | MacBook Pro 2018 で外付けキーボード併用時の FAT32/exFAT 読出し、または原因の記録 |
| 起動時間計測の常設 | PM epoch の段階タイマーを通常版に残し、診断パネルに 1 行の要約を出す。 | 版ごとに Ryzen・NUC5 の数値を記録 |
| 文書構造 | 文書整理 第 2 段：`docs/README.md` 索引、`Status:` 行、`docs/history/`、統合、利用者向けページの二言語化。 | リンク検査合格。README.md は索引と利用者向けページだけにリンク |

### v0.4.0 に含めないもの

Rust guest 実行と Rust SDK、`query`/`log`/`exit` 以外の syscall、アプリ向けの入力・ファイル・描画命令・通信 API、プリエンプションや CPU 時間制限、複数アプリの同時実行、ABI 互換性の固定、実機 NIC 通信、Mac 内蔵キーボード（T2）。

### 命名規則（所有者決定）

USB 上のファイル・ディレクトリーは `EFI/BOOT/BOOTX64.EFI` 以外に「Musha-OS 本体」を連想させる名前を付けない。名前は役割（`AUTORUN`、`DOCS`、`README`）を表し、製品名を使わない。
