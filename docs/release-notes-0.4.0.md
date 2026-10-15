# Musha-OS v0.4.0

Status: current (unpublished release-note skeleton, 2026-10-15). All release gates PENDING.

## English

### Publication record

Publication date: PENDING. Source/tag commit: PENDING. Candidate EFI SHA256: PENDING. Candidate asset manifest: PENDING. Final CI: PENDING. No v0.4.0 publication or physical acceptance is recorded here; v0.3.0 published assets/evidence are unchanged. This is a worksheet, not an announcement or authority to publish.

### Main changes and implementation boundary

The v0.4.0 development tree includes configurable startup, an empty shipped SETTINGS directory with bounded OS-owned FAT32 records, reorganized drive assets, experimental r2 TIME/WAIT/INPUT calls, C/Rust samples, relative USB mouse input, and viewer launch/return with device stop and arena cleanup. Local test results remain in the individual stage records and do not pre-fill this candidate's gates.

Stage6 contains USB HCI/LE pairing, stored reconnect and descriptor-driven HOGP keyboard/controller software paths. Implementation and host/software-fixture coverage are distinct from actual adapter, BLE keyboard and Xbox1914 compatibility: physical Bluetooth is unverified; HOGP keyboard hardware acceptance is deferred to v0.5.0. Classic/MagicA3118 is outside the reference LE path. IRK/RPA and unsupported security/descriptor paths remain limited; see the [current stage6 checkpoint](stage6-0.4.0-progress.md).

I218 networking is an opt-in implemented development path with physical link/DHCP/echo pending. Realtek requires exact PCI/revision/TxConfig profile evidence and its separate backend; unsupported profiles are not accepted. T2 has host models and shared input preparation, but live BCE MMIO/DMA/VHCI/internal keyboard integration remains unimplemented. Mac storage cause/repeatability remains a separate hardware gate. Refer to the [device matrix](device-support.md), [scope](release-scope-0.4.0.md) and [work plan](work-plan-0.4.0.md) for exact boundaries; no pending work is described as a completed product feature.

### Downloads (planned; not uploaded)

- `musha-os-0.4.0-x64.zip`
- `musha-os-0.4.0-x64-qemu.img.gz`
- `musha-os-0.4.0-c-sdk.zip`
- `musha-os-0.4.0-rust-sdk.zip`
- `SHA256SUMS`
- `build-manifest.json`

No v prefix in asset names. The image is QEMU-only; do not raw-copy it to a larger physical USB. The drive ZIP is extracted to an identified FAT32 volume; physical preparation is an owner operation. docs.zip is conditional on completed documentation cleanup/goal17 and is not promised by this skeleton. No deployment ZIP/candidate tar is planned. Both SDKs must pass their runnable-sample and relocation gates; archive existence alone is not completion.

### Running your program / viewer

Follow the [SDK guide](sdk-guide.md) and [viewer guide](file-viewer.md) for the current source contract. AUTORUN.ELF is optional and automatically attempted once; supported app exit/refusal/recoverable faults return to a fresh viewer. The ABI remains experimental, without application networking/POSIX/thread guarantees. Do not copy v0.3-only behavior or asset names into v0.4 acceptance.

### Validation scope

| Gate / 確認項目 | Status / 状態 | Evidence / 証拠 |
|---|---|---|
| QEMU / exact candidate regression | PENDING | — |
| Condition1: two clean same-source full-asset builds reproduce | PENDING | — |
| Condition2: accepted-runtime equality across documentation successor; otherwise renewed acceptance | PENDING | — |
| NUC5 cold1 / cold2 / cold3 (≥10s off, record AC) | PENDING | — |
| Ryzen cold1 / cold2 / cold3 and hot1 with BT dongle | PENDING | — |
| OS-owned SETTINGS write → reboot → read-back; CONFIG untouched | PENDING | — |
| macOS and Windows mount/filesystem verification after OS writes | PENDING | — |
| Rust Hello on NUC5 and Ryzen | PENDING | — |
| USB mouse sample on NUC5 and Ryzen | PENDING | — |
| Viewer launch/return ×3 without reboot | PENDING | — |
| Bluetooth Xbox1914 pairing, durable save, reconnect and SYS_INPUT | PENDING | — |
| NUC5 I218 link/DHCP/UDP12345 echo | PENDING | — |
| Ryzen Realtek exact-profile link/DHCP/UDP echo | PENDING | — |
| Mac same-topology storage A/B and FAT32/exFAT read or bounded cause/limit | PENDING | — |
| Mac T2 internal keyboard/viewer and external regression | PENDING | — |
| Documentation stage2 / formatter gate / index and links / docs.zip decision | PENDING | — |
| Final candidate source/hash/provenance and final-source CI (not run here) | PENDING | — |
| Published download byte/hash verification after authorized publication | PENDING | — |

Record candidate hashes, exact machine/topology/power conditions, raw photos/logs, time intervals and owner review before changing any status. [Hardware worksheet](hardware-acceptance-0.4.0.md) carries per-run evidence. Image-embedded documentation/license changes alter image bytes and require renewed acceptance under condition2; image-external document-only changes qualify only when every required runtime member remains identical.

## 日本語

### 公開記録

公開日・source/tag commit・候補EFI SHA256・配布manifest・final CIは全てPENDINGです。v0.4.0の公開/実機受入を記録したものではなく、v0.3.0公開物/証拠は不変です。本書は記入用骨組みで、告知や公開承認ではありません。

### 主な変更と実装境界

開発treeには起動設定、空で配布するSETTINGSとOS所有FAT32限定record、配布構成、実験的r2 TIME/WAIT/INPUT、C/Rust sample、USB相対mouse、device停止/arena cleanupを伴うviewer起動/復帰があります。各stageのローカル結果で今回候補gateを先取りしません。

工程6にはUSB HCI/LE pair・保存再接続・descriptorに基づくHOGP keyboard/controllerのsoftware経路があります。実装/host/software fixtureと、実adapter・BLE keyboard・Xbox1914互換は別です。実BT未検証、HOGP keyboard実機受入はv0.5.0延期。Classic/MagicA3118はLE基準外。IRK/RPA/security/descriptor等の制約は[工程6](stage6-0.4.0-progress.md)参照。

I218はopt-in開発実装で実link/DHCP/echo待ち。Realtekは実PCI/revision/TxConfig profileと別backendが必要で未対応profileを受理しません。T2はhost model/共通入力準備までで、実BCE MMIO/DMA/VHCI/内蔵入力接続は未実装。Mac storage原因/反復は別gate。[機器表](device-support.md)・[scope](release-scope-0.4.0.md)・[計画](work-plan-0.4.0.md)に従い未完了を製品対応済みとしません。

### 配布物（予定・未upload）

- `musha-os-0.4.0-x64.zip`
- `musha-os-0.4.0-x64-qemu.img.gz`
- `musha-os-0.4.0-c-sdk.zip`
- `musha-os-0.4.0-rust-sdk.zip`
- `SHA256SUMS`
- `build-manifest.json`

asset名はv無し。像はQEMU専用で大容量実USBへraw copyしません。drive ZIPは識別したFAT32へ展開し、実媒体準備は所有者操作です。docs.zipは文書第2段/目標17完了後の条件付きで、骨組み段階で約束しません。deployment ZIP/candidate tarは予定しません。両SDKは実行sample/再配置gateが必要でarchive存在だけで完了にしません。

### 自作programとviewer

現行[SDK案内](sdk-guide.md)・[viewer](file-viewer.md)に従います。任意AUTORUN.ELFは初回一度だけ自動起動し、対応する終了/拒否/回復可能fault後は新viewerへ戻ります。ABIは実験的でapp network/POSIX/threadを保証しません。v0.3限定挙動や旧asset名を今回の受入へ流用しません。

### 検証範囲

| Gate / 確認項目 | Status / 状態 | Evidence / 証拠 |
|---|---|---|
| QEMU/正確な候補回帰 | PENDING | — |
| 条件1：同一sourceからclean2buildで全配布物再現 | PENDING | — |
| 条件2：文書後継の受入runtime一致、不一致なら再受入 | PENDING | — |
| NUC5冷間1/2/3（off≥10秒、AC記録） | PENDING | — |
| Ryzen冷間1/2/3・dongle付き温間1 | PENDING | — |
| OS所有SETTINGS書込み→再起動→読戻し、CONFIG不変 | PENDING | — |
| OS書込み後macOS/Windows mount・filesystem検査 | PENDING | — |
| NUC5/RyzenでRust Hello | PENDING | — |
| NUC5/RyzenでUSB mouse sample | PENDING | — |
| 再起動無しviewer launch/return3回 | PENDING | — |
| BT Xbox1914 pair/永続保存/再接続/SYS_INPUT | PENDING | — |
| NUC5 I218 link/DHCP/UDP12345 echo | PENDING | — |
| Ryzen Realtek実profile link/DHCP/UDP echo | PENDING | — |
| Mac同一接続storage A/B・FAT32/exFAT又は原因/制約 | PENDING | — |
| Mac T2内蔵keyboard/viewer・外付け回帰 | PENDING | — |
| 文書第2段/formatter gate/索引リンク/docs.zip判断 | PENDING | — |
| 最終候補source/hash/provenance・final-source CI（今回未実行） | PENDING | — |
| 承認公開後downloadのbyte/hash照合 | PENDING | — |

status更新前に候補hash・実機/接続/電源条件・原写真/log・時間・所有者reviewを記録。[実機様式](hardware-acceptance-0.4.0.md)へrun毎証拠を保存。像内文書/license変更は像bytesを変え条件2の再受入が必要。像外文書だけでも必要runtime全member一致が条件です。
