# Long-term scenario: MAME on Musha-OS

## English

Status: **scenario, not a release scope.** Adopted by the owner on 2026-10-16 as the second application target of the [games scenario](scenario-games-godot.md), placed **between the SDL2 port and Mesa `softpipe`** in the roadmap. Nothing here is scheduled; version numbers are placeholders that the per-version scope documents decide. Decided items are marked ✔; everything else is an assumption to be checked when the corresponding version is scoped.

### The scenario

[MAME](https://www.mamedev.org/) (GPL-2.0-or-later, with BSD-3-Clause core parts) runs on Musha-OS from the USB drive as one statically linked application: a Musha OSD layer is written once, the ROM sets are read from the drive, and the emulated machine uses the keyboard, mouse, BLE keyboard and the Xbox controller through Musha input events. MAME is a good fit because its core never touches a GPU: every emulated machine renders into software bitmaps, and the OSD only copies them to the screen. It therefore needs **no `softpipe` and no OpenGL**, only `SYS_DISPLAY_PRESENT`, `SYS_AUDIO` and `SYS_INPUT`, and is reachable one step earlier than Godot.

### Why it is realistic ✔

| MAME layer | What it needs | Musha-OS source |
|---|---|---|
| Core (`emu/`, `devices/`, `lib/util/`) | C++17 with RTTI and exceptions, `std::thread`/`std::mutex`, a general-purpose allocator, zip/7z reading | libc + C++ runtime of v0.6.0 ([POSIX profile](posix-profile.md)); cooperative threads inside libc; `lib/util` has its own zip/7z and needs only `fopen`/`fseek`/`fread` |
| OSD video | a frame buffer of the emulated screen, RGB32, one present per frame | arena back buffer + `SYS_DISPLAY_PRESENT` (v0.5.0 goal 5); no GL |
| OSD sound | one stream, 48 kHz/16-bit/2-ch, with a small ring buffer | `SYS_AUDIO` (v0.5.0 goal 3) |
| OSD input | keyboard matrix, relative mouse, joystick axes/buttons | `SYS_INPUT` (v0.4.0): keyboard, relative mouse, Xbox gamepad |
| OSD time | microsecond `osd_ticks()` | `SYS_TIME` raised to µs (v0.6.0 candidate) |
| OSD files | ROM zips, read-only; `cfg/`, `nvram/`, snapshots, write | read-only file API (v0.5.0 goal 4) for ROMs; writes are an EROFS no-op in POSIX p1 |

MAME already ships an `osdmini` OSD that stubs every platform service; its shape (`video_init`, `sound_init`, `input_init`, `osd_ticks`, file I/O through the `osd_file` interface) is the template for `osd/musha`. The work is mostly glue, not new OS features.

### What Musha-OS has to add

The fill-ins beyond what the games scenario already asks for:

1. **`IMAGE_LIMIT` raised from 16 MiB** (`application-abi.md`). A MAME subset build with a few dozen drivers is several tens of MiB of `.text` and `.rodata`; the full build is hundreds of MiB. The ELF loader's image window must become at least 256 MiB, which is an ABI revision (the loader already maps segments on demand, so this is a limit change rather than a new mechanism).
2. **C++ exceptions and RTTI in the C++ runtime.** Godot can build with `-fno-exceptions`; MAME cannot (`emu_fatalerror`, `std::bad_alloc`). The v0.6.0 C++ runtime must include the unwinder (`libgcc_eh` or `libunwind`) and `.eh_frame` registration.
3. **Cooperative `std::thread`.** MAME uses threads for the work queue and for sound; `-numprocessors 1` keeps them to a handful that mostly sleep. The libc cooperative pthreads of POSIX p1 are enough as long as `std::thread`, `std::mutex`, `std::condition_variable` and `std::atomic` map onto them (libstdc++'s `gthr` layer).
4. **Arena raised to several GiB** (already a Godot item): MAME needs 256 MiB–1 GiB for typical arcade systems, more for CD-based ones.
5. **A writable application directory (POSIX p2).** `nvram/`, `cfg/`, `diff/` and `sta/` are written at exit; without them MAME still runs but forgets settings and high scores. Reuse the bounded-write design of `SETTINGS/` for a `APPS/<name>/` directory decided in p2; until then these writes return EROFS and the OSD treats them as advisory.
6. **Build system.** MAME uses GENie to generate makefiles; `osd/musha` needs a `TARGETOS=musha` entry with the Musha cross toolchain (`x86_64-musha-elf`, static, no PIC), and the SDL/Qt/OpenGL dependencies disabled. This is the largest piece of unknown work and should be the first thing tried once the v0.6.0 toolchain exists.

### Performance expectations

MAME is CPU-bound by design; the Musha-OS build is single-core without SMP, so performance equals one core at full speed. NUC5 (i5-5250U) runs 1980s and most early-1990s arcade systems at full speed; 3D arcade boards of the late 1990s do not run at speed on any single core. The Ryzen 7 5800X runs most 2D systems with headroom. `SYS_DISPLAY_PRESENT` at the native panel resolution with integer scaling is the planned path; MAME's own scaling runs on the CPU.

### Licensing ✔

MAME is GPL-2.0-or-later. It is **never linked into the kernel or the SDK** and is distributed, if at all, as a separate application image with its own source offer, so the kernel's Apache-2.0 license and the [third-party policy](third-party.md) are unaffected. The `osd/musha` layer is contributed under MAME's own license. ROMs are not distributed by this project: the scenario covers only machines whose ROMs the user holds legally, and the documentation says so.

### Roadmap (placeholders, revised order ✔)

| Version | Content | Meaning for the scenario |
|---|---|---|
| 0.5.0 (scoped) | display present, read-only files, audio, query, random, SDK runtime | display, sound and ROM paths exist |
| 0.6.0 (candidate) | libc layer, C++ runtime **with exceptions and RTTI**, real allocator, arena limit raised, **`IMAGE_LIMIT` raised**, µs time, cooperative `std::thread`; **SDL2 port** as the proving target | large C/C++ programs run |
| 0.6.x or 0.7.0 (candidate) | **MAME `osd/musha`**: subset build (a few dozen 2D drivers), ROMs from the drive, `SYS_AUDIO`, `SYS_INPUT`; POSIX p2 writable application directory for `nvram/`/`cfg/` | first large existing program runs unchanged in its core |
| 0.7.0 (candidate) | Mesa `softpipe` providing GL 3.3 on the CPU | prerequisite of Godot's Compatibility renderer |
| later | Godot platform port | games scenario reached |

### Open questions (to answer when each version is scoped)

1. New `IMAGE_LIMIT` value and whether the loader maps the image lazily or in full at start.
2. Whether the Musha C++ runtime is libstdc++ (GCC cross toolchain) or libc++ (Clang); MAME builds with both.
3. Whether the writable application directory is a p2 profile item or a Musha-specific `SYS_FILE_WRITE` with a per-application quota.
4. Minimum useful driver subset for the first build and the acceptance ROM (a free, redistributable test ROM such as the MAME test suite's own, not a commercial game).

## 日本語

状態：**シナリオであり、公開範囲ではない。** 2026-10-16 に所有者が、[ゲーム用途シナリオ](scenario-games-godot.md)の 2 つ目のアプリ目標として採用し、ロードマップ上で **SDL2 移植と Mesa `softpipe` の間**に置いた。ここに予定されたものはなく、版番号は仮置きで、各版の範囲文書が決める。決定済みは ✔、それ以外は各版の範囲を決める時に検証する仮定。

### シナリオ

[MAME](https://www.mamedev.org/)（GPL-2.0-or-later、コアの一部は BSD-3-Clause）が、静的リンクされた 1 本のアプリとして USB から Musha-OS 上で動く。Musha 用の OSD 層を一度書き、ROM セットを USB から読み、エミュレートされた機械がキーボード・マウス・BLE キーボード・Xbox コントローラーを Musha の入力イベント経由で使う。MAME が向いているのは、コアが GPU に一切触れないため：全てのエミュレート対象はソフトウェアのビットマップに描画し、OSD はそれを画面へ写すだけ。したがって **`softpipe` も OpenGL も不要**で、`SYS_DISPLAY_PRESENT`・`SYS_AUDIO`・`SYS_INPUT` だけで足り、Godot より 1 段早く到達できる。

### 現実的である理由 ✔

| MAME の層 | 要求 | Musha-OS 側の供給元 |
|---|---|---|
| コア（`emu/`、`devices/`、`lib/util/`） | RTTI と例外を含む C++17、`std::thread`/`std::mutex`、汎用アロケーター、zip/7z の読出し | v0.6.0 の libc と C++ ランタイム（[POSIX プロファイル](posix-profile.md)）。libc 内の協調スレッド。`lib/util` は自前の zip/7z を持ち、`fopen`/`fseek`/`fread` だけ要る |
| OSD 映像 | エミュレート画面のフレームバッファー（RGB32）をフレームごとに 1 回 present | arena のバックバッファー＋`SYS_DISPLAY_PRESENT`（v0.5.0 目標 5）。GL 不要 |
| OSD 音声 | 48 kHz/16 bit/2 ch のストリーム 1 本と小さなリングバッファー | `SYS_AUDIO`（v0.5.0 目標 3） |
| OSD 入力 | キーボードマトリクス、相対マウス、ジョイスティックの軸・ボタン | `SYS_INPUT`（v0.4.0）：キーボード、相対マウス、Xbox ゲームパッド |
| OSD 時刻 | µs 単位の `osd_ticks()` | `SYS_TIME` の µs 化（v0.6.0 候補） |
| OSD ファイル | ROM の zip は読出し専用。`cfg/`、`nvram/`、スナップショットは書込み | ROM は読出し専用ファイル API（v0.5.0 目標 4）。書込みは POSIX p1 では EROFS |

MAME には全てのプラットフォーム機能をスタブにした `osdmini` という OSD が同梱されており、その形（`video_init`、`sound_init`、`input_init`、`osd_ticks`、`osd_file` インターフェース経由のファイル I/O）が `osd/musha` の雛形になる。作業の大半は接着であり、新しい OS 機能ではない。

### Musha-OS 側で追加が必要なもの

ゲーム用途シナリオが既に要求しているものに加えて：

1. **`IMAGE_LIMIT` を 16 MiB から引上げ**（`application-abi.md`）。数十ドライバーに絞った MAME でも `.text` と `.rodata` で数十 MiB、全部入りは数百 MiB。ELF ローダーのイメージ窓を少なくとも 256 MiB にする必要があり、これは ABI 改版になる（ローダーはセグメント単位で写像しているので、新機構ではなく上限の変更）。
2. **C++ ランタイムの例外と RTTI。** Godot は `-fno-exceptions` でビルドできるが、MAME はできない（`emu_fatalerror`、`std::bad_alloc`）。v0.6.0 の C++ ランタイムにはアンワインダー（`libgcc_eh` か `libunwind`）と `.eh_frame` の登録が要る。
3. **協調型の `std::thread`。** MAME はワークキューと音声にスレッドを使う。`-numprocessors 1` で数本に抑えられ、大半は眠っている。POSIX p1 の libc 協調 pthread で足りるが、`std::thread`・`std::mutex`・`std::condition_variable`・`std::atomic` がその上に写ること（libstdc++ の `gthr` 層）が条件。
4. **arena を数 GiB に**（Godot の項目と共通）：MAME は一般的なアーケード基板で 256 MiB〜1 GiB、CD 系はそれ以上。
5. **書込み可能なアプリ用ディレクトリー（POSIX p2）。** `nvram/`、`cfg/`、`diff/`、`sta/` は終了時に書かれる。無くても動くが設定とハイスコアを忘れる。`SETTINGS/` の上限付き書込み設計を p2 で決める `APPS/<名前>/` に流用する。それまでは EROFS を返し、OSD は書込み失敗を致命としない。
6. **ビルドシステム。** MAME は GENie で makefile を生成する。`osd/musha` には `TARGETOS=musha` の項目と Musha のクロスツールチェーン（`x86_64-musha-elf`、静的、PIC 無し）が要り、SDL/Qt/OpenGL 依存を無効にする。ここが未知の作業で最も大きく、v0.6.0 のツールチェーンができた時点で最初に試すべき。

### 性能の見込み

MAME は設計上 CPU 律速で、Musha-OS は SMP 無しの単一コアなので、性能は 1 コアの全速に等しい。NUC5（i5-5250U）は 1980 年代と 1990 年代前半の大半のアーケード基板を実速で動かせる。1990 年代後半の 3D 基板はどの単一コアでも実速にならない。Ryzen 7 5800X は 2D 系の大半を余裕で動かす。画面はパネル解像度の `SYS_DISPLAY_PRESENT` に整数倍で載せる予定で、MAME 自身の拡大処理は CPU 上。

### ライセンス ✔

MAME は GPL-2.0-or-later。**カーネルにも SDK にも決してリンクせず**、配布するとしても独自のソース提供を伴う別のアプリイメージとして配布するので、カーネルの Apache-2.0 と[第三者コード方針](third-party.md)には影響しない。`osd/musha` 層は MAME 自身のライセンスで提供する。ROM は本プロジェクトでは配布しない：シナリオは利用者が合法に保有する ROM の機械だけを対象とし、文書にそう明記する。

### ロードマップ（仮置き。順序を改訂 ✔）

| 版 | 内容 | シナリオにとっての意味 |
|---|---|---|
| 0.5.0（範囲決定済み） | 画面 present、読出し専用ファイル、音声、問合せ、乱数、SDK ランタイム | 画面・音・ROM の経路が揃う |
| 0.6.0（候補） | libc 層、**例外と RTTI 付き**の C++ ランタイム、本物のアロケーター、arena 上限引上げ、**`IMAGE_LIMIT` 引上げ**、µs 時刻、協調型 `std::thread`。検証対象として **SDL2 の移植** | 大きな C/C++ プログラムが動く |
| 0.6.x か 0.7.0（候補） | **MAME `osd/musha`**：数十の 2D ドライバーに絞ったビルド、USB からの ROM、`SYS_AUDIO`、`SYS_INPUT`。`nvram/`/`cfg/` 用の POSIX p2 書込み可能アプリディレクトリー | 大きな既存プログラムがコア無改変で初めて動く |
| 0.7.0（候補） | Mesa `softpipe` で CPU 上に GL 3.3 | Godot Compatibility レンダラーの前提 |
| その後 | Godot プラットフォーム層の移植 | ゲーム用途シナリオの達成 |

### 未解決の問い（各版の範囲を決める時に答える）

1. 新しい `IMAGE_LIMIT` の値と、ローダーがイメージを遅延で写すか起動時に全部写すか。
2. Musha の C++ ランタイムを libstdc++（GCC クロス）にするか libc++（Clang）にするか。MAME はどちらでもビルドできる。
3. 書込み可能アプリディレクトリーを p2 プロファイルの項目にするか、アプリごとの上限付き Musha 固有 `SYS_FILE_WRITE` にするか。
4. 最初のビルドに含める最小限のドライバー集合と、受入れに使う ROM（商用ゲームではなく、MAME の試験用など再配布可能な自由な ROM）。
