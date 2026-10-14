# v0.3.0 — Developer preview: run your own C program at Ring 3

## English

Current acceptance basis (2026-10-14): IMG_7633 shows one cold-start NO MEDIUM with PRE OWN D8=0; IMG_7634 shows one Hello/exit 0 with D0=0x7FF/D8=0xF already routed. Operations after run 1 and power-off durations for runs 2/3 are UNKNOWN; the owner approved this correction. Three consecutive cold-start successes are not proven. Earlier success counts below remain historical owner reports, not acceptance of the next candidate. See [hardware acceptance](hardware-acceptance-0.3.0.md).

Historical local normal test candidate: 5e4af31ece007550ba4d41a8a5c58d8b7a3b493e8665f277b7817134dd8911af, rebuilt after owner-reported diagnostic cold-boot 3/3 successes on NUC5 and Ryzen (Ryzen also hot 1/1). Manifest/QEMU: out/nuc5-accepted-normal/. This is uncommitted source; NUC5 normal-candidate cold boots passed 3/3 (owner report; IMG_7627.MOV shows Hello and exit 0, associated through the preceding USB deployment). Ryzen normal-candidate cold 3/3 and hot 1/1 also passed (owner report; IMG_7628 2.mov confirms Hello/exit 0). Slow discovery remains a known issue. Mac Basic Data boot, final packaging and current-source remote CI remain pending. The older revision and publication-provenance records below are retained as history.

Historical local hardware-test revision: Address Device recovery, normal EFI `2d8b3a0669108cc6ece04c696dc84292e180a1f6bc2e85f185695ef9a168a09f`, diagnostic EFI `7eba0377ffdc39fea24544a52c8fed86bbf7254c2869d753fceebaffb948676a`; uncommitted source and physical acceptance pending. [Validation and provenance](handoff/0.3.0-address-recovery-results.md). The package/CI records below describe the preceding committed candidate.

Historical Intel-routing candidate: it addressed an unconfirmed hypothesis after a failed NUC5 cold boot with USB keyboard and SanDisk attached. Hardware recovery is NOT yet verified. See [routing policy](intel-xhci-routing.md). Remote CI success below applies to the preceding 4319b58 source, not this new candidate.

Candidate notes updated: 2026-10-14. Publication date: not set.

Status: **release candidate, not published.** Final candidate source is not yet selected. Historical build sources `01f6efc859646fada7ef28fffa9322d266245469` and `4319b58` are retained below. Items marked PENDING are completed before the tag; the owner decides publication after the [scope conditions](release-scope-0.3.0.md) are met. Published v0.2.0 assets are unchanged.

### Main changes

- Owner decision (2026-10-14): the automatic boot filename is `AUTORUN.ELF`; the previous `MUSHA.ELF` is not accepted. This implementation choice does not change the ABI, syscalls, NOTE or SDK-generated ELF. When AUTORUN.ELF is present, the viewer does not start.

- Run one external C application per boot: place an SDK-built ELF named `AUTORUN.ELF` in the root of the FAT32/exFAT data medium; the kernel validates it, shuts down devices, and executes it at Ring 3 (CPL 3) with its own stack, arena and read-only start record. If the file is absent, the v0.2.0 read-only viewer starts as before.
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
2. Copy `build/hello-c.elf` to the root of the USB as `AUTORUN.ELF` (ASCII case is ignored; duplicate matching names are rejected).
3. Boot. The screen shows `Loading AUTORUN.ELF ...`, then the program's `log` output, then `AUTORUN.ELF exited: status <n>`. Reboot to return to the OS.

Limits: one application per boot; file size 64 bytes to 16 MiB with a 5-second read deadline (the 13 KB Hello reads well within it; large files may reach the deadline); operations are `query`, `log` (UTF-8, at most 4096 bytes per call, printable ASCII and newline rendered, other bytes shown as `.`) and `exit`. No input, file, display-command or network calls are available to applications. A user fault shows `AUTORUN.ELF faulted` with vector, error code and RIP/RSP and halts; a refused file shows `AUTORUN.ELF rejected: <reason>` and halts.

**The ABI is experimental candidate r1.** Syscall numbers, the start record, the NOTE format and memory layout may change in a later version without compatibility. There is no libc, POSIX, Rust SDK, dynamic loading or threads.

### Controls and limits of the viewer

Unchanged from v0.2.0: external USB Boot Keyboard required; Up/Down selects, Enter opens, Page Up/Down navigates, Esc returns, Q ends the session. 128 root files, 64 MiB per file, 10 seconds per page read. See the [viewer guide](file-viewer.md).

### Known issues

- When `AUTORUN.ELF` is present, the viewer does not start.

- MacBook Pro 2018: the 0.3.0 build boots and detects an external keyboard, but does not recognize the USB storage, so the viewer and external applications are unavailable there. Whether this is a regression from the 10/12 rc1 observation or a connection difference is not yet determined. The internal keyboard is unsupported (T2 unimplemented).
- NUC5: cold NO MEDIUM is observed in IMG_7633. The routing/grace fix requires retesting on the next candidate. Cold-boot repeatability is recorded under hardware validation below.
- Console scale is 1× below 2160 pixels in height, 2× from 2160 to 2879, and 3× from 2880. A 3840×2160 display uses 2×.
- Two keyboards on different controllers: the first selected device prevents input from the other (H15).
- Unoptimized debug-profile builds fault during filesystem discovery (vector 8); release-profile builds are unaffected.
- Physical NIC communication (I218-V/I219-V) is not implemented; controller detection is not networking.
- Returning to the OS after application exit without a reboot is not part of this release (C3 remains an opt-in probe).

### Validation scope

Historical validation: host tests 261. The Intel-routing candidate passed all 186 QEMU cases (`out/intel-routing/report.json`), covering external ELF loading on Basic Data and legacy ESP media, console/exit/fault/refusal display, absent-file viewer pixel regression with zero debug bytes, viewer/cooperative/hub regressions, and the C1/C2/C3 probes (8/14/20). The SDK was rebuilt in a separate directory containing spaces and reproduced the same Hello hash. Deployment and update tools were tested on virtual images and regular directories.

Hardware: during development, the external Hello ran at Ring 3 on NUC5i5RYH (GOP 1600×1200) and on a Ryzen 7 machine (GOP 3840×2160) with on-screen output and exit status 0; these runs used development EFIs, not the candidate. Candidate acceptance on the exact release EFI — three cold boots each on NUC5 and Ryzen, and Basic Data boot on MacBook Pro 2018 — is **PENDING** and will be recorded in [hardware acceptance](hardware-acceptance-0.3.0.md). QEMU and host results are not hardware evidence.

### Final artifact verification

SDK Hello ELF SHA-256: `d60006f88e1e36409867546df31c778cf3d5fb7c077fba376ebb5bd3fbb2810f` (13,312 bytes).

Historical source: `01f6efc859646fada7ef28fffa9322d266245469` (candidate build). EFI SHA-256: `cb0f532d24fb12c238069cec2b906c74f11fdb91b9745855ab5079c86196f23c` (460,288 bytes). Toolchain: Rust 1.99.0, Apple clang 21.0.0.
Remote CI: SUCCESS on candidate source `4319b58dda03400032c0307eaaa3b85fe8150ac0`: [run 38365638298](https://github.com/hirofumi-iwasaki/musha-os/actions/runs/38365638298). GP/DF injection, all remaining workflow regressions and normal candidate packaging passed; draft creation was skipped. This supersedes failed run 38360736372 on 04d986d. Logs are in `out/repro-4319b58/remote-job.log`. The CI-built EFI is a separate artifact, not an attestation of the local USB EFI hash above. Asset download and hash verification after release upload: PENDING.

Publication provenance: procedure A (reproducible EFI) adopted. `/Brepro` alone still differed in 20 metadata bytes; `/Brepro` + `/debug:none` produced two byte-identical clean local builds, SHA-256 `f7d21cb4c96f5ed286562c0564b2de0662f9a3cb9473893b7d7586c7fcdba139` (472,576 bytes), from 54ac242 plus the target-config patch. Evidence: `out/repro-brepro-nodebug/report.json`; this is a reproducibility experiment, not the final candidate or hardware acceptance. Build and accept the next approved candidate. After documentation-only updates, rebuild the final commit with the same inputs/toolchain and require full-file equality with the hardware-accepted EFI. Generate final packages/manifest from that final commit and tag that same source; do not relabel the earlier manifest. Any mismatch blocks publication for investigation and owner decision. Final-source equivalence and hardware acceptance remain PENDING. See [reproducible build conditions](release-distribution-policy.md) and [development instructions](development.md).

---

## 日本語

現在の受入判定（2026-10-14）：IMG_7633で冷間NO MEDIUMを1回（PRE OWN D8=0）、IMG_7634で切替済みD0=0x7FF/D8=0xFのHello・終了0を1回確認。run 1後の操作とrun 2/3の電源断時間は所有者も正確には覚えておらずUNKNOWNです。所有者はこの訂正を承認済みです。冷間3回連続成功は未証明。以下の成功回数は過去の所有者報告として保持し、次候補の受入には転用しません。[実機受入記録](hardware-acceptance-0.3.0.md)を参照。

過去の通常版実機試験候補は 5e4af31ece007550ba4d41a8a5c58d8b7a3b493e8665f277b7817134dd8911af。所有者報告で診断版のNUC5・Ryzen各冷間3/3成功（Ryzenはホット1/1も成功）後に再生成。manifest・QEMUはout/nuc5-accepted-normal/。未コミットソースであり、NUC5通常版候補の冷間起動3/3は成功（所有者申告、IMG_7627.MOVでHello・終了0を確認、直前USB配備と関連付け）。Ryzen通常版候補も冷間3/3・ホット1/1成功（所有者申告、IMG_7628 2.movでHello・終了0）。探索の待ち時間が長い課題あり。MacのBasic Data起動・最終梱包・現行ソースのリモートCIは未完了。以下の旧版・公開来歴は過去の記録として保持する。

過去のローカル実機試験版はAddress Device復旧版です。通常EFI `2d8b3a0669108cc6ece04c696dc84292e180a1f6bc2e85f185695ef9a168a09f`、診断EFI `7eba0377ffdc39fea24544a52c8fed86bbf7254c2869d753fceebaffb948676a`。未コミット・実機受入前です。[検証と来歴](handoff/0.3.0-address-recovery-results.md)。以下の梱包・CI記録は前のコミット済み候補の記録です。

USBキーボード・SanDisk接続時のNUC5冷間起動失敗を受け、未確定のIntelルーティング仮説へ対応する当時の候補でした。現行修正の実機受入は未確認です。[切替方針](intel-xhci-routing.md)を参照。以下のリモートCI成功は前ソース4319b58の結果であり、新候補の結果ではありません。

候補ノート更新日：2026-10-14。公開日：未定。

状態：**公開候補、未公開。** 最終候補ソースは未選定。以下の `01f6efc859646fada7ef28fffa9322d266245469` と `4319b58` は過去のビルド記録です。「未完了」の項目はタグ作成前に埋め、公開可否は[公開範囲](release-scope-0.3.0.md)の条件を満たした後に所有者が判断します。公開済み v0.2.0 配布物は変更しません。

### 主な変更

- 所有者決定（2026-10-14）：自動実行名は `AUTORUN.ELF` とし、旧 `MUSHA.ELF` は受理しません。実装上の選択でありABI・syscall・NOTE・SDK生成ELFは変更しません。AUTORUN.ELFがあるとビューアーは起動しません。

- 自作 C アプリの実行：SDK でビルドした ELF を `AUTORUN.ELF` の名前で FAT32/exFAT データ媒体のルートに置くと、カーネルが検査し、機器を停止してから Ring 3（CPL 3）で実行します。専用スタック・arena・読出し専用の起動情報を渡します。ファイルがなければ従来の v0.2.0 読出し専用ビューアーが起動します。
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
2. `build/hello-c.elf` を USB のルートに `AUTORUN.ELF` としてコピーします（ASCII の大小文字は区別しません。同名の重複は拒否します）。
3. 起動すると `Loading AUTORUN.ELF ...` の後にプログラムの `log` 出力、続いて `AUTORUN.ELF exited: status <n>` が表示されます。OS に戻るには再起動します。

制限：1起動につき1アプリ。ファイルは 64 byte〜16 MiB、読込み期限は5秒（13 KB の Hello は余裕で読めますが、大きなファイルは期限に達することがあります）。使える操作は `query`、`log`（UTF-8、1回最大 4096 byte、ASCII 印字可能文字と改行を描画、他は `.`）、`exit` のみです。入力・ファイル・描画命令・通信の呼出しはありません。ユーザー例外は `AUTORUN.ELF faulted` と vector・error code・RIP/RSP を表示して停止、拒否は `AUTORUN.ELF rejected: <reason>` を表示して停止します。

**ABI は実験候補 r1 です。** syscall 番号、起動情報、NOTE 形式、メモリ配置は後続の版で互換性なく変わる可能性があります。libc、POSIX、Rust SDK、動的ロード、スレッドはありません。

### ビューアーの操作と制限

v0.2.0 から変更なし：外付け USB Boot Keyboard が必要。上下で選択、Enter で開く、Page Up/Down でページ移動、Esc で戻る、Q で終了。ルート 128 ファイル、1ファイル 64 MiB、1ページの読出し 10 秒。[ビューアー](file-viewer.md)を参照。

### 既知の問題

- `AUTORUN.ELF` があるとビューアーは起動しません。

- MacBook Pro 2018：0.3.0 は起動し外付けキーボードを検出しますが、USB ストレージを認識しないため、ビューアーと外部アプリは使えません。10/12 の rc1 で観測した FAT32 認識との違いが退行か接続構成差かは未特定です。内蔵キーボードは未対応（T2 未実装）。
- NUC5：IMG_7633で冷間NO MEDIUMを観測。ルーティング・猶予修正は次候補で再確認が必要です。冷間起動の再現性は下記の実機確認で記録します。
- コンソールの倍率は画面高さ2160px未満で1倍、2160〜2879pxで2倍、2880px以上で3倍です。3840×2160では2倍です。
- 別コントローラーの2台のキーボード：先に選ばれた機器が他方の入力を妨げます（H15）。
- 未最適化の debug 構成はファイルシステム探索中に例外（vector 8）を起こします。release 構成には影響しません。
- 実機の NIC 通信（I218-V/I219-V）は未実装です。制御器の検出は通信ではありません。
- アプリ終了後に再起動なしで OS へ戻る機能は本版に含みません（C3 は任意試験構成のままです）。

### 検証範囲

過去の検証：ホスト試験261件。Intelルーティング候補のQEMU全186件合格（`out/intel-routing/report.json`）：Basic Data と従来 ESP 両媒体での外部 ELF 読込み、コンソール・終了・例外・拒否の表示、ファイル不在時のビューアーのピクセル回帰（debug 出力 0 byte）、ビューアー・協調 I/O・ハブの回帰、C1/C2/C3 試験（8/14/20）。SDK は空白を含む別ディレクトリーで再ビルドし、同一の Hello ハッシュを再現しました。配備・更新ツールは仮想イメージと通常ディレクトリーで試験しました。

実機：開発中に NUC5i5RYH（GOP 1600×1200）と Ryzen 7 機（GOP 3840×2160）で外部 Hello が Ring 3 で動作し、画面出力と終了コード 0 を確認しました。これらは開発版 EFI での結果で、候補 EFI ではありません。候補 EFI での受入——NUC5・Ryzen 各3回の冷間起動、MacBook Pro 2018 の Basic Data 起動——は**未完了**で、[実機受入記録](hardware-acceptance-0.3.0.md)に記録します。QEMU とホストの結果は実機の根拠にしません。

### 最終配布物の確認

SDK Hello ELF SHA-256：`d60006f88e1e36409867546df31c778cf3d5fb7c077fba376ebb5bd3fbb2810f`（13,312 byte）。

過去のソース：`01f6efc859646fada7ef28fffa9322d266245469`（候補ビルド）。EFI SHA-256：`cb0f532d24fb12c238069cec2b906c74f11fdb91b9745855ab5079c86196f23c`（460,288 byte）。ツールチェーン：Rust 1.99.0、Apple clang 21.0.0。
リモート CI：候補ソース `4319b58dda03400032c0307eaaa3b85fe8150ac0` の[run 38365638298](https://github.com/hirofumi-iwasaki/musha-os/actions/runs/38365638298)でSUCCESS。GP/DF注入、残りの全workflow試験、通常版候補の梱包まで合格し、draft作成はスキップしました。04d986dの失敗run 38360736372に対する再試験結果です。ログは `out/repro-4319b58/remote-job.log`。CI生成EFIは別成果物であり、上記ローカルUSB候補のハッシュを証明するものではありません。Releaseアップロード後の取得・ハッシュ照合は未完了です。

公開来歴：方式A（EFI再現ビルド）を採用します。`/Brepro` 単独ではメタデータ20バイトが不一致でしたが、`/Brepro` + `/debug:none` により54ac242＋ターゲット設定変更からのローカルクリーンビルド2回が完全一致しました。SHA-256は `f7d21cb4c96f5ed286562c0564b2de0662f9a3cb9473893b7d7586c7fcdba139`（472,576バイト）、証拠は `out/repro-brepro-nodebug/report.json`。これは再現性実験であり最終候補や実機受入ではありません。次の承認済み候補をビルド・受入後、文書のみの更新を含む最終コミットを同じ入力・ツール版で再ビルドし、実機受入済みEFIとの全バイト一致を要求します。最終配布物・manifestはその最終コミットから生成し、同じソースへタグを付けます。旧manifestの来歴は付け替えません。不一致なら公開を止めて調査・所有者判断へ戻します。最終ソースでの一致と実機受入は未完了です。[再現ビルド条件](release-distribution-policy.md)・[開発手順](development.md)を参照。
