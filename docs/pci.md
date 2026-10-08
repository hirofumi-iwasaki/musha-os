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
