# Ryzen Ethernet identification / Ryzen Ethernetの特定

Current trace follow-up: detailed control/stage/timing and BAR evidence requires a newly built `log=trace` EFI; F8/8 selects trace. The earlier development EFI hashes and observations below predate trace and keep their original settings. Concise device facts remain at devices; no old artifact is relabeled or regenerated here. See [boot log](boot-log.md).

現行Trace仕上げ：詳細制御／stage／計測／BAR証拠は新sourceからbuildした `log=trace` EFIで採取し、F8／8もtraceへ強制します。下記の旧開発EFI hash／観測はTrace追加前なので元の設定と来歴を保持します。devicesは機器事実の要約です。本追記で旧artifactを付け替えたり再生成したりしません。[boot log](boot-log.md)参照。


Status: current (read-only diagnostic procedure; physical capture pending).

## English

### Current echo opt-in boundary (2026-10-16 JST)

With an EFI implementing the new default-off switch, the owner enables a subsequent network test by setting `lan.echo=on` in `SETTINGS/CONFIG.TXT` on the FAT32 boot volume; preserve other keys and record bytes/hash plus EFI source/features/hash. DHCP is independent of echo. Confirm actual enabled state, then require a real lease and a separately returned UDP12345 token as in the [I218 echo procedure](i218-hardware-quickcheck.md). This setting does not turn the read-only TxConfig diagnostic into a driver, grant Realtek hardware admission or waive profile/stop checks. Identification-only runs need no echo opt-in. Old EFIs below retain their original parser/service behavior.

### Development EFI read-only capture (2026-10-15)

Use an owner-approved development EFI with **`t2-diagnostics,diagnostics-detail`** and `log=devices` (F8/8 can force devices). This is a local build feature, not a claim that the NIC works. Record EFI SHA256, CONFIG, board, cold interval (power off at least10 seconds) and AC state. The normal build does not perform this capture. Copying EFI to physical USB and running it remain owner operations.

On the devices inventory/journal pages photograph `RTL BDF`, PCI10EC:8168 revision, subsystem vendor:device, PCI command, BAR0/BAR2 type/base/length/raw and `RTL TXCONFIG40`. A zero length means firmware resource evidence was unavailable, not a measured zero-sized BAR. The diagnostic reads PCI configuration only; it never probes BAR sizes, changes command/BME, enables decode, initializes PHY/OCP or writes NIC MMIO. Its only NIC register access is one volatile32-bit TxConfig read at offset0x40, guarded across subsequent display calls.

Firmware PCI I/O resource descriptors must agree with the live memory BAR. The entire page-aligned region must pass the existing memory-map checks (physical bounds, no RAM/runtime/reserved/other-MMIO overlap), then be mapped UC/read-only/NX in the OS tables. PCI identity, BARs and memory decode are rechecked before reading. Missing evidence produces `RTL TXCONFIG SKIP RESOURCE/UC/DECODE` and no read. UC is a page-table choice with existing PAT/MTRRs preserved; it is not inferred from the PCI ID. Unsupported or nonmatching PCI devices receive no NIC register access. Diagnostic PCI discovery is limited to the first matching segment-zero function.

Linux is unnecessary for this path. `lspci -xxx` dumps PCI configuration space; it does not obtain the TxConfig MMIO register value. Do not substitute a configuration offset0x40 value. Do not guess addresses or use arbitrary register-write tools. If the safe path skips, retain all screenshots and the reason for resource review.

Supply the raw capture, exact motherboard/NIC identification, EFI hash and BDF/revision/subsystem/BAR evidence for MAC-version selection. Only then review the corresponding PHY/OCP/firmware requirements and implement reset/link, rings/frame adapter, existing lwIP DHCP/UDP and stage5 stop/restart. Host negative tests and QEMU82574 regression precede an opt-in Ryzen physical trial. PCI revision plus TxConfig is identification evidence, not permission to perform an unreviewed register sequence. The current Realtek host prototype has no runtime transport; normal LAN defaults remain unchanged pending physical acceptance.


The corrected detailed development EFI is `out/t2-detail-segment-fix-20261016/files/EFI/BOOT/BOOTX64.EFI`, SHA256 `710cf02f0a669631f81a05d0d1c21690c98f51bd8c1c4055c36b6e2dc39c52fb`, built from commit `12a4aa0c1395856d8e5a56959fb6b1803107f86c` plus the recorded uncommitted diagnostic-only Ready-stop correction. It includes the FS/GS takeover correction. The old `59a701b1…` build had `t2-diagnostics` alone: Realtek-specific rows entered the journal but were not shown by its normal summary. `log=devices` alone cannot add the missing compiled page flow. The new `diagnostics-detail` build renders and rotates retained journal pages; look for RTL BDF, subsystem/command, BAR0/BAR2 and TXCONFIG40 or its explicit skip reason. For sustained diagnostic hold use owner-prepared media with no AUTORUN; do not delete an existing application to obtain it. An AUTORUN can divert into application execution. This is an evidence build, not a normal boot/acceptance image.

### Retained host identification routes

Status: identification required before goal13 Realtek driver work. Owner scope requires the Ryzen machine's Realtek Ethernet controller, but the exact PCI device and motherboard are **not identified** from the retained metadata. Reviewed: six JSON/text/log files under `out/ryzen-*`, the Ryzen hardware results, and `docs/hardware-evidence/2026-10-13/ryzen7/provenance.json`. They establish boot/video/provenance observations, not a NIC model. No chip or board is inferred from video appearance, CPU name or xHCI count.

The owner performs the following read-only capture on the same Ryzen machine. Do not change firmware settings, drivers, partitions or USB contents. Record the date and which physical Ethernet socket is connected. Keep USB Ethernet adapters separate from the motherboard PCI NIC.

Owner update2026-10-15: ASUS TUF GAMING X570-PLUS and ASUS L8200A/RTL8111H-class were reported;10EC:8168 is expected, not measured. See the appended [hardware configuration](hardware-verification-status.md). The new devices diagnostic records the first Ethernet PCI ID/revision regardless of vendor; all Ethernet observations enter its bounded journal. xHCI ID/revision is also shown. Multiple NICs, omitted journal entries or missing subsystem IDs still require the OS capture below.

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

After review, append the explicit identifiers and evidence path to a new Ryzen section of [hardware verification status](hardware-verification-status.md). Match the device-specific programming reference before implementation. Earlier normal devices diagnostics selected only Intel Ethernet. The current read-only inventory adds the first Ethernet of any vendor and bounded journal observations; missing/omitted rows still do not prove hardware absence. Cross-check subsystem IDs and multiple NICs with the host capture. I218-V and Realtek need separate driver initialization and physical link/DHCP/OS-echo evidence; QEMU82574 proves neither physical controller.

## 日本語

### 現行echo有効化の境界（2026-10-16 JST）

新しい既定off switch対応EFIで後続の通信試験を行う場合、所有者がFAT32起動媒体の`SETTINGS/CONFIG.TXT`へ他keyを保持して`lan.echo=on`を設定し、原bytes/hashとEFI source/features/hashを記録。DHCPとechoは独立です。実効enabled、実lease、[I218 echo手順](i218-hardware-quickcheck.md)の独立したUDP12345返信を確認します。この設定は読取専用TxConfig診断をdriverに変えず、Realtek実機有効化やprofile/停止確認を省略しません。特定だけのrunはecho有効化不要。下記旧EFIは元parser/service挙動のままです。

### 開発EFIの読取り専用採取（2026-10-15）

所有者承認対象の開発EFIを**`t2-diagnostics,diagnostics-detail`**・`log=devices`で使用します（F8/8でもdevices強制可）。これはローカルbuild機能でNIC動作の証明ではありません。EFI SHA256、CONFIG、board、電源断10秒以上の冷間条件とAC状態を記録。通常版は本採取を行いません。実USBへのcopy・実行は所有者操作です。

devices inventory/journalで`RTL BDF`、PCI10EC:8168 revision、subsystem vendor:device、PCI command、BAR0/BAR2の種別/base/長さ/raw、`RTL TXCONFIG40`を撮影。長さ0はfirmware資源根拠が得られなかった意味で、実測サイズ0ではありません。診断はPCI設定を読み取るだけで、BARサイズprobe、command/BME変更、decode有効化、PHY/OCP初期化、NIC MMIO書込みを行いません。唯一のNIC registerアクセスはoffset0x40のvolatile32-bit読取り1回で、表示再呼出しでも増えません。

firmware PCI I/O資源descriptorとlive memory BARの一致が必要です。ページ整列した全領域を既存memory-map検査（物理範囲、RAM/runtime/予約領域/別MMIOとの非重複）へ通し、OS page tableでUC・読取り専用・NXへmapします。読取り前にPCI識別・BAR・memory decodeを再照合。未確定なら`RTL TXCONFIG SKIP RESOURCE/UC/DECODE`とし読みません。UCは既存PAT/MTRRを保持するpage table属性で、PCI IDからの推測ではありません。不一致deviceのNIC registerには触れず、segment0の最初の一致functionだけを採取します。

Linuxは不要です。`lspci -xxx`はPCI設定空間のdumpでTxConfig MMIO値は得られず、設定offset0x40の値を代用できません。アドレス推測・任意register書込みtoolは使いません。skip時は理由と全写真を資源review用に保持します。

raw採取値、正確なboard/NIC、EFI hash、BDF/revision/subsystem/BAR根拠を揃えMAC版を選定。その後に対応PHY/OCP/firmware要件、reset/link、ring/frame adapter、既存lwIP DHCP/UDP、工程5停止/再起動を実装します。host負例とQEMU82574回帰後にRyzen任意feature実機試験へ進みます。PCI revisionとTxConfigは識別根拠で未review書込み手順の許可ではありません。現Realtekはhost prototypeでruntime通信未接続、通常LAN既定は実機合格まで維持します。

修正詳細診断EFIは`out/t2-detail-segment-fix-20261016/files/EFI/BOOT/BOOTX64.EFI`、SHA256 `710cf02f0a669631f81a05d0d1c21690c98f51bd8c1c4055c36b6e2dc39c52fb`、commit `12a4aa0c1395856d8e5a56959fb6b1803107f86c`＋記録した診断限定Ready停止の未commit修正由来でFS/GS takeover修正を含みます。旧`59a701b1…`は`t2-diagnostics`だけでRTL専用行をjournalへ保持しても通常summaryには出しませんでした。`log=devices`だけでは未compileの詳細page経路を追加できません。新`diagnostics-detail`版はjournalを描画・巡回し、RTL BDF、subsystem/command、BAR0/BAR2、TXCONFIG40又はskip理由を採取できます。継続holdには所有者が準備するAUTORUN無し診断媒体を使用し、既存appを削除して用意しません。AUTORUNがあるとapp経路へ進む場合があります。通常boot/受入像ではなく証拠採取版です。

### 保持するhost識別手順

状態：目標13 Realtek driver着手前の特定待ち。所有者scopeはRyzen機のRealtek Ethernetを必須とするが、保持metadataから正確なPCI deviceとmotherboardは**未特定**。`out/ryzen-*` 配下のJSON/text/log計6件、Ryzen実機結果、`docs/hardware-evidence/2026-10-13/ryzen7/provenance.json` を確認した。起動・動画・来歴の観測はあるがNIC型番の根拠ではない。動画の見た目、CPU名、xHCI数からchip/boardを推定しない。

所有者が同じRyzen機で以下を読取り専用採取する。firmware設定、driver、partition、USB内容は変更しない。日時と接続する物理Ethernet端子を記録し、USB Ethernet adapterをmotherboard PCI NICと分ける。

所有者追記2026-10-15：ASUS TUF GAMING X570-PLUS、ASUS L8200A/RTL8111H系の申告あり。10EC:8168は見込みで未実測。[実機構成追記](hardware-verification-status.md)参照。新devices診断はvendorを限定せず先頭EthernetのPCI ID/revisionを表示し、全Ethernet観測を上限付きjournalへ記録します。xHCI ID/revisionも表示します。複数NIC、journal省略、subsystem ID不足は以下のOS採取で補います。

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

レビュー後、[実機確認状況](hardware-verification-status.md)の新Ryzen節へ明示IDと証拠pathを追記し、device固有の資料と照合して実装する。以前の通常devicesはIntel Ethernetだけを選んでいた。今回の読取り専用一覧は全vendorの先頭Ethernetと上限付きjournalを追加するが、行の欠落・省略は機器不存在を証明しない。subsystem IDと複数NICはhost採取で相互確認する。I218-VとRealtekは別初期化と実機link／DHCP／OS echo証拠が必要で、QEMU82574合格をどちらの実機対応にも読み替えない。

Development EFI identity / 開発EFIの識別: [three diagnostic artifacts and original provenance](hardware-acceptance-0.4.0.md#development-efi). Select the purpose-specific artifact and record CONFIG; these are not normal-candidate acceptance. / 用途別artifactとCONFIGを識別し、通常候補受入とは区別してください。
