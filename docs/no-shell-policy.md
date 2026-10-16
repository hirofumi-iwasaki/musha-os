# Design principle: no command shell

## English

Status: **owner decision, 2026-10-15.** Applies to every version until revised by the owner. Related: [games scenario](scenario-games-godot.md), [v0.5.0 scope](release-scope-0.5.0.md), [boot readiness policy](boot-readiness-policy.md).

### Principle

Musha-OS does not have, and will not add, a command shell. There is no `sh`, `bash`, `zsh` or an equivalent of `COMMAND.COM`. The reason is not the shell program itself but what it requires underneath: a process model with `fork`/`exec`, pipes, signals, terminals, environment variables, a current directory, exit-status propagation and job control. Musha-OS is built on the opposite model — **one application per session, cooperative I/O, the kernel owning devices and memory** — and adopting a shell would mean adopting the UNIX process model with it. That is a change to what Musha-OS is, not a feature.

Concretely, Musha-OS has none of the following and does not plan them: shell, processes, concurrent applications, pipes, signals, tty/pty, environment variables, current directory, user accounts or file permissions. The file viewer is the only launcher; the application is the unit of execution. The libc layer planned for the games scenario does not change this: it provides the C library over Musha syscalls, not a POSIX process model (see the [Musha POSIX profile](posix-profile.md)).

### What the request usually means, and the answer without a shell

| Underlying need | Answer |
|---|---|
| Choose among several programs | Launch from the viewer (v0.4.0 goal 10) |
| Pass arguments to a program | An argument file beside the ELF (`AUTORUN.ARG`, proposed) or `app.*` keys in `CONFIG.TXT`; the text is delivered through the start record. Candidate for v0.5.0 |
| Interactive text programs (REPL, editor) | Ordinary applications using `log`, `input` and the console-control calls (v0.5.0 candidate) |
| Run programs in sequence (batch) | **Chain launch**: an application exits naming the next ELF; the kernel tears down, then launches it. No fork, no concurrency. Candidate for v0.5.0. A third party may build a command-line interpreter as an ordinary application on top of this; Musha-OS does not ship one |
| See or control the machine remotely | Log output over UDP or serial (later candidates); not a shell |

### Not a shell, by design

The viewer is a launcher and a read-only file inspector. It will not grow a command line, wildcards, redirection or scripting. Chain launch moves control from one application to the next; it does not run two at once and it does not connect their input and output.

## 日本語

状態：**所有者決定、2026-10-15。** 所有者が改めるまで全ての版に適用する。関連：[ゲーム用途シナリオ](scenario-games-godot.md)、[v0.5.0 範囲](release-scope-0.5.0.md)、[起動と API の統合方針](boot-readiness-policy.md)。

### 原則

Musha-OS はコマンドシェルを持たず、今後も追加しない。`sh`・`bash`・`zsh` も、`COMMAND.COM` に相当するものも無い。理由はシェルというプログラムそのものではなく、その下に必要になるもの——`fork`/`exec`、パイプ、シグナル、端末、環境変数、カレントディレクトリー、終了ステータスの伝播、ジョブ制御というプロセスモデル——にある。Musha-OS はその反対のモデル——**1 セッションに 1 アプリ、協調 I/O、カーネルによる機器とメモリーの一元所有**——の上に作られており、シェルを入れることは UNIX のプロセスモデルを一緒に入れることになる。それは機能の追加ではなく、Musha-OS が何であるかの変更である。

具体的に、Musha-OS には次のものが無く、計画もしない：シェル、プロセス、複数アプリの同時実行、パイプ、シグナル、tty/pty、環境変数、カレントディレクトリー、利用者アカウント、ファイルのアクセス権。ファイルビューアーが唯一のランチャーで、アプリが実行の単位である。ゲーム用途シナリオで計画する libc 層もこれを変えない：Musha の syscall 上に C ライブラリーを提供するものであり、POSIX のプロセスモデルではない（[Musha POSIX プロファイル](posix-profile.md)参照）。

### 「シェルが欲しい」の裏にある要望と、シェル無しでの答え

| 裏にある要望 | 答え |
|---|---|
| 複数のプログラムから選びたい | ビューアーからの起動（v0.4.0 目標 10） |
| プログラムに引数を渡したい | ELF と同じ場所の引数ファイル（`AUTORUN.ARG`、提案）か `CONFIG.TXT` の `app.*` キー。文字列は start record で渡す。v0.5.0 の候補 |
| 対話的なテキストプログラム（REPL、エディター） | `log`・`input`・コンソール制御（v0.5.0 候補）を使う普通のアプリ |
| 順番に実行したい（バッチ） | **連鎖起動**：アプリが次の ELF を指定して終了し、カーネルが後始末の後にそれを起動する。fork も同時実行も無い。v0.5.0 の候補。第三者がこの上にコマンド行インタープリターを普通のアプリとして作ることはできるが、Musha-OS は同梱しない |
| 遠隔から見たい・操作したい | ログの UDP 送出やシリアル出力（後の候補）。シェルではない |

### 設計上「シェルではない」もの

ビューアーはランチャーであり読出し専用のファイル閲覧器である。コマンド行、ワイルドカード、リダイレクト、スクリプトは持たせない。連鎖起動はあるアプリから次のアプリへ制御を移すだけで、2 つを同時に動かさず、入出力を接続しない。
