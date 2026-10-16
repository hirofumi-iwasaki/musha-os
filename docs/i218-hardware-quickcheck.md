# NUC5 I218 LAN quickcheck / NUC5 I218 LAN実機1ページ手順

Status: current (procedure only, 2026-10-15). Physical networking remains unverified.

## English

For a v0.4.0 EFI implementing [boot-log](boot-log.md), use `log=devices` and photograph continuous white-on-black lines, with PM timestamps and scrolling. Devices now gives concise per-device decisions; use `log=trace` (or the F8/8 startup override) for detailed transfer/stage/timing and inventory evidence. The ring retains all detail at every display level. No rotating diagnostic pages, old colored status words or PM footer are required. Record one sequential trace log for detailed evidence: PCI/PHY/reset/link, backend, budget/init and lease times, actual DHCP address or exact optional-lan refusal. Photograph kernel ERROR lines separately from lan unavailability; preserve vector number, error/n-a and RIP. Old IMG_7654 panel/exception words below belong to its exact EFI only. Hardware acceptance is still pending.

For the new default-off implementation, the owner prepares the FAT32 boot volume's `SETTINGS/CONFIG.TXT` with `lan.echo=on` alongside the requested log/display settings, preserving other settings and recording its exact bytes/hash. Use only an EFI known to implement this switch; old recorded EFIs keep their old behavior. Confirm the effective enabled state before UDP12345 testing. Default off still permits DHCP, so a lease alone does not imply an echo listener. This is a host configuration instruction, not authority for the agent to update USB.

Recorded2026-10-16 NUC5 trial (`IMG_7654.MOV`, previous EFI268b9a49…): **FAIL** at software-NVM refusal, followed by separate #GP during app CPU recovery. No DHCP/echo or cold criteria confirmed. [Diagnosis and corrected development EFI](nuc5-segment-recovery-0.4.0.md); physical retest remains pending. Preserve that failure and do not count local QEMU recovery as a physical pass. For corrected EFI2275d088… record three separate checks: (1) no #GP and USB READ → AUTORUN → ready/viewer recovery, (2) LAN succeeds or the NVM reason remains visible while boot continues nonfatally, (3) UDP echo only after an actual DHCP lease. This EFI retains NVM refusal; it is primarily a CPU-recovery/fallback retest, not an expectation of LAN success.

1. Connect NUC5 Ethernet and the Mac to the same IPv4 LAN with a working DHCP server. Record switch/router, cable/ports, Mac interface/IP, NUC model/firmware and link LEDs. Connect boot USB and external keyboard directly; photograph topology. A direct cable without DHCP does not satisfy this test.
2. Identify the approved development EFI by commit/hash and **`i218-network`** feature. Build-only command: `cargo build --locked --release --target x86_64-unknown-uefi -p musha-boot --features i218-network`. Output is `target/x86_64-unknown-uefi/release/musha-boot.efi` unless target-dir overrides it. `tools/build-usb.py` has no arbitrary-features option. Packaging/copying to physical USB is a separate owner operation, not performed here. `i218-phy-probe`/`i218-power-probe` alone do not enable LAN; the normal build does not establish physical I218 support.
3. Use `log=trace` for this detailed PHY/reset/budget record, cold boot ≥10 s and record AC state. Capture actual Ethernet PCI ID/revision/BDF (8086:15A3 is a candidate), MAC/PHY/reset/link diagnostics, supplied DHCP IPv4 and ready/error log lines. Initialization shares one5-second budget across PHY/reset/flash waits. Expiry disables and quarantines LAN, then boot continues only after PCI confirms BME off; an unconfirmed stop remains a safe halt. Photograph the concrete phases reason and devices backend/link/address; retain trace for PHY/reset/init/lease durations. There is no static-IP fallback; initial/no-address timeout is 30 s. A link LED or detected PCI function alone is not communication.
4. On macOS Terminal run `nc -u -w 3 <NUC_DHCP_IP> 12345`, replacing the placeholder with the observed lease. Type a unique short ASCII token and Return. Record **a second returned copy** of the same token (the first visible line may be terminal local echo). Ctrl-C exits if needed. No `-l` listener: Musha is the UDP server. When explicitly enabled, `net/port/shim.c` binds UDP12345 and echoes the same payload to the sender; accepted payload limit1472 bytes. Capture Mac output plus OS UDP count before/after. Repeat a new token after opening/returning from a viewer ELF; record keyboard/file progress.
5. Stop normally and capture failure/stop log lines if present. Record exact failing stage, timeout and whether viewer remains usable; do not promote QEMU82574 results to I218 evidence. No router reconfiguration or physical media writing is performed by this procedure preparation.

| Record | Value |
|---|---|
| Date/operator; EFI commit/features/hash; CONFIG/hash | |
| Topology, link LEDs; actual PCI ID/revision/BDF, PHY/MAC | |
| Cold interval/AC; DHCP address/time or error | |
| Mac IP/interface; sent/returned tokens; UDP counts before/after | |
| Viewer/file/input and shutdown result; photos/log paths; PASS/FAIL/BLOCKED | |

See [implementation limits](i218-network-0.4.0-progress.md) and [LAN plan](lan-plan-0.4.0.md).

## 日本語

[bootlog](boot-log.md)実装v0.4.0 EFIは`log=devices`でPM時刻付き白黒逐次log/scrollを撮影。devicesは機器判断を簡潔に表示し、詳細transfer/stage/timing・一覧は`log=trace`又はF8/8起動overrideで採取。ringは全表示levelで詳細を保持。診断巡回頁・旧色status語・PM footerは条件にしません。PCI/PHY/reset/link/backend/budget/init/lease時間、実DHCP又は任意LAN拒否理由を1本のtrace logで採取。kernel ERRORをLAN非対応と分けvector/error又はn-a/RIPを保持。下記IMG7654旧panel/例外語は当時EFIのまま保持。 実機受入は未完です。

新しい既定off実装のecho試験では、所有者がFAT32起動媒体の`SETTINGS/CONFIG.TXT`へ他設定を保持して`lan.echo=on`を設定し、log/displayと原bytes/hashを記録します。このswitch対応のEFIだけを使い、旧記録EFIの挙動は変更しません。実効enabled状態を確認してUDP12345を試験。既定offでもDHCPは継続するため、lease取得だけではecho listener有効ではありません。host設定の手順で、agentへのUSB更新許可ではありません。

2026-10-16記録のNUC5試行（`IMG_7654.MOV`・旧EFI268b9a49…）はsoftware-NVM拒否、その後app CPU復帰時の別#GPにより**FAIL**。DHCP/echo・冷間条件は未確認。[診断・修正開発EFI](nuc5-segment-recovery-0.4.0.md)参照、実機再確認は未実施。失敗を保持し、QEMU回復を実機合格と数えません。修正EFI2275d088…では3点を別記録：①#GP無しでUSB READ→AUTORUN→ready/viewer復帰、②LAN成功又はNVM理由付きの非致命boot継続、③実DHCP lease取得時のみUDP echo。本EFIはNVM拒否を保持するため、LAN成功を期待する試験ではなくCPU復帰/正常fallback再試験が主目的です。

1. NUC5有線LANとMacをDHCP serverのある同一IPv4 LANへ接続。switch/router、cable/port、Mac interface/IP、NUC型番/firmware、link LEDを記録。起動USB/外付けkeyboardを直結し接続写真を保存。DHCP無し直結は本試験を満たしません。
2. 承認対象の開発EFIをcommit/hashと**`i218-network`** featureで識別。buildのみのcommandは`cargo build --locked --release --target x86_64-unknown-uefi -p musha-boot --features i218-network`。target-dir指定がなければ出力は`target/x86_64-unknown-uefi/release/musha-boot.efi`。`tools/build-usb.py`に任意feature引数はありません。物理USBへのpackage/copyは別途所有者操作で、本作業では実行しません。PHY/power probeだけではLANを有効にせず、通常buildも実I218対応の証拠ではありません。
3. 詳細PHY/reset/budget記録は`log=trace`でoff ≥10秒の冷間起動、AC状態も記録。実Ethernet PCI ID/revision/BDF（8086:15A3は候補）、MAC/PHY/reset/link、DHCP IPv4、ready/error log行を撮影。PHY/reset/flash待ち全体で5秒budgetを共有します。超過時はLANを隔離し、PCIでBMEoff確認後のみboot継続。停止未確認は安全haltを維持します。phasesの理由とdevicesのbackend/link/addressを撮影し、PHY/reset/init/lease所要はtraceで保持。static fallbackは無く初回address待ちは30秒。LED/PCI検出だけでは通信成功ではありません。
4. Mac Terminalで`nc -u -w 3 <NUC_DHCP_IP> 12345`（実leaseへ置換）。短い固有ASCII tokenを入力しReturn。同じtokenの**2度目の返り**を確認（最初の表示は端末local echoの場合あり）。必要ならCtrl-Cで終了。`-l`は使いません。Musha側がUDP serverです。明示有効時だけ`net/port/shim.c`はUDP12345にbindし、受信payloadを送信元へそのまま返します（上限1472 byte）。Mac出力とOS UDP counterの前後を撮影。viewer ELFを開いて戻った後も別tokenで反復し、入力/file進行を記録。
5. 通常停止し失敗/停止log行も保存。止まった段階・timeout・viewer操作可否を明記。QEMU82574成功をI218証拠にしません。本手順準備ではrouter変更・実媒体書込みを行いません。

| 記録欄 | 値 |
|---|---|
| 日時/実施者、EFI commit/features/hash、CONFIG/hash | |
| 接続/LED、実PCI ID/revision/BDF、PHY/MAC | |
| 冷間秒数/AC、DHCP address/時間又はerror | |
| Mac IP/interface、送信/返信token、UDP counter前後 | |
| viewer/file/input/停止、写真/log path、PASS/FAIL/BLOCKED | |

[実装制限](i218-network-0.4.0-progress.md)・[LAN計画](lan-plan-0.4.0.md)参照。

Development EFI identity / 開発EFIの識別: [three diagnostic artifacts and original provenance](hardware-acceptance-0.4.0.md#development-efi). Select the purpose-specific artifact and record CONFIG; these are not normal-candidate acceptance. / 用途別artifactとCONFIGを識別し、通常候補受入とは区別してください。
