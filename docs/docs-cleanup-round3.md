# Documentation cleanup round3 / 文書整理第3回

Status: current (2026-10-15). Documentation changes only; release/hardware gates remain pending.

## English

Completed the approved stage2 index, Status metadata (77 formerly unlabelled pages), four consolidation groups (external API; Linux/Windows; ACPI; memory attributes), and an exFAT current contract plus five preserved development histories. Ten former source pages now redirect to canonical contracts and original history, keeping old references valid. Dates and test counts in history remain their original-stage evidence. Added PCI/NUC5 diagnostic discovery links. NUC5 capture remains a dated record, not a current acceptance contract.

The six prioritized user entry points have English/Japanese current guidance. Hardware status now has an English chronological explanation while its complete original body remains byte-identical. The historical Japanese policy archive remains original, while the current distribution contract is bilingual. The complete chronological historical archive is not newly translated in this pass.

Deferred by explicit boundaries: root README rewrite/navigation and formatter claims await the six-run formatted-media gate; nine README-referenced historical moves remain protected at their original paths; owner4 already has Status and its decisions remain untouched; concurrent LAN/Bluetooth/T2 documentation is maintained by its implementation owner. New0.4 release/acceptance worksheets are deliberately separate and uncommitted. Their final releases/0.4.0 location needs owner review while the requested entry paths remain in use. No docs.zip is qualified: --docs-complete is an explicit owner acceptance assertion, and index existence alone does not close these gates.

[Verification manifest](cleanup-round3-verification.json) records original hashes, metadata-only checks and integration link substitutions. Local Markdown links and diff whitespace checks pass. This documentation commit carries no runtime changes or physical results. Embedded user-document edits can change future generated image bytes, so accepted-runtime condition2 must be reevaluated rather than assumed.

## 日本語

承認済第2段の索引・Status不足77ページ・4統合群（外部API、Linux/Windows、ACPI、memory属性）、exFAT現行正本＋5履歴を整備。元10ページは現行/履歴への互換入口として残し既存参照を維持。歴史本文の日付/試験数は当時のまま。PCI/NUC5診断導線を追加しNUC5採取は過去記録として維持。

優先6利用者入口に現行英日案内を用意。実機状況には年代別英語説明を追加し、元本文全体はbyte保持。過去policy日本語原文は保存し現行契約を二言語化。過去archive全編の新規英訳は本作業では行っていません。

留保：root README全面改稿/索引化/formatter表明は6run実機gate後。README参照9件の移動は保護により原位置。owner4は既存Statusがあり決定本文不変。並行LAN/BT/T2文書は実装担当が管理。新0.4公開/受入骨組みは別途未commitで、指定path使用中のreleases/0.4.0移行は要review。docs.zipは未認定で、--docs-completeは所有者受入判断、索引の存在だけで全gate完了にしません。

[検証manifest](cleanup-round3-verification.json)へ元hash/metadata一致/統合link置換を記録。Markdownリンク/差分空白検査合格。文書commitにruntime/実機結果を含めません。像へ入る利用文書変更は将来image bytesを変え得るため条件2を再評価します。

Follow-up: handoff14 and current5 metadata were added without body changes; original SDK reference5 are classified by index and preserved byte-for-byte. / 追補：handoff14/current5に本文不変metadata追加、原典SDK5は索引分類でbytes保持。
