# Boot/API integration 3.0 — local validation and remaining gates

## English

Latest local evidence: [A](handoff/0.3.0-release-a-results.md), [C SDK](handoff/0.3.0-release-c-results.md), [development hardware observations](hardware-acceptance-0.3.0.md), [deferred issues](open-issues-0.3.0.md). Candidate hardware cold boots remain NOT PERFORMED.

Scope: A/B on `release/0.3.0`, starting HEAD `322ee6754b2092f73c663ac39b483a5a3c8b9093`, initially clean. Existing v0.2.0 implementation and branch retained. No commit/push/tag/Release/remote setting/physical USB/PDF operation. Local workflow editing was separately confirmed by the owner after automatic approval review initially rejected it; workflow execution was not requested or performed.

### Completed local scope

- A: typed readiness/app/service states; explicit record versus row-diff rendering; OS-owned cooperative runtime; actual mounted API before FILE_VIEWER readiness; recovery display profile; first software frame separately timed; normal RAM/file-content diagnostic dependency and eight-second paging removed.
- B: section-12 decision ledger; generated C/Rust start and NOTE records, status/number/limit candidates; C/Rust raw wrappers; crt0/static linker/C ELF; unsafe/raw versus safe pure-validation Rust layers; target/CMake/sample skeletons; host ELF checker; compile/conformance tooling. No ABI compatibility is publicly fixed.
- README and existing design references updated. Supplied policy 3.0 and handoff retained byte-for-byte with verified SHA256SUMS. Local Actions definition now includes SDK paths and compile/conformance check, not deployment or guest claims.

### Tests and evidence

| Check | Result / evidence |
|---|---|
| Host workspace excluding boot | 228 passed, including readiness ordering/headless and actual boot diagnostic recording/overflow/dirty-row/small-framebuffer tests |
| Diagnostic OFF/detail host variants | passed |
| C/Rust public layouts | start and NOTE 64 bytes/alignment 8/all offsets; C native execution plus x64 C/Rust compile |
| SDK C static ELF | generated and accepted; 3 LOAD segments |
| Malformed ELF/NOTE | 26 cases rejected, including duplicate/missing NOTE, unknown profile/flags, W^X/overlap/range/alignment |
| Syscall wrapper | generated assembly confirms RCX→R10; RDX input/output constraint and result decoding checked; no SYSCALL execution |
| Rust pure validation | 2 host tests; invalid status/reserved/overflow/overlap rejected |
| Rust samples | UEFI x64 compilation/explicit SysV LLVM IR only; no Rust user ELF |
| Relocated SDK | compiled in a separate path containing spaces |
| QEMU FAT32 same controller | key navigation/content hashes/read-only images/DMA cleanup passed |
| QEMU exFAT split controllers | navigation/multiple pages/content hashes/read-only/DMA cleanup passed |
| QEMU cancellation/disconnect/timeout | retained storage + keyboard; pending request cleanup/DMA stop passed |
| QEMU diagnostic OFF/detail | same FAT32 operations and monotonic readiness milestones passed |
| Normal no-feature EFI | list/open/back pixels match probe reference; zero debug bytes; image unchanged |
| Cooperative networking/input | traffic + keyboard absent + five-level hub passed; ARP/ICMP/257 UDP echoes, ring wraps, checksums/DMA stop |
| App error injection | both DMA controllers stopped before halt |
| Source hygiene | cargo fmt and git diff --check |

Host toolchain: Rust 1.99.0 (b940084d7), Apple Clang 21.0.0, bundled LLD 23.1.1. Exact SDK commands/versions: `out/sdk-check/report.json`; generated sample and IR alongside it. QEMU logs/screenshots: `out/qemu-viewer-*`, `out/qemu-cooperative-*`; normal pixels: `out/v3-normal-final-pixels/result.json`. These are local ignored outputs, not release artifacts.

Representative commands (with project-local Rust toolchain on PATH):

```sh
cargo test --locked --workspace --exclude musha-boot
cargo test --locked -p musha-platform --features diagnostics-off
cargo test --locked -p musha-platform --features diagnostics-detail
cargo fmt --all -- --check
python3 tools/check-sdk.py
python3 tools/check-sdk.py --sdk '/private/tmp/musha sdk relocated v3' --output out/sdk-relocated
cargo build --locked --release --target x86_64-unknown-uefi -p musha-boot --features file-viewer-probe
python3 tools/make-usb-image.py out/v3-verified.img --efi target/x86_64-unknown-uefi/release/musha-boot.efi
python3 tools/smoke-viewer.py --firmware-dir /opt/homebrew/share/qemu --boot-image out/v3-verified.img --data-image out/v3-verified.img --format fat32
python3 tools/smoke-viewer.py --firmware-dir /opt/homebrew/share/qemu --boot-image out/v3-verified.img --data-image out/v3-exfat.img --split-controller
```

Additional probe builds used `file-viewer-probe,live-file-probe` for cancel/disconnect and `file-viewer-probe,file-app-timeout` for timeout; `smoke-viewer.py --case` selects each case. `qemu-debug` with `smoke-cooperative.py --case traffic|no-keyboard` and `--hub-depth 5` checks retained legacy paths; `app-step-error` with `--case app-error` checks failure cleanup. All image operations targeted ordinary files under out, never devices.

The first FAT32 trial used the separate long-file fixture against a test expecting the 16-byte bundled file; it failed and was rerun with the correct fixture. An unoptimized debug-profile EFI compiled but raised QEMU vector 8 during filesystem discovery; unoptimized runtime support is **not verified**. Existing release-profile builds passed; no claim that the debug failure was fixed. An early disconnect assertion ran before final boot times were emitted; the test now waits for session completion. These failures are not counted as passes.

### Remaining A/B qualifications

A is the static/cooperative transition, not independent app scheduling. Record formatting still uses bounded existing text records; a fully numeric external report schema is future work. PM multi-wrap stalls/early EFI time, deliberately slowed physical displays, mandatory-clock failure on hardware, and physical USB/hardware behavior were not verified. Diagnostic row guards are host tests, not actual small-GOP hardware. Runtime raw-pointer/DMA invariants remain under the existing BSP/interrupt-disabled assumptions.

B is a foundation: custom Rust target JSON inspected, but core/alloc not rebuilt; Rust ELF/link/run and C↔Rust FP execution remain open. CMake is an untested skeleton. Host ELF validation is not the kernel loader, and neither memory mapping nor BSS zeroing is executed by B. No FPU/privilege safety is implied by compilation. CI file edited locally; remote Actions were not run. Deployment/release tooling and published binaries are unchanged.

### C: external Ring 3 Hello gate

C1 update: the [opt-in fixed assembly probe](ring3-prototype.md) implements and tests the privilege transition. It does not complete the following SDK/external-app acceptance gate; A/B results above remain historical.

1. Pin CPU manual revisions and toolchain/rust-src; settle selectors/MSRs, return-frame and exception/NMI/IST design; rebuild core/alloc for user target.
2. Implement kernel ELF/NOTE validation, disjoint user mappings, RO start record, BSS zeroing and stack guards.
3. Implement reviewed stack-switch/ABI bridge/syscall/IRETQ, fault-safe copy and integer/raw error conventions; no successful stub.
4. Run C and Rust Hello with first user syscall proving APP_RUNNING; invalid ELF/pointer/length/return tests; raw GPR and FPU/SSE preservation; C↔Rust integer/double/mixed-struct/stack arguments.
5. Separate OS readiness, launch failure, app exit/failure and first software frame. Only then review external compatibility, with all assigned numbers still explicitly experimental until approval.

C2 update (2026-10-13): [embedded SDK C ELF execution](ring3-elf.md) adds kernel ELF/NOTE validation, zeroed BSS/arena, read-only start-info and SDK C query/log/exit. All 14 C2 QEMU cases, 8 C1 regression cases and 233 workspace host tests passed. Normal-viewer list/open/back still matches the prior reference. Rust execution, recovery/rollback and the full C acceptance gate remain open. Evidence: `out/ring3-elf/report.json`, `out/ring3/report.json`, `out/ring3-elf/host-tests.log`, `out/ring3-elf/normal-viewer-pixels/result.json`.

C3 follow-up: [lifecycle/reclamation](ring3-lifecycle.md) adds an opt-in saved kernel continuation, bounded page-table rollback, arena scrubbing, repeated launch and continuation into the existing static application. It is not general scheduling or a stable external lifecycle ABI.

External-file follow-up: [bounded stream loading](external-elf.md) now connects FAT32/exFAT to C2 in the normal build, with [screen log/exit/fault output](external-app.md). [Step 2b](handoff/0.3.0-step2b-results.md) and [step 2c](handoff/0.3.0-step2c-results.md) record evidence, failures/retries, exact EFI hashes and outstanding hardware/publication gates. These extend the historical A/B/C1/C2/C3 results above; they do not complete the broader C/D scope.

### D: practical I/O / SDK gate

Design versioned report and opaque kind/owner/permission/generation handles, bounded wait/events, copied drawing, read-only stream operations and cancel/deadline completion. Reuse existing internal drivers through checked adapters. Prove disconnected/stale/canceled requests cannot reuse live DMA memory; validate input overflow/key release and multi-controller/hub paths. Then package reproducible SDK and perform physical hardware tests. Network/write/thread/POSIX support remains separate scope.

Next approval is required before commit/push, external CI execution if requested, tag/Release/publication, remote settings or physical USB changes. None is necessary for retaining these local edits.

## 日本語

最新のローカル証拠：[A](handoff/0.3.0-release-a-results.md)、[C SDK](handoff/0.3.0-release-c-results.md)、[開発版実機観測](hardware-acceptance-0.3.0.md)、[先送り課題](open-issues-0.3.0.md)。公開候補の実機冷間起動は未実施です。

A/Bのローカル実装を追加しました。ホスト228件、SDK公開型・C ELF・異常ELF/NOTE26件、QEMUのFAT32／exFAT・別controller・取消し／切断／期限・表示モード比較・通常画面・通信／5段ハブ・アプリ障害を確認しました。ブランチとHEADは維持し、commit／push／Release／実USB／PDFは変更していません。

Aは静的リンク・協調実行の移行段階です。Bはコンパイル／整合試験までで、Ring 3実行済みではありません。専用Rust targetのcore/alloc再構築、Rust ELF、C/Rust浮動小数点のguest相互実行、カーネルloader／安全なsyscall／IRETQはCへ残します。実用外部I/OはDです。公開互換性固定済み項目はありません。

追記：C2ではSDK C ELFのRing 3実行・カーネルELF検査・BSS／arena初期化・読出し専用起動情報を実装しました。QEMU 14件、C1回帰8件、ホスト233件が合格しました。上記A/Bの記録とは区別し、Rust実行・回復処理・工程C全体の完了は未達です。

未最適化debug EFIでのvector 8は未解決・未検証事項として残します。合格結果は既存手順と同じrelease構成です。初期のfixture選択誤りと終了待ち不足は修正して再試験しました。実機と遠隔Actionsは未実行です。次のcommit・公開・実USB操作には別途承認が必要です。

外部ファイル読込みの追記：[ストリーム読込み経路](external-elf.md)を通常版へ統合し、[画面ログ・終了・障害表示](external-app.md)を追加しました。[2b](handoff/0.3.0-step2b-results.md)・[2c](handoff/0.3.0-step2c-results.md)に証拠、不合格と再試験、EFIハッシュ、残件を記録しています。実機と公開条件、工程C／D全体の完了を示すものではありません。
