# Device support matrix — draft / 機器対応表・草案

## English

Initial evidence-linked stub,2026-10-15. No existing Claude draft was present at this path when created. It is a starting point for the planned documentation pass, not a claim that goal17 is complete. “Supported” below always names the tested boundary; owner-reported specifications, QEMU behavior and physical acceptance are separate. Unknown IDs/revisions remain unknown. [Scope](release-scope-0.4.0.md) controls reference devices; [hardware status](hardware-verification-status.md) preserves historical results.

| Category | Device/chip | PCI/USB ID | Connection/path | Status and tested boundary | Machine | Evidence |
|---|---|---|---|---|---|---|
| Input | USB HID boot keyboard | Lenovo17EF:6009 observed on NUC5; other IDs vary | USB→xHCI→boot keyboard | Supported in recorded typing/exit tests; no universal HID claim | NUC5 / Ryzen | [Accepted records](hardware-acceptance-0.3.0.md) |
| Input | USB relative mouse / wheel | Unknown target IDs | USB HID→planned shared SYS_INPUT | Planned goal9; absolute QEMU tablet is separate; viewer remains keyboard UI | NUC5 / Ryzen | [Work plan](work-plan-0.4.0.md) |
| Input | T2 internal keyboard/trackpad | Unknown target IDs | PCI BCE→input | Required goal15, unconnected model; not usable support | MacBook T2 | [BCE research](macbook-t2-keyboard-0.2.0-research.md) |
| Storage | USB BOT mass storage | SanDisk0781:55A9 observed on NUC5; other IDs vary | USB→xHCI→BOT→FAT32/exFAT read | Recorded file reads supported; bounded OS-owned FAT32 writes locally tested, physical writes pending | NUC5 / Ryzen | [Acceptance](hardware-acceptance-0.3.0.md), [SETTINGS](settings.md) |
| Storage | WD SN7702TB | Unknown | Native PCIe NVMe | Owner-reported hardware; native storage driver outside current implemented path | Ryzen | [Owner configuration](hardware-verification-status.md) |
| Storage | Mac boot-media path | Unknown | Firmware discovery→native USB storage | Mandatory goal14 planned; do not infer from GOP success | Macs | [Work plan](work-plan-0.4.0.md) |
| Display | Firmware GOP / RX7900XTX24GB | GPU PCI ID unknown | UEFI GOP→framebuffer | Recorded GOP rendering; no native GPU acceleration claim; GPU model owner-reported | NUC5 / Ryzen | [Hardware status](hardware-verification-status.md) |
| Wired LAN | Intel I218-V | Candidate8086:15A3; actual revision pending | PCI→Intel MAC/PHY→lwIP | Planned physical DHCP/echo; optional PHY probe is not networking | NUC5 | [I218 probe](i218-phy-probe.md), [LAN plan](lan-plan-0.4.0.md) |
| Wired LAN | ASUS L8200A / RTL8111H-class | Expected10EC:8168; actual revision/subsystem pending | PCIe→separate Realtek driver→lwIP | Required goal13, driver not implemented | Ryzen X570 | [Identification](ryzen-realtek-identification.md) |
| Wired LAN | Intel I225-LM | Unknown | PCIe→separate future MAC path | Planned trial after acquisition, no acceptance | Planned NUC11TNKi3 | [Owner configuration](hardware-verification-status.md) |
| Wired LAN | Intel82574 | QEMU8086:10D3 | Emulated PCI→current network path→lwIP | QEMU cooperative DHCP/UDP tested; not physical I218/Realtek evidence | QEMU | [Stage3 regression](stage3-0.4.0-progress.md) |
| Wireless | Intel AC7265 / AX201 | Exact Wi-Fi IDs unknown; NUC5 USB8087:0A2A enumerated separately | AC7265 PCIe / AX201 CNVi | No Wi-Fi driver support; AX201 owner-planned, not acquired | NUC5 / planned NUC11 | [Acceptance](hardware-acceptance-0.3.0.md), [owner configuration](hardware-verification-status.md) |
| Bluetooth | CSR8510-class dongle; Xbox1914 / BLE Magic Keyboard | Exact dongle VID:PID/revision pending | USB HCI→LE SMP/ATT/GATT/HOGP | Required goal12 planned; Classic and onboard AC7265/AX201 Bluetooth outside reference path; optional HOGP mouse not a gate | NUC5 / Ryzen | [Work plan](work-plan-0.4.0.md) |
| Audio | Realtek S1200A | Unknown | Onboard audio | Out of scope; owner-reported chip | Ryzen | [Owner configuration](hardware-verification-status.md) |
| USB host | Intel9-series xHCI | Observed8086:9CB1; revision pending | PCI→xHCI | Recorded keyboard/storage path supported, port-specific results apply | NUC5 | [Acceptance](hardware-acceptance-0.3.0.md) |
| USB host | AMD xHCI | Owner-reported1022:149C; revision pending | PCI→xHCI, rear Gen1 A×4 / Gen2 A×2 C×1 | Existing Ryzen USB evidence; exact chip/rear-port mapping remains unmeasured | Ryzen X570 | [Hardware status](hardware-verification-status.md) |
| Hubs / USB4 | USB2 hubs / NUC11 TB4×2 | Hub-specific IDs unknown | USB2 hub chain / future TB4/USB4 path | Five-tier hubs tested in QEMU only; TB4/USB4 routing not established by USB2 support | QEMU / planned NUC11 | [Stage3 regression](stage3-0.4.0-progress.md), [owner configuration](hardware-verification-status.md) |

## 日本語

2026-10-15の証拠リンク付き初期stubです。作成時にこのpathの既存Claude草稿はありませんでした。文書整備の出発点で、目標17完了の主張ではありません。「対応」は必ず検証した範囲を意味し、所有者申告・QEMU・実機受入を区別します。不明ID/revisionは未確認のままです。[scope](release-scope-0.4.0.md)が基準機器を定め、[実機状況](hardware-verification-status.md)は過去証拠を保持します。

| 分類 | 機器/chip | PCI/USB ID | 接続/経路 | 状態と検証範囲 | 機種 | 証拠 |
|---|---|---|---|---|---|---|
| 入力 | USB HID boot keyboard | NUC5でLenovo17EF:6009実測、他IDは機器次第 | USB→xHCI→boot keyboard | 記録した入力/終了試験で対応、HID全般ではない | NUC5 / Ryzen | [受入記録](hardware-acceptance-0.3.0.md) |
| 入力 | USB相対mouse / wheel | 対象ID未確認 | USB HID→計画中SYS_INPUT共通経路 | 目標9計画、絶対QEMU tabletとは別、viewerはkeyboard UI | NUC5 / Ryzen | [計画](work-plan-0.4.0.md) |
| 入力 | T2内蔵keyboard/trackpad | 対象ID未確認 | PCI BCE→input | 必須目標15、model未接続で利用可能な対応ではない | MacBook T2 | [BCE調査](macbook-t2-keyboard-0.2.0-research.md) |
| 記憶 | USB BOT mass storage | NUC5でSanDisk0781:55A9実測、他IDは機器次第 | USB→xHCI→BOT→FAT32/exFAT読取り | 記録したfile読出し対応、OS所有FAT32限定書込みはlocal検証・実機待ち | NUC5 / Ryzen | [受入](hardware-acceptance-0.3.0.md)、[SETTINGS](settings.md) |
| 記憶 | WD SN7702TB | 未確認 | native PCIe NVMe | 所有者構成情報、native storage driverは現実装経路外 | Ryzen | [所有者構成](hardware-verification-status.md) |
| 記憶 | Mac起動媒体経路 | 未確認 | firmware discovery→native USB storage | 必須目標14計画、GOP成功から推定しない | Macs | [計画](work-plan-0.4.0.md) |
| 表示 | firmware GOP / RX7900XTX24GB | GPU PCI ID未確認 | UEFI GOP→framebuffer | GOP描画記録、native GPU加速の主張なし、GPU型番は所有者申告 | NUC5 / Ryzen | [実機状況](hardware-verification-status.md) |
| 有線LAN | Intel I218-V | 候補8086:15A3、実revision待ち | PCI→Intel MAC/PHY→lwIP | 実DHCP/echo計画、任意PHY probeは通信対応ではない | NUC5 | [I218 probe](i218-phy-probe.md)、[LAN計画](lan-plan-0.4.0.md) |
| 有線LAN | ASUS L8200A / RTL8111H系 | 見込み10EC:8168、実revision/subsystem待ち | PCIe→別Realtek driver→lwIP | 必須目標13、driver未実装 | Ryzen X570 | [特定手順](ryzen-realtek-identification.md) |
| 有線LAN | Intel I225-LM | 未確認 | PCIe→別の将来MAC経路 | 入手後試行計画、受入なし | 予定NUC11TNKi3 | [所有者構成](hardware-verification-status.md) |
| 有線LAN | Intel82574 | QEMU8086:10D3 | 仮想PCI→現network経路→lwIP | QEMU協調DHCP/UDP検証、実I218/Realtek証拠ではない | QEMU | [工程3回帰](stage3-0.4.0-progress.md) |
| 無線 | Intel AC7265 / AX201 | 正確なWi-Fi ID未確認、NUC5 USB8087:0A2Aは別に列挙済み | AC7265 PCIe / AX201 CNVi | Wi-Fi driverなし、AX201は所有者予定・未入手 | NUC5 / 予定NUC11 | [受入](hardware-acceptance-0.3.0.md)、[所有者構成](hardware-verification-status.md) |
| Bluetooth | CSR8510系dongle、Xbox1914 / BLE Magic Keyboard | dongle厳密VID:PID/revision待ち | USB HCI→LE SMP/ATT/GATT/HOGP | 必須目標12計画、Classicと内蔵AC7265/AX201 BTは基準経路外、任意HOGP mouseはgate外 | NUC5 / Ryzen | [計画](work-plan-0.4.0.md) |
| 音声 | Realtek S1200A | 未確認 | 内蔵audio | 対象外、chipは所有者申告 | Ryzen | [所有者構成](hardware-verification-status.md) |
| USB host | Intel9-series xHCI | 実測8086:9CB1、revision待ち | PCI→xHCI | keyboard/storageの記録経路で対応、port別結果に従う | NUC5 | [受入](hardware-acceptance-0.3.0.md) |
| USB host | AMD xHCI | 所有者情報1022:149C、revision待ち | PCI→xHCI、背面Gen1 A×4 / Gen2 A×2 C×1 | Ryzen既存USB証拠、chipと背面port対応は未実測 | Ryzen X570 | [実機状況](hardware-verification-status.md) |
| Hub / USB4 | USB2 hub / NUC11 TB4×2 | hub固有ID未確認 | USB2 hub chain / 将来TB4/USB4 | 5段hubはQEMUのみ、USB2対応でTB4/USB4経路は確定しない | QEMU / 予定NUC11 | [工程3回帰](stage3-0.4.0-progress.md)、[所有者構成](hardware-verification-status.md) |
