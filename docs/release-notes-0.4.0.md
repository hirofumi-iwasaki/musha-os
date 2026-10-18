# Musha-OS v0.4.0

Status: current (unpublished release-note skeleton, 2026-10-16). All release gates PENDING.

## English

### Publication record

Publication date: PENDING. Source/tag commit: PENDING. Candidate EFI SHA256: PENDING. Candidate asset manifest: PENDING. Final CI: PENDING. No v0.4.0 publication or physical acceptance is recorded here; v0.3.0 published assets/evidence are unchanged. This is a worksheet, not an announcement or authority to publish.

### Main changes and implementation boundary

Owner-approved display replacement: [boot-log](boot-log.md) defines one full-frame sequential white/black console for startup/device decisions/app output/errors, concise devices-level decisions, trace-level transfer/stage/timing detail, PM timestamps, wrapping/scrolling and an all-detail event ring. F8/8 selects trace. Phases starts with `Musha-OS 0.4.0 starting`; `Musha-OS 0.4.0 ready` follows required USB-host/boot-media/keyboard discovery and precedes app start. Ready level includes app start/exit/fault/rejection and raw output. The PAT entry-1 WC update after EBS and before table construction passed the final workspace suite (438 tests, including the 26 memory tests) and seven startup-level/F8/8 QEMU cases with seven independent full-pixel comparisons. These are one final checkpoint, not added to earlier overlapping runs. One PAT write and temporary two-write MTRR restore were verified with installed GOP index-1/RW/NX and DMA UC leaves; PAT0=WB is required for CR3/nonleaf page walks; audit/CPU prerequisites fail closed. Physical effective WC and speed remain unverified, and old UC-fallback artifacts do not prove this update. It removes diagnostic panels/pages, colored status words, PM footer, startup background fill, red exception band and separate80x24 app console. The viewer keeps its own layout; viewer→app clears the console and app→viewer repaints it. Required failures remain visible at all levels; optional LAN/BT reasons are nonfatal only where cleanup is confirmed. Serial mirroring remains a v0.5.0 goal. Implementation/tests and exact-EFI hardware photographs are required before this contract closes a release gate; older panel-based evidence remains historical.

The v0.4.0 development tree includes configurable startup, an empty shipped SETTINGS directory with bounded OS-owned FAT32 records, reorganized drive assets, experimental r2 TIME/WAIT/INPUT calls, C/Rust samples, relative USB mouse input, and viewer launch/return with device stop and arena cleanup. Local test results remain in the individual stage records and do not pre-fill this candidate's gates.

Stage6 contains USB HCI/LE pairing, stored reconnect and descriptor-driven HOGP keyboard/controller software paths. Implementation and host/software-fixture coverage are distinct from actual adapter, BLE keyboard and Xbox1914 compatibility: physical Bluetooth is unverified; HOGP keyboard hardware acceptance is deferred to v0.5.0. Classic/MagicA3118 is outside the reference LE path. IRK/RPA and unsupported security/descriptor paths remain limited; see the [current stage6 checkpoint](stage6-0.4.0-progress.md).

I218 networking is an opt-in implemented development path with physical link/DHCP/echo pending. Realtek requires exact PCI/revision/TxConfig profile evidence and its separate backend; unsupported profiles are not accepted. T2 has host models and shared input preparation, but live BCE MMIO/DMA/VHCI/internal keyboard integration remains unimplemented. Mac storage cause/repeatability remains a separate hardware gate. Refer to the [device matrix](device-support.md), [scope](release-scope-0.4.0.md) and [work plan](work-plan-0.4.0.md) for exact boundaries; no pending work is described as a completed product feature. The [final ready/PAT record](boot-log-ready-pat-0.4.0-results.md), [trace record](boot-log-trace-0.4.0-results.md) and [SDK packaging record](sdk-packaging-0.4.0-progress.md) provide local evidence, not formal candidate or physical acceptance.

UDP echo decision (2026-10-16 JST): **off by default in the new v0.4.0 implementation**. An owner enables the intended test with `lan.echo=on` in the FAT32 boot volume's `SETTINGS/CONFIG.TXT`, preserving other keys and recording the file hash. DHCP remains independent; an address is not proof that UDP12345 echo is enabled. This does not enable a gated NIC or claim physical networking. Old published/development EFIs retain their original behavior. Verify source/hash and effective state rather than assuming an old parser recognizes the new key.

### Kernel size worksheet (not candidate acceptance)

Under the [owner size policy](size-budget.md), record exact source/features/hash and manifest measurements. This normal build is not automatically the owner-defined published full configuration. Raw section sizes include PE alignment padding; virtual `.data`/BSS and image size are RAM, not file bytes.

| Configuration | EFI file | .text / .rdata / .data raw | .data/BSS virtual; image virtual | Previous-release delta |
|---|---|---|---|---|
| Normal candidate (`efi_size`) | NOT MEASURED for this candidate | NOT MEASURED | NOT MEASURED | NOT MEASURED; owner-recorded v0.3 baseline475,648 B, asset/hash correspondence must be recorded |
| Minimal (`efi_size_minimal`) | UNAVAILABLE, null | UNAVAILABLE | UNAVAILABLE | UNAVAILABLE; LAN/BT optional-driver feature separation missing; not a zero-byte result |

The development measurements in size-budget are dated source observations, not substituted candidate values. Minimal remains a tracked requirement, not satisfied by a same-byte no-default-features build. Release gates above remain PENDING.

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

[最終ready/PAT記録](boot-log-ready-pat-0.4.0-results.md)、[trace記録](boot-log-trace-0.4.0-results.md)、[SDK配布検証](sdk-packaging-0.4.0-progress.md)はlocal証拠で、正式候補・実機受入とは別です。


### 公開記録

公開日・source/tag commit・候補EFI SHA256・配布manifest・final CIは全てPENDINGです。v0.4.0の公開/実機受入を記録したものではなく、v0.3.0公開物/証拠は不変です。本書は記入用骨組みで、告知や公開承認ではありません。

UDP echo採用判断（2026-10-16 JST）：**新v0.4.0実装は既定off**。echo試験では所有者がFAT32起動媒体の`SETTINGS/CONFIG.TXT`へ他keyを保持して`lan.echo=on`を設定しhashを記録。DHCPは独立で、address取得だけではUDP12345 echo有効の証明ではありません。閉じたNIC gateを開かず実通信受入を主張しません。旧公開/開発EFIの動作は不変、旧parserが新keyに対応するとはせずsource/hashと実効状態を確認します。

### kernelサイズ記入欄（候補受入ではない）

[所有者のサイズ方針](size-budget.md)に従い正確なsource/features/hashとmanifest計測を記録します。この通常版を所有者定義の公開full構成と自動的に同一視しません。raw sectionはPE alignment padding込み、仮想.data/BSS・imageはRAMでfile bytesではありません。

| 構成 | EFI file | .text/.rdata/.data raw | .data/BSS virtual・image virtual | 前版差分 |
|---|---|---|---|---|
| 通常候補（efi_size） | 今回候補は未計測 | 未計測 | 未計測 | 未計測。所有者記録の0.3基準475,648 Bとのasset/hash対応を記録して比較 |
| 最小（efi_size_minimal） | 利用不可・null | 利用不可 | 利用不可 | 利用不可。LAN/BTの任意driver feature分離が未達で0 Bではない |

size-budgetの開発計測は各source当時の観測で、今回候補の値へ流用しません。minimalは継続要件で、同一bytesのno-default-features buildを達成扱いにしません。公開gateはPENDINGのままです。

### 主な変更と実装境界

所有者承認の表示置換：[bootlog](boot-log.md)で起動/device/app/errorを全画面白黒逐次consoleへ統一し、devicesの簡潔な機器判断、traceの詳細transfer/stage/timing、PM時刻・折返し/scroll・全詳細event ringを使用。F8/8はtraceを選択。phases先頭は`Musha-OS 0.4.0 starting`、必須USB host/起動媒体/keyboard探索後・app開始前に`Musha-OS 0.4.0 ready`を表示。readyにもapp開始/終了/fault/拒否・生出力を表示します。PAT entry 1 WC変更（EBS後・table構築前）は最終workspace438件（memory26件を内包）と起動5level/F8/8のQEMU7件・独立全pixel比較7件に合格。単一の最終checkpointで、旧重複試行を加算しません。PAT1回書込み・MTRR一時2回書込み復元、導入済GOP index 1/RW/NXとDMA UC leafを確認し、CR3/nonleaf page walkのためPAT0=WBを必須とし、監査/CPU前提不足は安全拒否します。実機実効WC・速度は未確認で、旧UC fallback artifactを今回の証拠にしません。診断panel/page・色status・PM footer・起動背景全fill・赤例外帯・80x24専用app枠は廃止。viewerは従来layoutで、app起動でconsole clear/復帰でviewer再描画。必須失敗は全level、任意LAN/BT理由は停止確認済みの範囲で非致命。serialはv0.5.0。実装/試験/正確なEFI実機写真が揃うまでgateを完了せず、旧panel証拠は過去記録として保持します。

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


## SETTINGS writer and Bluetooth bond save — English (2026-10-18 development update)

The SETTINGS writer (only `SETTINGS/PAIRING.DAT`) now reads only the FAT sectors a save needs, so a save on a 30.8 GB, 16 KiB-cluster FAT32 stick takes tens of sector reads instead of ~29,400 (IMG_7741 exceeded the 20,000-read budget). By owner decision it no longer runs a whole-FAT integrity check; it accepts a SETTINGS directory with ARCHIVE/HIDDEN/SYSTEM/READ_ONLY bits or a long name, and a missing or invalid backup FSInfo (primary only, with a warning). Volume labels, a non-directory SETTINGS and an invalid primary FSInfo are still refused. Every refusal is logged with its step, check, reads and time, and the viewer names the kind. Pairing now requests the peer identity key (IRK) and resolves private addresses on reconnect. Physical bond save, reconnect and Xbox key distribution remain unverified. Rules: [SETTINGS files](settings.md).

## SETTINGS書込みとBluetoothボンド保存 — 日本語（2026-10-18開発更新）

SETTINGS writer（対象は`SETTINGS/PAIRING.DAT`のみ）は保存に必要なFATセクターだけを読むようになり、30.8 GB・16 KiBクラスターのFAT32媒体でも約29,400回ではなく数十回の読出しで済みます（IMG_7741は20,000回の予算を超過）。所有者決定により、FAT全体の整合性検査は行いません。ARCHIVE/HIDDEN/SYSTEM/READ_ONLY付きや長い名前付きのSETTINGS、バックアップFSInfoの欠落・不正（主FSInfoのみで続行し警告）を受け付けます。ボリュームラベル、ディレクトリーでないSETTINGS、主FSInfoの不正は従来どおり拒否します。拒否は段階・検査・読出し数・時間とともに記録し、ビューアーに種別を出します。ペアリングで相手のidentity鍵（IRK）を要求し、再接続でprivate addressを解決します。実機のボンド保存・再接続・Xboxの鍵配布は未確認です。規則は[SETTINGSファイル](settings.md)を参照してください。


## Bluetooth own address per bond — English (2026-10-18 development update)

Each new pairing now uses a fresh random static own address from the pairing DRBG, set before connecting and saved with that bond (`PAIRING.DAT` version 3); reconnects reuse it. This lets pairing recover without another device when a controller kept an old bond that this host lost (IMG_7743–7745), and keeps working when the dongle is replaced or the built-in adapter is used. Bonds in older records keep the controller's public address. A peer that stays silent is disconnected after 5 s, deferred for 10 s and retried up to three times. Physical pairing, save and reconnect remain unverified.

## Bluetoothの自アドレスをボンドごとに — 日本語（2026-10-18開発更新）

新規ペアリングごとに、ペアリング用DRBGから新しいランダム静的アドレスを作り、接続前に設定してそのボンドと一緒に保存します（`PAIRING.DAT` version 3）。再接続はそのアドレスを再利用します。このホストが失ったボンドをコントローラーが保持し続けている場合（IMG_7743〜7745）でも、他の機器を使わずにペアリングをやり直せます。ドングルを交換しても、内蔵アダプターを使っても有効です。旧形式の記録のボンドはコントローラーのpublicアドレスのままです。応答しない相手は5秒で切断し、10秒除外して最大3回やり直します。実機のペアリング・保存・再接続は未確認です。
