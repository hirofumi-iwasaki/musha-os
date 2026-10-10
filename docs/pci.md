# PCI device diagnostics

## English

Musha-OS 0.1.0 scans buses 0–255 and devices 0–31 in segment 0 using
x86 PCI Configuration Mechanism 1 (CF8 / CFC).
Skip a device if function 0's vendor ID is FFFF.
When the header's multifunction bit is set, also inspect functions 1–7.
Output vendor / device IDs, class / subclass / programming interface,
and BDF to the QEMU diagnostic log.

Do not write anything except the configuration-address selection.
At this stage, do not probe BAR sizes, enable bus mastering, reconfigure bridges, or reset devices.
Only the boot CPU accesses CF8/CFC pairs, with interrupts disabled.
Output device information sequentially without variable-length storage.

Display the total device count, xHCI count for class 0C0330, and Intel Ethernet count
for vendor 8086 and class/subclass 0200.
Detection alone does not establish driver support. Continue diagnostics even if devices are not found.
Finalize individual hardware NIC support when implementing PCI ID allowlists and PHY initialization.

### QEMU validation

Detected seven devices with q35, qemu-xhci, and e1000e.
xHCI was 1B36:000D; Intel 82574 was 8086:10D3.
Smoke checks both class / IDs and the enumeration-complete marker.
The NIC is connected to QEMU's user network, but Musha-OS does not yet start communication.

### Limitations and next work

Assume firmware-assigned bus numbers.
Multiple segments and ECAM / MCFG are unsupported.
Physical NUC5 / NUC8 PCI IDs remain unconfirmed.
Continue ACPI validation / preservation, PM timer / HPET work,
BAR range reservation / mapping, and xHCI / NIC ownership handoff, initialization, and recovery.

PCI enumeration itself does not change configuration registers.
Separate [xHCI initialization](xhci.md) disables PCI Command Bus Master Enable after stopping the controller.

Specification reference: PCI configuration access (CF8 / CFC) in the
[Intel E8501 chipset datasheet](https://www.intel.com/content/dam/doc/datasheet/e8501-chipset-north-bridge-datasheet.pdf).

---

## 日本語

**PCI機器の診断**

Musha-OS 0.1.0は、x86のPCI Configuration Mechanism 1（CF8 / CFC）で
segment 0のbus 0〜255、device 0〜31を走査する。
function 0のvendor IDがFFFFならスキップし、headerのmultifunction bitが
立つ機器ではfunction 1〜7も調べる。vendor / device ID、class / subclass /
programming interfaceとBDFをQEMU診断ログに出す。

設定アドレスの選択以外には書き込まない。BARのサイズ計測、バスマスター許可、
ブリッジの再設定、リセットはこの段階では行わない。
起動CPUだけがアクセスし、割込みを禁止した状態でCF8/CFCの組を扱う。
デバイス情報は逐次出力し、可変長の保存領域は使わない。

画面には全機器数、class 0C0330のxHCI数、vendor 8086かつclass/subclass
0200のIntel Ethernet数を表示する。検出だけでドライバ対応とは判定しない。
未検出でも診断を継続する。実機の個別NIC対応はPCI ID allowlistとPHY初期化の
実装時に確定する。

### QEMUでの検証

q35、qemu-xhci、e1000eの構成で7機器を検出した。
xHCIは1B36:000D、Intel 82574は8086:10D3だった。
smoke試験は両者のclass / IDと列挙完了マーカーを検査する。
NICにはQEMUのuser networkを接続するが、Musha-OSはまだ通信を開始しない。

### 制約と次の作業

ファームウェアが割り当てたbus番号を前提とする。
複数segmentやECAM / MCFGは未対応。NUC5 / NUC8の実機PCI IDは未確認。
ACPIの検証・保存、PM timer / HPET、BAR範囲の予約とマッピング、
xHCI / NICの所有権移行・初期化・復旧処理を続ける。

PCI列挙そのものは設定レジスタを変更しない。別の[xHCI初期化](xhci.md)では、
停止後にPCI CommandのBus Master Enableを解除する。

仕様参照: [Intel E8501 chipset datasheet](https://www.intel.com/content/dam/doc/datasheet/e8501-chipset-north-bridge-datasheet.pdf)
のPCI configuration access（CF8 / CFC）。

## H16 shared BAR parser

UEFI resource decoding and direct-MMIO policy are separate. Both controller discovery and recovery peer checks use the shared decoder; unsupported translation and malformed resources remain blocking. See [H16 design](pci-h16-resource-parser-plan.md).

## H17 consistency inspection

H17 captures the first rejected peer's complete BAR layout, PCIIO read statuses, bounded before/after snapshots, and segment-0 CF8/CFC comparisons. A descriptor/config type mismatch remains blocking; diagnostic evidence does not grant recovery permission. No machine-specific exception or configuration write is added. See [H17 inspection design](pci-h17-bar-consistency-inspection-plan.md).

H17 stores before/after/repeat config values and per-read status in fixed boot-CPU storage outside BootInfo. Structural descriptor decoding and BAR-slot classification are diagnostic helpers; they do not relax memory recovery policy. Changed or failed config snapshots remain blocking. The fixed diagnostic journal holds 512 lines with an explicit overflow marker. Platform tests (22), UEFI release build, and single/multiple-controller QEMU GPT/input smoke passed; firmware-specific evidence requires the next physical boot.

## H18 logical BAR index mapping

Configuration DWORD slots and UEFI logical resource indices are advanced separately: a 64-bit BAR consumes two slots but one resource index. Zero BAR slots retain an index. Recovery peer memory descriptors must also match the configuration BAR base; mismatch remains blocking. Existing recovery permission conditions are unchanged. See [H18 design and verification](pci-h18-bar-index-plan.md).

## H19 conditional peer overlap proof

With explicit user approval, a mismatched peer's ordinary 32-bit memory BAR may use a conservative naturally aligned aperture bound, checked together with its firmware range. Both ranges must be disjoint from the recovery target. Configuration reads must agree across PCIIO and CF8, remain stable, and be rechecked immediately before restoration. Own-controller mismatches, unsupported BARs, ambiguity, overflow, or evidence-capacity exhaustion remain blocking. No peer writes or BAR sizing probes are added. See [H19 implementation and verification](pci-h19-peer-resource-plan.md).
