# SDK C ELF execution prototype — C2

## English

Status: opt-in implementation, locally executed under QEMU TCG on 2026-10-13. Candidate ABI r1 remains experimental; public compatibility is not fixed. This extends [C1](ring3-prototype.md), not the normal viewer or published release.

`ring3-elf-probe` embeds a build-selected immutable ELF into the EFI. After CPU/memory setup, the kernel independently validates ELF64 little-endian static x86-64 ET_EXEC and one exact aligned Musha NOTE. It bounds file/program headers, image/arena/stack sizes, requires a file-backed RX entry, rejects overlapping pages, W+X, dynamic/TLS/interpreter/unknown segments and unsupported metadata. The host checker is independent and cannot authorize kernel loading.

LOAD segments receive fresh owned backing, zero-filled BSS/padding and R/NX, RX or RW/NX user mappings. A zeroed application arena, RW/NX stack with an unmapped lower guard and read-only/NX start-info page use the candidate virtual layout. All virtual ranges are checked for existing mappings before installation. The probe retains supervisor aliases and the C1 physical arena limits. Mapping/allocation failure halts; rollback, reclamation and application relaunch are not implemented.

The SDK crt0 receives start-info in RDI and a zero RSI through the explicit Win64-to-SysV assembly boundary. SDK C Hello runs at CPL3 and calls query/log/exit. Log requires the NOTE log permission, accepts at most 4096 UTF-8 bytes and validates/copies the entire range through permanent owned supervisor aliases before emitting bytes. This is bounded single-thread probe copying, not general fault-recoverable copyin/copyout, pinning or a production permission service. Return RIP is restricted to file-backed executable bytes and RSP to the loaded stack. Start flags, report handle and reserved fields are zero.

### Local reproduction and evidence

Use the project Rust toolchain, Clang, Python and QEMU/EDK2:

```sh
cargo test --locked --workspace --exclude musha-boot
python3 tools/check-sdk.py
python3 tools/smoke-ring3-elf.py --firmware-dir /opt/homebrew/share/qemu
python3 tools/smoke-ring3.py --firmware-dir /opt/homebrew/share/qemu
cargo build --locked --release --target x86_64-unknown-uefi -p musha-boot
```

The C2 harness uses `out/ring3-elf/target`, stages local firmware/files and records commands, ELF hashes and outcomes in `out/ring3-elf/report.json`. `--case hello` selects one case; omit it to retain a full report. `MUSHA_USER_ELF` is a required build-time file for the feature. No guest syscalls run on the host. Only release-profile QEMU execution is claimed.

All 14 C2 cases passed:

| Case | Evidence |
|---|---|
| hello | SDK C Hello, CPL3, kernel syscall stack, query/log/exit |
| abi | Start record, BSS/arena zeroing, writable arena boundaries, C double calculation, unknown call, pointer overflow/range/length/UTF-8 rejection, cross-page log copy |
| start-ro | User start-info write faults with #PF error 7 |
| stack-guard | Read below stack faults with #PF error 4 |
| log-denied | Missing log permission returns denied and no message |
| Nine malformed ELFs | Magic, W+X, dynamic PH, entry, file overflow, NOTE revision/permissions, overlapping LOADs and missing NOTE rejected before CPL3 entry |

Regression results: **233 workspace host tests**, all **8 C1 cases**, SDK compile/layout checks with **26 malformed ELF cases**, and the normal-viewer list/open/back pixel comparison passed. Normal virtual images remained unchanged. Evidence is in `out/ring3-elf/host-tests.log`, `out/ring3/report.json`, `out/ring3-elf/sdk/report.json` and `out/ring3-elf/normal-viewer-pixels/result.json`.

The pure host parser also covers all truncations of its fixture and malformed headers, duplicate NOTE, overlapping pages and BSS copying. SDK compile/layout checks and kernel execution evidence are separate.

### Remaining stage C/D work

[C3 lifecycle follow-up](ring3-lifecycle.md) adds bounded rollback, repeated launches and OS continuation in a separate opt-in configuration. The one-shot C2 behavior and results above are retained.

- Rust ELF with pinned custom-target core/alloc; mixed C/Rust integer, double, struct and stack arguments; broader FP/register conformance.
- External-file acquisition is implemented in the [normal one-shot path](external-elf.md), with [screen log/exit/fault presentation](external-app.md). Production recovery/rollback and repeated launches remain outside the normal build. The embedded C2 probe still treats nonzero exit as a failed test; the external application path displays any exit code and halts.
- General copyin/copyout, capabilities/handles, interruption/reentry, CPU budgets/preemption, SMP, current architecture-manual review and physical hardware validation.
- Readiness/launch/exit integration and practical I/O (stage D). C2 does not claim external OS readiness, a supported SDK or public ABI freeze.

## 日本語

C1に続く任意構成として、SDKのC Helloを実際にRing 3で実行しました。カーネル側ELF/NOTE検査、ページ権限、BSSのゼロ初期化、アプリ用arena、読出し専用起動情報を追加しています。通常版ファイラーと公開済み配布物は変更しません。

QEMUの14ケースが合格しました。正常なSDK起動、起動情報・メモリ初期化、ログの範囲・長さ・UTF-8・権限検査、ページをまたぐコピー、起動情報の書込み禁止、スタック保護、不正ELF 9種類の起動前拒否を確認しました。

ELFはビルド時にEFIへ埋め込む試験方式です。外部ファイルからの通常起動、Rust実行、C/Rust相互呼出し、終了・障害後のOS復帰、資源回収、実用I/Oと実機検証は残ります。終了・障害時は停止します。候補値・実装済み・QEMU検証済みを区別し、公開互換性は固定していません。

commit・push・タグ・Release・遠隔設定・実USB・確定PDFの操作は行っていません。

外部ファイル取得と画面ログ・終了／障害表示は[通常版の一回実行経路](external-app.md)へ統合しました。埋込みC2試験の非0終了判定は維持し、外部アプリは非0コードも表示して停止します。通常版のOS復帰・反復起動と実機検証は残件です。
