# v0.3.0 release scope and publication conditions

Current acceptance update (2026-10-14): candidate 9f8d999 hardware gate PASS. NUC5: existing IMG_7645 diagnostic D8=0 once plus normal cold 3/3, accepted by owner; Ryzen: cold 3/3 + hot 1/1; Mac: Basic Data boot. Normal-run D8 was not measured. This supersedes earlier pending acceptance wording below. Post-acceptance equivalence and CI/publication remain pending. See [hardware acceptance](hardware-acceptance-0.3.0.md).

現行受入更新（2026-10-14）：候補9f8d999の実機ゲート合格。NUC5は所有者決定で既存IMG_7645診断D8=0証拠1回＋通常版冷間3/3、Ryzen冷間3/3＋ホット1/1、Mac Basic Data起動を採用。通常版D8は未測定です。下記の過去の未完了表記より本更新を優先します。受入後の一致確認・CI・公開は未完了。[実機受入](hardware-acceptance-0.3.0.md)。

## English

Decided: 2026-10-13, approved by the project owner. Target branch: `release/0.3.0`.
This document defines the publication scope of v0.3.0. It does not state that any item is implemented.
Implementation status is tracked in [validation and gates](boot-api-validation.md), [C1](ring3-prototype.md), [C2](ring3-elf.md) and [external loading](external-elf.md).
The v0.1.0 and v0.2.0 scopes, notes and validation records are unchanged.

### Positioning

v0.3.0 is a **developer preview: run your own C program on Musha-OS from a USB boot**.

The central goal is that a developer compiles a C Hello program with the supplied SDK, places the ELF on the USB
data medium, boots the same USB on a PC, and sees the program's output on the screen while it runs at Ring 3.
The stage A/B/C1/C2 results from [policy 3.0](boot-readiness-policy.md) are the foundation; v0.3.0 completes
only the part of stage C that this experience requires. The rest of stage C and all of stage D move to later versions.

The ABI stays **experimental candidate r1**. Publishing v0.3.0 does not fix public compatibility; numbers, layouts
and the NOTE format may change in a later version. Release documentation must say so.

### Required goals

**Owner update, 2026-10-13 — NOT PERFORMED:** the release candidate must still be tested on NUC5 and Ryzen with three cold boots each. MacBook Pro 2018 still requires candidate Basic Data boot verification only; its current storage failure is unresolved and investigation is deferred to v0.4.0 or later. These pending acceptance requirements supersede the older one-machine/Ryzen-or-Mac wording below. Historical development Hello success is not candidate acceptance.

| Item | v0.3.0 scope | Completion condition |
|---|---|---|
| External ELF loading | The kernel reads one fixed-name ELF from the FAT32/exFAT data medium through the existing bounded stream file service, validates it with the C2 checks and maps it | A SDK-built ELF copied to the medium runs without rebuilding the EFI. Oversized, malformed and missing files are rejected or skipped with an on-screen reason |
| Screen output | `log` output is rendered on the display by kernel-owned text drawing | Hello text is visible on QEMU and on physical hardware without a serial or debug port. No framebuffer or hardware register is exposed |
| Exit and fault handling | `exit` shows the result code and stops; user faults show the existing diagnostic and stop | The result or fault is readable on screen. DMA/device state follows the existing stop-before-reuse rules. Restarting the machine returns to normal operation |
| Default behavior | Without the fixed-name file, the normal build starts the v0.2.0 read-only viewer as before | Existing viewer list/open/back pixel regression and QEMU viewer cases pass unchanged |
| C SDK | Clang-based SDK (crt0, linker script, libmusha, headers, app-check, sample) can be copied out of the repository and used in another directory | `tools/check-sdk.py` passes in a relocated path; the documented commands reproduce the Hello ELF and its hash |
| Hardware | Ring 3 execution verified on at least one physical machine | Hello output observed on screen on a Ryzen or MacBook Pro 2018 configuration already recorded in [hardware status](hardware-verification-status.md) |

The user-facing description is: "You can build a C program with the SDK, put it on the USB, and Musha-OS runs it at Ring 3 and shows its output. The API and ABI are experimental and may change."

### Design boundaries for v0.3.0

- Owner decision (2026-10-14): the automatic boot filename is `AUTORUN.ELF`; the previous `MUSHA.ELF` is not accepted. This implementation choice does not change the ABI, syscalls, NOTE or SDK-generated ELF. When AUTORUN.ELF is present, the viewer does not start.

- One application per boot. The fixed file name and its location are decided during implementation and documented before publication. No launcher, menu or file selection for applications.
- The loaded ELF receives the candidate r1 start record, arena and stack; `query`, `log` and `exit` are the only operations. `log` is bounded as in C2. No input, time, file, display-command or network syscall is added.
- `exit` and faults halt as in C2. Returning to the OS, resource reclamation and relaunch without reboot are not required.
- Output rendering is kernel text drawing into a bounded region; scrolling, colors and layout are implementation choices and not part of the ABI.
- The boot FAT32 partition type changes from EFI System Partition to Microsoft Basic Data so that macOS and Windows show the USB and users can place `AUTORUN.ELF` (decided 2026-10-13, [distribution policy 4.4](release-distribution-policy.md)). The OS accepts both types; existing ESP media keep booting. Publication requires boot from a Basic Data medium on the recorded machines.
- Hardware verification of the Ring 3 path (SYSCALL MSRs, NX, TSS stack, CET rejection, user page permissions) is a new risk category separate from v0.2.0 USB/xHCI verification. The build used on hardware must be the release-profile normal build, not a probe feature build.

### Excluded from v0.3.0

The following remain future work and must not appear as supported in release documentation:

- Rust guest execution and a Rust SDK with the custom user target and rebuilt core/alloc. The existing Rust layout/wrapper checks remain compile-only.
- Return to the OS after application exit or fault, repeated launches, rollback after partial loading, CPU budgets, preemption, SMP.
- General fault-recoverable copyin/copyout, capabilities/handles, boot-report wire record, versioned report API.
- Practical I/O (stage D): input events, wait/deadline, display commands, file handles for applications, UDP, write operations.
- Fixing the public ABI, libc/POSIX, dynamic loading, threads.
- NUC5/NUC8 hardware results, physical NIC networking, internal Mac keyboard, the H15 multiple-keyboard issue, and the unoptimized debug-profile vector 8 issue are not v0.3.0 conditions. They remain open and are reported as such.

### Relationship to pending v0.2.0 hardware acceptance

The v0.2.0 hardware acceptance items (viewer navigation, exFAT reading, new deployment tooling on a physical USB) were carried over at publication.
They are performed in the same hardware session as the v0.3.0 Hello test when the machine is available, but they are recorded separately and are not v0.3.0 publication conditions.
Their outcome updates the v0.2.0 records and [hardware status](hardware-verification-status.md).

### Validation and publication conditions

1. Host tests cover ELF/NOTE validation for files read from the stream service, including truncated reads, size limits, read errors and medium removal before loading completes.
2. QEMU verifies: Hello from a FAT32 medium and from an exFAT medium; output on screen; exit code display; a faulting ELF; a missing file falling through to the viewer; each malformed-ELF rejection reason on screen; existing viewer, cooperative I/O, hub and C1/C2 probe regressions.
3. The SDK is checked from a relocated directory with recorded tool versions. The Hello ELF hash in the release notes must match the SDK output.
4. A normal release-profile EFI is built from clean source without probe features. Checksums, configuration, source commit and limitations are recorded.
5. Hardware: the same EFI and SDK Hello ELF run on at least one recorded physical configuration with on-screen output. Cold boot is repeated three times. Results use the [hardware test record template](hardware-test-record-template.md) and a `hardware-acceptance-0.3.0.md` record.
6. Publication is decided after items 1–5. Host and QEMU results are never presented as hardware evidence. Distribution follows the [distribution policy](release-distribution-policy.md): initial image plus normal-update ZIP, with the SDK as a separate archive.

### Development order

1. Commit and push the current A/B/C1/C2 local work on `release/0.3.0` after separate approval, so that the preview baseline exists outside one machine.
2. Load the ELF from the stream file service with bounded reads; reuse the C2 validator and mapping; add rejection reasons.
3. Route `log` to kernel text rendering; add exit/fault display.
4. Package and document the C SDK; refresh README and write release notes 0.3.0 in both languages.
5. QEMU regression, clean build, hardware session, publication decision.

Rust guest work may continue in parallel only if it does not delay items 2–5.

## 日本語

決定日: 2026-10-13。プロジェクト所有者承認。対象ブランチ: `release/0.3.0`。
本書はv0.3.0の公開範囲を定めるもので、各項目の実装完了を示すものではない。
実装状況は[検証と残件](boot-api-validation.md)、[C1](ring3-prototype.md)、[C2](ring3-elf.md)、[外部読込み](external-elf.md)で管理する。
v0.1.0・v0.2.0の公開範囲、ノート、検証記録は変更しない。

### 位置づけ

v0.3.0は **開発者プレビュー：自作のCプログラムをUSBブートのMusha-OS上で動かす** 版とする。

中心目標は、開発者が同梱SDKでC言語のHelloをコンパイルし、そのELFをUSBデータ媒体へ置き、
同じUSBからPCを起動すると、Ring 3で実行されたプログラムの出力が画面に表示されることである。
[方針3.0](boot-readiness-policy.md)の工程A・B・C1・C2の成果を土台とし、v0.3.0ではこの体験に必要な
工程Cの部分だけを完成させる。工程Cの残りと工程D全体は後続の版へ送る。

ABIは **実験候補r1** のままとする。v0.3.0の公開は公開互換性の固定ではなく、番号・配置・NOTE形式は
後続の版で変わりうる。公開資料にはその旨を明記する。

### 必須目標

**2026-10-13所有者更新・未実施：** 公開候補EFIでNUC5・Ryzen各3回の冷間起動を実施予定です。MacBook Pro 2018の候補Basic Data起動確認も未実施で、確認範囲は起動のみです。現行ストレージ未認識は未解決、調査は0.4.0以降へ先送りします。この未達の受入条件を、下記の旧「1台以上／RyzenまたはMac」条件より優先します。過去の開発版Hello成功を公開候補の合格とは扱いません。

| 項目 | v0.3.0の範囲 | 完了条件 |
|---|---|---|
| 外部ELFの読込み | 既存の上限付きstream fileサービスでFAT32/exFATデータ媒体上の固定名ELFを1つ読み、C2の検査を経て配置する | EFIを再ビルドせずに、媒体へ複製したSDK製ELFが動く。大きすぎる・不正・存在しないファイルは理由を画面に示して拒否または通常起動へ移る |
| 画面出力 | `log` の出力をカーネル所有のテキスト描画で画面に表示する | シリアルやデバッグポートなしで、QEMUと実機の画面でHelloの文字が見える。framebufferやハードウェアレジスターは公開しない |
| 終了と障害 | `exit` は結果コードを表示して停止、ユーザー例外は既存の診断表示で停止する | 結果または障害が画面で読める。DMA・機器は既存の停止前再利用禁止の規則に従う。再起動で通常動作に戻る |
| 既定動作 | 固定名ファイルがなければ、通常版は従来どおりv0.2.0の読出し専用ビューアーを起動する | 既存のビューアー一覧・表示・戻るのピクセル回帰とQEMUビューアー試験が変更なしで合格する |
| C SDK | Clangベースのsdk（crt0、リンカースクリプト、libmusha、ヘッダー、app-check、サンプル）をリポジトリ外へ複製して使える | 別ディレクトリーで `tools/check-sdk.py` が合格し、文書のコマンドでHello ELFとそのハッシュを再現できる |
| 実機 | 実機1台以上でRing 3実行を確認する | [実機確認状況](hardware-verification-status.md)に記録済みのRyzen機またはMacBook Pro 2018構成で、Helloの画面出力を確認する |

利用者向けの説明は「SDKでCプログラムを作ってUSBへ置くと、Musha-OSがRing 3で実行し出力を表示する。APIとABIは実験版で変更の可能性がある」とする。

### v0.3.0の設計上の境界

- 所有者決定（2026-10-14）：自動実行名は `AUTORUN.ELF` とし、旧 `MUSHA.ELF` は受理しません。実装上の選択でありABI・syscall・NOTE・SDK生成ELFは変更しません。AUTORUN.ELFがあるとビューアーは起動しません。

- 1回の起動で実行するアプリは1つ。固定ファイル名と配置場所は実装時に決め、公開前に文書化する。ランチャー、メニュー、アプリのファイル選択は作らない。
- 読み込んだELFには候補r1の起動情報・arena・スタックを渡し、操作は `query`・`log`・`exit` のみ。`log` の上限はC2と同じ。入力・時刻・ファイル・描画命令・通信のsyscallは追加しない。
- `exit` と障害はC2と同様に停止する。OSへの復帰、資源回収、再起動なしの再実行は要求しない。
- 出力描画はカーネルによる有界領域へのテキスト描画とする。スクロール・色・配置は実装上の選択であり、ABIには含めない。
- 起動用FAT32のパーティション種別をEFI System PartitionからMicrosoft Basic Dataへ変更し、macOS・WindowsでUSBが表示され利用者が`AUTORUN.ELF`を置ける構成にする（2026-10-13決定、[配布方針4.4](release-distribution-policy.md)）。OSは両種別を受け入れ、既存のESP媒体も起動できる。公開には記録済み実機でのBasic Data媒体からの起動確認を要する。
- Ring 3経路（SYSCALL用MSR、NX、TSSスタック、CET検出による拒否、ユーザーページ権限）の実機確認は、v0.2.0のUSB/xHCI確認とは別の新しい危険分類として扱う。実機に使うビルドはrelease構成の通常版とし、試験用feature付きビルドを使わない。

### v0.3.0の必須範囲から外すもの

次は後続作業とし、公開資料で対応済みと表記しない。

- Rust製アプリの実行、専用user targetとcore/alloc再構築を伴うRust SDK。既存のRust配置・wrapper検査はコンパイル検査のまま。
- アプリ終了・障害後のOS復帰、繰返し起動、部分読込み後のロールバック、CPU時間制限、プリエンプション、SMP。
- 汎用の障害回復可能なcopyin/copyout、ケイパビリティ／ハンドル、起動レポートの転送形式、版付きレポートAPI。
- 実用I/O（工程D）：入力イベント、待機・期限、描画命令、アプリ向けファイルハンドル、UDP、書込み。
- 公開ABIの固定、libc/POSIX、動的ロード、スレッド。
- NUC5/NUC8の実機結果、実機NIC通信、Mac内蔵キーボード、H15の複数キーボード問題、未最適化debug構成のvector 8は、v0.3.0の条件に含めない。未解決として報告する。

### v0.2.0実機受入の残件との関係

v0.2.0公開時に持ち越した実機受入項目（ビューアー操作、exFAT読出し、新配備ツールの実USB検証）は、
実機が使える同じ機会にv0.3.0のHello試験と併せて実施するが、記録は分け、v0.3.0の公開条件にはしない。
結果はv0.2.0の記録と[実機確認状況](hardware-verification-status.md)へ反映する。

### 検証と公開条件

1. ホスト試験で、streamサービスから読んだファイルに対するELF/NOTE検査を確認する。途中切断、サイズ上限、読出し失敗、読込み完了前の媒体取外しを含める。
2. QEMUで次を確認する：FAT32媒体とexFAT媒体からのHello、画面出力、終了コード表示、障害を起こすELF、ファイル不在時のビューアーへの移行、不正ELF各種の拒否理由表示、既存のビューアー・協調I/O・ハブ・C1/C2試験の回帰。
3. 別ディレクトリーへ複製したSDKでツール版を記録して検査する。リリースノートに記すHello ELFのハッシュはSDK出力と一致させる。
4. 試験用featureを含まないrelease構成の通常版EFIをクリーンなソースから作る。チェックサム、構成、ソースコミット、制限事項を記録する。
5. 実機：同じEFIとSDK製Hello ELFを記録済みの実機構成1台以上で動かし、画面出力を確認する。冷間起動を3回繰り返す。[実機試験記録様式](hardware-test-record-template.md)と `hardware-acceptance-0.3.0.md` に記録する。
6. 1〜5の後に公開可否を判断する。ホスト・QEMUの結果を実機の根拠として示さない。配布は[配布方針](release-distribution-policy.md)に従い、初回用イメージと通常更新ZIPに加え、SDKを別アーカイブとする。

### 開発順序

1. 現在のA/B/C1/C2のローカル作業を、別途承認のうえ `release/0.3.0` へcommit・pushし、プレビューの基準を1台の機械の外に置く。
2. stream fileサービスから上限付きでELFを読み、C2の検査・配置を再利用し、拒否理由を追加する。
3. `log` をカーネルのテキスト描画へ接続し、終了・障害の表示を追加する。
4. C SDKを梱包・文書化し、READMEを更新して二言語のリリースノート0.3.0を書く。
5. QEMU回帰、クリーンビルド、実機試験、公開判断。

Rust製アプリの作業は、2〜5を遅らせない範囲でのみ並行して進めてよい。
