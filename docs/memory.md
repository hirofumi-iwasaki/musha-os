# Page tables, reserved regions, and RAM arena

## English

Implemented on 2026-10-08. Targets the BSP at ring 0, with interrupts disabled and four-level paging.

### Reservations and ownership

Before Boot Services exit, allocate a normal 64KiB stack, 128KiB map buffer,
4KiB BootInfo, 32KiB emergency stack, and 1MiB page-table pool as LoaderData.
Allocate a dedicated 1MiB DMA pool below 4GiB using AllocateMaxAddress.
Save the EFI image base / size as values from the Loaded Image Protocol.
Do not reclaim Boot Services, Runtime Services, or ACPI regions.

Read the final memory map using the descriptor stride and parse types, physical addresses, page counts, and attributes.
Require descriptor version 1, a stride of at least 40 bytes and a multiple of 8,
page-aligned regions, no arithmetic overflow, and no region overlap.
Explicitly reserve the image, stack, BootInfo, map, emergency stack,
table pool, DMA pool, and GOP; reject initialization if they overlap.

Arena candidates are only EfiConventionalMemory regions with WB capability
and without Runtime attributes. RP/RO/XP are protection capabilities, not
current access settings; owned page tables enforce RW/NX.
See [attribute correction](memory-attribute-policy.md).
Exclude reserved ranges and page 0, then select 16–64MiB from the largest contiguous region.
Halt with MEMORY LOW if less than 16MiB is available. Recheck overlaps after selection.
Do not expose the DMA pool to applications; it is dedicated to device drivers.

Call memory initialization only once. Pass the arena to the diagnostic application
as the sole `&mut [u8]`, allowing arbitrary internal layout.
Currently the only operation is diagnostic pattern writing and volatile comparison at both ends of each page.
A general-purpose allocator, stable external application ABI, and process isolation are not provided yet.
BootInfo and reserved regions remain valid until halt.

### Our own page tables

Use 4KiB identity-mapped pages. Map only the EFI image, dedicated stack,
BootInfo, memory map, CPU tables, emergency stack, table pool, GOP, arena,
dedicated DMA pool, and validated xHCI BAR.
Do not broadly map unused RAM, UEFI services, or all device MMIO.
The first 4KiB of the normal stack is an unmapped guard page.

Validate PE32+ section information in the image; map headers RO/NX,
code RO/execute, and writable sections RW/NX.
Reject write+execute, page-level section overlap, out-of-range sections,
and section alignment other than 4096.
Data, stacks, arena, and GOP are non-executable; all pages are supervisor-only.
Enable CR0.WP and EFER.NXE, flush old global translations by disabling CR4.PGE,
and switch CR3 to our own root. Check CR3 by readback.

Check CPU NX, PAT, and physical-address width.
Preserve firmware PAT / MTRRs and select existing WB / UC PAT entries:
WB for ordinary RAM, UC for GOP / DMA pool / xHCI MMIO.
Before changing CR3, use MFENCE / WBINVD to drain dirty caches from old mappings.
Effective attributes in combination with MTRRs follow CPU rules; no WC optimization is performed.

### Validation and unsupported configurations

Host tests validate malformed maps, overflow, gaps and overlaps in reservations,
page alignment, insufficient capacity, PE code / data attributes, and malformed sections.
QEMU tests cover normal boot, the 64MiB arena, null writes, code writes,
NX violations, stack guards, and #UD / #GP / #DF on our own root.
Every exception displays diagnostics and halts.

Currently diagnose and reject active LA57 / PCID, absent NX / PAT,
addresses outside the lower canonical range, unsupported map / PE formats, and insufficient table-pool capacity.
Dynamic page-table expansion, table-pool reclamation, emergency-stack guards,
IOMMU / dynamic DMA, ACPI mappings, hardware PAT / MTRR compatibility,
and testing every arena byte remain future work.

References: [Intel SDM](https://www.intel.com/content/www/us/en/developer/articles/technical/intel-sdm.html),
[UEFI memory map](https://uefi.org/specs/UEFI/2.10/07_Services_Boot_Services.html)

---

## 日本語

**ページテーブル・予約領域・RAM arena**

2026-10-08実装。BSP、ring 0、割込み無効、4段pagingを対象とする。

### 予約と所有権

Boot Services終了前に通常stack 64KiB、map 128KiB、BootInfo 4KiB、
緊急stack 32KiB、ページテーブルpool 1MiBをLoaderDataとして確保する。
専用DMA pool 1MiBはAllocateMaxAddressで4GiB未満に確保する。
Loaded Image ProtocolからEFIイメージの基点・サイズを値として保存する。
Boot Services領域、Runtime Services領域、ACPI領域は回収しない。

最終メモリマップをdescriptor strideで読み、型・物理アドレス・ページ数・属性を解析する。
descriptor version 1、strideが40byte以上かつ8の倍数、全領域のページ境界、
算術overflow、領域間の重複を検査する。イメージ・stack・BootInfo・map・
緊急stack・table pool・DMA pool・GOPを明示予約し、重複していれば初期化を拒否する。

arenaの候補はEfiConventionalMemoryでWB対応属性を持ち、Runtime属性がない領域だけ。
RP／RO／XPは保護設定能力で、現在のアクセス権とは限らない。自前ページテーブルでRW／NXを保証する。
[属性修正方針](memory-attribute-policy.md)を参照。予約範囲とページ0を除外し、最大の連続領域から
16〜64MiBを選択する。16MiBに満たない場合はMEMORY LOWとして停止する。
選択後の重複も再検査する。DMA poolはアプリへ公開せず、機器ドライバ専用とする。

メモリ初期化は一度だけ呼ぶ。arenaは唯一の `&mut [u8]` として診断アプリへ渡し、
アプリ内部で自由に配置できる。現在は各ページの両端へpatternを書き、
volatileで照合する診断のみ。汎用アロケータ、安定した外部アプリABI、
プロセス分離はまだ提供しない。BootInfoと予約領域の寿命は停止まで。

### 自前ページテーブル

4KiBページの恒等マッピングを使う。EFIイメージ、専用stack、BootInfo、
メモリマップ、CPUテーブル、緊急stack、table pool、GOP、arena、専用DMA poolと検証済みxHCI BARだけをマップする。
未使用RAM、UEFIサービス、機器MMIO全体を一括でマップしない。
通常stackの先頭4KiBは未マップのガードページとする。

イメージ内PE32+のsection情報を検査し、ヘッダーをRO/NX、コードをRO/execute、
書込可能sectionをRW/NXとする。write+execute、ページ単位のsection重複、
範囲外section、4096以外のsection alignmentは拒否する。
データ・stack・arena・GOPは実行禁止。全ページをsupervisor専用とする。
CR0.WPとEFER.NXEを有効にし、旧global translationをCR4.PGEの無効化でflushして
CR3を自前rootへ切り替える。CR3を読戻しで検査する。

CPUのNX・PAT・物理アドレス幅を確認する。ファームウェアのPAT / MTRRを変更せず、
既存PAT中のWBとUCを選択し、通常RAMへWB、GOP / DMA pool / xHCI MMIOへUCを指定する。
CR3切替前にMFENCE / WBINVDで旧mappingのdirty cacheを排出する。
MTRRとの組合せによる実効属性はCPUの規則に従い、WC最適化は行わない。

### 検証と未対応条件

ホストで不正map、overflow、予約の穴と重複、ページ境界、容量不足、
PEのコード・データ属性と不正sectionを検証する。
QEMUで通常起動、64MiB arena、null書込、コード書込、NX違反、stack guard、
自前root上の#UD / #GP / #DFを確認する。全ての例外は診断後停止する。

現時点でLA57 / PCIDが有効な構成、NX / PATなし、低位canonical範囲外のアドレス、
未対応map / PE形式、table pool不足は診断して拒否する。
ページテーブルの動的拡張、table pool回収、緊急stack guard、IOMMU / 動的DMA、
ACPIのマッピング、実機でのPAT / MTRR適合性、arenaの全byte試験は後続作業。

参照: [Intel SDM](https://www.intel.com/content/www/us/en/developer/articles/technical/intel-sdm.html)、
[UEFIメモリマップ](https://uefi.org/specs/UEFI/2.10/07_Services_Boot_Services.html)
