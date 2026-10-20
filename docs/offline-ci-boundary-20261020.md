# Offline boundary CI checkpoint — 2026-10-20

## English

`tools/offline-ci-regression.py` runs four bounded QEMU checks from one disposable output directory. The release-candidate workflow invokes it in a separate macOS step after the existing multi-controller smoke. It builds an r2 C sample, requires split keyboard/mouse/storage input with all three controllers stopped, and injects a stop failure that must quarantine the arena. It then builds the partial-metadata SETTINGS probe and requires its 4096-byte-sector BOT interruption, before/after sector checks and native `fsck_msdos -n` classification. Last, a LAN-off diagnostic EFI must still boot, handle keyboard exit, access storage/files, and report the disabled network state. Bluetooth is unchanged.

The local run at `/private/tmp/musha-offline-ci-regression-20261020-rust199/` passed all four actual QEMU cases using Rust 1.99.0 and QEMU 11.1.2. The FAT checker returned 0 before and 206 after the intentional partial transfer, as required; this is recovery-needed evidence, not a clean filesystem claim. `timings.json` records 78.9 seconds total on a warm local toolchain: sample build 2.0, split input 23.6, split stop failure 21.2, metadata build 12.8, partial metadata 4.8, LAN-off build 8.8, and LAN-off boot 5.7 seconds. A cold CI runner can take longer; the existing macOS job has a 30-minute limit. The remote workflow has not run for this change.

The local repository included pre-existing owner changes during this run. The result verifies the current working source and disposable QEMU images; it is not clean candidate provenance, physical acceptance, or release approval. No USB device was written.

## 日本語

`tools/offline-ci-regression.py` は廃棄可能な出力directoryで4件のQEMU検査を実行します。release-candidate workflowでは既存の複数controller試験後に独立stepで呼び出します。r2 C sampleを生成し、keyboard・mouse・storageが別controllerの場合の入力と3 controller全ての停止、停止失敗時のarena隔離を確認します。次にSETTINGS metadataの4096-byte sector BOT転送中断、変更sectorとmacOS標準`fsck_msdos -n`の分類を確認します。最後にLAN無効の診断EFIで起動、keyboard終了、storage/file読出し、LAN無効表示を確認します。Bluetoothは変更していません。

手元のRust 1.99.0、QEMU 11.1.2で4件の実QEMU検査が合格しました。証拠は上記output directoryにあります。意図的な部分転送の前後でFAT検査statusは0、206となり、修復が必要な状態として扱います。温まったtoolchainで合計78.9秒でした。CIの初回buildはこれより長くなる可能性があり、既存macOS jobは30分制限です。今回の変更でremote workflowはまだ実行していません。

このlocal実行には既存の所有者未commit変更が含まれます。現作業sourceと廃棄可能なQEMU imageの検証であり、clean候補の来歴、実機受入、公開承認ではありません。物理USBへの書込みはしていません。

### Clean-source cross-check / clean sourceでの再確認

A separate local clone at `/private/tmp/musha-offline-ci-clean-source-20261020/` was checked out at pushed commit `a906d0e`; only the new runner was copied into that clone. Its `git status` showed only `?? tools/offline-ci-regression.py`. All four real QEMU cases passed again. Evidence is in `/private/tmp/musha-offline-ci-clean-results-20261020/`; `timings.json` totals 78.1 seconds (2.1, 23.8, 20.9, 12.5, 4.9, 8.5, 5.4 seconds in runner order). The partial-transfer check again returned `fsck_msdos` status 0 before and 206 after. This removes the owner-dirty-source limitation for the local functional result, while remote CI and physical acceptance remain open. The 28 pre-existing owner files still match the preservation audit's current SHA-256 values byte for byte.

別のlocal cloneをpush済みcommit `a906d0e` に固定し、新runnerだけをcopyして4件の実QEMU検査を再実行しました。`git status` に出た変更は `?? tools/offline-ci-regression.py` のみで、全件合格しました。証拠は上記clean results directoryにあり、合計78.1秒です。部分転送の前後で `fsck_msdos` statusは再び0、206でした。このlocal機能結果には所有者の未commit sourceは混ざっていません。remote CIと実機受入は別途必要です。従前の所有者file 28件は保全監査時のSHA-256と全て一致しています。
