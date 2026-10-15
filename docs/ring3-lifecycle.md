# Application return and reclamation prototype — C3

Status: current

## English

Opt-in `ring3-lifecycle-probe` extends [C2](ring3-elf.md) with three sequential launches of the same embedded ELF, reclamation after each launch and continuation into the existing PCI/cooperative application path. No default build, published binary or public ABI changes. Values remain candidate r1.

### Implemented scope

An explicit Win64 assembly wrapper saves its kernel continuation stack, nonvolatile GPRs and x87/SSE state outside the app arena. Normal and nonzero exit return a status to this continuation. Selected synchronous CPL3 exceptions (#DE/#UD/#GP/#PF) terminate the app and return to the same continuation; no faulting user instruction resumes. Kernel exceptions, NMI/#MC/#DF and kernel-stack invariant failures remain fatal. The exception entry resets FP control state before executing Rust.

A bounded snapshot of the existing page-table pool restores all prelaunch mappings, ancestor permissions and the allocation watermark, clears new table pages and reloads CR3. This also restores the supervisor-stack guard identity page before wiping the entire reusable arena. TSS.RSP0, segment selectors, syscall/FS/GS MSRs and modified control registers are restored, and loaded-app metadata is cleared. The next launch reuses the same virtual/physical storage. Partial mapping failure follows the same rollback path.

The snapshot is deliberately a **BSP-only transaction**, with interrupts disabled and no concurrent table writers or global-page mappings. Its fixed capacity matches the current 256-page table pool. It is not a general allocator, address-space switcher or scheduler. No Drop-bearing resources may be acquired inside the abandoned launch/syscall/fault frames: the assembly escape does not unwind Rust. This restriction must be redesigned before adding general handles, DMA or concurrent execution.

Private probe records distinguish `OUTCOME=0` (exit, DETAIL is status), `1` (app exception, DETAIL is vector), and `2` (loader/validated-return rejection). These are debug records, not a frozen public event schema. `C3_OS_RESUMED` means the probe has returned; subsequent existing OS_READY/APP_ENTERING records belong to the static cooperative app, not an external-runtime readiness claim.

### Reproduce

```sh
python3 tools/smoke-ring3-elf.py --firmware-dir /opt/homebrew/share/qemu --lifecycle
python3 tools/smoke-ring3-elf.py --firmware-dir /opt/homebrew/share/qemu
python3 tools/smoke-ring3.py --firmware-dir /opt/homebrew/share/qemu
cargo test --locked --workspace --exclude musha-boot
```

The lifecycle runner stages only local QEMU files in `out/ring3-lifecycle`, records exact commands/ELF hashes in `report.json`, and requires three matching reclaim records followed by continuation into the existing app. Cases cover Hello, initialized ABI/arena state, permission denial, exit 42, protected start-page and stack-guard faults, invalid instruction/privileged instruction/division faults, nine malformed ELFs, injected partial-map failure, and a CPL0 invalid instruction that must remain fatal. The two fault-injection features are local test configurations.

ABI reference checked: [Microsoft x64 calling convention](https://learn.microsoft.com/en-us/cpp/build/x64-calling-convention?view=msvc-170), especially shadow space, stack alignment, nonvolatile GPR/XMM registers and FP control state. The wrapper reserves 32 bytes of call home space and a separately aligned FXSAVE area. Full CPU architecture review remains open as recorded in C1.

### Local validation (2026-10-13)

All **20 C3 cases** passed: 19 require three launch/reclaim cycles and return into the static app; the kernel-fault case requires a fatal stop. All **14 C2 cases**, **8 C1 cases**, **233 workspace host tests**, and SDK checks including **26 malformed ELF cases** also passed. The normal no-feature build passed list/open/back pixel regression, emitted no debug bytes and left both virtual images unchanged. Evidence: `out/ring3-lifecycle/report.json`, `out/ring3-elf/report.json`, `out/ring3/report.json`, `out/ring3-lifecycle/host-tests.log`, `out/ring3-lifecycle/normal-viewer-pixels/result.json`.

### Remaining

External disk-file loading, multiple different apps, preemption/time limits, SMP, general fault-safe copy services, resource handles/DMA cleanup, a stable public lifecycle record, Rust guest and C/Rust ABI tests, exhaustive CPU-state/exception review and physical hardware validation remain open. A guest infinite loop still monopolizes the CPU. C3 does not finish stage C or practical I/O stage D.

## 日本語

C3の任意構成で、同じELFを3回起動し、終了・対象のユーザー例外・読み込み失敗のたびに割当てを戻してアプリ用メモリを消去します。その後、既存OSサービスと静的アプリの起動処理へ復帰します。通常版と公開済み配布物は変更していません。

正常終了と終了コード42、保護ページ違反、不正命令・特権命令・除算例外、不正ELF、割当て途中の失敗を検査します。カーネル内の例外は復旧扱いにせず停止させます。検証記録は上記ローカル出力に保存します。

ローカル検証はC3 20件、C2 14件、C1 8件、ホスト233件が合格しました。SDK検査と通常ファイラーの画面・操作回帰も合格しました。実機と遠隔Actionsは実行していません。

割込み無効・単一CPU・同時実行なしの限定方式です。Rustの巻戻し処理を経由しないため、起動中のフレームに自動解放が必要な資源を追加できません。外部ファイル起動、一般的な資源管理、CPU時間制限、Rust実行、実機検証は残ります。公開互換性は固定していません。
