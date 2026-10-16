# Kernel size budget and tracking

## English

Status: **owner decision, 2026-10-16.** Standing rule from v0.4.0 on. Related: [release distribution policy](release-distribution-policy.md), [release notes 0.4.0](release-notes-0.4.0.md).

### Rule

1. Every release records the size of `BOOTX64.EFI` and its section breakdown (`.text`, `.rdata`, initialized `.data` on disk, and the virtual size of `.data`/BSS as RAM reserved at load) in `build-manifest.json` (`efi_size` object) and as one line in the release notes, together with the delta against the previous release.
2. Two configurations are measured and recorded: **full** (the published normal build with every driver enabled by default) and **minimal** (the same source with all optional drivers — LAN, Bluetooth, T2, and later audio/ACPI/igc/Realtek — compiled out). The gap between them is the cost of hardware support and must stay attributable to named features.
3. Optional drivers stay behind Cargo features so that the minimal configuration can always be built; a driver that is useless on a given reference machine must not be unconditionally linked in. Normal builds may still enable them by default.
4. Static RAM reservations (buffers in `.data`/BSS) are listed with their owner and size when they exceed 1 MiB; raising the application arena limit beyond the current static scheme requires a dynamic allocation design, not a larger static buffer.
5. Review thresholds: no action is required below 1 MiB on disk. Above 1 MiB, the next release notes must include a size review (what grew, whether `opt-level = "z"`, LTO settings or dead-code removal were evaluated). Above 4 MiB, growth requires an owner decision.

### Measurements

| Build | File | `.text` | `.rdata` | `.data` on disk | `.data`/BSS virtual (RAM) | Source |
|---|---|---|---|---|---|---|
| v0.3.0 published `BOOTX64.EFI` | 475,648 B (465 KiB) | — | — | — | — | release assets |
| v0.4.0 development, normal build (2026-10-16, `875631c`) | 513,024 B (501 KiB) | 375,808 B | 16,896 B | 118,272 B | 17.3 MiB | `target/x86_64-unknown-uefi/release/musha-boot.efi` |
| v0.4.0 development EFI with `i218-network`, `t2-diagnostics`, Bluetooth | 721,920 B (705 KiB) | 574,976 B | 22,528 B | 122,368 B | 18.5 MiB | `out/i218-segment-fix-20261016` |

Source size at `875631c`: about 43,000 lines of Rust across the workspace crates (`boot` 14.8k, `platform` 7.5k, `fs` 5.5k, `xhci` 2.8k, `bce` 2.7k, `net` 2.3k, `api` 1.9k, `input` 1.9k, `memory` 1.3k, `framebuffer` 1.1k, `viewer` 1.1k) plus vendored lwIP in C.

<!-- static-ram-inventory-en -->
### Static RAM inventory (source inspection, 2026-10-16)

Owners here name the responsible subsystem/source, not a person. A buffer size is not an EFI file size. Source declarations identify these reservations above 1 MiB in the external-application configuration; the exact linked section totals still come from each built PE image.

| Owner / reservation | Size and basis | Storage / responsibility | Source |
|---|---|---|---|
| External ELF loader: `external::STAGING` | Byte array exactly16,777,216 B (16 MiB), plus `Reader` and tuple padding; full symbol size not separately measured here | Static zero-fill within the loaded image; CPU-owned staging, never DMA. The file limit does not itself allocate RAM, but this actual static array does | [declaration and range](../boot/src/external.rs), [MAX_BYTES](../api/src/bootfile.rs) |
| Application recovery: `ring3::lifecycle::SNAPSHOT` | 1,052,672 B on x86-64:256×4096 B snapshot + two8-byte `usize` fields, rounded up to4096-byte struct alignment | Static page-table rollback snapshot. Payload is exactly1 MiB, while the aligned object exceeds1 MiB; linked in external-app/lifecycle configurations | [static owner](../boot/src/ring3/lifecycle.rs), [ProbeSnapshot layout](../boot/src/memory.rs), [PAGE](../memory/src/lib.rs) |

No other individually declared static reservation above1 MiB was found in this source pass, including the configured lwIP pools. This is source-level ownership accounting, not a substitute for symbol/section measurement or a sum of peak live RAM. Tests and future feature variants require their own inventory.

### Do not combine these size categories

- `efi_size.file_bytes` is the actual `BOOTX64.EFI` file. Section `raw_bytes` is PE SizeOfRawData, including alignment padding; `virtual_bytes` is VirtualSize. `.data`/BSS virtual totals and `image_virtual_bytes` describe PE virtual layout, not additional bytes on disk or measured resident/peak RAM; section VirtualSize is not page-rounded allocation.
- The16 MiB maximum ELF file and the real16 MiB staging array are separate facts. An ELF segment's file bytes (`p_filesz`) and memory bytes (`p_memsz`, including zero-fill/BSS) differ; the guest IMAGE_LIMIT bounds its mapped image window. They are not the kernel EFI's file/BSS size.
- The application arena is selected from the firmware memory map at runtime (minimum16 MiB, maximum64 MiB), excluding reserved ranges. It is not a64 MiB static BSS array. Page tables and USB DMA are separately allocated with UEFI AllocatePages; each256-page table/controller block is1 MiB, not “above1 MiB” individually, though aggregate/controller count can exceed it. Guest STACK_LIMIT=1 MiB is a virtual ABI limit, not another file or static array.
- References: [arena selection](../boot/src/memory.rs), [UEFI reservations](../boot/src/main.rs), [guest ABI limits](../sdk/rust/musha-sys/src/generated.rs), [ELF file/memory validation](../memory/src/elf.rs), [lwIP pool configuration](../net/port/lwipopts.h).

### Manifest configuration boundary

Normal-build measurements use `efi_size`. Minimal measurements use `efi_size_minimal` only when optional-driver exclusion is real. At this checkpoint minimal is unavailable: LAN and Bluetooth remain unconditional dependencies; `--no-default-features` alone does not establish a minimal build. T2 is opt-in already. The manifest records `efi_size_minimal: null` with `efi_size_minimal_status` naming the blocking drivers/reason; this is neither zero bytes nor a measured equal-size minimal build. The owner’s full/minimal rule above remains in force.

<!-- /static-ram-inventory-en -->

### Outlook (estimates, not commitments)

v0.4.0 published with LAN and Bluetooth in the normal build: about 700 KiB. v0.5.0 with HD Audio and USB Audio, an AML interpreter (ACPICA-class code is several hundred KiB on its own), Classic Bluetooth, generic HID parsing, i8042, igc and Realtek drivers: 1.0–1.5 MiB, which crosses the first review threshold. Later versions: the libc layer, SDL and Mesa live in applications, not in the kernel.

## 日本語

状態：**所有者決定、2026-10-16。** v0.4.0 以降の常時の規則。関連：[配布方針](release-distribution-policy.md)、[0.4.0 リリースノート](release-notes-0.4.0.md)。

### 規則

1. 公開のたびに `BOOTX64.EFI` のサイズとセクション内訳（`.text`、`.rdata`、ディスク上の初期化 `.data`、および読込み時に RAM として確保される `.data`/BSS の仮想サイズ）を `build-manifest.json`（`efi_size` オブジェクト）に記録し、前版との差分とともにリリースノートに 1 行で載せる。
2. **全部入り**（全ドライバーを既定で有効にした公開通常ビルド）と**最小**（同じソースから任意ドライバー——LAN、Bluetooth、T2、将来の音声／ACPI／igc／Realtek——を全て外したもの）の 2 構成を測って記録する。両者の差がハードウェア対応のコストであり、名前の付いた feature に帰属できる状態を保つ。
3. 任意ドライバーは Cargo の feature の後ろに置き、最小構成を常にビルドできるようにする。ある基準機で無意味なドライバーを無条件にリンクしない。通常ビルドで既定有効にすることは構わない。
4. 静的な RAM 確保（`.data`/BSS のバッファー）は 1 MiB を超えるものについて所有者と大きさを列挙する。アプリの arena 上限を現在の静的方式より大きくするには、静的バッファーの拡大ではなく動的確保の設計が必要。
5. 見直しの閾値：ディスク上 1 MiB 未満は何もしない。1 MiB 超では次のリリースノートにサイズの見直し（何が増えたか、`opt-level = "z"`・LTO 設定・不要コード削減を検討したか）を含める。4 MiB 超の増加は所有者判断。

### 計測

| ビルド | ファイル | `.text` | `.rdata` | ディスク上 `.data` | `.data`/BSS 仮想（RAM） | 出所 |
|---|---|---|---|---|---|---|
| 公開済み v0.3.0 の `BOOTX64.EFI` | 475,648 B（465 KiB） | — | — | — | — | 公開配布物 |
| v0.4.0 開発中、通常ビルド（2026-10-16、`875631c`） | 513,024 B（501 KiB） | 375,808 B | 16,896 B | 118,272 B | 17.3 MiB | `target/x86_64-unknown-uefi/release/musha-boot.efi` |
| v0.4.0 開発 EFI（`i218-network`、`t2-diagnostics`、Bluetooth 入り） | 721,920 B（705 KiB） | 574,976 B | 22,528 B | 122,368 B | 18.5 MiB | `out/i218-segment-fix-20261016` |

`875631c` 時点のソース量：ワークスペースの Rust が約 43,000 行（`boot` 14.8k、`platform` 7.5k、`fs` 5.5k、`xhci` 2.8k、`bce` 2.7k、`net` 2.3k、`api` 1.9k、`input` 1.9k、`memory` 1.3k、`framebuffer` 1.1k、`viewer` 1.1k）と、C のまま取り込んだ lwIP。

<!-- static-ram-inventory-ja -->
### 静的RAM一覧（source確認、2026-10-16）

所有者は担当者名でなく責任subsystem/sourceです。bufferサイズはEFIファイルサイズではありません。外部app構成で1 MiB超となる宣言を確認し、実際にlinkされたsection合計は各PE成果物から別途計測します。

| 所有subsystem／確保 | 大きさ・根拠 | 種類・責任 | source |
|---|---|---|---|
| 外部ELF loader：`external::STAGING` | byte配列は正確に16,777,216 B（16 MiB）、別にReaderとtuple padding。symbol全体は本棚卸で未計測 | loaded image内のstatic zero-fill。CPU所有stagingでDMAではない。file上限自体でRAMは増えないが、このstatic配列は実確保 | [宣言/範囲](../boot/src/external.rs)、[MAX_BYTES](../api/src/bootfile.rs) |
| app回復：`ring3::lifecycle::SNAPSHOT` | x86-64で1,052,672 B：256×4096 B＋8byte usize×2をstructの4096byte alignmentへ切上げ | static page-table rollback snapshot。payloadはちょうど1 MiB、aligned objectは1 MiB超。external-app/lifecycle構成でlink | [static所有箇所](../boot/src/ring3/lifecycle.rs)、[layout](../boot/src/memory.rs)、[PAGE](../memory/src/lib.rs) |

本source確認では、設定済lwIP poolを含め、個々の宣言が1 MiB超となる他のstatic確保は見つかっていません。これはsource責任箇所の棚卸で、symbol/section実測や同時live RAM合計の代用ではありません。試験用・将来feature構成は別途確認します。

### 混同しないサイズ

- `efi_size.file_bytes`は実BOOTX64.EFIファイル。sectionの`raw_bytes`はalignment padding込みのPE SizeOfRawData、`virtual_bytes`はVirtualSizeです。`.data`/BSS仮想合計・`image_virtual_bytes`はPE virtual layoutで、disk bytesや実測resident/peak RAMではありません。section VirtualSizeはpage単位の実確保へ切上げた値でもありません。
- ELFファイル上限16 MiBと実staging配列16 MiBは別の事実です。ELF segmentのfile bytes（p_filesz）とzero-fill/BSS込みmemory bytes（p_memsz）は異なり、guest IMAGE_LIMITはmapped image windowの制限です。kernel EFIのfile/BSSサイズとは別です。
- app arenaはfirmware memory mapから予約範囲を除外して実行時選択（最小16 MiB、最大64 MiB）し、64 MiBのstatic BSS配列ではありません。page table・USB DMAはUEFI AllocatePagesで別確保。256pageのtable/controller blockは各1 MiBで個別には「超」ではなく、合計はcontroller数等で増えます。guest STACK_LIMIT=1 MiBもvirtual ABI上限で別file/static配列ではありません。
- 根拠：[arena選択](../boot/src/memory.rs)、[UEFI予約](../boot/src/main.rs)、[guest ABI上限](../sdk/rust/musha-sys/src/generated.rs)、[ELF検証](../memory/src/elf.rs)、[lwIP pool設定](../net/port/lwipopts.h)。

### manifest構成の境界

通常構成は`efi_size`、最小構成は任意driverを実際に除外できた場合だけ`efi_size_minimal`へ記録します。現段階はLAN/BTが無条件依存で最小構成は利用不可、`--no-default-features`だけでは最小版を証明しません。T2は既にopt-inです。manifestは`efi_size_minimal: null`と`efi_size_minimal_status`へ阻害driver/理由を記し、0 Bや同サイズの計測済minimalとは扱いません。上記所有者のfull/minimal規則は維持します。

<!-- /static-ram-inventory-ja -->

### 見通し（見積もりであり約束ではない）

LAN と Bluetooth を通常ビルドに入れた v0.4.0 公開版：約 700 KiB。HD Audio と USB Audio、AML 解釈器（ACPICA 級のコードは単体で数百 KiB）、Classic Bluetooth、汎用 HID 解釈、i8042、igc と Realtek のドライバーを入れた v0.5.0：1.0〜1.5 MiB で、最初の見直し閾値を越える。それ以降：libc 層、SDL、Mesa はアプリ側に入り、カーネルは太らない。
