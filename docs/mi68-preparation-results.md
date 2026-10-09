# MI68 preparation results / 公開準備の結果

## English

Date: 2026-10-09 (Japan time). Local environment: macOS / Apple Silicon,
Rust/Cargo 1.99.0, Apple Clang 21.0.0, QEMU 11.1.2.
Integration branch: `release/mi68-preparation`, based on USB build `a2e396b`.
The earlier Linux attempt could not run Rust because its downloaded LLVM library
was incomplete. That limitation does not apply to this Mac; the tests below were
actually run here. These results do not represent physical hardware acceptance.

| Work | Result |
|---|---|
| Plan and draft release notes | Imported from the separate thread; English first, Japanese second |
| Clean checkout normal EFI build | Passed from a new local clone of a2e396b with an empty target directory |
| Host tests | 65 passed; repeated in the clean checkout; formatting passed |
| Image, USB bundle and release packaging tests | 9 passed (4 + 1 + 4) |
| Source hashes | lwIP 185, FreeBSD 8, r-efi AUTHORS 1 matched |
| QEMU diagnostic regression | USB/input, packet-level ARP/ICMP/UDP, cooperative traffic/idle/no keyboard/disconnect/link down, seven CPU/page faults, four USB/storage timeouts and corrupt GPT/FAT32 passed |
| Additional failure cases | See local cooperative-tx-timeout.log and cooperative-app-error.log; both require a generated image containing MUSHA.TXT |
| Normal candidate | Independently built without debug/fault/probe features; boot, greeting, RAM, file reads, input, Esc and DMA stop visually verified in QEMU |
| Candidate contents | Normal EFI, MUSHA.TXT, 64MiB QEMU image, generator, full reference docs, licenses, r-efi AUTHORS, provenance and SHA-256 |
| Mac CI | Added the same automated regression and candidate-generation steps; GitHub execution is a separate result |

The network test verified 97 UDP echoes, ARP/ICMP, ring wraps, invalid checksums
and DMA shutdown. A first app-error run using QEMU's virtual FAT directory did
not reach the intended injected failure because the snapshot file was unavailable.
The test and CI were corrected to use a generated GPT/FAT32 image containing
MUSHA.TXT; preserve that condition when repeating the test.

Logs: `out/mi68-validation/`; normal candidate screenshot:
`out/qemu-mi68-normal/screen.ppm`. Artifacts: `dist/mi68-candidate-20261009/`.
The artifact manifest records the generating commit, working-tree state, tool
versions, features and file hashes. Archive metadata is normalized; identical
EFI binaries across different build environments are not guaranteed.

The first GitHub CI run stopped because the minimal Rust profile omitted rustfmt.
The workflow now installs rustfmt explicitly; the rerun is tracked separately.

### Remaining acceptance work

- Freeze a final release commit after fixes and hardware results; the preparation commit is not a v0.1.0 tag.
- Verify the final candidate in GitHub's Mac CI; local Mac success does not imply CI success.
- NUC5/NUC8 boot, input, RAM, USB reads, repeat boots and 30-minute integrated tests: NOT RUN.
- I218-V/I219-V ARP/ICMP/UDP: not implemented; NOT RUN.
- Exact-capacity physical USB identification, image generation and readback: NOT RUN.
- Decide whether to retain the full existing criteria or explicitly approve a limited initial development release; the existing criteria remain in force.
- Main merge, v0.1.0 tag and GitHub Release: not performed.

The finalized MI68 flyer was not modified. No physical disk was written.

## 日本語

日付：2026-10-09（日本時間）。環境：macOS / Apple Silicon、Rust/Cargo 1.99.0、
Apple Clang 21.0.0、QEMU 11.1.2。作業ブランチは`release/mi68-preparation`、
開始点はUSBビルドの`a2e396b`です。
別スレッドのLinux環境ではRustのLLVM取得物が不完全でしたが、このMacでは
以下を実際に実行できました。実機の合格記録ではありません。

| 作業 | 結果 |
|---|---|
| 計画・リリースノート草稿 | 別スレッドから取り込み、英語・日本語の順に整備 |
| 新規チェックアウトの通常EFIビルド | a2e396bを新しくcloneし、空のtargetから成功 |
| ホスト試験 | 65件成功。新規cloneでも再実施。フォーマット検査も成功 |
| イメージ・USB一式・候補配布試験 | 9件成功（4＋1＋4） |
| 第三者ソースのハッシュ | lwIP 185、FreeBSD 8、r-efi AUTHORS 1が一致 |
| QEMUデバッグ回帰 | USB・入力、実パケット通信、協調処理5条件、CPU/ページ例外7条件、USB/媒体タイムアウト4条件、GPT/FAT32破損が成功 |
| 追加異常系 | cooperative-tx-timeout.logとcooperative-app-error.logに記録。MUSHA.TXT入り実イメージを使用 |
| 通常版の配布候補 | 独立ビルド。QEMUで挨拶、RAM、ファイル読出し、入力、Esc、DMA停止を画面確認 |
| 配布内容 | 通常EFI、MUSHA.TXT、64MiB QEMUイメージ、生成ツール、参照文書、ライセンス、r-efi帰属表示、生成元とSHA-256 |
| Mac CI | 同じ自動試験と候補生成を定義。GitHubでの実行結果は別途確認 |

通信試験ではARP/ICMP、UDP echo 97回、リング周回、不正チェックサム、DMA停止を確認しました。
最初のアプリ異常試験ではQEMUの仮想FATディレクトリを使ったため、ファイルを取得できず
意図した障害注入に到達しませんでした。MUSHA.TXT入りGPT/FAT32イメージを使うよう
試験とCIを修正しました。再試験でもこの条件を維持します。

ログは`out/mi68-validation/`、通常版画面は`out/qemu-mi68-normal/screen.ppm`、
成果物は`dist/mi68-candidate-20261009/`です。manifestにコミット、変更状態、
ツール版、features、ハッシュを記録します。アーカイブの属性は正規化していますが、
異なるビルド環境でEFIが完全一致することは保証していません。

GitHub CIの初回はminimalプロファイルにrustfmtがなく停止しました。
CIでrustfmtを明示的に導入するよう修正し、再実行結果を別に確認します。

### 残る合格条件

- 修正と実機結果を反映して最終リリースコミットを固定する。準備コミットはv0.1.0タグではない。
- GitHubのMac CIで最終候補を確認する。ローカル成功とCI成功は別に記録する。
- NUC5/NUC8の起動、入力、RAM、USB読出し、反復起動、30分統合試験：未実施。
- I218-V/I219-VのARP/ICMP/UDP：未実装・未実施。
- 実USBの識別、正確な容量でのイメージ生成、読戻し：未実施。
- 現行条件を維持するか、範囲を限定した初期開発版とするかを確定する。現行条件は維持している。
- mainへのマージ、v0.1.0タグ、GitHub Release：未実施。

確定済みのMI68チラシは変更していません。物理ディスクへの書き込みも行っていません。
