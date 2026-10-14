# Ring 3 transition prototype — C1

## English

Status: implemented as an **opt-in, one-shot fixed assembly fixture**, tested locally under QEMU TCG. This starts stage C; it does not complete the external application or SDK execution gate. ABI numbers remain experimental candidate r1. No public compatibility is fixed.

The normal build still runs the statically linked viewer at Ring 0. `ring3-probe` diverts after core CPU/memory/timer setup and before the normal PCI/application session. It never resumes the viewer, advertises external OS readiness, or invokes a supplied ELF. Every exit or fault halts the probe. No release asset or physical USB is involved.

### Implemented boundary

- Candidate image address `0x400000`: one copied RX user page. Candidate stack top `0x70000000`: one zeroed RW/NX user page, with the lower page unmapped. Existing kernel leaves remain supervisor-only. Mapping a present leaf is rejected; the loader cannot silently replace a kernel mapping.
- Backing storage is taken exclusively from the boot arena, restricted to the 32–256 MiB physical window for this fixture. Missing space or virtual-address collisions fail closed. Supervisor identity aliases remain; this is user W^X, not elimination of every writable kernel alias.
- A separate 64 KiB supervisor stack has a lower unmapped guard. TSS.RSP0 and explicit SYSCALL entry use its top. The kernel validates the actual transition/fault frame location.
- Prototype selectors: kernel CS/SS `0x08/0x10`, TSS `0x18`, user SS/CS `0x2b/0x33`. STAR selects kernel CS; LSTAR names the naked assembly entry; SFMASK clears user flags; EFER enables SCE while retaining NXE. Returns use IRETQ, never SYSRET.
- The first two ordinary entry instructions save user RSP to permanent supervisor storage and load the kernel stack. No Rust prologue or stack access precedes this. The wrapper preserves raw GPRs other than the declared results/clobbers, reserves Win64 home space, and uses an aligned 512-byte FXSAVE64/FXRSTOR64 area. Kernel FP control defaults are restored before Rust dispatch. RCX/R11 are cleared before returning.
- User x87/SSE state is initialized; AVX/OSXSAVE and FSGSBASE are disabled for this probe. CET-enabled environments are rejected. FS/GS bases are zero. Returned flags retain only arithmetic flags and bit 1. RIP must be inside copied code and RSP inside the known user stack; invalid returns halt before IRETQ.
- Candidate query/log/exit numbers come from the generated SDK ledger. Log length is bounded; known fixture regions are translated to supervisor aliases after overflow/range checks. Unknown calls and invalid ranges return negative status with zero secondary result. This is **not** a general copyin/copyout implementation or a permission/handle service.
- NMI, debug and machine-check exceptions use a fatal-only IST stack; double fault retains its separate IST. These handlers never resume. Nested-event recovery, scheduling, interrupt returns and SMP are not implemented.

The fixture checks CPL=3 and initial stack alignment, selected raw register sentinels, representative x87/XMM0/XMM15 state and MXCSR across a syscall, cleared privileged/direction flags, error results, bounded log output and exit. Its RDI/RSI are test controls, **not** the SDK start-info record. It deliberately supplies a supervisor alias for the negative access test. Do not reuse this entry as a public ABI.

### Reproduce locally

Use the existing project-local Rust toolchain on PATH, plus Clang, Python and QEMU/EDK2:

```sh
cargo test --locked --workspace --exclude musha-boot
python3 tools/check-sdk.py
python3 tools/smoke-ring3.py --firmware-dir /opt/homebrew/share/qemu
cargo build --locked --release --target x86_64-unknown-uefi -p musha-boot
```

The harness uses a separate `out/ring3/target`, staging folder and writable firmware-variable copy. `--case` selects a single case. `MUSHA_RING3_CASE` is a build-time test choice, tracked by Cargo. Logs, command arrays and EFI hashes are recorded in `out/ring3/report.json` and per-case directories. The program hashes only the small EFI file, not a physical disk. Debug/unoptimized EFI execution is not claimed.

| QEMU case | Required result |
|---|---|
| normal | CPL3 checks, query on kernel stack, user log, exit success |
| ro | User write to code: #PF, error 7 |
| nx | Execution on user stack: #PF, error 21 |
| guard | Read below user stack: #PF, error 4 |
| supervisor | Read supervisor alias: #PF, error 5 |
| rsp | Noncanonical user RSP: rejected before return, without #GP |
| privileged | User CLI: #GP, error 0 |
| nmi-transition | QMP NMI before RSP switch: IST verified, saved CS=8 and user RSP, fatal vector 2 |

The NMI case deliberately pauses at CPL0 while RSP still points at the user stack. The fixture emits `@@` using stack-free port writes, then the runner injects NMI. This is deterministic evidence for that vulnerable interval, not exhaustive asynchronous-event coverage.

### Local validation result (2026-10-13)

All eight QEMU cases above passed with release-profile EFI. The OS workspace host suite passed **230 tests** (including two new range/return tests). Existing SDK compilation/layout checks and **26 malformed ELF/NOTE rejection cases** passed; SDK guest execution remains false. Results are separate from the historical A/B report.

The no-feature release build also passed the existing normal-viewer pixel regression: list/open/back matched the prior normal-build reference, no debug bytes were emitted, and both virtual images were unchanged. Evidence: `out/ring3/normal-viewer-pixels/result.json`. Commands used (the output image/directory must not already exist):

```sh
python3 tools/make-usb-image.py out/ring3/normal-viewer.img --efi target/x86_64-unknown-uefi/release/musha-boot.efi
python3 tools/smoke-normal-viewer.py --firmware-dir /opt/homebrew/share/qemu --boot-image out/ring3/normal-viewer.img --data-image out/ring3/normal-viewer.img --reference out/v3-normal-final-pixels --output out/ring3/normal-viewer-pixels
```

The reference is the previously verified local A/B normal-viewer fixture, not a newly generated reference from the candidate under test.

### Remaining stage C work

Follow-up: [C2 ELF/SDK C execution](ring3-elf.md) implements bounded loading, BSS/start record/arena and SDK C Hello. The results below retain the historical C1 scope.

1. Kernel-side bounded ELF/NOTE validation, page allocation/rollback, BSS zeroing, read-only start record and a real app arena. The existing host checker cannot replace these checks.
2. Run SDK C Hello using its actual SysV entry, then Rust Hello after pinned rust-src/core rebuilding. Test mixed C/Rust integer, double, struct and stack arguments; expand full FP/register conformance.
3. General page-aware fault-safe user copies and capability/permission handling; app fault/exit cleanup and return to the OS rather than halting the machine. Integrate readiness/launch/exit records only when the external runtime exists.
4. Enforceable CPU budgets/preemption, full exception/reentry review, more memory layouts/CPU models and real hardware. Practical external I/O remains stage D.

Architecture reference actually consulted: Intel SDM Volume 2B **253667-060US (September 2016), SYSCALL pp. 4-668–4-669**, [official PDF](https://www.intel.com/content/dam/www/public/us/en/documents/manuals/64-ia-32-architectures-software-developer-vol-2b-manual.pdf). It documents that software must save/switch RSP and provides the STAR/LSTAR/FMASK behavior. The [current manual index](https://www.intel.com/content/www/us/en/developer/articles/technical/intel-sdm.html) advertises revision 093; its redirected PDFs were unavailable through the browser tool. Full review against that revision, including IRETQ/IST and newer CPU features, remains a stage-C gate; no claim of a completed security review.

## 日本語

工程Cの最初の実装として、`ring3-probe`限定の固定アセンブリ試験を追加しました。QEMUでCPL3への遷移、syscall時の専用カーネルスタックへの切替、IRETQによる復帰、照会・有界ログ・終了を検証します。通常版は従来のRing 0ファイラーのままです。

ローカル結果は8ケース合格、ホスト試験230件合格、SDK不正ELF/NOTE 26件の拒否確認です。通常版の一覧・ファイル表示・戻る操作は既存の検証済み画面と一致しました。

正常系に加え、コード書込み・スタック実行・ガードページ・カーネル領域の保護、不正RSP、特権命令、スタック切替前のNMIを検証する8ケースを用意しました。NMIはCPL0／ユーザーRSPのまま停止させた区間に注入し、IST利用を確認します。障害・終了後は停止し、OSへ復帰して次のアプリを動かす処理は未実装です。

これはSDKのC/Rust ELF実行ではありません。起動引数も試験専用です。ELF/NOTEのカーネル検査、BSS・読出し専用起動情報・アプリarena、一般的な安全なコピー、権限管理、C/Rust相互呼出し、障害回復、CPU時間制限、実機検証は残ります。公開互換性は固定していません。

現行Intel SDMの索引は093ですが、本文を確認できたSYSCALL資料は公式060USです。最新版全体との照合を完了済みとはしません。ブランチ・先行変更を保持し、commit／push／タグ／Release／遠隔設定／実USB／PDF操作は行いません。
