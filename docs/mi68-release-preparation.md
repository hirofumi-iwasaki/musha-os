# MI68 / Musha-OS v0.1.0 release preparation

> 2026-10-10: v0.1.0 is approved as a limited initial development release. [Final scope / 確定した公開範囲](release-scope-0.1.0.md) supersedes the original acceptance gates below. Earlier unreleased statements are historical.

> 最新の実機確認状況（2026-10-10）は[配布資料用の実機確認状況](hardware-verification-status.md)を参照。以下の過去の準備・検証記録は、その記録日時点の内容です。

## English

Created: 2026-10-09 (Japan time). Status: preparation in progress; unreleased.

### Release decision

The current policy and implementation plan require the same USB image to pass
QEMU, NUC5 and NUC8 tests: display, RAM, USB input, FAT32 reads and wired
ARP/ICMP/UDP. Physical testing and NUC networking remain incomplete. Generating
this document or a bundle does not relax those conditions or publish v0.1.0.

An MI68 initial development release could explicitly limit support to verified
functions and environments. Adopting that option requires updating the policy
and implementation plan, with reasons and revised acceptance criteria, before
creating a tag. Otherwise present the development demo and release v0.1.0 later.

### Required work and completion criteria

| Priority | Work | Completion criterion |
|---|---|---|
| First | Define release scope | Document environments, functions, exclusions, experimental APIs and changes to existing criteria |
| Required | Freeze candidate commit | Demo, distribution and tests use the same source; prioritize fixes afterward |
| Required | Clean Mac build | Fresh checkout builds EFI with pinned Rust, Clang and Cargo.lock |
| Required | QEMU regression | Boot, display, RAM, input, FAT32, traffic and major failure cases pass |
| Required for hardware demo | NUC5 tests | Display after ExitBootServices, RAM, own xHCI input and USB reads work |
| Required by current policy | NUC LAN and NUC8 | Verify I218-V/I219-V ARP/ICMP/UDP; document any scope change |
| Required | Distribution | Normal EFI, GPT/FAT32 image, SHA-256, build/boot/safe USB instructions |
| Required | Licensing | Include Apache-2.0 and third-party terms and attribution |
| Required | Documentation | README, release notes, test records, limitations and recovery agree with actual behavior |
| Last | GitHub publication | Tag the tested commit v0.1.0 and attach tested artifacts to its Release |

### Before hardware arrives

- [ ] Confirm release scope and acceptance criteria; existing criteria remain in force.
- [ ] Record candidate commit and tool versions.
- [ ] Build and run host tests from a fresh Mac checkout.
- [ ] Run QEMU normal, idle-input, disconnect, timeout, corrupt-media and exception tests.
- [ ] Restore the normal build; exclude debug, fault, timeout and experimental PHY features.
- [ ] Generate a bundle with tools/prepare-release.py; retain hashes and provenance.
- [ ] Enter actual results in release-notes-0.1.0-draft.md.
- [ ] Check licensing and third-party SHA256SUMS.

### After hardware arrives

- [ ] Record NUC model, RAM, firmware, PCI IDs, GOP, USB and keyboard models.
- [ ] Identify the USB using external/USB status, model, exact capacity and unplug/replug comparison.
- [ ] Test directly connected devices, UEFI boot and Secure Boot disabled.
- [ ] Check RUNTIME READY, RAM, USB READ OK, FAT32 READ OK and key press/release.
- [ ] Photograph before/after Esc, session completion, DMA stop and error lines.
- [ ] Under current criteria, run 10 cold boots, 10 restarts and 30 minutes of integrated input/file/UDP per machine.
- [ ] Retain the shared image SHA-256 and hardware-test-record-template.md results.
- [ ] Unsupported LAN is not successful communication; untested cases stay NOT RUN.

### USB capacity and identification

The 64MiB image is for QEMU. Copying it directly onto a 32GB disk leaves the backup
GPT away from the physical end and fails the current GPT checks. Generate a new
regular image with --size-bytes equal to the measured physical capacity; do not
infer it from the 32GB label. See usb-install-macos.md. Alternatively copy the
normal EFI and MUSHA.TXT onto an existing FAT32 USB as described in test-usb-build.md;
this does not satisfy the shared raw-image acceptance test by itself.

Identify the target on the user's Mac before any destructive operation. Never
write the internal SSD, NAS or an unrelated external drive. Packaging tools do
not write physical disks.

### Event preparation and announcement

Stop adding features the day before the event. Test the demo USB, spare USB, NUC,
power, display cable and keyboard together. Prepare QEMU and hardware recordings
or photos as backups. Defer hubs, mice, gamepads, AHCI/NVMe, audio and broad PC support.

Distinguish implemented, QEMU-verified, hardware-under-test and unimplemented
features. Only after scope confirmation and test results, a limited-release
announcement could say: “Musha-OS v0.1.0 initial development release: verified in
QEMU; NUC hardware validation continues.”

### Work record

Original main: eeba07f950ee6abe7e2fa49d0be99136f3f0538e.
This local integration starts from the USB build commit a2e396b on
release/mi68-preparation. No scope change, main merge, tag, Release publication
or physical write is performed during preparation. See mi68-preparation-results.md.

## 日本語


作成日: 2026-10-09（日本時間）。状態: 準備中、未リリース。

## 公開の判断

現行の `policy-v0.1.md` と `implementation-plan-0.1.0.md` では、同一USB
イメージでQEMU・NUC5・NUC8を検証し、画面・RAM・USB入力・FAT32読出し・
有線LANのARP/ICMP/UDP通信を合格させる。実機未検証とNUC LAN未実装は残件。
この文書や配布物の生成だけで、合格条件を緩和したりv0.1.0を公開したりしない。

MI68に合わせた初期開発版として公開する案は、確認できた環境・機能に範囲を
限定し、未確認の実機とNUC LANを明示する。採用する場合は、タグ作成前に
方針書と実装計画を更新し、変更理由と新しい合格条件を記録する。
間に合わない場合は開発版の発表とデモを行い、正式なv0.1.0公開を後日にする。

## 必須作業と完了条件

| 優先度 | 作業 | 完了条件 |
|---|---|---|
| 最初 | 公開範囲の確定 | 対象環境・機能・未対応・実験APIを明記し、現行条件との差を確定 |
| 必須 | 候補コミット固定 | 展示・配布・試験が同じコード。以後は不具合修正を優先 |
| 必須 | Macクリーンビルド | 新しいcheckoutから固定Rust・Clang・Cargo.lockでEFIを生成 |
| 必須 | QEMU回帰試験 | 起動、描画、RAM、入力、FAT32、通信、主要異常系が候補で合格 |
| 実機展示に必須 | NUC5試験 | UEFI終了後も表示・RAM・独自xHCI入力・USB読出しが動作 |
| 現行条件では必須 | NUC LANとNUC8 | I218-V/I219-VでARP・ICMP・UDPを確認。範囲変更時は明示 |
| 必須 | 配布物 | 通常版EFI、GPT/FAT32イメージ、SHA-256、生成・起動・安全なUSB手順 |
| 必須 | ライセンス | Apache-2.0本体と第三者コードの条件・著作権表示を添付 |
| 必須 | 文書 | README、release notes、試験記録、既知制約、復旧方法が現状と一致 |
| 最後 | GitHub公開 | 検証したcommitにv0.1.0タグ、同じ成果物をReleaseへ添付 |

## 到着前に行うこと

- [ ] 公開範囲と合格条件を確定する（既存条件はまだ維持）。
- [ ] 候補commitとツール版を記録する。
- [ ] Macの新規checkoutでビルドとホスト試験を実施する。
- [ ] QEMUの正常・無入力・切断・timeout・破損媒体・例外試験を実施する。
- [ ] 通常版へ戻す。qemu-debug、fault、timeout、実験PHY probeを配布版へ入れない。
- [ ] `tools/prepare-release.py` で候補配布物を生成し、hashと生成元を保存する。
- [ ] `release-notes-0.1.0-draft.md` に実際の検証結果を記入する。
- [ ] ライセンス表示と第三者ソースのSHA256SUMSを確認する。

## 到着後に行うこと

- [ ] NUC型番、RAM、UEFI版、PCI ID、GOP、USB・キーボード型番を記録する。
- [ ] USBを外付け・型番・実容量・抜き差しで識別してから書き込む。
- [ ] ハブなしの直結USB、Secure Boot無効、UEFI起動で初期検証する。
- [ ] RUNTIME READY、RAM、USB READ OK、FAT32 READ OK、キー押下・解放を確認。
- [ ] Esc前後を撮影し、終了・DMA停止の表示とエラー行を記録する。
- [ ] 現行条件では各機10回のcold boot、10回再起動、30分の入力・file・UDP統合試験。
- [ ] 同一imageのSHA-256と結果を `hardware-test-record-template.md` に基づき保存。
- [ ] LAN driver UNSUPPORTEDを通信成功と扱わない。未試験はNOT RUNのまま記録。

## USB媒体の安全確認とイメージ容量

64MiBイメージはQEMU用。32GB USBへのrawコピーだけでは、GPT副headerが
実媒体末尾に来ず、現在のruntimeのGPT検査を通らない。実機にはUSBの正確な
容量byteと同じ大きさのイメージを `--size-bytes` で新しい通常ファイルへ生成する。
公称「32GB」から容量を推定しない。詳しくは `usb-install-macos.md` を参照。

消去・書込みは利用者のMacで対象を識別してから行う。内部SSD、NAS、別の外付け
媒体へ書き込まない。候補配布ツールは `/dev` を開かず、物理媒体へ書き込まない。

## 前日と当日

前日には機能追加を止め、展示USBと予備USB、NUC、電源、映像ケーブル、
キーボードを一式で確認。QEMUデモと実機録画・写真を予備に用意する。
USBハブ、マウス、ゲームパッド、AHCI/NVMe、音声、幅広いPC対応は後続版へ回す。

発表時は「対応済み」「QEMUのみ確認」「実機検証中」「未実装」を区別する。
実機未確認で範囲を限定した開発版を公開する場合の文言案:

> Musha-OS v0.1.0 初期開発版を公開。QEMUで検証済み、NUC実機検証は継続中。

この文言は範囲変更の確定と試験結果を反映してから使用する。

## 今回の着手記録

元のmain: `eeba07f950ee6abe7e2fa49d0be99136f3f0538e`。
MI68準備は `release/mi68-preparation` で行う。ローカル統合はUSBビルド`a2e396b`から開始した。
方針変更、mainへのmerge、タグ作成、GitHub Release公開、実機への書込みはまだ行わない。
今回の検証結果は `mi68-preparation-results.md` に記録する。
