# Boot log — single sequential console

## English

Status: **owner decision, 2026-10-16.** Replaces every previous boot-time display: the diagnostic panels, the left-column discovery lines, the `PM ms` footer, the colored status words, the red exception band and the separately placed application console. Applies to v0.4.0 (goal 6 is re-done on this specification) and later. Related: [v0.4.0 scope](release-scope-0.4.0.md) goals 6, 7, 16; [v0.5.0 scope](release-scope-0.5.0.md) goal 11 (serial mirror); [startup configuration](startup-configuration.md).

### Principle

The screen during startup is **one text log**: lines appended top to bottom, scrolled up when the bottom is reached, nothing drawn anywhere else. A user sees only what the configured level allows; a developer sees the same log with more lines. There is no second layout for diagnostics.

### 1. Screen and text

- One console covering the whole GOP frame, origin at the top-left cell. Glyphs are the existing 6×8 font at the existing scale (`clamp(round(height/600),1,4)`), row pitch 10×scale. Columns = floor(width / (6×scale)), rows = floor(height / (10×scale)); lines longer than the column count are wrapped at the column boundary.
- **No background fill at startup.** The framebuffer is not cleared; the first line is drawn over whatever the firmware left. Each drawn cell paints its own 6×8 background (black) under the glyph, so text is always readable. Scrolling repaints only the text area that changes.
- **One foreground color (white) on black. No other colors.** Failures are marked by text (`FAIL`, `ERROR`), not color.
- The framebuffer mapping must use a write-combining attribute (PAT WC) when available; UC-only mapping is a defect to fix, not a reason to draw less.

### 2. Line format

```
[  1.609] usb: host ready (xhci 8086:9cb1 rev 03)
```

- At `devices`, every line starts with `[sss.mmm]` seconds since the PM-timer epoch, right-aligned in 9 characters. At `phases`, `ready` and `quiet` the prefix is omitted.
- After the prefix: a lowercase subsystem tag and a colon (`core:`, `memory:`, `timer:`, `usb:`, `storage:`, `keyboard:`, `mouse:`, `lan:`, `bluetooth:`, `display:`, `settings:`, `app:`, `viewer:`), then one sentence in plain ASCII, no trailing period, no uppercase status words.
- Identifiers that a user may need to report (PCI IDs, USB VID:PID, sizes, hashes) are printed in hexadecimal without `0x` for PCI/USB IDs and with `0x` for addresses.
- One event, one line. No multi-line blocks except the application's own `log` output.

### 3. What each level shows

`quiet` — nothing, except required-phase failures (section 5).

`ready` — a single line when all required devices are discovered:

```
Musha-OS 0.4.0 ready
```

`phases` — one line per phase, in order of completion:

```
Musha-OS 0.4.0 starting
core: runtime ready
timer: ready
usb: host ready
storage: boot media ready (28.7 GiB)
keyboard: ready
settings: CONFIG.TXT loaded (log=phases)
lan: not available (no supported controller)
app: starting AUTORUN.ELF
Hello Musha-OS!
app: AUTORUN.ELF exited, status 0
viewer: starting
Musha-OS 0.4.0 ready
```

`devices` — `phases` plus one concise factual line per recognized device or decision, with timestamps. Repeated discovery of the same USB identity is retained at `trace`, not repeated on the devices screen. Settings/GOP/media/LAN phase summaries are replaced by their fuller single factual line at devices; their original phase records remain in trace and the ring:

```
Musha-OS 0.4.0 starting
[  0.012] settings: CONFIG.TXT loaded (log=devices display=max)
[  0.014] display: gop 3840x2160 stride 3840 bgr, grid 160x54 scale 4
[  0.165] core: runtime ready (image 0x140000000 size 0x123b000, arena 64 MiB)
[  0.170] timer: pm epoch ready
[  0.420] pci: 34 devices
[  0.425] usb: xhci 0 at 03:00.3 (1022:149c rev 00)
[  0.426] usb: xhci 1 at 0b:00.0 (1022:149c rev 00)
[  1.102] usb: host ready (2 controllers routed, 3 skipped)
[  1.380] storage: sandisk ultra at xhci 0 port 4 superspeed (0781:5591)
[  1.602] storage: boot media ready (fat32, 28.7 GiB)
[  1.603] storage: README.TXT ok (1296 bytes, fnv 9a42a948c590f507)
[  1.611] keyboard: logitech at xhci 0 port 2 (046d:c31c) boot protocol
[  1.611] mouse: logitech m100 at xhci 0 port 3 (046d:c077) boot protocol
[  1.620] lan: realtek 10ec:8168 rev 15 at 05:00.0, txconfig 0x3c800000
[  1.621] lan: not available (no driver for 10ec:8168)
[  1.700] app: AUTORUN.ELF 1234 bytes, note r2 permissions 15
[  1.702] app: starting AUTORUN.ELF
Hello Musha-OS!
[  1.790] app: AUTORUN.ELF exited, status 0
[  1.795] viewer: starting
Musha-OS 0.4.0 ready
```

The exact devices summaries and trace details are defined by the implementation, and every value that the current diagnostic panels show and that a hardware record needs (PCI vendor:device:revision and BDF, USB VID:PID, port and speed, GOP mode, image/arena ranges, media size, marker hash, PM-epoch times) must appear as a line here. Hardware summaries use devices; complete diagnostic evidence uses trace. Values omitted from concise summaries remain in trace and the ring.

`trace` — `devices` plus every low-level stage, control request, transfer result, timing pair and raw diagnostic field formerly shown at devices. This is the detailed hardware-capture level; F8 / digit 8 forces trace. All five display levels retain every record in the same 64 KiB ring. No trace filter deletes ring content.

The first phases/devices/trace line is exactly `Musha-OS 0.4.0 starting`; the startup-ready terminal line is exactly `Musha-OS 0.4.0 ready`, without timestamps or a tag. Device/trace records retain timestamps. Proper names and app bytes preserve case. UEFI and initialization progress belongs in trace; phases folds it into `core: runtime ready`. The USB phase tag is `usb`; display includes the selected dimensions. LAN reports observable availability/link/address or a concrete failure reason, never a development-certification disclaimer.

### 4. Application output, viewer, return

- `app: starting <name>` is followed directly by the application's `log` bytes, appended to the same console (wrapped at the column boundary; printable ASCII and newline only, as today). The application does not get its own 80×24 area.
- On exit, fault or rejection, the kernel appends `app: <name> exited, status N`, `app: <name> faulted, #GP (vector 13) error 0x18 rip 0x…` or `app: <name> rejected, <reason>`.
- When the viewer starts it owns the whole screen (its own layout, unchanged by this document). When an application is launched from the viewer, the console is cleared to black and the log continues from the top with `app: starting <name>`. When the application returns to the viewer, the viewer repaints its own screen.
- When an application has presented pixels through the future display API (v0.5.0), the console is hidden until the application ends.

### 5. Failures and halts

- A required-phase failure is printed at **every** level, including `quiet`, as `ERROR: <subsystem>: <reason>` followed by `Musha-OS halted` and the machine halts. Example:

```
ERROR: storage: no boot media found (2 controllers, 0 media)
Musha-OS halted
```

- A kernel CPU exception prints `ERROR: cpu: exception #GP (vector 13) error 0x18 rip 0x140056d89 cr2 0x0` (error omitted as `error n/a` for vectors that push none), then `Musha-OS halted`. No red band, no separate box.
- An optional-device failure (LAN, Bluetooth) is one `phases` line with the reason and never halts.

### 6. Scrolling and limits

- When the next line would fall below the last row, the console scrolls up by one row; the top line is lost from the screen but remains in the in-memory ring (section 7).
- Maximum line length before wrapping: the column count. Lines are never truncated silently; a wrapped continuation starts with two spaces.
- No animation, no delays for readability. If a human needs to read a transient line, it is in the ring and on the serial mirror.

### 7. Ring buffer and mirrors

- Every line, regardless of display level, is appended to an in-memory ring (64 KiB) with its timestamp and the level it belongs to. The screen shows the subset the configured level allows; the ring holds everything at `trace` detail.
- The ring is the single source for every mirror: the serial port (v0.5.0 goal 11), the `trace` key override (F8 / 8 selects trace and shows all retained detail), and any future UDP or file sink. A mirror never has its own formatting.

### 8. Removal list

The following are removed, not hidden: the diagnostic panel pages (`diagnostic_pages`, the T2-D1 / EXTERNAL ELF memory page), the left-column `CORE READY` / `DEVICE DISCOVERY` / `PCI DEVICES` / `XHCI` / `INTEL LAN` / `TIMER READY` block, `USB READ OK` / `USB BYTES` / `FAT32 READ OK` / `APP FILE READ OK` words, the `PM ms core=… ready=…` footer, the red `CPU EXCEPTION` band, the `boot_heading` banner, the separately positioned application console and its heading `AUTORUN.ELF (Ring 3)`, the background fill, and all color tables for status words. Documents that describe these (`hardware-diagnostics.md`, `hardware-test-record-template.md`, `display-grid-0.3.0.md`, `external-app.md` console section, QEMU screen references) are updated in the same change. Historical records keep their photographs and are not edited.

### 9. Verification

- QEMU: screen references regenerated for the five levels, for the halt case and for the application exit/fault cases; a text dump of the ring compared as the primary check, pixels as the secondary check.
- Hardware: the next NUC5 and Ryzen runs are photographed at `devices`; the photographs become the new examples in `hardware-test-record-template.md`.

### 10. Implementation decisions and complete devices/trace inventory

The implementation keeps a 65,536-byte ring inside the reserved LoadedImage, with whole-record eviction, a loss counter, level, PM milliseconds and an early/firmware flag. No allocation or firmware handle is retained. Settings and GOP decisions enter it before ExitBootServices; their unavailable PM epoch is represented by zero plus the early flag. This does not claim time since EFI entry. A record exceeding 4,095 formatted bytes becomes an explicit log error, rather than a silently truncated sentence. Application syscall limits and bytes accepted by `SYS_LOG` remain unchanged.

The console uses a fixed CPU text shadow (384 columns by 128 rows), including 4K and 8K GOP grids under the specified scale. It never reads framebuffer pixels to scroll. Binding writes no pixels; only new cells and changed scroll rows are painted. Viewer pixels retain their existing layout. Selecting an ELF from the viewer clears black once, then appends `app: starting <catalog name>`; the catalog name is copied before scoped handles close (non-ASCII units display as dots without changing identity). App exit/fault lines identify the same name and include status or vector/error/RIP. No framebuffer display syscall or new public ABI is introduced.

Only an existing PAT WC entry may be selected for GOP pages. PAT and MTRRs are never changed; WB RAM and strong-UC MMIO/DMA stay unchanged. Missing WC, or CR0 cache-disable/no-write-through state, selects UC fallback and records the reason. This is a documented fallback, not a claim that WC was enabled on an untested machine. Pixel packing, stride, glyph values and bounds remain unchanged. Architectural references: [Intel SDM, volume 3A, sections 13.5.2.2 and 13.12.4](https://cdrdv2-public.intel.com/868137/325462-089-sdm-vol-1-2abcd-3abcd-4.pdf), and [AMD APM volume 2, section 7.8.5](https://docs.amd.com/api/khub/documents/sD1_QL~h4Afq2_tvzxqqSQ/content). Batch SFENCE drains WC stores; it is not vsync. The old UC full-frame fill and repeated whole-console redraw were code-level cost sources; physical elapsed rendering time has not been measured.

`pci`, `device` and `time` are the additional retained tags: PCI inventory, generic legacy diagnostic fields, and timing records respectively. `xhci` is folded into `usb`, GOP into `display`, and firmware/runtime progress into `core`; the untagged product header replaces `boot`. The complete tag set is `core`, `memory`, `timer`, `usb`, `storage`, `keyboard`, `mouse`, `lan`, `bluetooth`, `display`, `settings`, `app`, `viewer`, `pci`, `device`, `time`. Device/trace kernel events have timestamps; raw app bytes are not rewritten. Mandatory `ERROR` and the product name may contain uppercase letters. The `MUSHA: LOG` / retained `MUSHA: RING` development debug-port capture is the primary QEMU evidence, not an implemented serial output feature.

All previously diagnostic values feed this same log, including:

| Producer | Complete retained field groups / physical evidence |
|---|---|
| Boot/settings/GOP | Read/close/default/fallback/F8-or-8 decisions; log/display/pair; dimensions, stride, pixel format, base, bytes; selected PAT index/type/fallback |
| CPU/core/memory | CPU-table and paging decisions; memory-map size/stride/version and rejected descriptors/attributes; image/table/stack/emergency/USB-DMA/LAN-DMA/arena base and size; physical range/overlap/permission decisions |
| PCI inventory | Every present BDF, vendor:device, revision and class; xHCI index/firmware selection, total/limit, backend identity and resource decision |
| USB topology | Controller BDF; root port, route, slot, VID:PID, speed, class/subclass/protocol; recognized keyboard/mouse/BOT/hub/HCI kind; hub generation/depth/reset; selected input/storage controller and scan result |
| USB transport/failures | Stage and failure chronology; control request/index/length/slot; completion code/residual/slot/endpoint, expected TRB address and all four raw event words; stopped/BME-disabled or unconfirmed/quarantined result |
| Storage/file/marker | Block size/count/capacity; partition/filesystem/mount/read decisions; README byte length and full build marker hash; missing/unsupported/mismatch; writer preflight/ownership/bounds/write/sync/failure result |
| LAN | PCI/backend, reset/PHY/NVM/software-configuration refusal, five-second initialization budget reason, link and initialization duration; DHCP state/address/acquisition duration, UDP echo result/counters/ring wrap; stop/restart/quarantine |
| Bluetooth/input | HCI discovery/reset/transport/session/generation decisions; pairing entry/gate/cancel/timeout/save outcome; supported report/notification/input kind and unsupported reason; never key/passkey/secret material |
| Realtek read-only capture | BDF, vendor:device:revision, subsystem, PCI command; BAR0/BAR2 raw/type/base/safe resource length; UC/RO/NX/decode/identity decision; exactly-one TxConfig40 read or explicit skip reason; no BME/MMIO write |
| T2/Mac diagnostics | SMBIOS identity; PCI/BCE and inherited state; protocol/resource/ownership evidence and refusal; unconnected BCE and unverified storage remain explicit |
| Runtime/app/timing | Startup milestones, PM epoch timestamps and duration pairs; file stream identity/cancellation; app loading/validation/start/exit/reject/fault; DMA stop before scrub/reuse and viewer restart |

Actual PCI revisions, safe BAR resource values, Realtek TxConfig40, physical USB speed/topology, I218 link/NVM behavior and Bluetooth device compatibility still require their machine records. Fixture values are not physical measurements. No field is moved into a hidden secondary panel. Repeated producer events may evict old ring records; the loss counter makes that bounded-history limit explicit.

## 日本語

状態：**所有者決定、2026-10-16。** 従来の起動時表示（診断パネル、左列の探索表示、`PM ms` の下端行、色付きの状態語、赤い例外帯、別配置のアプリコンソール）を全て置き換える。v0.4.0（目標 6 をこの仕様でやり直す）以降に適用。関連：[v0.4.0 範囲](release-scope-0.4.0.md) 目標 6・7・16、[v0.5.0 範囲](release-scope-0.5.0.md) 目標 11（シリアル複写）、[起動設定](startup-configuration.md)。

### 原則

起動中の画面は **1 本のテキストログ**である。行は上から下へ追記され、下端に達したら上へスクロールし、他の場所には何も描かない。利用者は設定したレベルが許す行だけを見、開発者は同じログを行数を増やして見る。診断用の別レイアウトは持たない。

### 1. 画面と文字

- GOP の全画面を 1 つのコンソールとし、原点は左上のセル。字形は既存の 6×8 フォントと既存の倍率（`clamp(round(高さ/600),1,4)`）、行ピッチは 10×倍率。桁数 = floor(幅 / (6×倍率))、行数 = floor(高さ / (10×倍率))。桁数を超える行は桁境界で折り返す。
- **起動時に背景を塗りつぶさない。** フレームバッファーは消去せず、最初の行をファームウェアが残した画面の上に描く。描くセルごとに 6×8 の背景（黒）を字形の下に塗るので、文字は常に読める。スクロール時は変化するテキスト領域だけを描き直す。
- **前景は白 1 色、背景は黒。他の色は使わない。** 失敗は色ではなく文字（`FAIL`、`ERROR`）で示す。
- フレームバッファーの map は可能なら write-combining 属性（PAT WC）を使う。UC のみの map は直すべき欠陥であり、描画を減らす理由にしない。

### 2. 行の書式

```
[  1.609] usb: host ready (xhci 8086:9cb1 rev 03)
```

- `devices` では各行の先頭に PM タイマー起点からの秒 `[sss.mmm]` を 9 桁右寄せで付ける。`phases`・`ready`・`quiet` では付けない。
- 接頭辞の後に小文字のサブシステム名とコロン（`core:`、`memory:`、`timer:`、`usb:`、`storage:`、`keyboard:`、`mouse:`、`lan:`、`bluetooth:`、`display:`、`settings:`、`app:`、`viewer:`）、続けて ASCII の 1 文。末尾のピリオドは付けず、大文字の状態語は使わない。
- 利用者が報告に使う識別子（PCI ID、USB VID:PID、サイズ、ハッシュ）は 16 進で、PCI/USB ID は `0x` 無し、アドレスは `0x` 付き。
- 1 事象 1 行。アプリ自身の `log` 出力以外に複数行のブロックは無い。

### 3. レベルごとの表示

`quiet` — 何も出さない。ただし必須フェーズの失敗（第 5 節）は出す。

`ready` — 必須機器の探索が完了した時点で 1 行：

```
Musha-OS 0.4.0 ready
```

`phases` — フェーズごとに 1 行、完了順：

```
Musha-OS 0.4.0 starting
core: runtime ready
timer: ready
usb: host ready
storage: boot media ready (28.7 GiB)
keyboard: ready
settings: CONFIG.TXT loaded (log=phases)
lan: not available (no supported controller)
app: starting AUTORUN.ELF
Hello Musha-OS!
app: AUTORUN.ELF exited, status 0
viewer: starting
Musha-OS 0.4.0 ready
```

`devices` — 認識した機器と判断ごとに1行、時刻付き。settings／GOP／media／LANのphase重複は対応する詳細1行へ統合し、元phaseはtrace／ringへ保持：

```
Musha-OS 0.4.0 starting
[  0.012] settings: CONFIG.TXT loaded (log=devices display=max)
[  0.014] display: gop 3840x2160 stride 3840 bgr, grid 160x54 scale 4
[  0.165] core: runtime ready (image 0x140000000 size 0x123b000, arena 64 MiB)
[  0.170] timer: pm epoch ready
[  0.420] pci: 34 devices
[  0.425] usb: xhci 0 at 03:00.3 (1022:149c rev 00)
[  0.426] usb: xhci 1 at 0b:00.0 (1022:149c rev 00)
[  1.102] usb: host ready (2 controllers routed, 3 skipped)
[  1.380] storage: sandisk ultra at xhci 0 port 4 superspeed (0781:5591)
[  1.602] storage: boot media ready (fat32, 28.7 GiB)
[  1.603] storage: README.TXT ok (1296 bytes, fnv 9a42a948c590f507)
[  1.611] keyboard: logitech at xhci 0 port 2 (046d:c31c) boot protocol
[  1.611] mouse: logitech m100 at xhci 0 port 3 (046d:c077) boot protocol
[  1.620] lan: realtek 10ec:8168 rev 15 at 05:00.0, txconfig 0x3c800000
[  1.621] lan: not available (no driver for 10ec:8168)
[  1.700] app: AUTORUN.ELF 1234 bytes, note r2 permissions 15
[  1.702] app: starting AUTORUN.ELF
Hello Musha-OS!
[  1.790] app: AUTORUN.ELF exited, status 0
[  1.795] viewer: starting
Musha-OS 0.4.0 ready
```

devicesは機器又は判断ごとの事実を1行へ要約し、同じUSB identityの反復探索はtraceへ保持します。PCI vendor:device:revision／BDF、USB VID:PID／port／speed、GOP、image／arena、媒体、marker hash、PM時刻を含む全診断値はdevices要約又はtrace詳細へ記録します。機器の要約写真はdevices、完全な診断証拠はtraceで採取します。

`trace` は `devices` に加え、低水準stage・制御転送・transfer結果・計測対・旧診断fieldを全て表示する詳細採取レベルです。F8／数字8はtraceへ強制します。5表示レベルすべてで同じ64KiB ringへ全recordを保持し、trace filterで履歴を削除しません。

phases／devices／traceの先頭はexact `Musha-OS 0.4.0 starting`、起動完了末尾はexact `Musha-OS 0.4.0 ready` とし、この2行にはtimestamp／tagを付けません。機器／trace recordのtimestampは保持し、固有名とapp byteの大文字小文字を変更しません。UEFIと初期化の逐次経過はtraceへ置き、phasesでは `core: runtime ready` に畳みます。USB phase tagはusb、displayは実選択解像度を含みます。LANは実際の可否／link／address又は具体的失敗理由を示し、開発認証の注記を画面へ出しません。

### 4. アプリ出力、ビューアー、復帰

- `app: starting <名前>` の直後に、アプリの `log` のバイト列を同じコンソールへ追記する（桁境界で折り返し。表示可能 ASCII と改行のみ、現行どおり）。アプリに専用の 80×24 領域は与えない。
- 終了・例外・拒否時にカーネルが `app: <名前> exited, status N`、`app: <名前> faulted, #GP (vector 13) error 0x18 rip 0x…`、`app: <名前> rejected, <理由>` を追記する。
- ビューアーが起動すると画面全体をビューアーが所有する（レイアウトは本文書の対象外で変更しない）。ビューアーからアプリを起動したときはコンソールを黒で消去し、`app: starting <名前>` から先頭に続きを書く。アプリがビューアーへ戻るとビューアーが自分の画面を描き直す。
- 将来の画面 API（v0.5.0）でアプリが画素を present した後は、アプリ終了までコンソールを隠す。

### 5. 失敗と停止

- 必須フェーズの失敗は `quiet` を含む**全レベル**で `ERROR: <サブシステム>: <理由>` と `Musha-OS halted` を出して停止する。例：

```
ERROR: storage: no boot media found (2 controllers, 0 media)
Musha-OS halted
```

- カーネルの CPU 例外は `ERROR: cpu: exception #GP (vector 13) error 0x18 rip 0x140056d89 cr2 0x0`（エラーコードを積まないベクターでは `error n/a`）の後に `Musha-OS halted`。赤帯や別枠は使わない。
- 任意機器（LAN、Bluetooth）の失敗は理由付きの `phases` 行 1 本で、停止しない。

### 6. スクロールと上限

- 次の行が最終行より下になるときは 1 行分上へスクロールする。先頭行は画面から消えるが、メモリー上のリング（第 7 節）には残る。
- 折り返し前の行長の上限は桁数。黙って切り捨てることはせず、折り返しの継続行は空白 2 つで始める。
- アニメーションも、読みやすさのための待ちも入れない。一瞬で流れる行を人が読む必要があるなら、それはリングとシリアル複写にある。

### 7. リングバッファーと複写先

- 表示レベルに関係なく、全ての行を時刻と所属レベル付きでメモリー上のリング（提案：64 KiB）に追記する。画面は設定レベルが許す部分集合を表示し、リングは `trace` の詳細で全てを保持する。
- リングは全ての複写先の唯一の源である：シリアルポート（v0.5.0 目標 11）、`trace` 強制キー（F8／8 で trace を選択して全詳細を表示）、将来の UDP やファイルへの出力。複写先が独自の書式を持つことはない。

### 8. 撤去するもの

次のものは隠すのではなく撤去する：診断パネルの各ページ（`diagnostic_pages`、T2-D1／EXTERNAL ELF のメモリーページ）、左列の `CORE READY`／`DEVICE DISCOVERY`／`PCI DEVICES`／`XHCI`／`INTEL LAN`／`TIMER READY` のブロック、`USB READ OK`／`USB BYTES`／`FAT32 READ OK`／`APP FILE READ OK` の語、`PM ms core=… ready=…` の下端行、赤い `CPU EXCEPTION` 帯、`boot_heading` のバナー、別配置のアプリコンソールとその見出し `AUTORUN.ELF (Ring 3)`、背景の塗りつぶし、状態語の色表。これらを説明している文書（`hardware-diagnostics.md`、`hardware-test-record-template.md`、`display-grid-0.3.0.md`、`external-app.md` のコンソール節、QEMU の画面基準）は同じ変更で更新する。過去の記録は写真を保持し、編集しない。

### 9. 検証

- QEMU：5 レベル、停止ケース、アプリの終了・例外ケースの画面基準を再生成する。主検査はリングのテキストダンプの比較、副検査が画素比較。
- 実機：次の NUC5・Ryzen の起動を `devices` で撮影し、その写真を `hardware-test-record-template.md` の新しい例にする。

### 10. 実装上の判断と devices／trace 全項目

65,536 byte のリングは予約済み LoadedImage 内に置き、レコード単位の追出し、損失数、レベル、PM ミリ秒、firmware 終了前フラグを保持する。動的割当てや firmware handle は保持しない。設定と GOP の判断は ExitBootServices 前に記録し、PM epoch 不明は 0 と early フラグで示す。EFI entry からの時間とは主張しない。整形後 4,095 byte を超えるレコードは明示的なログエラーとし、黙って文を切らない。SYS_LOG の syscall 上限と受理する byte は変えない。

コンソールは CPU 側の固定 shadow（384 列 × 128 行）を持ち、規定倍率の 4K／8K GOP を含む。スクロールで framebuffer を読み戻さない。bind は pixel を書かず、新しい cell とスクロールで変わる文字領域だけ描く。viewer の既存配置は保持する。viewer から ELF を選択した時だけ黒消去し、app: starting <catalog name> と続ける。選択catalog名をfile service終了前にコピーし、非ASCII UTF-16 code unitは表示だけドット化してfile identityを変更しない。終了／fault は同じcatalog名と status または vector／error／RIP を記録する。表示 syscall や公開 ABI は追加しない。

GOP には既存 PAT の WC entry だけを使う。PAT／MTRR を変更せず、WB RAM と strong-UC の MMIO／DMA を保持する。WC が無い場合や CR0 cache-disable／no-write-through 状態では UC に fallback し理由を記録する。これは未試験機で WC を有効化したとの主張ではない。根拠は [Intel SDM volume 3A §13.5.2.2／13.12.4](https://cdrdv2-public.intel.com/868137/325462-089-sdm-vol-1-2abcd-3abcd-4.pdf) と [AMD APM volume 2 §7.8.5](https://docs.amd.com/api/khub/documents/sD1_QL~h4Afq2_tvzxqqSQ/content)。pixel 値、stride、glyph、境界は不変で、batch 後 SFENCE は WC store の完了であり vsync ではない。旧 UC 全画面 fill と console 全再描画はコード上の負荷要因だが、実機描画時間は未測定である。

追加tagはPCI一覧のpci、旧診断fieldのdevice、計測のtimeです。xhciはusb、GOPはdisplay、firmware/runtime経過はcoreへ畳み、bootはtag無し製品見出しへ統合します。全tag集合はcore／memory／timer／usb／storage／keyboard／mouse／lan／bluetooth／display／settings／app／viewer／pci／device／timeです。機器／traceのkernel行はtimestamp付きとし、アプリ生 byte は書換えない。必須 ERROR と製品名の大文字は許す。開発用 debug-port の MUSHA: LOG／MUSHA: RING を QEMU の主証拠とするが、serial 出力実装とは扱わない。

| producer | 保持する全項目群／実機採取値 |
|---|---|
| boot／設定／GOP | 読取り・close・default・fallback・F8/8 の判断、log/display/pair、幅高さ・stride・format・base・bytes、PAT index/type/fallback |
| CPU／core／memory | CPU table・paging、map size/stride/version・拒否 descriptor/attribute、image/table/stack/emergency/USB-DMA/LAN-DMA/arena の base/size、範囲・重複・権限判断 |
| PCI | 全 BDF・vendor:device・revision・class、xHCI index/firmware 選択・総数・上限、backend と resource 判断 |
| USB 構成 | controller BDF、root port/route/slot/VID:PID/speed/class/subclass/protocol、keyboard/mouse/BOT/hub/HCI 種別、hub 世代・段数・reset、input/storage 選択と scan 結果 |
| USB 転送／失敗 | stage と failure の時系列、control request/index/length/slot、completion/residual/slot/endpoint、期待 TRB と raw event 全 4 word、停止／BME 無効または未確認／隔離 |
| storage／file／marker | block size/count/capacity、partition/filesystem/mount/read、README 長と build marker 全 hash、欠落・未対応・不一致、writer 所有性・範囲・write/sync/失敗 |
| LAN | PCI/backend、reset/PHY/NVM/software 設定拒否、5 秒 budget 理由、link と初期化所要時間、DHCP state/address/所要時間、UDP echo/counter/ring wrap、停止・再開・隔離 |
| Bluetooth／入力 | HCI discovery/reset/transport/session/generation、pair 入口・gate・取消・期限・保存結果、report/notification/input 種別と拒否理由。鍵・passkey・秘密値は出さない |
| Realtek 読取り専用採取 | BDF/vendor:device:revision/subsystem/PCI command、BAR0/BAR2 raw/type/base/安全な長さ、UC/RO/NX/decode/identity 判断、TxConfig40 一回読取りまたは skip 理由。BME/MMIO write 無し |
| T2／Mac | SMBIOS、PCI/BCE と継承状態、protocol/resource/ownership と拒否。未接続 BCE／未検証 storage を明記 |
| runtime／app／time | startup milestone、PM epoch timestamp と duration、stream identity/cancel、app loading/validation/start/exit/reject/fault、DMA 停止後の scrub/reuse と viewer 再開 |

実 PCI revision／安全な BAR 値／Realtek TxConfig40／物理 USB speed と構成／I218 link と NVM／Bluetooth 対象互換は各実機記録が必要である。fixture 値を実測と扱わない。別 panel へ隠さず、反復イベントで古い記録が追い出された場合は損失数により履歴上限を明示する。
