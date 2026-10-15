# Ryzen Ethernet identification / Ryzen Ethernetの特定

## English

Status: identification required before goal13 Realtek driver work. Owner scope requires the Ryzen machine's Realtek Ethernet controller, but the exact PCI device and motherboard are **not identified** from the retained metadata. Reviewed: six JSON/text/log files under `out/ryzen-*`, the Ryzen hardware results, and `docs/hardware-evidence/2026-10-13/ryzen7/provenance.json`. They establish boot/video/provenance observations, not a NIC model. No chip or board is inferred from video appearance, CPU name or xHCI count.

The owner performs the following read-only capture on the same Ryzen machine. Do not change firmware settings, drivers, partitions or USB contents. Record the date and which physical Ethernet socket is connected. Keep USB Ethernet adapters separate from the motherboard PCI NIC.

### Windows

1. Open Device Manager → Network adapters → the physical wired adapter → Properties → Details → Hardware Ids. Copy every PCI ID line, including `VEN_10EC`, `DEV_....`, `SUBSYS_........` and `REV_..`; also record adapter name and Device instance path/location. A display name containing Realtek alone is insufficient. If multiple adapters exist, identify the motherboard socket and record each separately.
2. In PowerShell, capture motherboard identity without serial numbers:

```powershell
Get-CimInstance Win32_BaseBoard | Select-Object Manufacturer, Product, Version
```

3. Save the text/screenshots as a new evidence directory with machine/date. Redact unrelated serial numbers/MAC addresses. Do not replace old evidence.

### Linux alternative

On an already available Linux environment on that machine, capture:

```sh
lspci -Dnnk -d 10ec:
cat /sys/class/dmi/id/board_vendor
cat /sys/class/dmi/id/board_name
cat /sys/class/dmi/id/board_version
```

Select the PCI **Ethernet class0200** entry, not a Realtek card reader/audio device. Save full BDF, vendor:device, subsystem IDs, revision and kernel driver; `lspci -Dnnvv -s <BDF>` supplies the additional fields if needed. Missing DMI values remain UNKNOWN and can be supplemented by the board's printed product/revision or its firmware system-information screen. Do not guess from a similar product.

| New evidence field | Current status |
|---|---|
| Realtek Ethernet vendor:device / subsystem / revision / BDF | UNKNOWN |
| Motherboard manufacturer / product / revision | UNKNOWN |
| Capture date / OS / exact socket / evidence path | NOT CAPTURED |

After review, append the explicit identifiers and evidence path to a new Ryzen section of [hardware verification status](hardware-verification-status.md). Match the device-specific programming reference before implementation. Current normal devices diagnostics do not enumerate every unsupported PCI vendor into the visible panel; no Realtek line does not prove no Realtek hardware. A subsequent read-only PCI diagnostic can cross-check the host capture. I218-V and Realtek need separate driver initialization and physical link/DHCP/OS-echo evidence; QEMU82574 proves neither physical controller.

## 日本語

状態：目標13 Realtek driver着手前の特定待ち。所有者scopeはRyzen機のRealtek Ethernetを必須とするが、保持metadataから正確なPCI deviceとmotherboardは**未特定**。`out/ryzen-*` 配下のJSON/text/log計6件、Ryzen実機結果、`docs/hardware-evidence/2026-10-13/ryzen7/provenance.json` を確認した。起動・動画・来歴の観測はあるがNIC型番の根拠ではない。動画の見た目、CPU名、xHCI数からchip/boardを推定しない。

所有者が同じRyzen機で以下を読取り専用採取する。firmware設定、driver、partition、USB内容は変更しない。日時と接続する物理Ethernet端子を記録し、USB Ethernet adapterをmotherboard PCI NICと分ける。

### Windows

1. デバイスマネージャー → ネットワークアダプター → 物理有線adapter → プロパティ → 詳細 → ハードウェアID。`VEN_10EC`／`DEV_....`／`SUBSYS_........`／`REV_..` を含むPCI行をすべてコピーし、adapter表示名・デバイスインスタンスパス／場所も記録。Realtekという表示名だけでは不足。複数adapterはmotherboard端子を特定し別々に記録する。
2. PowerShellでserial番号を含めずboardを採取する。

```powershell
Get-CimInstance Win32_BaseBoard | Select-Object Manufacturer, Product, Version
```

3. machine/date付きの新しい証拠directoryへtext／写真を保存。無関係なserial／MACは除き、旧証拠を上書きしない。

### Linuxの代替手順

その機体で既に利用できるLinux環境から採取する。

```sh
lspci -Dnnk -d 10ec:
cat /sys/class/dmi/id/board_vendor
cat /sys/class/dmi/id/board_name
cat /sys/class/dmi/id/board_version
```

Realtek card reader／audioではなくPCI **Ethernet class0200** を選ぶ。完全BDF、vendor:device、subsystem、revision、kernel driverを保存し、不足項目は `lspci -Dnnvv -s <BDF>` で補う。DMI欠落はUNKNOWNとし、基板印字の製品/revisionかfirmwareシステム情報画面で補足できる。似た製品から推定しない。

| 新しい証拠項目 | 現状 |
|---|---|
| Realtek Ethernet vendor:device／subsystem／revision／BDF | 未特定 |
| motherboard manufacturer／product／revision | 未特定 |
| 日時／OS／物理端子／証拠path | 未採取 |

レビュー後、[実機確認状況](hardware-verification-status.md)の新Ryzen節へ明示IDと証拠pathを追記し、device固有の資料と照合して実装する。現行通常devices表示は未対応PCI vendor全部を表示するものではなく、Realtek行が無いことは機器不存在の証明ではない。後の読取り専用PCI診断でhost採取値を相互確認する。I218-VとRealtekは別初期化と実機link／DHCP／OS echo証拠が必要で、QEMU82574合格をどちらの実機対応にも読み替えない。
