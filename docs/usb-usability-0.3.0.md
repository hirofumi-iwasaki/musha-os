# Candidate USB usability / 候補USBの利用者向け整備

Status: record (2026-10-14)

## English

Owner decision: 2026-10-14. Local implementation and test evidence only.
Push, distribution candidate generation and physical USB updates remain pending
owner approval. No ABI, syscall, NOTE, SDK ELF or disk layout change.

### Changes and migration

- README.TXT: exact owner text with VERSION substitution, ASCII/CRLF, 72 columns
  or fewer, under 4 KiB. Generated in the initial image, files bundle, update
  ZIP and tar bundle; recorded by their manifests and checksums.
- DIAGNOSE.TXT: replaces the diagnostic marker. Contents remain 16 bytes
  (`Hello Musha-OS!\n`), FNV `9A42A948C590F507`. Runtime cached-file selection,
  FAT short name, exFAT UTF-16 selection, fixture names and diagnostic messages
  are updated. External automatic execution remains AUTORUN.ELF.
- Disk layout remains 1. Update manifest schema 2 declares only the retired
  MUSHA.TXT, and requires the new marker. Schema 1 remains accepted without
  removal. Old deployment tools reject schema 2. Use the matching tools.
- Dry-run reports removals; apply verifies host backups before replacement and
  deletion. Edited old markers are backed up too. No other user file is deleted.
  The update manifest is installed and every managed file is read back into
  `verified-files`; `verified-absent` checks the retired name. Repeat updates do
  not replace unchanged files. No full USB image or full-capacity hash is used.

### Why third_party was missing

`out/autorun-usb/provenance.json` records deployment from 9f100ce and a cleanup
matching the then-current initial image's **two files** (EFI and old marker).
Its `removed_files` contains all eight license/attribution files, LICENSE and
NOTICE; backups are under `out/autorun-usb/volume-backup`. This was not a lost
file inside the regular update ZIP: that package already contained the notices.
The cleanup's initial-image-only selection caused the omission.

The next approved USB update must use the full managed files ZIP. Its
`verified-files` must contain EFI/BOOT/BOOTX64.EFI, DIAGNOSE.TXT, README.TXT,
LICENSE, NOTICE, the eight third_party files, and update-manifest.json.
Do not repeat the two-file cleanup. AUTORUN.ELF is separately placed and hashed.
Initial images still carry the minimal boot files plus README; license texts
travel in the accompanying files/deployment archives, as previously documented.

### Pixel reference changes

`tools/migrate-marker-reference.py` derives new expected FAT32 frames from
`out/release-final-d/v030-pixels`, rather than accepting new guest captures.
Only list/text count and selected-name rows (2, 5), and list file rows (9, 10)
change. DIAGNOSE.TXT remains first and README.TXT follows. Marker page bytes
remain unchanged. `tools/viewer-reference.py` still derives the normal absent
status row. ExFAT uses the renamed fixture and the checked viewer frames for
its normal-build comparison. The list order of other exFAT fixture files is
unchanged.

Evidence root: `out/usb-usability/`. Tests and exact residual legacy-name lines
are recorded there; historical documents retain old names with a dated note.
No new result is described as physical hardware validation.

## 日本語

所有者決定：2026-10-14。今回はローカル実装・試験のみです。push・配布候補生成・
実USB更新は報告後の所有者承認待ちです。ABI・syscall・NOTE・SDKのELFと
ディスク構成は変更しません。

### 変更・移行

- README.TXTは指定文面にVERSIONを埋め込み、ASCII・CRLF・72桁以内・4 KiB未満。
  初回像・files一式・更新ZIP・tarのルートに配置し、manifestとチェックサムに含めます。
- 診断マーカーをDIAGNOSE.TXTに変更。16 byteとFNV `9A42A948C590F507` は不変。
  キャッシュ選択、FAT短名、exFATのUTF-16照合、フィクスチャ、診断表示を更新します。
  自動実行用のAUTORUN.ELFとは別です。
- ディスクlayout 1を維持し、更新manifest schema 2で旧MUSHA.TXTのみを廃止対象に
  指定します。新マーカーを必須とし、schema 1には削除を適用しません。
  旧配備ツールはschema 2を拒否するため、同梱の新ツールを使用します。
- dry-runで削除予定を表示。applyで旧ファイルをホストへ退避・照合してから置換・
  削除します。編集済み旧マーカーも退避し、それ以外の利用者ファイルは削除しません。
  manifest自体も配置し、全管理ファイルをverified-files、旧名不在をverified-absentへ
  記録します。同じ内容は差し替えず、USB全容量の像生成・ハッシュ計算も行いません。

### third_party欠落の経緯

`out/autorun-usb/provenance.json` は9f100ceからの配備時に、当時の初回イメージの
2ファイル（EFIと旧マーカー）へ揃えた清掃を記録しています。removed_filesには
ライセンス類8ファイルとLICENSE・NOTICEがあり、volume-backupへ退避されています。
通常更新ZIPからの脱落ではなく、初回像の2ファイルだけを残す選択が原因でした。

次回承認後は全管理ファイルを持つ更新ZIPを使い、verified-filesにEFI・DIAGNOSE.TXT・
README.TXT・LICENSE・NOTICE・third_partyの8ファイル・update-manifest.jsonが
揃うことを確認します。2ファイルへの清掃は繰り返しません。AUTORUN.ELFは別途
配置・ハッシュ記録します。初回像はREADMEを加えた最小構成で、ライセンスは
従来どおりfiles/deploymentアーカイブと一緒に配布します。

### ピクセル参照

旧参照から指定の文字行を独立描画するmigrate-marker-reference.pyを追加しました。
新しいゲスト画面をそのまま正解にはしていません。FAT32の行2・5（件数・選択名）と
一覧の行9・10（DIAGNOSE.TXT、README.TXT）だけを更新し、本文データは不変です。
通常版の不在案内は既存のviewer-reference.pyで導出します。exFATは改名した
フィクスチャの検証済みビューアー画面と通常版を比較し、他の試験ファイル順序は維持します。

証拠は `out/usb-usability/` に保存。旧名の残存一覧は理由とともに同ディレクトリーへ
記録し、過去の実機・試験文書には当時の名前である旨を注記します。
新しい実機検証結果としての記載は行いません。

## Local validation result / ローカル検証結果

- Rust host: 272 passed. Python packaging/distribution: 30 passed.
- Virtual Basic Data/ESP make → deploy → update: both passed; GPT preserved,
  old marker backed up and removed, unchanged EFI untouched, second update empty.
- QEMU: 198 distinct cases covered, 197 passed and **one intermittent case**.
  C1/C2/C3, five-level hubs, split controllers, both partition types, FAT32
  diagnostics, normal FAT32/exFAT pixels (debug output zero), and delayed
  attachment 200/800/5000 ms are covered.
- The first normal Basic Data duplicate test used an obsolete fixed root offset.
  Its fixture was corrected to locate AUTORUN by name; retry passed and skipped
  cases were completed. No guest behavior or rejection expectation was relaxed.
- Debug Basic Data `timeout`: isolated current-EFI repeats passed twice and
  failed once with `TRANSFER TIMEOUT` before the expected file deadline.
  The **unchanged 3d3d6eb source and previously built EFI** reproduced the same
  failure once in three isolated runs (pass/fail/pass). This is a pre-existing
  unstable case, not evidence of stable acceptance. Investigation remains open;
  runtime deadlines and expected text are unchanged. Retain all failure logs.
- Normal test EFI SHA-256 (not a distribution candidate):
  `982fa2ac30a64182a4a0c495a4f64ceea706d46c537ac28ffc88aab96d646da7`.
  A separate rebuild after formatting produced identical bytes.

Commands and outcomes: `out/usb-usability/report.json`, `extra-report.json`,
`retry-debug-report.json`, `completion-debug-report.json`, `summary.json`.
The baseline comparison is `baseline-isolated-{1,2,3}`. Residual old-name lines
and reasons: `legacy-marker-occurrences.txt` / `.json`; changed paths:
`changed-files.txt`; third-party provenance audit: `third-party-audit.json`.
These local out/ files are not committed. The two commits separate README.TXT
addition from marker migration. Push, candidate packaging, USB update and
physical acceptance are still pending approval/execution.

ホスト272件、配布・梱包30件、Basic Data／ESP仮想媒体の往復2件が合格しました。
QEMUは198ケースを網羅し、197件合格・1件は間欠的失敗です。C1/C2/C3、5段ハブ、
分割制御器、両パーティション、FAT32診断、通常版FAT32/exFATピクセル一致・
debug 0 byte、200/800/5000 ms遅延接続を含みます。

最初の通常版Basic Data重複試験には古い固定ディレクトリー位置が残っていました。
名前検索へ直して再試験が合格し、中断で未実行だったケースも完了しています。
デバッグ版Basic Dataのtimeoutは、単独3回で成功・成功・転送タイムアウトでした。
変更前3d3d6ebのソースと既存EFIでも、単独3回で成功・同じ失敗・成功となりました。
既存の不安定ケースとして未解決を明示し、全回帰安定合格とは扱いません。
実行期限や期待文言を緩める変更はしていません。

通常版試験EFIのSHA-256は上記の値です。書式整形後の別ビルドでも一致しました。
証拠・残存旧名一覧・変更ファイル一覧・第三者ライセンス退避の照合は上記out/へ
保存しています。README追加とマーカー改名を別コミットとし、push・配布候補生成・
実USB更新は承認待ち、実機受入は未実施です。
