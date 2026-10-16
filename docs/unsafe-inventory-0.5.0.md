# Unsafe inventory / unsafe 棚卸し — v0.5.0 preparation

Status: research snapshot (2026-10-16), code unchanged. Source HEAD `2c3598175510b6c740e761dbc562574fbd038863`; first-party Rust files have individual hashes in `out/security-survey/inventory.json`. This is an inventory, not a safety certification. Owner policy: [security policy](security-policy.md), [SECURITY.md](../SECURITY.md).

## English

Measured 14 package crates (11 root-workspace + 3 SDK; workspace-only manifests are not crates), 150 `.rs` files. Columns separate `unsafe {}` blocks, `unsafe fn`, `unsafe trait`, `unsafe impl` and unsafe extern blocks. All configurations and source-local unit tests are included; `test-dir` counts blocks in a `tests/` path only. Unsafe attributes such as `unsafe(naked)`/`unsafe(no_mangle)` and extern declarations without an unsafe keyword are not these categories. Macro expansion, generated `OUT_DIR`/`target`, `out/`, `third_party/`, toolchain/vendor trees, project `sources/`, symlinks, C/assembly and dependencies are excluded. Comments, nested block comments, strings/raw strings and character literals are masked before matching. Thus these are lexical source counts, not compiled/AST or per-feature counts.

| Crate | Blocks | test-dir | unsafe fn | trait | impl | extern block | Near SAFETY | SAFETY comments |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| musha-api | 7 | 1 | 1 | 0 | 0 | 0 | 2 | 2 |
| musha-bce | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| musha-boot | 426 | 0 | 62 | 0 | 14 | 2 | 11 | 13 |
| musha-framebuffer | 22 | 0 | 6 | 0 | 0 | 0 | 1 | 1 |
| musha-fs | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| musha-input | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| musha-memory | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| musha-net | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| musha-platform | 2 | 0 | 1 | 0 | 0 | 0 | 0 | 0 |
| musha | 7 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| musha-sys | 1 | 0 | 1 | 0 | 0 | 0 | 0 | 0 |
| musha-samples | 6 | 0 | 4 | 0 | 0 | 1 | 2 | 2 |
| musha-viewer | 1 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| musha-xhci | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| **Total** | **472** | **1** | **75** | **0** | **14** | **3** | **16** | **18** |

“Near SAFETY” means a preceding comment containing literal `SAFETY:` ends within five source lines. It is deliberately only proximity: a shared/irrelevant comment can overcount, and a farther contract, `# Safety` documentation or differently worded invariant can be missed. The other 456 blocks are candidates for review, not 456 proven unsafe defects. Existing explanatory comments and unsafe-function caller contracts need manual association and content review. Reproducer: `python3 out/security-survey/inventory.py > out/security-survey/inventory.json`; the local measurement script is retained, not installed in CI.

**CI proposal only:** a future `tools/check-unsafe-inventory.py` should use a pinned Rust syntax parser, fixed crate/exclusion/config rules and a committed per-crate baseline. Report block/fn/trait/impl/extern deltas separately; fail increases lacking a reviewed reason keyed to path/item, and require baseline/reason changes in the same review. Count comments separately from checking each block's actual invariant. No CI script, baseline enforcement or code comments were changed by this survey.

## 日本語

所有者方針に沿う調査時点の棚卸しです。root workspace 11＋SDK 3の計14 crate、150 Rust fileを対象に、`unsafe {}`、unsafe関数・trait・impl・extern blockを分けて上表へ集計しました。全cfgとsrc内unit testを含み、`test-dir`はパス中の `tests/` だけです。vendor/依存、third_party、generated/OUT_DIR/target、out、toolchain、同期sources、symlink、C/assemblyは対象外。comment・文字列等を除く字句計測であり、macro展開・AST・実際のfeature別binaryの件数ではありません。unsafe属性（naked/no_mangle等）やunsafe指定のないextern宣言も別対象です。

472 blockのうち前5行以内に `SAFETY:` commentがあるものは16、同comment自体は18です。近接する無関係な説明を拾う一方、遠い契約や `# Safety` 等は拾わないため、残456件をそのまま欠陥とは判定しません。各blockとの対応・前提・説明内容を人が確認する必要があります。scriptとfile別hashは上記outに保存しました。既存コード・コメント・CIは変更していません。

CI案は文書内の提案のみです。固定したRust構文parserと対象規則でcrate別基準を作り、block/fn/trait/impl/externの増減を分離し、path/itemに結び付くreview済み理由のない増加を失敗にする方式を検討します。基準変更と理由は同じreviewに含め、件数監視と実際の安全条件の確認を混同しません。
