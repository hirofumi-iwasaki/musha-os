# v0.3.0 — Developer preview: run your own C program at Ring 3

## English

Candidate notes updated: 2026-10-13. Publication date: not set.

Status: **release candidate, not published.** Candidate source `4319b58dda03400032c0307eaaa3b85fe8150ac0` on `release/0.3.0`; display source commit `4319b58`. Items marked PENDING are completed before the tag; the owner decides publication after the [scope conditions](release-scope-0.3.0.md) are met. Published v0.2.0 assets are unchanged.

### Main changes

- Run one external C application per boot: place an SDK-built ELF named `MUSHA.ELF` in the root of the FAT32/exFAT data medium; the kernel validates it, shuts down devices, and executes it at Ring 3 (CPL 3) with its own stack, arena and read-only start record. If the file is absent, the v0.2.0 read-only viewer starts as before.
- Kernel-owned text console for application `log` output (up to 80 columns × 24 rows, bounded by the framebuffer and scaled by display height), with on-screen exit status, user-fault and refusal messages.
- Select the largest RGB/BGR GOP mode before leaving UEFI, and show the chosen width/height/stride in the diagnostic panel.
- Change the single boot FAT32 partition type from EFI System Partition to Microsoft Basic Data so macOS and Windows show the USB as a normal drive. The OS still accepts existing ESP media.
- Separate relocatable **C SDK** archive: crt0, linker script, libmusha, headers, ABI schema, ELF checker, C Hello and a CMake entry point.
- Boot readiness separation (policy 3.0): typed readiness/application states, explicit record and row-diff rendering, OS-owned device lifetime.

### Downloads

- `musha-os-v0.3.0-x64-usb.img.gz`: 64 MiB GPT/Basic Data FAT32 boot image for QEMU and as the initial/recovery source image.
- `musha-os-v0.3.0-x64-files.zip`: normal-update EFI, managed files and update manifest.
- `musha-os-v0.3.0-x64-c-sdk.zip`: the C SDK. See the [SDK guide](sdk.md).
- `musha-os-v0.3.0-x64-deployment.zip`: deployment tools and instructions.
- `musha-os-0.3.0-candidate.tar.gz`: complete bundle with normal EFI, boot image, synthetic exFAT test image, licenses and documentation.
- `build-manifest.json` / `SHA256SUMS`: source commit, build conditions and artifact hashes.

Normal updates back up, verify and replace only changed managed files; they do not generate, write or hash an entire USB device. Do not raw-copy the small image onto a larger USB. An initial physical-device writer is still not provided; Windows/Linux physical-device update adapters are not implemented. See the [deployment guide](distribution-usage.md).

### Running your own program

1. Extract the C SDK anywhere (Python 3, Clang with x86-64 ELF support and LLVM LLD are required) and run `python3 tools/check-sdk.py --c-only --sdk . --output build`. The checker records the Hello ELF SHA-256 in `build/report.json`.
2. Copy `build/hello-c.elf` to the root of the USB as `MUSHA.ELF` (ASCII case is ignored; duplicate matching names are rejected).
3. Boot. The screen shows `Loading MUSHA.ELF ...`, then the program's `log` output, then `MUSHA.ELF exited: status <n>`. Reboot to return to the OS.

Limits: one application per boot; file size 64 bytes to 16 MiB with a 5-second read deadline (the 13 KB Hello reads well within it; large files may reach the deadline); operations are `query`, `log` (UTF-8, at most 4096 bytes per call, printable ASCII and newline rendered, other bytes shown as `.`) and `exit`. No input, file, display-command or network calls are available to applications. A user fault shows `MUSHA.ELF faulted` with vector, error code and RIP/RSP and halts; a refused file shows `MUSHA.ELF rejected: <reason>` and halts.

**The ABI is experimental candidate r1.** Syscall numbers, the start record, the NOTE format and memory layout may change in a later version without compatibility. There is no libc, POSIX, Rust SDK, dynamic loading or threads.

### Controls and limits of the viewer

Unchanged from v0.2.0: external USB Boot Keyboard required; Up/Down selects, Enter opens, Page Up/Down navigates, Esc returns, Q ends the session. 128 root files, 64 MiB per file, 10 seconds per page read. See the [viewer guide](file-viewer.md).

### Known issues

- MacBook Pro 2018: the 0.3.0 build boots and detects an external keyboard, but does not recognize the USB storage, so the viewer and external applications are unavailable there. Whether this is a regression from the 10/12 rc1 observation or a connection difference is not yet determined. The internal keyboard is unsupported (T2 unimplemented).
- NUC5: an older development EFI showed `No medium` twice; not reproduced on the current build. Cold-boot repeatability is recorded under hardware validation below.
- Console scale is 1× below 2160 pixels in height, 2× from 2160 to 2879, and 3× from 2880. A 3840×2160 display uses 2×.
- Two keyboards on different controllers: the first selected device prevents input from the other (H15).
- Unoptimized debug-profile builds fault during filesystem discovery (vector 8); release-profile builds are unaffected.
- Physical NIC communication (I218-V/I219-V) is not implemented; controller detection is not networking.
- Returning to the OS after application exit without a reboot is not part of this release (C3 remains an opt-in probe).

### Validation scope

Host tests: 255. The new display candidate passed 186 QEMU case/configuration checks (`out/release-display/report.json`), covering external ELF loading on Basic Data and legacy ESP media, console/exit/fault/refusal display, absent-file viewer pixel regression with zero debug bytes, viewer/cooperative/hub regressions, and the C1/C2/C3 probes (8/14/20). The SDK was rebuilt in a separate directory containing spaces and reproduced the same Hello hash. Deployment and update tools were tested on virtual images and regular directories.

Hardware: during development, the external Hello ran at Ring 3 on NUC5i5RYH (GOP 1600×1200) and on a Ryzen 7 machine (GOP 3840×2160) with on-screen output and exit status 0; these runs used development EFIs, not the candidate. Candidate acceptance on the exact release EFI — three cold boots each on NUC5 and Ryzen, and Basic Data boot on MacBook Pro 2018 — is **PENDING** and will be recorded in [hardware acceptance](hardware-acceptance-0.3.0.md). QEMU and host results are not hardware evidence.

### Final artifact verification

SDK Hello ELF SHA-256: `d60006f88e1e36409867546df31c778cf3d5fb7c077fba376ebb5bd3fbb2810f` (13,312 bytes).

Source: `4319b58dda03400032c0307eaaa3b85fe8150ac0` (candidate build). EFI SHA-256: `41a61efcd96edf6142bf81a37ae2ca90ca4396ea71386da698f8cac8bccd2ae8` (455,680 bytes). Toolchain: Rust 1.99.0, Apple clang 21.0.0.
Remote CI: SUCCESS on candidate source `4319b58dda03400032c0307eaaa3b85fe8150ac0`: [run 38365638298](https://github.com/hirofumi-iwasaki/musha-os/actions/runs/38365638298). GP/DF injection, all remaining workflow regressions and normal candidate packaging passed; draft creation was skipped. This supersedes failed run 38360736372 on 04d986d. Logs are in `out/repro-4319b58/remote-job.log`. The CI-built EFI is a separate artifact, not an attestation of the local USB EFI hash above. Asset download and hash verification after release upload: PENDING.

Publication provenance: UNRESOLVED (owner decision required). Two clean builds of `4319b58` produced different raw EFI hashes; the differences are confined to COFF/debug timestamps and the CodeView identifier (`out/repro-4319b58/report.json`, `pe-difference.json`). Metadata-excluded equality does not meet byte-identical acceptance. The proposed documentation-only tag exception has therefore not been adopted. Current policy and release tools require the tag commit to match the build-manifest source commit. Tagging a later documentation commit while retaining an earlier candidate EFI is not currently supported. Resolve the release source and packaging procedure before tagging; preserve the tested EFI hash and do not relabel its build origin.

---

## 日本語

候補ノート更新日：2026-10-13。公開日：未定。

状態：**公開候補、未公開。** 候補ソースは `release/0.3.0` の `4319b58dda03400032c0307eaaa3b85fe8150ac0`、表示変更ソースは `4319b58`。「未完了」の項目はタグ作成前に埋め、公開可否は[公開範囲](release-scope-0.3.0.md)の条件を満たした後に所有者が判断します。公開済み v0.2.0 配布物は変更しません。

### 主な変更

- 自作 C アプリの実行：SDK でビルドした ELF を `MUSHA.ELF` の名前で FAT32/exFAT データ媒体のルートに置くと、カーネルが検査し、機器を停止してから Ring 3（CPL 3）で実行します。専用スタック・arena・読出し専用の起動情報を渡します。ファイルがなければ従来の v0.2.0 読出し専用ビューアーが起動します。
- アプリの `log` 出力をカーネル所有のテキストコンソール（最大80桁×24行、画面内に収めて高さに応じて拡大）に表示し、終了コード・ユーザー例外・拒否理由も画面に出します。
- UEFI を抜ける前に最大の RGB/BGR GOP モードを選び、幅・高さ・stride を診断パネルに表示します。
- 起動用 FAT32 のパーティション種別を EFI System Partition から Microsoft Basic Data に変更し、macOS・Windows で通常の USB として表示されるようにしました。既存の ESP 媒体も引き続き起動できます。
- 再配置可能な **C SDK** を別アーカイブで配布：crt0、リンカースクリプト、libmusha、ヘッダー、ABI スキーマ、ELF 検査、C Hello、CMake 入口。
- 起動分離（方針3.0）：起動状態・アプリ状態の型付け、記録と差分描画の分離、OS による機器寿命の所有。

### 配布物

- `musha-os-v0.3.0-x64-usb.img.gz`：QEMU 用および初回・復旧用の 64 MiB GPT/Basic Data FAT32 起動イメージ。
- `musha-os-v0.3.0-x64-files.zip`：通常更新用の EFI・管理ファイル・更新 manifest。
- `musha-os-v0.3.0-x64-c-sdk.zip`：C SDK。[SDK 手順](sdk.md)を参照。
- `musha-os-v0.3.0-x64-deployment.zip`：配備ツールと手順。
- `musha-os-0.3.0-candidate.tar.gz`：通常 EFI、起動イメージ、合成 exFAT 試験イメージ、ライセンス、文書の一式。
- `build-manifest.json` / `SHA256SUMS`：ソースコミット、ビルド条件、配布物のハッシュ。

通常更新は変更された管理ファイルだけを退避・照合・置換し、USB 全体の生成・書込み・ハッシュ走査は行いません。小型イメージを大容量 USB へ raw コピーしないでください。実デバイス向けの初回書込みアダプターは未提供、Windows/Linux の実デバイス更新アダプターは未実装です。[配備手順](distribution-usage.md)を参照。

### 自作プログラムの実行

1. C SDK を任意の場所に展開し（Python 3、x86-64 ELF 対応の Clang、LLVM LLD が必要）、`python3 tools/check-sdk.py --c-only --sdk . --output build` を実行します。Hello ELF の SHA-256 は `build/report.json` に記録されます。
2. `build/hello-c.elf` を USB のルートに `MUSHA.ELF` としてコピーします（ASCII の大小文字は区別しません。同名の重複は拒否します）。
3. 起動すると `Loading MUSHA.ELF ...` の後にプログラムの `log` 出力、続いて `MUSHA.ELF exited: status <n>` が表示されます。OS に戻るには再起動します。

制限：1起動につき1アプリ。ファイルは 64 byte〜16 MiB、読込み期限は5秒（13 KB の Hello は余裕で読めますが、大きなファイルは期限に達することがあります）。使える操作は `query`、`log`（UTF-8、1回最大 4096 byte、ASCII 印字可能文字と改行を描画、他は `.`）、`exit` のみです。入力・ファイル・描画命令・通信の呼出しはありません。ユーザー例外は `MUSHA.ELF faulted` と vector・error code・RIP/RSP を表示して停止、拒否は `MUSHA.ELF rejected: <reason>` を表示して停止します。

**ABI は実験候補 r1 です。** syscall 番号、起動情報、NOTE 形式、メモリ配置は後続の版で互換性なく変わる可能性があります。libc、POSIX、Rust SDK、動的ロード、スレッドはありません。

### ビューアーの操作と制限

v0.2.0 から変更なし：外付け USB Boot Keyboard が必要。上下で選択、Enter で開く、Page Up/Down でページ移動、Esc で戻る、Q で終了。ルート 128 ファイル、1ファイル 64 MiB、1ページの読出し 10 秒。[ビューアー](file-viewer.md)を参照。

### 既知の問題

- MacBook Pro 2018：0.3.0 は起動し外付けキーボードを検出しますが、USB ストレージを認識しないため、ビューアーと外部アプリは使えません。10/12 の rc1 で観測した FAT32 認識との違いが退行か接続構成差かは未特定です。内蔵キーボードは未対応（T2 未実装）。
- NUC5：旧開発版 EFI で `No medium` を2回観測。現行版では未再現。冷間起動の再現性は下記の実機確認で記録します。
- コンソールの倍率は画面高さ2160px未満で1倍、2160〜2879pxで2倍、2880px以上で3倍です。3840×2160では2倍です。
- 別コントローラーの2台のキーボード：先に選ばれた機器が他方の入力を妨げます（H15）。
- 未最適化の debug 構成はファイルシステム探索中に例外（vector 8）を起こします。release 構成には影響しません。
- 実機の NIC 通信（I218-V/I219-V）は未実装です。制御器の検出は通信ではありません。
- アプリ終了後に再起動なしで OS へ戻る機能は本版に含みません（C3 は任意試験構成のままです）。

### 検証範囲

ホスト試験255件。新表示候補のQEMU全回帰186件合格（`out/release-display/report.json`）：Basic Data と従来 ESP 両媒体での外部 ELF 読込み、コンソール・終了・例外・拒否の表示、ファイル不在時のビューアーのピクセル回帰（debug 出力 0 byte）、ビューアー・協調 I/O・ハブの回帰、C1/C2/C3 試験（8/14/20）。SDK は空白を含む別ディレクトリーで再ビルドし、同一の Hello ハッシュを再現しました。配備・更新ツールは仮想イメージと通常ディレクトリーで試験しました。

実機：開発中に NUC5i5RYH（GOP 1600×1200）と Ryzen 7 機（GOP 3840×2160）で外部 Hello が Ring 3 で動作し、画面出力と終了コード 0 を確認しました。これらは開発版 EFI での結果で、候補 EFI ではありません。候補 EFI での受入——NUC5・Ryzen 各3回の冷間起動、MacBook Pro 2018 の Basic Data 起動——は**未完了**で、[実機受入記録](hardware-acceptance-0.3.0.md)に記録します。QEMU とホストの結果は実機の根拠にしません。

### 最終配布物の確認

SDK Hello ELF SHA-256：`d60006f88e1e36409867546df31c778cf3d5fb7c077fba376ebb5bd3fbb2810f`（13,312 byte）。

ソース：`4319b58dda03400032c0307eaaa3b85fe8150ac0`（候補ビルド）。EFI SHA-256：`41a61efcd96edf6142bf81a37ae2ca90ca4396ea71386da698f8cac8bccd2ae8`（455,680 byte）。ツールチェーン：Rust 1.99.0、Apple clang 21.0.0。
リモート CI：候補ソース `4319b58dda03400032c0307eaaa3b85fe8150ac0` の[run 38365638298](https://github.com/hirofumi-iwasaki/musha-os/actions/runs/38365638298)でSUCCESS。GP/DF注入、残りの全workflow試験、通常版候補の梱包まで合格し、draft作成はスキップしました。04d986dの失敗run 38360736372に対する再試験結果です。ログは `out/repro-4319b58/remote-job.log`。CI生成EFIは別成果物であり、上記ローカルUSB候補のハッシュを証明するものではありません。Releaseアップロード後の取得・ハッシュ照合は未完了です。

公開来歴：未解決（所有者判断待ち）。`4319b58`のクリーンビルド2回でEFIの生バイトのSHA-256は不一致でした。差分はCOFF/debugの時刻とCodeView識別子に限定されます（`out/repro-4319b58/report.json`、`pe-difference.json`）。メタデータを除いた一致はバイト一致の受入条件を満たさないため、文書のみの後続コミットをタグ対象にする例外は採用していません。現行方針と公開ツールはタグのコミットとbuild-manifestのビルド元コミットの一致を要求します。先行候補のEFIを維持して後続の文書コミットへタグを付ける方式は現在未対応です。タグ作成前に公開ソースと梱包手順を確定し、検証済みEFIのハッシュと実際のビルド元を保持してください。
