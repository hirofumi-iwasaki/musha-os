# exFAT and cooperative file reads / exFAT・協調file読出し

Status: current (2026-10-15). Source contract; physical acceptance is separate.

## English

### Reader and media contract

`fs/src/exfat.rs` is allocation-free/no_std and read-only. It accepts exFAT revision1.00 with one FAT and512/4096-byte logical sectors matching the medium; cluster size is bounded to32MiB and processed by sectors. Discovery considers superfloppy, MBR primary type07 and GPT Microsoft Basic Data, checks signatures and rejects ambiguous multiple exFAT volumes. GPT headers/entry arrays, CRC, range/overlap and supported entry geometry are validated. No hybrid/extended partition or damaged-side recovery is promised. Mount/read requires immutable media; rediscovery or detach invalidates the prior mount.

The caller supplies Workspace, media geometry, selected volume bounds and a full-sector read callback. `entries()` exposes original UTF-16 root names (up to255 code units), at most128 files and a bounded1MiB directory scan. No subdirectory browsing, TexFAT or write callback. Files above64MiB remain subject to a read limit. `read_at`/planner operations read at most4096 bytes; EOF/empty reads return0. Errors do not publish partial output. Metadata, range/overflow, checksum, bitmap/up-case and chain/declared-length checks are not skipped for a small read. ValidDataLength tails read as zero; corrupt main boot is not silently recovered from backup.

### Cooperative transport and file API

The reusable planner separates CPU filesystem decisions from block submission/completion. Runtime services bounded USB BOT work between keyboard/network polls; synchronous full-chain host readers must not be called from an application callback. FAT32/exFAT share root catalog, opaque generation-scoped file IDs and list/open/read/close semantics in the [internal API](app-api.md). Partial progress returns Again with ownership retained. The application owns its output buffer and no user pointer is retained for later DMA.

Closing/canceling/disconnecting invalidates handles immediately but an in-flight request remains quarantined until completion/confirmed transport stop. The model alone does not stop hardware. Stale generation/request completions are rejected; no arena/DMA reuse is permitted on uncertain stop. Reader/viewer/app lifecycle currently supports FAT32 and exFAT through the shared bounded path; [stage5](stage5-0.4.0-progress.md) records local launch/return regression. This is distinct from experimental external r2 input/time calls; no external general-purpose file/network/write syscall is promised by this page. OS-owned SETTINGS writes are FAT32-only and separate from exFAT data.

### Fixtures and evidence

`tools/make-exfat-fixture.py` creates new ordinary test image files, never formats or mounts a physical device. Its normal/valid-tail and corrupt checksum/chain/bitmap cases produce a manifest with image and expected-file hashes. Fixture up-case behavior is deliberately limited and not a substitute for broad real-formatter compatibility. `cargo test --locked -p musha-fs` and `python3 tools/test-exfat-fixture.py` cover the reader/fixture contract; no new result is asserted merely by listing commands.

Original design, implementation stages, exact test counts, fault cases and QEMU observations remain in [design](history/exfat-design.md), [reader](history/exfat-reader.md), [fixtures](history/exfat-fixtures.md), [stream service](history/stream-file-api.md) and [BOT integration](history/bot-polling.md). Their early “unconnected” statements describe their recorded stages, not today's source. Physical mount/read/storage and formatter gates stay in [hardware status](hardware-verification-status.md) and version-specific acceptance. USB throughput, arbitrary media/layouts and power-failure recovery are not established by host/QEMU tests.

## 日本語

### readerと媒体契約

`fs/src/exfat.rs`はallocation無し/no_std/読取専用。revision1.00/FAT1本、媒体と一致する512/4096byte sector、最大32MiB clusterをsector単位で処理。superfloppy・MBR primary07・GPT Basic Dataから署名を確認し複数候補は拒否。GPT両header/entry array/CRC/範囲/重複/対応geometryを検査。hybrid/拡張partition/破損片側からの復旧を保証しません。mount/read中の媒体不変が前提で再探索/切断時は旧mountを失効。

呼出し側Workspace/媒体geometry/volume範囲/完全1sector read callbackを用意。`entries()`は元UTF-16 root名（最大255code unit）、最大128file・directory走査1MiB。subdirectory/TexFAT/write callbackなし。64MiB超fileはread上限対象。`read_at`/plannerの読出しは最大4096byte、EOF/空readは0、失敗時に部分出力しません。小readでもmetadata/範囲/overflow/checksum/bitmap/up-case/chain/宣言長検査を維持。ValidDataLength以後は0、main boot破損をbackupから黙って復旧しません。

### 協調transportとfile API

再利用plannerがCPU判断とblock要求/完了を分離。runtimeはkeyboard/network pollの間に有界USB BOTを進め、同期host全chain readerをapp callbackから呼びません。FAT32/exFATはroot catalog・世代付きopaque ID・list/open/read/closeを[内部API](app-api.md)で共用。Again中は所有権を保持しapp buffer pointerを後のDMAへ保存しません。

close/cancel/disconnectでhandleは即失効しても実行中要求は完了/transport停止確認まで隔離。modelだけでdevice停止を実現しません。古い世代/要求完了を拒否し、停止不確実ならarena/DMA再利用禁止。現reader/viewer/appは共通FAT32/exFAT経路を使用し[stage5](stage5-0.4.0-progress.md)にlocal launch/return回帰を記録。外部r2 input/timeとは別で汎用file/network/write syscallを本書で約束しません。OS所有SETTINGS書込みはFAT32限定、exFAT dataとは別です。

### fixtureと証拠

`tools/make-exfat-fixture.py`は新規通常test imageだけを作り実device format/mountしません。normal/valid-tail/checksum/chain/bitmap破損caseとimage/file hash manifestを生成。独自up-caseは限定的で一般formatter互換の代用ではありません。`cargo test --locked -p musha-fs`・`python3 tools/test-exfat-fixture.py`は検証入口でcommand記載自体を新合格結果にしません。

設計/各実装段階/当時試験数/異常系/QEMU観測は[design](history/exfat-design.md)・[reader](history/exfat-reader.md)・[fixtures](history/exfat-fixtures.md)・[stream](history/stream-file-api.md)・[BOT](history/bot-polling.md)へ原文保持。初期の未接続表記は当時の状態。実機mount/read/storage/formatter gateは[実機状況](hardware-verification-status.md)と版毎受入で区別しhost/QEMUから実throughput/全媒体/停電復旧を保証しません。
