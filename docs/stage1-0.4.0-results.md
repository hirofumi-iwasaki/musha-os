# v0.4.0 stage1 implementation checkpoint / 工程1実装の区切り

## English

Status: uncommitted development implementation, 2026-10-15; based on pushed0f81c4839143cc5bf133e716e65d22da5965d52c. This is software verification, not release or hardware acceptance. The [scope](release-scope-0.4.0.md) and [plan](work-plan-0.4.0.md) remain authoritative. No new implementation commit/push, tag, Release, physical USB write or published-asset replacement was performed.

### Implemented

Goals1/2/3/11 software: schema3 ZIP-comment metadata with65,535-byte bound/strict fields/duplicate-key rejection; v-free asset names and independent build-manifest; empty SETTINGS plus EFI/DOCS/README root and four user documents/licenses; deterministic nested FAT32/VFAT image; build-derived README length/FNV validated on FAT32/exFAT. Optional updater uses trusted prior-package hashes, refuses unknown/modified managed and obsolete files, backs up off-volume, writes no state, preserves SETTINGS contents and removes only known empty obsolete directory shells. Parent symlinks/case aliases/device changes/insufficient space are tested.

prepare-release/build-usb/image builders, package/draft/compare consumers and workflow are aligned. Workflow no longer requires candidate.tar or an unpacked candidate bundle; it decompresses the QEMU asset and generates exFAT locally. C/Rust archives are separate; Rust is explicitly a source scaffold. release=True and draft attachment reject incomplete SDK/sample gates. SDK sample hooks exist, but complete C/Rust sample payloads are not yet demonstrated. Optional docs.zip needs goal17/index.

Owner addition2026-10-15: image-embedded documentation/license changes are outside condition2 and require renewed acceptance. Full image/member comparison stays mandatory. A classifier regression accepts image-external docs only when simulated already-validated complete SDK/runtime inventories are identical; this is not Rust execution evidence. Embedded-doc changes are independently shown to change image bytes and fail runtime equivalence.

README retains its v0.3.0 banner/download links; only a development-asset note was added, without a new claimed USB installation procedure. Current user pages are English then Japanese; the dated Japanese viewer checkpoint is retained verbatim beneath its new English explanation. Original distribution usage through v0.3.0 was copied byte-for-byte to [historical usage](distribution-usage-through-0.3.0.md). Hardware/handoff records are unchanged.

### Verified and evidence

- Rust workspace host274 tests, all pass. Initial legacy-fixture import/chain compatibility failures and one old lowercase marker test were fixed and the full host suite rerun.
- Python52 tests, all pass: distribution23, package4, comparison5, image4, bundle1, exFAT7, QEMU harness8. Retained version/flags/dirty/provenance, workflow privilege, bounded image I/O, published-draft immutability/tag/hash/API-reuse checks alongside new contract tests. API mocks isolate attachment rules; the separate real scaffold gate prevents any remote call.
- Local debug, viewer-probe and normal EFI builds pass. Existing unused hold_cycle warning remains. C SDK compile/conformance and normal QEMU C Hello pass.
- Nine QEMU cases pass: FAT32 marker diagnostics, exFAT/FAT32 probe viewers, normal FAT32/exFAT pixel matches with debug0, normal C Hello, marker content/length tamper and missing marker. Final-source normal FAT32 was checked again.
- New probe pixel references were captured using deterministic fixtures after their page hashes/known bytes passed, then normal-mode State::Absent references were derived by the existing viewer-reference tool. Old references were preserved. This establishes normal/probe agreement for the new one-root-file layout, not equality with historical two-file screenshots, and pixel equality alone is not DMA proof.
- Virtual mounted FAT32 make/deploy/update passes BasicData and ESP: GPT unchanged, known obsolete marker backed up/removed, only modified DOCS file replaced, EFI timestamp preserved, second update no changes, no USB state. Generator image also passes host fsck_msdos -n.
- Two fixture packagings from identical inputs produce identical complete development asset sets. This is not a formal two-clean-build candidate reproduction.

Local evidence is under out/stage1-040/: verification.json; host-tests.log; *-tests.log; debug/viewer/normal build logs; qemu-fat32/debug.log; viewer-fat32 and viewer-exfat frames/logs; normal-fat32/normal-exfat/normal-final-fat32/result.json; hello-normal/report.json; marker-negative/report.json; virtual-roundtrip/report.json; sdk-check/report.json. These local generated files are not committed artifacts.

### Remaining gates

No physical hardware, formatter six-run gate, full existing QEMU matrix, GitHub Actions execution, formal candidate generation or formal two-clean-build reproduction was performed. Goals4–10/12–17 implementation and hardware conditions remain later stages. In particular: OS stays read-only, Rust/r2 is not runnable, settings/log/display and viewer return/pairing are not implemented. Do not treat goal1/2/3/11 release acceptance as complete: exact-candidate hardware checks, complete SDK payloads and formal provenance conditions still need acceptance. No decision is pending for the image-doc policy; it is recorded in scope/plan/policy/usage.

Intentional old names remain in obsolete migration tests/catalogs, generic FS legacy parser fixtures, historical documents and the dated migrate-marker-reference.py utility (not used for v0.4.0). They are not current distribution consumers.

## 日本語

状態：2026-10-15の未コミット開発実装。push済0f81c48が基点。ソフトウェア検証であり公開・実機受入ではありません。[範囲](release-scope-0.4.0.md)と[計画](work-plan-0.4.0.md)を正とします。新実装commit/push/tag/Release/物理USB/公開物置換は無し。

### 実装した範囲

目標1/2/3/11のsoftware：schema3 ZIP comment（上限65,535、厳密field/重複key拒否）、v無しassetと別build-manifest、空SETTINGS/EFI/DOCS/READMEと利用者4文書/license、決定的FAT32/VFAT、生成README長/FNVのFAT32/exFAT検証。任意updaterは信頼前版ZIPのhashを使い未知/変更済の管理・廃止fileを拒否、host backup、状態file無し、SETTINGS内容保持、既知の空旧directoryだけ削除。親symlink/case alias/device変化/容量不足も試験。

prepare-release/build-usb/image/SDK/draft/compare/workflowを整合。candidate tar/unpacked bundle依存を除去しQEMU asset解凍＋exFATローカル生成。C/Rust別archive、Rustはsource scaffold明記。release=Trueとdraftは未完成SDK/sampleで拒否。sample hookはあるがC/Rust全sample payload実証は後工程。docs.zipは17/index後。

所有者追加判断で像内文書/licenseは条件2外・再受入、全image/member比較保持。分類試験の「完成SDK/runtime一致」は検証済inventoryの模擬で、Rust実行証拠ではない。像内文書変更でimageが変わりruntime比較が失敗することも別試験。

README v0.3.0バナー/公開リンクを保持し開発名注記だけ追加。簡易USBの新しい実績付き手順は追加していません。利用者文書は英→日、viewerの当時の日本語checkpointは原文保持。旧配布利用手順は[履歴](distribution-usage-through-0.3.0.md)へbyte同一保存。hardware/handoff履歴不変。

### 検証と証拠

Rust host274件全合格。legacy fixture import/chainと旧小文字markerの初回不具合は修正後全再実行。Python52件全合格（distribution23/package4/comparison5/image4/bundle1/exFAT7/harness8）。版/flags/dirty/来歴・workflow権限・大容量I/O境界・公開draft不変/tag/hash/reuseも保持。API mockは添付規則の独立試験で、別の実scaffold gateがremote前に止めます。

debug/viewer/通常EFI build合格、既存hold_cycle未使用warning残存。C SDK compile/conformanceと通常QEMU Hello合格。QEMU9件（FAT32診断、exFAT/FAT32 viewer、通常両FS画素一致/debug0、C Hello、marker内容/長さ/欠落）合格、最終source通常FAT32も再確認。決定的fixture/既知byte・pagehash確認後の新probe参照から通常State::Absent参照を既存toolで導出し、旧参照保持。旧2file画面との一致は主張せず、画素だけでDMA安全性も証明しません。

仮想mount BasicData/ESP更新2件合格：GPT保持、旧marker退避削除、DOCS変更1fileのみ、EFI時刻保持、2回目変更無し、状態file無し。生成FAT32はhost fsck_msdos -nも正常。同一入力fixture2梱包で全開発asset一致、正式clean2buildの証明ではありません。

証拠はout/stage1-040/verification.json、host-tests.log、各*-tests.log、build log、QEMU各log/frame/result、hello/marker-negative/virtual-roundtrip report、sdk-check/report.json。生成物はcommitしません。

### 未実施と後工程

実機/formatter6回/既存全QEMU matrix/Actions/正式候補生成/正式clean2buildは未実施。目標4–10/12–17は後工程。OS書込み/Rust/r2/設定/log/display/viewer復帰/pairingは未実装。目標1/2/3/11もrelease受入完了とは扱わず、正確な候補実機・完成SDKpayload・正式来歴条件を後で受け入れます。像内文書方針の未決定はなくscope/plan/policy/usageへ記録済み。

旧名は移行catalog/試験、汎用FS legacy fixture、歴史文書、日付付きmigrate-marker-reference.py（v0.4.0未使用）に意図して残しています。現配布consumerではありません。
