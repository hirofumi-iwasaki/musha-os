# Stage4 first implementation checkpoint —2026-10-15

## English

Historical first checkpoint: the later [r2 runtime checkpoint](stage4-r2-0.4.0-progress.md) supersedes the unconnected-runtime status below.

Status: meaningful first vertical slice implemented and locally verified; stage4 is not complete. Changes remain uncommitted on `release/0.4.0`, based on `fc1cce8`; its three approved local commits remain unpushed. [Scope](release-scope-0.4.0.md) and [work plan](work-plan-0.4.0.md) decisions are unchanged. No USB, remote CI, tag, release or formal candidate operation was performed.

### Implemented and verified

- Runnable Rust r1 no_std Hello and start-record, explicit SysV entry/crt0, generated64-byte NOTE, existing linker/W^X plan. Dedicated static SSE2/no-red-zone target; pinned1.99.0 rust-src/core/compiler_builtins rebuild. Core and guest share target options. Bootstrap enables only build-std/JSON-target facilities. Toolchain source component is declared; no built-in softfloat target is substituted.
- Guest-only safe QUERY/LOG/EXIT wrappers, checked result decoding, bounded synchronous text borrow, copied start record. No user/kernel reference escapes or new syscall success stub. Existing canonical r1 schema/generated definitions/NOTE acceptance/dispatcher are unchanged.
- `sdk/tools/build-rust.py` builds two ELF samples and app-checks them, records toolchain/target/lock hashes. `tools/check-sdk.py --rust-guest` includes the custom build alongside existing C conformance. Separate development C/Rust archives remain; Rust stays development-marked with its release gate closed until stage4 acceptance. An extracted Rust archive rebuilt from an independent path containing spaces produced byte-identical samples; the C archive excludes Rust source/target/builder.
- Proposed r2 values and48-byte explicit LE common record in [ABI ledger](application-abi.md); bounded64-entry queue/drop/overflow and WAIT decision model in `platform/src/user_io.rs`. These are unconnected pure models, not a new supported ABI. Four tests cover record bytes/reserved zeros, keyboard/gamepad bounds, relative mouse, overflow/FIFO and wait0/deadline/input priority.

### Evidence

Evidence root: `out/stage4-040/`. Host6 unit tests passed: user_io4, Rust wrapper/result/start validation2. The SDK checker also ran its native C layout executable and26 malformed-ELF rejection cases (separate categories, not QEMU counts). QEMU5 final cases passed: Rust Hello/start-record2, existing C1 normal1, C2 ABI1, normal C FAT32 external Hello1. These are representative regression checks, not full QEMU regression.

| Evidence | Result |
|---|---|
| `check-sdk/report.json` | Existing C conformance/26 rejections, rebuilt Rust core/ELFs |
| `rust-qemu-final/report.json` | Both Rust guests query/log/exit successfully; start arena/stack four values match |
| `c1/report.json`, `c2/report.json`, `c-normal/report.json` | Existing execution paths pass |
| `relocated/build result/report.json` | Standalone source build in path with spaces; sample hashes match repository build |
| `verification.json` | Aggregated counts, hashes and limitations |

Hello SHA-256: `8a640a1906a14073a9a8ea7a48857ee576026ac9e634b44e746abd8ae7ee04ae`; start-record: `146d3b5852eec04011a4d2fb8acbff23e8aadb44b1b0c1d33411a96c954d070b`.

The initial build required the pinned compiler's `-Zjson-target-spec` flag; the builder now supplies it. Initial start-record QEMU assertion expected lowercase hexadecimal but the guest correctly emitted uppercase and exited0. Only the expectation changed, and both final guests passed. No kernel execution failure was observed.

### Remaining stage4 work

Promote the r2 proposal coherently across schema/generator/NOTE/features/permissions/loader/SDK. Implement actual monotonic TIME, WAIT0/timeout/input wake with safe runtime USB progress, checked writable supervisor-alias copyout and validate-before-pop INPUT. Define final timeout/wake/generation semantics and state reconciliation after overflow; lost clicks/motion are not reconstructed by the queue. No new syscall numbers are accepted yet.

Connect keyboard up/down/usages/modifiers, USB relative boot mouse/buttons, descriptor-specific wheel and fixtures, then gamepad event consumers. Separate QEMU usb-mouse from absolute usb-tablet and test permissions, bad buffers, empty/overflow/error paths. Viewer mouse UI remains excluded; normal viewer return is stage5 and Bluetooth stage6. C↔Rust mixed FP execution, full regression and real NUC5/Ryzen Rust/key/mouse samples are unverified. This checkpoint does not satisfy the complete scope goals8/9 or hardware gates.

## 日本語

初回到達点の記録です。後続[r2実接続記録](stage4-r2-0.4.0-progress.md)が以下のruntime未接続状態を置換します。

状態：最初の縦断実装をローカル検証済み、工程4全体は未完です。`release/0.4.0` の `fc1cce8` を基点に未commit。先行承認済み3commitも未pushのまま。[scope](release-scope-0.4.0.md)・[計画](work-plan-0.4.0.md)の決定を変更せず、USB/remote CI/tag/Release/正式候補操作は行っていません。

### 実装・検証済み

- Rust r1 no_std Hello/start-record、明示SysV entry/crt0、生成64byte NOTE、既存linker/W^Xを接続。専用static SSE2/red-zone無しtargetと固定1.99.0 rust-src/core/compiler_builtins再構築を使い、coreとguestのtarget設定を一致。Bootstrapはbuild-std/JSON target用だけ。toolchainへsource componentを宣言し、既成softfloat targetで代替しません。
- guest専用安全QUERY/LOG/EXIT wrapper、結果検査、上限付き同期text借用、起動recordコピー。user/kernel参照をcall外へ返さず、新call成功stubもありません。既存r1 schema/生成定義/NOTE受入/dispatcherは不変です。
- build-rust.pyがELF2本を生成・app-check検証し、compiler/target/lock hashを記録。check-sdk.py --rust-guestで既存C検査と統合。開発C/Rust archiveを分け、工程4受入までRust公開gateは閉じたままです。Rust archiveを空白付き別pathへ展開しcoreから再buildした2本がbyte一致。C archiveへRust source/target/builderは入りません。
- [ABI文書](application-abi.md)とuser_io.rsへr2番号候補・48byte LE共通record、64件queue/drop/overflowとWAIT状態modelを追加。未接続の純粋modelで、対応済みABIではありません。4試験でwire/予約ゼロ・keyboard/gamepad上限・相対mouse・overflow/FIFO・wait0/期限/input優先を検査しました。

### 証拠

根拠はout/stage4-040/。host unit計6件（user_io4、Rust wrapper/result/start2）が合格。SDK checkerは別分類でnative C layoutと異常ELF拒否26件も実行。最終QEMUはRust2・C1 normal1・C2 ABI1・通常C FAT32 Hello1の計5case合格です。代表回帰であり全QEMUではありません。英語節の証拠表・SHA-256と同じ内容を参照してください。

初buildは固定compilerが要求する-Zjson-target-spec不足で拒否されたためbuilderへ追加。初start-record試験は小文字16進期待とguest大文字出力の差でassert失敗しましたが、guestは当初からexit0。期待だけを修正し最終2本合格。kernel実行失敗は観測していません。

### 工程4残件

r2をschema/generator/NOTE/feature/permission/loader/SDKへ同時昇格し、実TIME・WAIT0/timeout/input wake・安全なruntime USB進行・書込み可能supervisor alias経由copyout・取出し前検査INPUTを接続する必要があります。timeout/wake/generationとoverflow後状態reconcileは未固定。queueは失われたclick/移動を復元しません。現kernelは新call番号を受け入れません。

keyboard down/up/usage/modifier、USB相対boot mouse/button、descriptor別wheel fixture、gamepad consumerは未接続。usb-mouseと絶対usb-tabletを分け、権限/異常buffer/empty/overflow/errorを試験する必要があります。viewer mouse UIは対象外、通常viewer復帰は工程5、Bluetoothは工程6。C/Rust混在FP実行・全回帰・NUC5/Ryzen実機Rust/key/mouse sampleも未実施。目標8/9全体と実機gateの完了主張ではありません。
