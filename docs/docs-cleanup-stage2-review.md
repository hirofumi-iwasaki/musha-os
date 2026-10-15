# Stage2 cleanup review / 文書整理第2段review

Status: current (2026-10-15). Owner approved the listed moves after initial review rejection. 18 moves completed; 9 protected README-reference moves held. No merges executed.

Automatic approval review rejected the proposed bulk cleanup (script command: `python3 /private/tmp/musha-cleanup.py`): many document moves/deletions plus broad link rewrites risk historical records/concurrent work, and approval did not clearly cover the concrete bulk change. The rejected tool invocation never created its script or ran it. Existing original paths remain present; no docs/history directory was created by this task. The proposal below awaits specific approval; it is not split into smaller move batches to bypass the rejection.

自動承認reviewは多数文書移動/削除・広範リンク書換が過去記録/並行作業に影響し、具体的一括変更の承認が不明として拒否。拒否された呼出しはスクリプト作成から未実行。元pathは現存し、本作業でhistoryは作成していません。以下は承認用の具体案で、拒否の迂回実行はしません。

## Exact proposed moves / 提案移動一覧

Historic prose stays byte-identical except relative Markdown link destinations. SHA256 below identifies each complete pre-move file; after move verify normalized text with only reviewed target substitutions removed equals the original. Keep before/after manifest and exact patch for review. / 過去本文は相対Markdownリンク先以外をbyte保持。以下SHA256を基準に、承認済リンク置換のみを除いた本文一致を検査し、前後manifest/patchを保存します。

| Old → proposed new | Original SHA256 | Protected references: hold until separately allowed |
|---|---|---|
| `docs/pci-h14-recovery-plan.md` → `docs/history/pci-h14-recovery-plan.md` | `73e20c3763ba66abf75e30057fd8ccdfa2bab6216ce686bc59bb8a41d9074ab2` | none in protected list |
| `docs/pci-h15-resource-evidence-plan.md` → `docs/history/pci-h15-resource-evidence-plan.md` | `3e902d2df04a07727330ceb9a2d6045fc051882727f38725d5361f2b92984e1e` | none in protected list |
| `docs/pci-h16-resource-parser-plan.md` → `docs/history/pci-h16-resource-parser-plan.md` | `29a4de6f2993e2a1bd34a042575bbb2b751a78067cc484c1a38311f1ad127672` | none in protected list |
| `docs/pci-h17-bar-consistency-inspection-plan.md` → `docs/history/pci-h17-bar-consistency-inspection-plan.md` | `c7a6dc1800b26a4833da89cf9cc5163a0a40be640b6efde160eff064c2a24014` | none in protected list |
| `docs/pci-h17-hardware-results.md` → `docs/history/pci-h17-hardware-results.md` | `fed373ab499d6132a84893c86040c6452cde362d57c7b39c682752d94db1111e` | none in protected list |
| `docs/pci-h18-bar-index-plan.md` → `docs/history/pci-h18-bar-index-plan.md` | `eb90a29bf593e57b7d816b911c49f8a77b0deca75fc16f80fd8026bb53aa385e` | none in protected list |
| `docs/pci-h18-hardware-results.md` → `docs/history/pci-h18-hardware-results.md` | `a533b4e74a8b765db520c1b96b64ae60bd1c18ef112eed70bbe464208c9d3c58` | none in protected list |
| `docs/pci-h19-peer-resource-plan.md` → `docs/history/pci-h19-peer-resource-plan.md` | `f30ac93340f774ab1af5c518f348e74a41434cc66d5943d8bfce95b5cec7b072` | none in protected list |
| `docs/pci-h19-hardware-results.md` → `docs/history/pci-h19-hardware-results.md` | `9266f38911ef794825225b3da829a521e5a9c93ecd82434b58096813da5dc1df` | none in protected list |
| `docs/ryzen-h4-hardware-results.md` → `docs/history/ryzen-h4-hardware-results.md` | `0e601cf7e28fd6afb17c2a7b8f855fc883e4f8b278312725f76ba50b54be425e` | README.md |
| `docs/mac-memory-diagnostics.md` → `docs/history/mac-memory-diagnostics.md` | `f41dcf090a8ba3e40c04f636488ad90d068c535f2b78e12669b8f991912e09e6` | README.md |
| `docs/mac-usb-next-steps.md` → `docs/history/mac-usb-next-steps.md` | `f1a7489dfd088e9b42c56769896b6fd66ee8c404c0b9389cce43e336f5be1af9` | none in protected list |
| `docs/usb-h3-diagnostics-plan.md` → `docs/history/usb-h3-diagnostics-plan.md` | `5950ee9787eab7c46b1983f60765b1b9ecf07e46065cbe34cbb2a16046e93833` | README.md |
| `docs/acpi-timer-diagnostics-plan.md` → `docs/history/acpi-timer-diagnostics-plan.md` | `dd0d72149d8c3b50e79940b71c07e189021a1767bf98353b5879ff72a079f2b9` | README.md |
| `docs/mi68-display.md` → `docs/history/mi68-display.md` | `3160cfe88e54c3fffda3c619bad3ea70da2c32960f27ceee61bcfe6a50a91f4d` | none in protected list |
| `docs/mi68-preparation-results.md` → `docs/history/mi68-preparation-results.md` | `74dd09b6433603b8cb0d1cd93a5fb87bc180a3d82f9fd531c283a5773d283069` | README.md |
| `docs/mi68-release-preparation.md` → `docs/history/mi68-release-preparation.md` | `f8d4cbe4988d5153128f6e41b3678c50648f873c8424b0a783dc32778ee49552` | README.md |
| `docs/t2-d1-diagnostics.md` → `docs/history/t2-d1-diagnostics.md` | `13240c43e33802c2158fec011734175dea96ed38dea7542fa5b329f3295c3f92` | none in protected list |
| `docs/t2-host-model.md` → `docs/history/t2-host-model.md` | `36989e61a1e467f336dc31960cd16194f435b4c0a887a5a13f056cfe4008497c` | none in protected list |
| `docs/bce-registration-memory.md` → `docs/history/bce-registration-memory.md` | `c1312bd217a3b63aafc987d58a62d6f7ba00b6f09cd882a957b43a257f284307` | none in protected list |
| `docs/hid-bce-prehardware.md` → `docs/history/hid-bce-prehardware.md` | `c27bd2f0c6857b8a0ca930106b3e841268b854e06c5212f498e0f3a67ac49e63` | none in protected list |
| `docs/input-prehardware-validation.md` → `docs/history/input-prehardware-validation.md` | `2368fa736120fe889f3c2759e202970e177de9ee26a3be1c69478e828202dc9d` | none in protected list |
| `docs/macbook-t2-keyboard-0.2.0-research.md` → `docs/history/macbook-t2-keyboard-0.2.0-research.md` | `ca5b4edccafde2ecc84e693194f5506c28197b55226c2b52e7ea0d241d2a5d42` | none in protected list |
| `docs/pre-hardware-work-plan.md` → `docs/history/pre-hardware-work-plan.md` | `d1238fc794d8c3dbcf001f8bf4aeefdf6cb77ef4c3cd510308f2b9612a8c130c` | README.md |
| `docs/distribution-validation.md` → `docs/history/distribution-validation.md` | `7c8b89609f7694ce0f41ecd6a6b0e19fc037611c3dc89be3c4e9ba124f1aae95` | none in protected list |
| `docs/usb-controller-diagnosis.md` → `docs/history/usb-controller-diagnosis.md` | `9d63e04ba4b3a78ed7cc7d4cae9990de675c0bb0170886d1689922588e33998f` | README.md |
| `docs/usb-multi-controller-hub-plan.md` → `docs/history/usb-multi-controller-hub-plan.md` | `2ddd91c59dd67e2dffd855dc822a2a4280a4d904d6a768ae44de5fef4e3a825f` | README.md |

## Link update boundary and exclusions / 参照更新範囲・除外

Resolve each local Markdown destination from its original directory before moving, then rewrite links in moved/current docs and developer references to the same destination. Run tools/check-doc-links.py before and after each approved stage, requiring zero missing targets; this checker excludes anchors and remote URLs. Capture reverse-reference list first. No code/runtime/USB/Release body/remote URL is changed. `git mv` and index work require coordination after approval, not during another agent commit.

移動前のdirectoryを基準にlocal Markdownリンクを解決し、移動文書/現行docs/開発参照の相対先だけ補正。前後のリンク検査0欠損と被参照一覧を保存。anchor/remote URL検査は別。code/runtime/USB/Release本文/remote URLは変更しません。git mv/index操作は承認後に並行commitと調整します。

Protected: root README, owner4 scope0.4/scope0.5/no-shell/scenario, active LAN/stage6/third-party/I218 checkpoint docs. Any move requiring their link edits is held. Published scopes/notes/validation/acceptance for0.1–0.3 remain at original paths. Active T2 documents must also receive review before classification as history because they remain implementation prerequisites. No wholesale Status insertion into original hardware records.

保護：root README、owner4、並行編集LAN/stage6/third-party/I218記録。これらのリンク修正を要する移動は留保。公開済0.1–0.3版文書を移動しません。T2は現行実装前提でもありhistory分類を再確認。過去実機本文へStatusを一括挿入しません。

## Merge proposal / 統合案（未実行）

| Sources | Proposed canonical destination | Preservation rule |
|---|---|---|
| application-api.md | application-abi.md | Preserve original as dated history; replace obsolete external-API statements only in current contract after code check. |
| usb-install-linux.md / usb-install-windows.md | distribution-usage.md | Carry platform limitations in English/Japanese; preserve historical originals until all references reviewed. |
| acpi-read-policy.md | acpi-timer.md | Separate current pre-EBS read contract from H9 evidence; retain evidence/hash in history. |
| memory-attribute-policy.md | memory.md | Separate current permissions/capability contract from H7 evidence; retain history. |
| exfat-design/reader/fixtures/stream-file-api/bot-polling.md | exfat.md + history records | Derive current contract from code, preserve original chronological test/results verbatim. No draft supplied as current implementation truth. |

Remaining translations: full chronological hardware-verification-status and any old Japanese-only policy paragraphs; retain originals while adding reviewed English. Existing file-viewer/distribution-usage already have English and Japanese. macOS/build entry points now include English guidance. Full cleanup is incomplete. / 過去実機記録の完全英訳、旧policy日本語段落英訳は残件。既存viewer/usageは二言語、macOS/build入口に英語案内追加済。第2段全完了とはしません。

## Execution checkpoint / 実行到達点

Owner approval received after the rejection above; the earlier rejection remains historical. All 27 original SHA256 values matched before execution. Executed 18 unblocked table moves and corresponding Markdown destinations only. Nine README-reference entries remain at their original locations to preserve root README. Protected files remained SHA256-identical. See [machine-readable verification](history-move-verification.json) for exact substitutions and hashes. No Git index/commit/push or merge was performed. / 拒否後に所有者承認を受領し、元27hash一致後、保護参照なし18件のみ移動。README参照9件は原位置維持。保護file hash不変、統合・index/commit/pushなし。
