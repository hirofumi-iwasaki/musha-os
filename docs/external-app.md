# Running an external C application (development preview)

## English

The current `release/0.3.0` development build supports one external C ELF per boot, including the feature-free release-profile EFI. This is an implementation and local QEMU result, **not hardware acceptance or a published release**. Published v0.2.0 downloads are unchanged. The [ABI](application-abi.md) remains **experimental candidate r1**, with no public compatibility freeze.

Build the SDK C example using [the SDK instructions](sdk.md) and place the resulting ELF as `MUSHA.ELF` in the root directory of the data medium selected by the existing file service. FAT32 and exFAT are supported through that service. ASCII case differences are accepted; duplicate matching names are rejected. There is no launcher, directory search, or new multiple-medium search. If the file is absent, the existing read-only viewer starts. A machine without a readable file service retains its existing recovery display.

A matching file must be 64 bytes to 16 MiB. The kernel reads at most 4096 bytes per request into its own staging memory, with a **5000 ms deadline**. The size ceiling is not a throughput promise: local tests completed 16 KiB, while 1 MiB and 16 MiB reached the deadline before completion. Start with the 13,312-byte SDK Hello. See [read measurements and memory rationale](external-elf.md). A partial, interrupted or malformed image is never executed.

The display changes to `Loading MUSHA.ELF ...` on detection. After reading and verified device shutdown, the kernel checks ELF/NOTE and memory policy, then starts the application. It records OS_READY for the external application, APP_ENTERING before the privilege transition, and APP_RUNNING on the first `query`. It does not mark the static viewer as running for this path.

`log` displays text in a kernel-owned **80-column × 24-row** console at pixel (16,16). Glyphs are 6×8 cells at scale 2 if the framebuffer is at least 992×416, otherwise scale 1; drawing clips to the framebuffer. Lines wrap and overflow discards the oldest row. Printable ASCII and newline are interpreted. Every other byte becomes `.` like the viewer (a multibyte character can produce several dots). UTF-8 validation still occurs first. The existing log permission, 4096-byte limit, return status and written-byte count are unchanged. All input is copied through the C2 owned supervisor aliases before drawing; applications receive neither framebuffer addresses nor hardware registers.

- `exit`: `MUSHA.ELF exited: status <n>` displays the signed 32-bit result, including nonzero results, and halts.
- User exception: `MUSHA.ELF faulted` displays vector, error code and RIP/RSP in hexadecimal, then halts. Kernel exceptions retain the fatal diagnostic path.
- Refusal: `MUSHA.ELF rejected: <reason>` displays file, timeout, interrupted-session, ELF/NOTE, CPU, arena or mapping/guard refusal and halts. Internal `ELF_*` reason labels may appear.

Reboot to return to the OS. C3 rollback/relaunch/OS return stays opt-in; it is not part of this normal path. Rust guest execution, application file/input/network/display APIs, new syscalls, scheduling and a public ABI freeze remain outside this work. SDK distribution and three recorded hardware cold boots remain release gates.

## 日本語

`release/0.3.0`の開発版は、feature指定なしのrelease構成を含め、起動ごとに1個の外部C ELFを実行します。ローカルQEMUでの実装検証であり、**実機合格やリリース公開を意味しません**。公開済みv0.2.0配布物は不変です。ABIは**実験候補r1**のままで、公開互換性を固定していません。

[SDK手順](sdk.md)でCサンプルをビルドし、既存ファイルサービスが選択するデータ媒体のルートへ`MUSHA.ELF`として配置します。FAT32／exFATに対応し、ASCIIの大小文字は区別せず、同名重複は拒否します。ランチャー、ディレクトリ探索、複数媒体の追加探索はありません。不在なら従来の読出し専用ビューアー、読める媒体がなければ従来の復旧表示へ進みます。

サイズは64バイト～16MiB、1要求最大4096バイト、読込み期限は**5000ms**です。容量上限と期限内に読める容量は別です。QEMUでは16KiBが完読し、1MiB／16MiBは未完読で期限終了しました。まず13,312バイトのSDK Helloを使用してください。[測定・メモリ配置の根拠](external-elf.md)に詳細を記載しています。不完全・中断・不正なELFは実行しません。

検出後は`Loading MUSHA.ELF ...`を表示し、読込みと機器停止を確認してからELF／NOTE・メモリ検査と実行へ進みます。外部アプリ用のOS_READY、特権遷移前のAPP_ENTERING、最初の`query`でAPP_RUNNINGを記録し、静的ビューアーの起動記録と区別します。

`log`はカーネル所有の**80桁×24行**の領域へ描画します。原点は(16,16)、6×8の文字セルを使用し、画面が992×416以上なら2倍、それ以外は1倍で画面内に切り詰めます。折返しと最古行の破棄を行います。ASCII印字可能文字と改行以外は、既存ビューアーと同じ`.`へバイト単位で置換します。UTF-8検査、権限、4096バイト上限、戻り値・バイト数は不変です。C2の所有alias経由コピーを終えてから描画し、アプリに画面アドレスやレジスターを渡しません。

終了時は`MUSHA.ELF exited: status <n>`に符号付き32ビットの結果を表示して停止します。ユーザー例外では`MUSHA.ELF faulted`とvector／error／RIP／RSPを16進数で表示します。拒否は`MUSHA.ELF rejected: <reason>`で示し、ファイル・期限・中断・ELF・CPU・arena・配置／guardの拒否理由を残します。カーネル例外は従来の致命的停止です。

OSへ戻るには再起動します。C3の回収・反復起動・OS復帰は任意試験構成のままです。Rust guest、アプリ向け実用I/O、新syscall、スケジューリング、ABI固定は今回の対象外です。SDK配布と実機3回のコールドブートは公開までの残件です。
