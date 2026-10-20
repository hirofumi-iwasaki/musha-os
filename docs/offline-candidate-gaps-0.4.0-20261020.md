# v0.4.0 additional offline candidate checks — 2026-10-20

## English

This additional review follows [the earlier offline audit](offline-deep-audit-0.4.0-20261020.md). Its five listed work packages have since been implemented. Bluetooth remains paused, and no hardware acceptance is inferred here. Both checks below are implemented and tested locally; no further commit has been pushed after `1b8b238`.

| Priority | Finding and evidence | Bounded next step |
|---|---|---|
| P1 — provenance condition 2 | `tools/compare-candidates.py::compare` checked allowed diff paths and identical runtime members for `docs-only`, but did not establish that the right commit succeeded the left. Reversed or divergent documentation histories could be reported as a valid successor. The scope and work plan explicitly require a docs-only **successor** commit. | Commit `5de9f87` requires a distinct right commit with `git merge-base --is-ancestor left right`, while retaining the path and runtime checks. A real temporary Git history tests a valid successor, reverse order and a divergent branch. This commit remains local. |
| P2 — candidate image/ZIP consistency | `tools/prepare-release.py::stage` correctly supplies one payload to both the drive ZIP and QEMU image, but the independent candidate reader had checked only EFI identity within each candidate. The separate cross-candidate full-image check could not detect a self-consistent checksum set around a single candidate's mismatched README or DOCS. | `tools/candidate-image.py` now reads the generated candidate's bounded FAT file and directory chains, including nested VFAT DOCS names, and `compare-candidates.py::inventory` compares every managed file byte with the ZIP while requiring the exact initial root and empty SETTINGS. Tests recompute candidate image, manifest and checksum hashes after changing image README, DOCS or SETTINGS and verify rejection. Direct tests also reject a FAT cycle, nonempty SETTINGS and an extra empty DOCS directory. Existing real candidate A/B still pass same-source comparison. This implementation remains uncommitted. |

Realtek admission still requires measured TxConfig/subsystem/BAR/command and a reviewed silicon profile; `net/src/realtek.rs::admit` intentionally rejects all devices. I218 software NVM configuration requires measured OEM values. The T2 boot adapter requires observed BAR ownership, commands and DMA behavior. These are hardware and owner-decision gates, not safe offline implementation tasks. The selected-source reproducibility build and xHCI stop check are being handled separately and are not repeated here.

## 日本語

この追加確認は[先行オフライン監査](offline-deep-audit-0.4.0-20261020.md)に続くものです。そこに挙げた5件は実装済みです。Bluetoothは保留を維持し、ここで実機受入を認定しません。下表の両項目はローカルで実装・試験済みです。`1b8b238`以降のcommitはpushしていません。

| 優先 | 発見事項と根拠 | 限定した次の作業 |
|---|---|---|
| P1 — 来歴条件2 | `tools/compare-candidates.py::compare` の `docs-only` 判定は変更pathとruntime各要素の一致を調べていましたが、右側commitが左側の後続であることを確認していませんでした。逆順や分岐した文書履歴でも後続として通り得ます。scopeと工程計画は文書のみの**後続commit**を要求します。 | ローカルcommit `5de9f87` で、異なる右側commitに `git merge-base --is-ancestor left right` を要求し、既存のpath/runtime条件を維持しました。一時Git履歴で正しい後続の合格、逆順と分岐の拒否を確認しました。このcommitは未pushです。 |
| P2 — 候補imageとZIPの整合 | `tools/prepare-release.py::stage` は同一payloadからdrive ZIPとQEMU imageを生成しますが、独立した候補検査は候補内のEFIだけを照合していました。候補間のimage全体比較では、単一候補内でREADMEやDOCSが食い違い、hash類だけ整合する場合を検出できません。 | `tools/candidate-image.py` で生成候補専用の上限付きFAT読出しを追加し、階層化されたVFAT名を含む管理対象の全fileをZIPとbyte単位で比較します。初期rootの完全構成と空SETTINGSを要求します。imageのREADME・DOCS・SETTINGSを変えてmanifest・checksumを再計算したcaseの拒否、FAT循環・非空SETTINGS・余分な空DOCS directoryの拒否を試験しました。実候補A/Bのsame-source比較は合格しました。この実装は未commitです。 |

Realtek許可には実測TxConfig/subsystem/BAR/commandとreview済みsilicon profileが必要で、`net/src/realtek.rs::admit` は全機器を意図的に拒否します。I218のsoftware NVM設定には実測OEM値、T2 boot adapterにはBAR所有権・command・DMA挙動の観測が必要です。これらは実機と所有者判断のgateであり、安全に進められるオフライン実装ではありません。選定sourceの再現性buildとxHCI停止検査は別担当で進めており、ここには重複掲載しません。
