# ACPI handoff and time source

## English

### Boot contract

Prefer the ACPI 2.0 GUID in the UEFI Configuration Table; fall back to the ACPI 1.0 GUID if absent.
Obtain a memory map while Boot Services are active and read RSDP, RSDT / XSDT, and FADT.
Before each read, verify that the referenced range fits within a single LoaderData,
BootServicesData, RuntimeServicesData, ACPI Reclaim / NVS descriptor and is not read-protected.
Validate signatures, lengths, and checksums for RSDP and the SDTs in use.
Limit RSDP to 4KiB, SDTs to 64KiB, and root references to 512; reject duplicate FADTs.

Copy the System I/O PM timer port and 24 / 32-bit width from FADT into BootInfo.
Do not carry pointers or borrows to the original tables into the runtime.
After ACPI discovery, obtain the final memory map again and call ExitBootServices.
Original ACPI regions are not Conventional Memory eligible for the arena.
Add a separate table-preservation contract when AML / MCFG becomes necessary.

### PM timer

The PM timer runs at 3,579,545Hz. Apply the width mask to 32-bit I/O reads,
then extend to 64-bit ticks by accumulating modulo differences from the previous value.
Do not assume the initial value is zero. Return elapsed milliseconds using integer quotient and remainder.
For a 24-bit timer, calls must be less than 4.68 seconds apart; the operational limit is one second.
Multiple wraps cannot be reconstructed from reads alone.
Return an error on overflow of the 64-bit accumulator.

Prefer a nonzero, usable System I/O X_PM_TMR_BLK; otherwise use legacy PM_TMR_BLK.
Verify that the four-byte access fits within the I/O port address space.
Diagnose HW_REDUCED_ACPI, absent PM timers, and MMIO-only devices as unsupported.
HPET fallback, interrupts, sleep, and TSC calibration are not implemented.

### Diagnostics and validation

After UEFI exit and transition to our own page tables, poll to measure at least 100ms elapsed time.
Display TIMER READY on success. Limit polling to two million iterations to avoid waiting forever on a stopped timer.
This diagnostic limit is not a time-calibrated timeout.
Continue diagnostics if the timer is missing or stopped.
Future USB / NIC initialization requires a valid time source.

QEMU q35 successfully measured 100ms (0x64); normal smoke requires this success.
Host tests check corrupt FADTs, short tables, extended I/O ranges,
24 / 32-bit wraparound, and frequency conversion.
NUC5 / NUC8 hardware validation has not been performed.

Specification references:
[ACPI 6.5 hardware specification §4.8.3.3](https://uefi.org/specs/ACPI/6.5/04_ACPI_Hardware_Specification.html),
[ACPI 6.5 tables / FADT](https://uefi.org/specs/ACPI/6.5/05_ACPI_Software_Programming_Model.html).

---

## 日本語

**ACPIの引き継ぎと時間源**

### 起動時の契約

UEFI Configuration TableのACPI 2.0 GUIDを優先し、なければACPI 1.0 GUIDを使う。
Boot Servicesが有効な間にメモリマップを取得して、RSDP、RSDT / XSDT、FADTを読む。
各参照はLoaderData、BootServicesData、RuntimeServicesData、ACPI Reclaim / NVSの
単一descriptor内に収まること、read-protectedでないことを先に検査する。
RSDPと使用するSDTの署名、長さ、チェックサムを検査する。
RSDPは最大4KiB、SDTは最大64KiB、rootの参照数は最大512とし、重複FADTを拒否する。

FADTからSystem I/OのPM timerポートと24 / 32ビット幅をコピーしてBootInfoに保存する。
原本テーブルへのポインターや借用はランタイムへ持ち込まない。
ACPI探索後に最終メモリマップを再取得してExitBootServicesを行う。
ACPI原本領域はarena対象のConventional Memoryではない。
今後AML / MCFGが必要になった時点で、別途テーブル保存契約を追加する。

### PM timer

PM timerの周波数は3,579,545Hz。32ビットI/O読出しに幅のmaskを適用し、
前回値との差をmodulo加算して64ビットtickに延長する。
初期値はゼロと仮定しない。整数の商と余りから経過millisecondsを返す。
24ビットの場合、呼出し間隔は必ず4.68秒未満、運用上は1秒以下とする。
複数回のwrapを読出しだけで復元することはできない。
64ビット累積のoverflowはエラーを返す。

非ゼロかつ使用可能なX_PM_TMR_BLKのSystem I/Oを優先し、利用できなければ
legacy PM_TMR_BLKを使う。4byteアクセスがI/Oポート空間内に収まることを確認する。
HW_REDUCED_ACPI、PM timerなし、MMIOのみの機器では未対応を診断する。
HPET fallback、割込み、スリープ、TSC較正は未実装。

### 診断と検証

UEFI終了、自前ページテーブル移行後に100ms以上の経過をポーリングで測り、
成功時はTIMER READYを表示する。停止したtimerへの無限待ちを避けるため
200万回を上限とする。この上限は診断用で、時間で較正されたtimeoutではない。
未検出・停止時は診断を継続する。今後のUSB / NIC初期化は有効な時間源を必須とする。

QEMU q35で100ms（0x64）を測定できた。通常smoke試験はこの成功を必須にする。
ホスト試験でFADT破損・短いtable・extended I/O範囲、24 / 32bit wrapと
周波数換算を検査する。NUC5 / NUC8の実機検証は未実施。

仕様参照:
[ACPI 6.5 hardware specification §4.8.3.3](https://uefi.org/specs/ACPI/6.5/04_ACPI_Hardware_Specification.html)、
[ACPI 6.5 tables / FADT](https://uefi.org/specs/ACPI/6.5/05_ACPI_Software_Programming_Model.html)。
