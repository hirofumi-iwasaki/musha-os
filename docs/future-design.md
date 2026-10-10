# Future architecture / 将来のアーキテクチャ設計

## English

### Ring 3 application isolation

Added 2026-10-09. This is a design direction after 0.1.0, not an implemented capability or a commitment to a particular release.

The current runtime and statically linked diagnostic application both execute in x86-64 long mode at Ring 0. Rust and existing page protections do not create an application privilege boundary. For separately supplied applications, adopt Ring 3 as the future default; keep the kernel, exception/interrupt handlers, USB/NIC drivers and lwIP integration at Ring 0. A trusted, statically linked appliance may deliberately retain Ring 0, with its weaker fault isolation documented. Ring 3 does not require POSIX or SMP.

Required design:

- Give each application a private page-table root, user code/data/stack and guard pages. Kernel state, page tables, MMIO and DMA buffers remain supervisor-only. Use read-only executable code and non-executable writable data. Define an application image/loader contract, entry point, stack and teardown before loading external binaries; the UEFI executable is not automatically the future application format.
- Preserve the free RAM arena inside memory assigned to the application. This does not grant arbitrary physical-memory access. Account for allocations, initialize pages before reuse and revoke mappings on exit. Begin with one isolated application; independent multiple applications require separate address spaces and resource ownership.
- Extend GDT/TSS with user selectors and kernel entry stacks. Define gate privileges, register saving, nested faults and safe return validation. Select and audit the syscall entry/return mechanism during detailed design. Restrict IOPL and deny user port I/O through the TSS contract; user code cannot mask interrupts or control page tables.
- Replace direct kernel function pointers with a versioned syscall ABI and checked handles for drawing, input, time, files, UDP and arena management. Validate syscall numbers, pointers, lengths, overflow and permissions; use bounded copy-in/copy-out or managed shared buffers. Never use an unchecked user pointer as a DMA address.
- Render through kernel services by default; do not expose the full GOP framebuffer or hardware registers. Ring 3/page permissions alone do not protect against device DMA errors. IOMMU and driver isolation are separate future topics.
- Contain ordinary user faults by terminating the application, reporting its state and reclaiming resources while keeping the runtime available. Define exit/restart and outstanding-I/O ownership. Cancel or drain device operations before freeing their buffers. Fatal kernel faults retain a separate diagnostic/shutdown policy.
- Add bounded kernel work and timer-driven preemption or another enforceable CPU-budget mechanism before claiming robust isolation. Ring 3 alone cannot stop an infinite loop, and cooperative step calls cannot enforce yielding. Start with a single-core model; SMP is not a prerequisite.

Implementation stages and acceptance:

1. Specify the ABI, loader, memory layout, privilege transitions and resource lifecycle; retain the 0.1.0 scope.
2. Run a minimal Ring 3 greeting application with a private stack/arena and checked drawing/time calls; verify CPL, page permissions and safe kernel returns.
3. Test user page faults, kernel-memory writes, privileged instructions, invalid calls/pointers, stack overflow, allocation exhaustion and infinite loops. Verify containment, diagnostics, bounded recovery and successful execution of the next application.
4. Add file/input/network calls incrementally; test exit during pending device I/O and safe DMA-buffer reclamation. Validate in QEMU, then NUC5/NUC8.

Ring 3 is a protection boundary, not a complete security sandbox. Application format, binary compatibility, exact syscall mechanism, scheduling policy and multi-application API remain detailed-design decisions. Do not claim implementation until acceptance passes.

## 日本語

### Ring 3によるアプリ分離

2026-10-09追加。0.1.0より後の設計方針であり、実装済み機能や特定バージョンへの搭載確約ではない。

現在は実行環境と静的リンクされた診断アプリをともにx86-64ロングモードのRing 0で実行する。Rustや既存のページ保護だけではアプリの権限境界は成立しない。外部から提供されるアプリは将来Ring 3を標準とし、カーネル、例外・割込み処理、USB／NICドライバー、lwIP連携はRing 0に置く。信頼されたアプリを静的リンクする専用機では、障害分離が弱いことを明記しRing 0構成も明示的に選べる。POSIXやSMPは必須ではない。

必要な設計：

- アプリごとに独立したページテーブル、user code/data/stack、guard pageを用意する。カーネル状態、ページテーブル、MMIO、DMA bufferはsupervisor専用にする。実行コードは読出し専用、書込みデータは実行不可とする。外部バイナリを扱う前にアプリ形式／loader、entry point、stack、解放手順を定義する。現在のUEFI実行ファイルを、そのまま将来のアプリ形式とはしない。
- 自由RAM arenaはアプリに割り当てた領域内で維持し、任意の物理メモリへのアクセスは許可しない。確保量を管理し、再利用前に初期化、終了時にmappingを解除する。最初は1アプリを分離し、複数アプリを独立して隔離する場合は個別アドレス空間と所有権を用意する。
- GDT/TSSにuser selectorとカーネル入口用stackを追加する。gate権限、register保存、入れ子の例外、安全な復帰先検証を定義する。system call入口・復帰方式は詳細設計で選定・監査する。IOPLを制限し、TSSの契約でもuser port I/Oを拒否する。割込み禁止やページテーブル制御をアプリに許可しない。
- 直接のカーネル関数pointerを、版付きsystem call ABIと検証可能なhandleに置き換える。描画・入力・時刻・file・UDP・arena管理を必要に応じて提供し、call番号、pointer、長さ、overflow、access権限を確認する。有界のcopy-in/copy-outまたは管理された共有bufferを使い、未検証のuser pointerをDMA addressへ転用しない。
- 描画は原則カーネル経由とし、GOP framebuffer全体やhardware registerを公開しない。Ring 3とページ権限だけではデバイスの誤DMAを防げない。IOMMUやドライバー分離は別の将来課題とする。
- 通常のuser例外はアプリを終了し、状態を記録、resourceを回収して実行環境を維持する。終了・再起動・未完了I/Oの所有権を定義する。デバイス処理の中止または完了を待ってからbufferを解放する。重大なカーネル障害は別の診断・停止方針で扱う。
- 強い障害分離を掲げる前に、カーネル処理の有界化とtimerによるプリエンプションなど強制可能なCPU時間制限を導入する。Ring 3だけでは無限ループを止められず、協調step呼出しだけでは制御を返させることはできない。まず単一coreで検証し、SMPは前提としない。

実装段階と合格条件：

1. ABI、loader、メモリ配置、権限移行、resource寿命を仕様化する。0.1.0の範囲は維持する。
2. 独立stack／arenaと検証付き描画・時刻callでRing 3の最小Helloアプリを動かす。CPL、ページ権限、安全な復帰を確認する。
3. user page fault、カーネルメモリ書込み、特権命令、不正call／pointer、stack overflow、確保上限超過、無限ループを試す。封じ込め、診断、有界の回収、次のアプリの正常実行を確認する。
4. file／input／network callを段階的に追加する。未完了I/O中の終了とDMA bufferの安全な回収を試し、QEMUに続きNUC5／NUC8で確認する。

Ring 3は保護境界であり、それだけで完全なsecurity sandboxにはならない。アプリ形式、バイナリ互換性、system call方式、scheduler、複数アプリAPIは詳細設計で決める。合格条件を満たすまで実装完了を表明しない。
