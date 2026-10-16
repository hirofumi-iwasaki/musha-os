# Musha POSIX profile p1 — a POSIX library interface without a process model

## English

Status: **owner decision on direction, 2026-10-16; profile contents are a draft for v0.6.0.** Related: [no-shell principle](no-shell-policy.md), [games scenario](scenario-games-godot.md), [v0.5.0 scope](release-scope-0.5.0.md), [application ABI](application-abi.md).

### Principle

Musha-OS provides the **POSIX API as a library interface** for a single statically linked program, and does **not** provide the POSIX **process model**. In one sentence: everything a program can do inside itself is offered where Musha has the underlying capability; everything that creates, connects or signals other processes is permanently absent. The reference point for this cut is WASI preview1 — a POSIX subset with files, clocks, random numbers, standard streams and exit codes, and no `fork`, no signals, no terminals.

The profile is versioned (`p1`, `p2`, …) and published as a table of what is supported, what returns an error, and what will never exist. Musha-OS never claims "POSIX compliance"; it claims conformance to a named profile.

### Layers

1. **Musha syscalls** (ABI candidates r1–r3): `query`, `log`, `exit`, `time`, `wait`, `input`, and from v0.5.0 `display`, `file` (read-only), `random`, `clock`, `audio`.
2. **sysdeps**: the thin layer a libc needs from the OS (memory, time, files, standard streams, exit, random, thread switching), implemented on the syscalls above. This is the only Musha-specific code.
3. **libc and C++ runtime**: an unmodified libc built against the sysdeps, plus a static `libstdc++` or `libc++` with RTTI (exceptions optional).

libc candidates, to be decided when v0.6.0 is scoped by measuring the sysdeps each requires: **mlibc** (MIT, designed for new operating systems with a `sysdeps/` port layer, proven with `libstdc++`) as the first candidate; **musl** (MIT, Linux-syscall oriented, the widest compatibility for SDL/Mesa/Godot) as the comparison; **picolibc** (BSD, small, embedded-oriented) as the fallback for size-constrained builds. Static linking only; no dynamic loader.

### Profile p1 (target: v0.6.0)

| Area | p1 | Backed by |
|---|---|---|
| Memory: `malloc` family, `brk`/`sbrk`, anonymous `mmap`/`munmap` | supported; `mmap` of files is ENODEV | arena (limit raised in v0.6.0); allocator inside libc |
| Time: `clock_gettime(CLOCK_MONOTONIC)`, `nanosleep`, `usleep`, `sleep` | supported, microsecond resolution | `SYS_TIME` (µs in v0.6.0), `SYS_WAIT` |
| Time: `CLOCK_REALTIME`, `gettimeofday`, `time`, `localtime` | supported, fixed offset from `tz=` | `SYS_CLOCK` (v0.5.0 RTC) |
| Files, read-only: `open(O_RDONLY)`, `read`, `pread`, `lseek`, `fstat`, `stat`, `close`, `opendir`/`readdir`, `access` | supported on the boot medium mounted at `/` (read-only); `SETTINGS/` and `EFI/` are not visible | `SYS_FILE_*` (v0.5.0) |
| Files, write: `open(O_WRONLY|O_CREAT)`, `write`, `unlink`, `rename`, `mkdir` | EROFS in p1; p2 candidate: a bounded application directory (`APPS/<name>/`, same bounded-write rules as `SETTINGS/`), first needed by the [MAME scenario](scenario-mame.md) for `nvram/` and `cfg/` | — |
| Standard streams: `stdout`/`stderr` | supported; both append to the boot log console (`write(1/2)` → `log`) | `log` |
| Standard streams: `stdin` | supported as line input from the keyboard (canonical mode only; `read(0)` returns a line) | `SYS_INPUT` |
| `isatty`, `termios` | `isatty(0/1/2)` = 1; `tcgetattr` returns fixed values, `tcsetattr` accepted and ignored | console control (v0.5.0 candidate) |
| Random: `getrandom`, `/dev/urandom`, `arc4random` | supported | `SYS_RANDOM` |
| Program: `exit`, `_exit`, `atexit`, `abort` | supported; `abort` exits with status 134 after flushing | `SYS_EXIT` |
| Program: `argv`, `getenv`, `environ` | supported; `argv` from `AUTORUN.ARG`, environment from `CONFIG.TXT` keys `app.env.*` | start record, `SYS_CONFIG_GET` (v0.5.0 candidate) |
| Threads: `pthread_create/join/mutex/cond/key`, `sched_yield` | supported as **cooperative user-level threads inside libc**: one CPU, no preemption, switches only at blocking calls (`sleep`, `wait`, `read`, mutex/cond waits, `sched_yield`); `pthread_setschedparam` and real-time attributes are ENOSYS | no kernel change; `SYS_WAIT` for the idle wait |
| Sockets: `socket(AF_INET, SOCK_DGRAM)` and UDP `sendto`/`recvfrom` | ENOSYS in p1; UDP planned for a later profile; TCP not planned | application UDP (v0.5.0 candidate) |
| Processes: `fork`, `vfork`, `posix_spawn`, `execve`, `pipe`, `wait*`, `kill` to another pid | **never** (ENOSYS); `getpid` returns 1 | [no-shell principle](no-shell-policy.md); chain launch is a Musha-specific call |
| Signals: `sigaction`, `signal`, `sigprocmask`, `raise` | accepted and recorded; no asynchronous delivery; `raise(SIGABRT)` behaves as `abort`; a CPU exception ends the program with the kernel's `app: faulted` line, handlers are not invoked | kernel fault path |
| Users and permissions: `getuid`, `geteuid`, `getpwuid`, `chmod`, `chown`, `umask` | fixed values (uid 0, user `musha`); modifications are EROFS/ENOSYS | no accounts |
| Locale, `iconv` | `"C"`/`"C.UTF-8"` only | — |
| Dynamic loading: `dlopen`, `dlsym` | **never** | static linking only |
| `select`/`poll` | supported for the file descriptors that exist (stdin readiness, UDP later) | `SYS_WAIT` |
| `mprotect`, `madvise` | `mprotect` within the arena supported for W^X use by runtimes; `madvise` ignored | kernel page tables |

### Namespace

`/` is the boot medium, read-only. `/dev/null`, `/dev/zero`, `/dev/urandom`, `/dev/stdin`, `/dev/stdout`, `/dev/stderr` exist as pseudo-files. There is no `/proc`, `/sys`, `/tmp` in p1 (an in-memory `/tmp` is a p2 candidate). Paths are case-insensitive on FAT32/exFAT as on the medium.

### Conformance evidence (definition of done for p1)

1. The libc's own test suite (mlibc tests or musl `libc-test`) passes for every p1 "supported" row and reports the expected error for every "ENOSYS/EROFS" row.
2. Unmodified builds of **zlib**, **libpng**, **Lua**, **SQLite (read-only database)** and **SDL2** (software renderer, Musha video/audio/input backends) compile against the profile and run on NUC5 and the Ryzen 7 machine.
3. A C++ program using `std::vector`, `std::string`, `std::map`, iostreams and RTTI runs.
4. The profile table above is published with each row marked by test evidence, not by intention.

### What this is not

Not a Linux ABI, not Linux binary compatibility, not a shell, not a process model. A program that needs `fork`, pipes, signals between processes, or dynamic loading is outside Musha-OS by design, and the profile says so in its table rather than failing at run time in surprising ways.

## 日本語

状態：**方向性は所有者決定（2026-10-16）。プロファイルの中身は v0.6.0 向けの草案。** 関連：[シェルを持たない原則](no-shell-policy.md)、[ゲーム用途シナリオ](scenario-games-godot.md)、[v0.5.0 範囲](release-scope-0.5.0.md)、[アプリ ABI](application-abi.md)。

### 原則

Musha-OS は、静的リンクされた 1 本のプログラムに対して **POSIX API をライブラリーインターフェースとして提供**し、POSIX の**プロセスモデルは提供しない**。一言で言えば、プログラムが自分の中で行えることは Musha に裏付けとなる機能がある限り提供し、他のプロセスを作る・つなぐ・シグナルを送ることは永久に無い。この切り方の基準点は WASI preview1——ファイル・時計・乱数・標準ストリーム・終了コードを持ち、`fork` もシグナルも端末も無い POSIX 部分集合——である。

プロファイルは版管理（`p1`、`p2`、…）し、「対応」「エラーを返す」「永久に無い」の表として公開する。Musha-OS は「POSIX 準拠」を名乗らず、名前付きプロファイルへの適合を名乗る。

### 層

1. **Musha の syscall**（ABI 候補 r1〜r3）：`query`、`log`、`exit`、`time`、`wait`、`input`、v0.5.0 から `display`、`file`（読出し専用）、`random`、`clock`、`audio`。
2. **sysdeps**：libc が OS に要求する薄い層（メモリー、時刻、ファイル、標準ストリーム、終了、乱数、スレッド切替）を上記 syscall で実装する。Musha 固有のコードはここだけ。
3. **libc と C++ ランタイム**：sysdeps に対してビルドした無改変の libc と、RTTI 付きの静的 `libstdc++` または `libc++`（例外は任意）。

libc の候補は、v0.6.0 の範囲決定時に各候補の sysdeps 量を測って決める：第一候補は **mlibc**（MIT。新しい OS 向けに `sysdeps/` の移植層を持つ設計で、`libstdc++` の実績あり）、比較対象は **musl**（MIT。Linux syscall 志向だが SDL/Mesa/Godot との互換性が最も広い）、サイズ制約のある構成向けの後退先が **picolibc**（BSD。小さく組込み向け）。静的リンクのみで、動的ローダーは持たない。

### プロファイル p1（目標：v0.6.0）

| 分野 | p1 | 裏付け |
|---|---|---|
| メモリー：`malloc` 系、`brk`/`sbrk`、匿名 `mmap`/`munmap` | 対応。ファイルの `mmap` は ENODEV | arena（v0.6.0 で上限引上げ）。アロケーターは libc 内 |
| 時刻：`clock_gettime(CLOCK_MONOTONIC)`、`nanosleep`、`usleep`、`sleep` | 対応、µs 精度 | `SYS_TIME`（v0.6.0 で µs 化）、`SYS_WAIT` |
| 時刻：`CLOCK_REALTIME`、`gettimeofday`、`time`、`localtime` | 対応。時差は `tz=` の固定オフセット | `SYS_CLOCK`（v0.5.0 の RTC） |
| ファイル読出し：`open(O_RDONLY)`、`read`、`pread`、`lseek`、`fstat`、`stat`、`close`、`opendir`/`readdir`、`access` | 起動媒体を `/`（読出し専用）として対応。`SETTINGS/` と `EFI/` は見えない | `SYS_FILE_*`（v0.5.0） |
| ファイル書込み：`open(O_WRONLY|O_CREAT)`、`write`、`unlink`、`rename`、`mkdir` | p1 では EROFS。p2 の候補：上限付きアプリ用ディレクトリー（`APPS/<名前>/`、`SETTINGS/` と同じ上限付き書込み規則）。最初に必要とするのは [MAME シナリオ](scenario-mame.md)の `nvram/` と `cfg/` | — |
| 標準ストリーム：`stdout`/`stderr` | 対応。どちらも起動ログのコンソールへ追記（`write(1/2)` → `log`） | `log` |
| 標準ストリーム：`stdin` | キーボードからの行入力として対応（canonical モードのみ。`read(0)` は 1 行を返す） | `SYS_INPUT` |
| `isatty`、`termios` | `isatty(0/1/2)` = 1。`tcgetattr` は固定値、`tcsetattr` は受理して無視 | コンソール制御（v0.5.0 候補） |
| 乱数：`getrandom`、`/dev/urandom`、`arc4random` | 対応 | `SYS_RANDOM` |
| プログラム：`exit`、`_exit`、`atexit`、`abort` | 対応。`abort` はフラッシュ後に終了コード 134 で終了 | `SYS_EXIT` |
| プログラム：`argv`、`getenv`、`environ` | 対応。`argv` は `AUTORUN.ARG`、環境は `CONFIG.TXT` の `app.env.*` キー | start record、`SYS_CONFIG_GET`（v0.5.0 候補） |
| スレッド：`pthread_create/join/mutex/cond/key`、`sched_yield` | **libc 内の協調ユーザーレベルスレッド**として対応：1 CPU、プリエンプション無し、切替はブロックする呼出し（`sleep`、`wait`、`read`、mutex/cond の待ち、`sched_yield`）でのみ。`pthread_setschedparam` と実時間属性は ENOSYS | カーネル変更無し。待機は `SYS_WAIT` |
| ソケット：`socket(AF_INET, SOCK_DGRAM)` と UDP の `sendto`/`recvfrom` | p1 では ENOSYS。UDP は後のプロファイルで予定。TCP は予定無し | アプリ向け UDP（v0.5.0 候補） |
| プロセス：`fork`、`vfork`、`posix_spawn`、`execve`、`pipe`、`wait*`、他 pid への `kill` | **永久に無い**（ENOSYS）。`getpid` は 1 を返す | [シェルを持たない原則](no-shell-policy.md)。連鎖起動は Musha 独自の呼出し |
| シグナル：`sigaction`、`signal`、`sigprocmask`、`raise` | 受理して記録するが非同期配送は無い。`raise(SIGABRT)` は `abort` と同じ。CPU 例外はカーネルの `app: faulted` 行でプログラムを終了し、ハンドラーは呼ばれない | カーネルの例外経路 |
| 利用者と権限：`getuid`、`geteuid`、`getpwuid`、`chmod`、`chown`、`umask` | 固定値（uid 0、利用者名 `musha`）。変更は EROFS/ENOSYS | アカウント無し |
| ロケール、`iconv` | `"C"`/`"C.UTF-8"` のみ | — |
| 動的ロード：`dlopen`、`dlsym` | **永久に無い** | 静的リンクのみ |
| `select`/`poll` | 存在するファイル記述子（stdin の読取可、後に UDP）に対して対応 | `SYS_WAIT` |
| `mprotect`、`madvise` | ランタイムの W^X 用途として arena 内の `mprotect` に対応。`madvise` は無視 | カーネルのページテーブル |

### 名前空間

`/` は起動媒体で読出し専用。`/dev/null`、`/dev/zero`、`/dev/urandom`、`/dev/stdin`、`/dev/stdout`、`/dev/stderr` を疑似ファイルとして持つ。p1 には `/proc`、`/sys`、`/tmp` は無い（メモリー上の `/tmp` は p2 の候補）。パスは媒体（FAT32/exFAT）どおり大文字小文字を区別しない。

### 適合の証拠（p1 の完了定義）

1. libc 自身の試験スイート（mlibc の tests または musl の `libc-test`）が、p1 の「対応」行全てで合格し、「ENOSYS/EROFS」行全てで期待どおりのエラーを返す。
2. **zlib**、**libpng**、**Lua**、**SQLite（読出し専用データベース）**、**SDL2**（ソフトウェアレンダラー、Musha のビデオ・音声・入力バックエンド）の無改変ビルドがプロファイルに対してコンパイルでき、NUC5 と Ryzen 7 機で動作する。
3. `std::vector`、`std::string`、`std::map`、iostream、RTTI を使う C++ プログラムが動作する。
4. 上の表の各行が、意図ではなく試験の証拠で裏付けられた状態で公開される。

### これが何でないか

Linux ABI ではなく、Linux バイナリー互換でもなく、シェルでもプロセスモデルでもない。`fork`、パイプ、プロセス間シグナル、動的ロードを必要とするプログラムは設計上 Musha-OS の外にあり、プロファイルは実行時に不意に失敗させるのではなく、表でそう明記する。
