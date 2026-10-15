# Long-term scenario: games on Musha-OS (Godot as the placeholder target)

## English

Status: **scenario, not a release scope.** Adopted by the owner on 2026-10-15 as a way to order future work. Nothing here is scheduled; the version numbers in the roadmap are placeholders that the per-version scope documents decide. Decided items are marked ✔; everything else is an assumption to be checked when the corresponding version is scoped.

### The scenario

A game built with [Godot](https://godotengine.org/) (4.x, MIT) runs on Musha-OS from the USB drive after modest changes on the game side: the engine's platform layer (`OS`, `DisplayServer`, `AudioDriver`, rendering backend) is ported once, the game's `.pck` is read from the drive, and the game uses the keyboard, mouse, BLE keyboard and the Xbox controller through the Musha input events. This is the same shape as a console port of Godot.

### The GPU question ✔

Musha-OS does **not** take on a GPU driver in the 0.x series. The three options were weighed:

1. A real GPU driver plus a Vulkan or OpenGL implementation (Intel Gen8 on NUC5 is the best-documented candidate). Out of reach: the Mesa driver for one generation alone is far larger than this project.
2. **A CPU rasterizer that provides OpenGL 3.3 to Godot's Compatibility renderer ✔.** Mesa's `softpipe` (no LLVM dependency) is the candidate. Slow but usable for 2D and light 3D on the Ryzen 7 machine; low resolution on NUC5.
3. A software implementation of Godot's `RenderingDevice`. Rejected: it amounts to writing a SPIR-V interpreter (SwiftShader).

Consequence: the screen path for games is CPU rendering into an arena back buffer, presented with `SYS_DISPLAY_PRESENT` (v0.5.0 goal 5). GPU acceleration, if ever, starts from Intel Gen8 2D blits and is a separate decision.

### What Godot needs from the OS and SDK

| Need | Musha-OS today | Fill-in |
|---|---|---|
| C/C++ standard library | none (no libc) | **libc layer**: musl (MIT) ported onto Musha syscalls; static only |
| C++ runtime | none | static libstdc++ or libc++ with RTTI; exceptions not required (Godot builds with `-fno-exceptions`) |
| Memory | arena ≤ 64 MiB, bump allocation in SDK | real allocator (dlmalloc-class) over the arena; **arena limit raised to several GiB** |
| Threads | none | Godot's Web build is single-threaded; start without threads (explicit single-thread contract), cooperative threads later if needed |
| Time | `SYS_TIME` in ms | microsecond resolution (`OS::get_ticks_usec`) |
| Files | none (v0.5.0: read-only open/size/read/close) | read-only `.pck` access is enough; large reads at offset |
| Audio | none (v0.5.0: one 48 kHz/16-bit/2-ch output stream) | Godot mixes internally; one stream is enough; latency target ≤ 40 ms |
| Input | `SYS_INPUT` keyboard/mouse/gamepad (v0.4.0) | map to `DisplayServer` input events; gamepad axes/buttons from the Xbox controller |
| Display | `log` console only (v0.5.0: `SYS_DISPLAY_INFO/PRESENT`) | GL 3.3 on CPU (softpipe) presenting to the back buffer |
| Random, query | v0.5.0 r3 | — |
| Networking | none for applications | not required for the scenario |
| File writing (saves) | none; OS writes only `SETTINGS/` | a bounded application save file under a designated directory is a later decision |

### Roadmap (placeholders)

| Version | Content | Meaning for the scenario |
|---|---|---|
| 0.5.0 (scoped) | display present, read-only files, audio, query, random, SDK runtime | display, sound and data paths exist |
| 0.6.0 (candidate) | libc layer (musl), C++ runtime, real allocator, arena limit raised, µs time, single-thread contract; **SDL2 port** as the proving target (software renderer + Musha video/audio/input backends) | large C/C++ programs run; many existing 2D games run through SDL before Godot does |
| 0.7.0 (candidate) | Mesa `softpipe` providing GL 3.3 on the CPU | prerequisite of Godot's Compatibility renderer |
| later | Godot platform port: single-threaded, Compatibility renderer, `.pck` from the drive, `SYS_AUDIO`, `SYS_INPUT` | scenario reached |

### Licensing

Godot, Mesa, musl and SDL are MIT-family licenses and fit the existing third-party policy (`docs/third-party.md`): record source, version and changes; keep license texts under `DOCS/third_party/`.

### Open questions (to answer when each version is scoped)

1. Whether the libc layer is exposed as part of the SDK (a `musha-os-<ver>-libc.zip`) or built into the C SDK.
2. Memory model above a few GiB: arena growth vs. a second mapping call.
3. Whether saves need an application-owned write path, and under which directory.
4. Frame pacing: vsync is not available through GOP; present timing is application-driven.

## 日本語

状態：**シナリオであり、公開範囲ではない。** 2026-10-15 に所有者が、将来の作業順序を決める指針として採用。ここに予定されたものはなく、ロードマップの版番号は仮置きで、各版の範囲文書が決める。決定済みは ✔、それ以外は各版の範囲を決める時に検証する仮定。

### シナリオ

[Godot](https://godotengine.org/)（4.x、MIT）で作ったゲームが、ゲーム側の小さな手直しで USB から Musha-OS 上で動く。エンジンのプラットフォーム層（`OS`、`DisplayServer`、`AudioDriver`、描画バックエンド）を一度移植し、ゲームの `.pck` を USB から読み、キーボード・マウス・BLE キーボード・Xbox コントローラーを Musha の入力イベント経由で使う。Godot の家庭用機移植と同じ形。

### GPU の扱い ✔

Musha-OS は 0.x 系で GPU ドライバーを**持たない**。3 つの選択肢を比較した。

1. 本物の GPU ドライバーと Vulkan/OpenGL 実装（NUC5 の Intel Gen8 が最も資料の揃う候補）。1 世代分の Mesa ドライバーだけで本プロジェクトより遥かに大きく、手が届かない。
2. **CPU ラスタライザーで Godot の Compatibility レンダラーに OpenGL 3.3 を提供する ✔。** 候補は Mesa の `softpipe`（LLVM 不要）。遅いが、Ryzen 7 機なら 2D と軽い 3D に使え、NUC5 では低解像度に限る。
3. Godot の `RenderingDevice` をソフトウェア実装する。SPIR-V 解釈器（SwiftShader 相当）を書くことになるため却下。

帰結：ゲームの画面経路は arena 内のバックバッファーへの CPU 描画と `SYS_DISPLAY_PRESENT`（v0.5.0 目標 5）。GPU 加速は、やるとしても Intel Gen8 の 2D ブリットから始める別の判断。

### Godot が OS と SDK に要求するもの

| 要求 | 現在の Musha-OS | 埋めるもの |
|---|---|---|
| C/C++ 標準ライブラリー | 無し（libc 無し） | **libc 層**：musl（MIT）を Musha の syscall 上に移植。静的リンクのみ |
| C++ ランタイム | 無し | libstdc++ か libc++ の静的リンクと RTTI。例外は不要（Godot は `-fno-exceptions`） |
| メモリー | arena ≤ 64 MiB、SDK は bump 割当て | arena 上の本物のアロケーター（dlmalloc 級）。**arena 上限を数 GiB に引上げ** |
| スレッド | 無し | Godot の Web 向けビルドは単一スレッド。まずスレッド無し（単一スレッド契約を明示）、必要なら後で協調スレッド |
| 時刻 | `SYS_TIME` は ms | µs 精度（`OS::get_ticks_usec`） |
| ファイル | 無し（v0.5.0：読出し専用 open/size/read/close） | `.pck` の読出しで足りる。オフセット指定の大きな読出し |
| 音声 | 無し（v0.5.0：48 kHz/16 bit/2 ch の出力 1 本） | Godot は内部でミキシングするので 1 本で足りる。遅延目標 ≤ 40 ms |
| 入力 | `SYS_INPUT` のキーボード・マウス・ゲームパッド（v0.4.0） | `DisplayServer` の入力イベントへ写像。Xbox コントローラーの軸・ボタン |
| 画面 | `log` のコンソールのみ（v0.5.0：`SYS_DISPLAY_INFO/PRESENT`） | CPU 上の GL 3.3（softpipe）がバックバッファーへ描き present |
| 乱数・問合せ | v0.5.0 r3 | — |
| 通信 | アプリ向けは無し | シナリオには不要 |
| ファイル書込み（セーブ） | 無し。OS は `SETTINGS/` にのみ書く | 指定ディレクトリー下の上限付きアプリ保存ファイルは後の判断 |

### ロードマップ（仮置き）

| 版 | 内容 | シナリオにとっての意味 |
|---|---|---|
| 0.5.0（範囲決定済み） | 画面 present、読出し専用ファイル、音声、問合せ、乱数、SDK ランタイム | 画面・音・データの経路が揃う |
| 0.6.0（候補） | libc 層（musl）、C++ ランタイム、本物のアロケーター、arena 上限引上げ、µs 時刻、単一スレッド契約。検証対象として **SDL2 の移植**（ソフトウェアレンダラー＋Musha のビデオ・音声・入力バックエンド） | 大きな C/C++ プログラムが動く。Godot より先に既存の 2D ゲームの多くが SDL 経由で動く |
| 0.7.0（候補） | Mesa `softpipe` で CPU 上に GL 3.3 | Godot Compatibility レンダラーの前提 |
| その後 | Godot プラットフォーム層の移植：単一スレッド、Compatibility、USB からの `.pck`、`SYS_AUDIO`、`SYS_INPUT` | シナリオの達成 |

### ライセンス

Godot・Mesa・musl・SDL はいずれも MIT 系で、既存の第三者コード方針（`docs/third-party.md`）で扱える：採用元・版・変更内容を記録し、ライセンス本文を `DOCS/third_party/` に置く。

### 未解決の問い（各版の範囲を決める時に答える）

1. libc 層を SDK の一部として配布するか（`musha-os-<ver>-libc.zip`）、C SDK に組み込むか。
2. 数 GiB を超えるメモリーモデル：arena の拡張か、第 2 のマッピング呼出しか。
3. セーブにアプリ所有の書込み経路が要るか、要るならどのディレクトリー下か。
4. フレームの同期：GOP では vsync が得られないため、present のタイミングはアプリ主導。
