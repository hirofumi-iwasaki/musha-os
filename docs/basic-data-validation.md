# v0.3.0 Basic Data validation / Basic Data 検証記録

Status: current

Historical filename note (2026-10-14): recorded screens used `MUSHA.TXT`; new candidates use `DIAGNOSE.TXT`. Historical observations below are unchanged.

当時のファイル名は `MUSHA.TXT`。2026-10-14以降の候補は `DIAGNOSE.TXT` を使用します。過去の実機・検証記録は変更していません。


## English

2026-10-13, `release/0.3.0`. Owner decision: [distribution policy §4.4](release-distribution-policy.md).
This change is separate from GOP/console commit `b5cc6ce`. **Physical Basic Data boot
is pending on NUC5, MacBook Pro 2018 and Ryzen**. Existing physical ESP media were not
modified; their manual-mount guidance remains applicable. No release/public asset was changed.

### 1. Scope status and evidence

All paths below are relative to `out/basic-data-0.3.0/` unless stated otherwise.

| Required goal | Status | Evidence |
|---|---|---|
| External ELF loading on Basic Data and legacy ESP | QEMU verified | `external-normal-{basic-data,esp}/report.json`; `usb-boot-hello-{basic-data,esp}/report.json` |
| Kernel text log output | QEMU verified | normal reports: Hello, log-long, log-utf8; actual screenshots and decoded text |
| Exit and fault display | QEMU verified | normal reports: exit-42, fault-pf, fault-ud and invalid-ELF rejection cases |
| Default absent-ELF viewer | QEMU verified | `external-normal-{basic-data,esp}/absent/pixels/result.json`: exact list/text/list pixels, debug 0 bytes |
| GPT candidate union and strict ambiguity rejection | Implemented, host verified | `host-tests.log`: ESP and Basic Data, 512/4096-byte sectors, mixed/multiple candidates, existing corruption checks |
| Deterministic generation and type-preserving deployment/update | Implemented, host/virtual-disk verified | `image-tests.log`, `distribution-final.log`, `virtual-roundtrip-retry/report.json` |
| Viewer/cooperative regression on both types | QEMU verified | `regression/report.json`: 38 cases |
| NUC5 / MacBook Pro 2018 / Ryzen Basic Data boot | Not started | Owner hardware acceptance remains required; no local result is a hardware result |

Host workspace tests: **253 passed**. Tool suites: image **4**, distribution **18**,
USB build **1**, release package **6** passed. Virtual make → deploy → update:
**2 passed**. Each virtual disk retains byte-identical GPT metadata; only `MUSHA.TXT`
is replaced, unchanged EFI retains its modification time, and a second update has
no changes. These are new test images, not the physical dedicated USB.

Normal external ELF: **25 cases per type**. Additionally, **one normal Hello case
per type boots directly from the GPT USB image**, without a separate EFI boot disk.
The 38 viewer/cooperative cases cover FAT32 and existing separate exFAT fixtures,
same/split controllers, cancel/disconnect/timeout, unavailable media, traffic,
idle/no keyboard, keyboard detach, link-down, TX timeout, application error and hub depth 5.
Diagnostic external ELF: **26 cases per type**, including the recorded Basic Data
retest. Total **142 QEMU case/configuration passes** across the listed suites;
this count excludes the initial failed timeout attempt. Final aggregate: `report.json`.

### 2. Adopted behavior

- `make-usb-image.py` defaults to `--partition-type basic-data`, GUID
  `EBD0A0A2-B9E5-4433-87C0-68B6B72699C7`. Use `--partition-type esp` for legacy-type
  compatibility tests. Both use GPT partition name `Musha-OS` and FAT volume label
  `MUSHA-OS`. Same EFI, capacity and type produce identical bytes.
- FAT32 discovery counts the combined ESP/Basic Data **type candidates before BPB
  validation** and requires exactly one. It does not scan for a preferred BPB among
  ambiguous candidates. GPT CRCs, both copies, geometry, bounds, GUID uniqueness,
  protective MBR and overlap checks are unchanged.
- The runtime retains the first available supported medium in its existing scan
  order; it does not search other media for AUTORUN.ELF. Within a medium it tries exFAT
  first, then FAT32 only on `Unsupported`. A valid exFAT Basic Data plus FAT32 Basic
  Data (or ESP) therefore selects exFAT, irrespective of entry order. Direct FAT32
  discovery rejects the multiple candidates. Corrupt exFAT does not silently fall
  back to FAT32. Mixed partitions remain outside the supported product layout.
- Deployment preserves the input partition type and fixed FAT32 extent; it adjusts
  only disk-capacity-dependent GPT/MBR metadata. Normal updates accept both types
  but only replace changed files; no type conversion, full-medium image generation,
  write or hash operation is introduced.
- `update-usb.py` retains physical USB identity/safety checks and now records the
  partition type. The separate virtual test verifies its attached image is virtual;
  it does not relax the production CLI to accept arbitrary virtual/physical disks.
- The legacy packaging manifest's `image_scope` now describes Basic Data accurately.
  No ABI value, syscall, viewer rendering rule or default application behavior changes.

### 3. Failures and retests

1. Initial host run failed the old mixed ESP+exFAT test's expectation that direct
   FAT32 discovery succeeds. With both types eligible there are two candidates.
   The test now expects `Unsupported`, while explicit FAT32 mounting verifies its
   fixture and exFAT discovery still succeeds. The full workspace then passed.
2. Initial virtual roundtrip attached its test image, but plist decoding failed
   because an hdiutil deprecation warning was merged into stdout. Standard error is
   now separate; the test attachment was detached and both types passed on retry.
   Initial evidence: `virtual-roundtrip/report.json`; retest: `virtual-roundtrip-retry/`.
3. While another QEMU matrix was running, the Basic Data diagnostic `timeout` case
   reached USB `TRANSFER TIMEOUT` and displayed `Device session did not complete`
   instead of the expected file-read deadline. It is an initial failure, not a pass.
   `external-debug-basic-data/report.json` preserves it. With other QEMU runs stopped,
   the same EFI and direct USB-boot configuration passed the unchanged timeout assertion:
   `debug-basic-timeout-retry/report.json`. The concurrency-related cause is not proven;
   no production deadline or assertion was weakened. Remaining Basic Data cases use
   `debug-basic-disconnect-while-reading-retry/` and `debug-basic-absent-retry/`.

### 4. Exclusions and pending checks

No second distribution partition, new exFAT distribution region, existing-media type
rewrite, relaxed GPT validation, physical USB operation, push, tag, Release, GitHub
setting or public PDF change. Existing exFAT **test fixtures** are used for regression.
`Claude outputs/` is preserved and excluded from these commits.

Actual firmware boot compatibility, physical macOS/Windows auto-mount behavior and
acceptance on the three owner machines remain pending. Test success in macOS virtual
disks is not a claim about Explorer, Finder or physical firmware.

### 5. Normal binary and pixel reference

Normal EFI: `target-normal/x86_64-unknown-uefi/release/musha-boot.efi`.
SHA-256: `b743ac0129a9692adf865c37c33adb2c23ca2c0238baf5b9437ebe20faee2b1a`.
Features are empty. Both normal matrices and the two direct USB Hello tests use
these same bytes. New QEMU source builds may differ in PE timestamps.

Pixel reference: `out/v3-gop-max-pixels/`, derived from the preserved
`out/v3-normal-final-pixels/` by the preceding GOP change; see
[generation rationale and hashes](gop-mode-selection.md). **This partition change
requires no further pixel reference update.**

### Reproduction

Use the repository's configured Rust and QEMU toolchains. Output directories must be fresh.
Full child commands are retained in the QEMU JSON reports.

```sh
cargo test --locked --workspace --exclude musha-boot
python3 tools/test-usb-image.py
python3 tools/test-distribution.py
python3 tools/test-usb-build.py
python3 tools/test-release-package.py
cargo fmt --all -- --check
cargo build --locked --release --target x86_64-unknown-uefi -p musha-boot --target-dir out/basic-data-0.3.0/target-normal
python3 tools/test-partition-roundtrip.py --efi out/basic-data-0.3.0/target-normal/x86_64-unknown-uefi/release/musha-boot.efi --output out/partition-roundtrip-new
python3 tools/smoke-partition-types.py --firmware-dir /opt/homebrew/share/qemu --output out/partition-regression-new
# Repeat each command for basic-data and esp; run timing-sensitive QEMU cases serially.
python3 tools/smoke-external-elf.py --firmware-dir /opt/homebrew/share/qemu --normal --efi out/basic-data-0.3.0/target-normal/x86_64-unknown-uefi/release/musha-boot.efi --partition-type basic-data --output out/partition-normal-new
python3 tools/smoke-external-elf.py --firmware-dir /opt/homebrew/share/qemu --normal --efi out/basic-data-0.3.0/target-normal/x86_64-unknown-uefi/release/musha-boot.efi --partition-type basic-data --boot-from-data --case hello-fat32 --output out/partition-hello-new
python3 tools/smoke-external-elf.py --firmware-dir /opt/homebrew/share/qemu --partition-type basic-data --boot-from-data --output out/partition-debug-new
```

## 日本語

2026-10-13、`release/0.3.0`。所有者決定の実装で、先行GOP・文字サイズ変更
`b5cc6ce` とは別コミット。**NUC5・MacBook Pro 2018・RyzenのBasic Data実機起動は確認前**。
既存の専用ESP USBは未変更で、手動マウントの案内を維持する。

### 1. 必須目標と検証

外部ELF読込み・画面log・exit/fault表示・不在時の既定ビューアーは両種別で
「QEMUで検証」。証拠パスと対応表は英語節に記載した。ホスト253件、生成4件、
配布18件、USBビルド1件、パッケージ6件が合格。仮想媒体の往復2件ではGPT不変、
変更ファイルだけの差替え、2回目の差分なしを確認した。
通常版外部ELFは各25件、仮想USBそのものからのHello起動も各1件成功。
viewer/cooperativeは両種別合計38件。診断版外部ELFは再試験を含め各26件、
QEMU全体で142ケース・構成が合格（初回不合格の期限試験は合格数に含めない）。集計は `report.json`。
これらは実機結果ではない。

### 2. 採用値と規則

既定はBasic Data、互換用は `--partition-type esp`、GPT名は `Musha-OS`。
FAT32はESPとBasic Dataの合計候補が一つであることをBPB検証前に要求し、既存の
GPT検査をすべて維持する。媒体選択は「最初に利用可能な媒体」を保持し、同一媒体では
exFAT→FAT32（Unsupported時のみ）の順。対象外のexFAT/FAT32混在媒体では有効な
exFATが先に選ばれ、FAT32単独探索は複数候補として拒否する。
配備・通常更新は入力の種別を保持し、通常更新で全容量の生成・書込み・ハッシュ計算をしない。

### 3. 不合格と再試験

旧混在試験の「ESPだけを数える」期待を、新しい候補集合の拒否規則に合わせた。
仮想媒体試験の初回はhdiutil警告混入によるplist解析エラーで停止し、出力分離後に再試験成功。
診断版Basic Dataの期限試験は、QEMU同時実行中にファイル期限より先にUSB転送期限で停止した。
初回不合格を保存し、同じEFI・同じ条件で単独再試験して成功した。同時実行との因果は未確定。
期限や合格条件を緩めてはいない。各証拠は英語節に列挙した。

### 4. 除外項目と残件

配布媒体の2パーティション化、exFAT領域作成、既存ESP種別の書換え、GPT検査緩和、
実USB操作、push・タグ・Release・GitHub設定・公開PDFの変更はなし。
exFATは既存の試験用fixtureだけを回帰に使用した。`Claude outputs/` は保持してコミット対象外。
残件は所有者による3機の実機起動・ホストでの自動表示確認。完了までは公開上の互換性を確定しない。

### 5. 通常EFIと参照

通常版SHA-256は英語節の `b743ac…e2b1a`。ピクセル参照は先行GOP作業の
`out/v3-gop-max-pixels/` をそのまま使用し、今回の種別変更では更新していない。
