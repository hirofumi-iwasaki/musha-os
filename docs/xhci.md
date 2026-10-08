# First stage of xHCI initialization

## English

Target the first xHCI controller found in segment 0.
Following ownership acquisition, stop, and reset, [DMA / ring diagnostics](xhci-rings.md) are also implemented.
Port reset, USB enumeration, keyboard handling, and storage handling are not implemented.

### BARs and mapping

While UEFI is active, enumerate PCI I/O Protocols and inspect class 0C0330 and BDF.
Use GetBarAttributes to obtain BAR0's QWORD memory resource; copy its start and length.
FreePool the returned resource and handle list before obtaining the final memory map.
Do not write to a live hardware BAR to probe its size.

Require segment 0, no translation, a base of at least 1MiB aligned to 4KiB,
and a length from 4KiB to 1MiB.
Reject at boot if it overlaps reserved regions or RAM in the UEFI map.
Identity-map the entire BAR as UC / RW / NX; exclude it from the arena.
Validate alignment and BAR bounds for every MMIO access.
Do not initialize if memory decoding is disabled.

### State transitions and waits

1. Check capability length, xHCI 1.0–1.2, slot / port counts, and port-register ranges.
2. Verify that the PM timer advances. Scan at most 256 extended-capability entries.
3. If Legacy Support is present, set the OS-owned byte and wait at most 1000ms for BIOS-owned to clear.
   Do not forcibly clear BIOS-owned or modify operational registers on failure.
   After acquisition, disable defined SMI enables and acknowledge only defined W1C status bits.
4. Wait at most 1000ms for CNR to clear, then disable R/S, INTE, and HSEE.
   Wait at most 100ms for HCHalted.
5. Access PCI Command as 16 bits, disable Bus Master Enable, and verify readback.
   Do not change W1C bits in the adjacent PCI Status.
6. Set HCRST; wait at most 1000ms each for it and CNR to clear.
   Check HCHalted and 4KiB page-size support.

Each wait has both a time deadline and a five-million-iteration limit to avoid waiting forever on a stopped clock.
Poll the timer every iteration.
Leave the controller halted with bus mastering disabled.
Do not reclaim failed-controller DMA buffers or boot-time Boot Services RAM for the arena.
On device failure, display XHCI FAILED and proceed to the diagnostic application;
do not count this as a complete feature pass.

### Validation and remaining work

Detected eight ports with QEMU qemu-xhci and confirmed reset completion and disabled DMA.
The `xhci-timeout` feature waits for halted state to clear while keeping the controller halted,
checking a 20ms timeout and continuation to the diagnostic application.
This is QEMU-only and excluded from hardware builds.
Host tests checked rejection of BAR I/O resources, translation, and overflow.
The actual BIOS-owned to OS-owned handoff wait path requires hardware validation.
NUC5 Intel USB routing / EHCI handoff, NUC8-specific differences, and hardware errata remain untested.

DMA / ring diagnostics are verified through 600 No-Op commands and shutdown.
[USB enumeration](usb-enumeration.md) through Device Descriptor reads is also verified.

Primary sources: [Intel xHCI 1.2b](https://cdrdv2-public.intel.com/625472/625472_xHCI_Rev1_2b.pdf)
§4.2, 4.22.1, 5.4.1, 5.4.2, 7.1,
[UEFI PCI I/O GetBarAttributes](https://uefi.org/specs/UEFI/2.11/14_Protocols_PCI_Bus_Support.html).

---

## 日本語

**xHCI初期化の第1段階**

対象は最初に見つかったsegment 0のxHCIコントローラー一台。
所有権取得・停止・リセットに続き、[DMA / ring診断](xhci-rings.md)も実装済み。
port reset、USB列挙、キーボードとストレージ処理は未実装。

### BARとマッピング

UEFI稼働中にPCI I/O Protocolを列挙し、class 0C0330とBDFを調べる。
GetBarAttributesでBAR0のQWORD memory resourceを取得し、開始と長さをコピーする。
返却されたresourceとhandle一覧は最終メモリマップ取得前にFreePoolする。
実機の稼働中BARへサイズ計測の書込みは行わない。

segment 0、translationなし、1MiB以上のbase、4KiB境界、4KiB〜1MiBの長さを要求する。
予約済み領域やUEFI map上のRAMと重なる場合は起動時に拒否する。
BAR全体をUC / RW / NXで恒等マッピングする。arenaへは含めない。
すべてのMMIOアクセスでアラインメントとBAR内の範囲を検査する。
メモリデコードが有効でなければ初期化しない。

### 状態遷移と待機

1. capability length、xHCI 1.0〜1.2、slot / port数、port register範囲を確認する。
2. PM timerが進むことを確認する。extended capability chainを最大256項走査する。
3. Legacy SupportがあればOS-owned byteを立て、BIOS-owned解除を最大1000ms待つ。
   BIOS-ownedを強制解除せず、失敗時は運用レジスタを書き換えない。
   取得後は定義されたSMI enableを無効化し、定義されたW1C statusだけをackする。
4. CNR解除を最大1000ms待ち、R/S・INTE・HSEEを無効化する。
   HCHaltedを最大100ms待つ。
5. PCI Commandを16bitアクセスしBus Master Enableを解除・読戻し確認する。
   隣接するPCI StatusのW1C bitを変更しない。
6. HCRSTを立て、解除とCNR解除をそれぞれ最大1000ms待つ。
   HCHaltedと4KiB page size対応を確認する。

各待機は時間期限のほか500万回の上限を持ち、停止した時計への無限待ちを防ぐ。
タイマーは各周回でpollする。コントローラーはhalted、bus mastering無効のまま残す。
失敗したcontrollerのDMA bufferを回収せず、起動時のBoot Services RAMもarenaへ回収しない。
機器失敗時はXHCI FAILEDを表示して診断アプリへ進み、全機能合格とは扱わない。

### 検証と残る作業

QEMU qemu-xhciで8portを検出し、reset完了とDMA無効状態を確認した。
`xhci-timeout` featureではhaltedのまま解除待ちを行い、20msの期限切れと
診断アプリへの継続を検査する。これはQEMU専用で実機版に含めない。
ホストでBAR resourceのI/O種別、translation、overflowを拒否することを検査した。
BIOS-ownedからOS-ownedへ実際に譲渡される待機経路は、実機での確認が必要。
NUC5のIntel USB routing / EHCI handoff、NUC8固有差、実機errataも未検証。

DMA / ring診断はNo-Op 600回と停止まで確認済み。Device Descriptorまでの[USB列挙](usb-enumeration.md)も確認済み。

一次資料: [Intel xHCI 1.2b](https://cdrdv2-public.intel.com/625472/625472_xHCI_Rev1_2b.pdf)
§4.2、4.22.1、5.4.1、5.4.2、7.1、
[UEFI PCI I/O GetBarAttributes](https://uefi.org/specs/UEFI/2.11/14_Protocols_PCI_Bus_Support.html)。
