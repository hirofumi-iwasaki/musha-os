# NUC11 first boot quickcheck / NUC11初回起動1ページ手順

Status: current (future procedure, 2026-10-15). Ordered/planned hardware is not hardware acceptance.

## English

For a v0.4.0 EFI implementing [boot-log](boot-log.md), use `log=devices` and photograph continuous white-on-black lines, with PM timestamps and scrolling. Devices now gives concise per-device decisions; use `log=trace` (or the F8/8 startup override) for detailed transfer/stage/timing and inventory evidence. The ring retains all detail at every display level. No rotating diagnostic pages, old colored status words or PM footer are required. Capture PCI/USB/GOP inventory and selected-mode decisions as log lines, not panel pages; keep the full available GOP-mode list separate from the selected mode. Record NOT EXPOSED if an item is absent. AUTORUN output follows app: starting and app: exited in the same console before viewer repaint. Hardware acceptance is still pending.

1. After arrival record actual NUC11TNKi3 model/board, RAM/SSD, firmware version and display cable/monitor. Connect boot USB and external USB boot keyboard directly; photograph each port. No Supervisor password is required by this plan. Record whether Secure Boot can be set **off without setting a Supervisor password**. If firmware blocks this, record BLOCKED and exact screen; do not invent a password requirement or change unrelated security settings.
2. Owner prepares a separately identified candidate file-copy USB, preserving its existing contents first. Record EFI commit/hash/features and CONFIG bytes/hash; use `log=devices`, no AUTORUN for initial viewer boot. This document authorizes no formatter, raw write or USB execution by the agent. Select the actual UEFI USB entry; cold boot after ≥10 s off, recording AC state.
3. Photograph startup/runtime/ready or failing line, xHCI inventory **all displayed controllers with vendor:device/revision/BDF**, root/slot/port, keyboard and storage discovery, read result and viewer. Capture LAN inventory actual I225 vendor:device/revision/subsystem where available. Owner-reported I225-LM is not a measured ID and an enumerated I225 is not networking support.
4. Capture GOP **available mode list** (mode index, width×height, stride/pixel format if shown), selected mode and monitor connection. Use trace for the full mode list; if that build still omits it, record NOT EXPOSED and request a separate read-only diagnostic; do not report only the chosen mode as the whole list.
5. Test viewer arrows/Enter/Esc and record results. In a **separate owner-prepared run**, place an identified/hash-recorded compatible Hello ELF at `AUTORUN.ELF`, cold boot and capture automatic launch/output/exit. Record this change and run separately; an absent AUTORUN viewer run is not an AUTORUN success. Do not run settings-write/pairing or claim I225/AX201/TB4/USB4 acceptance from this first boot.

| Record | Value |
|---|---|
| Arrival/model/board/RAM/SSD/firmware; EFI/features/hash; CONFIG/hash | |
| Secure Boot off / no Supervisor password / BLOCKED screen | |
| Ports/topology; cold seconds/AC; xHCI IDs/revisions/BDFs | |
| I225 actual ID/revision/subsystem or unavailable | |
| GOP full modes/selected mode/display path or NOT EXPOSED | |
| Viewer/input/read; separate AUTORUN hash/output/exit | |
| Evidence paths; PASS/FAIL/BLOCKED and exact boundary | |

[Device matrix](device-support.md) distinguishes plans from evidence.

## 日本語

[bootlog](boot-log.md)実装v0.4.0 EFIは`log=devices`でPM時刻付き白黒逐次log/scrollを撮影。devicesは機器判断を簡潔に表示し、詳細transfer/stage/timing・一覧は`log=trace`又はF8/8起動overrideで採取。ringは全表示levelで詳細を保持。診断巡回頁・旧色status語・PM footerは条件にしません。PCI/USB/GOP一覧/選択判断をpanel頁でなくlog行として撮影。全GOP一覧と選択modeを区別し欠落はNOT EXPOSED。AUTORUNは同consoleのapp開始/終了行と出力を撮りviewer再描画まで記録。 実機受入は未完です。

1. 到着後に実NUC11TNKi3型番/board、RAM/SSD、firmware版、display cable/monitorを記録。起動USB/USB boot keyboardを直結しportを撮影。Supervisor passwordの設定を前提にせず、**passwordを設定しないままSecure Boot offにできるか**確認。firmwareが拒否すればBLOCKEDと画面を記録し、password必須と推測したり無関係なsecurity設定を変更しません。
2. 所有者が既存内容を保存して識別した候補file-copy USBを用意。EFI commit/hash/features、CONFIG内容/hashを記録し`log=devices`、初回はAUTORUN無し。agentはformat/raw write/実USB操作を行いません。実UEFI USB項目を選びoff ≥10秒の冷間起動、AC状態も記録。
3. startup/runtime/ready又は停止行、表示される**全xHCIのvendor:device/revision/BDF**、root/slot/port、keyboard/storage検出・読出し・viewerを撮影。LAN inventoryでI225の実vendor:device/revision/subsystem（表示可能分）を採取。所有者情報のI225-LMは実IDではなく、検出だけで通信対応にしません。
4. GOPの**利用可能mode一覧**（index、幅×高さ、表示されればstride/pixel format）、選択mode、monitor接続を記録。全mode一覧はtraceで採り、そのbuildでも無ければNOT EXPOSEDとして別の読取専用診断を依頼し、選択modeだけを全一覧としません。
5. viewer矢印/Enter/Escを試して記録。**所有者準備の別run**で、対応Hello ELFのhashを記録し`AUTORUN.ELF`として置き、冷間起動して自動launch/output/exitを撮影。変更とrunを分け、AUTORUN無しviewer起動をAUTORUN成功としません。初回からsettings書込み/pairingやI225/AX201/TB4/USB4受入へ広げません。

| 記録欄 | 値 |
|---|---|
| 到着/型番/board/RAM/SSD/firmware、EFI/features/hash、CONFIG/hash | |
| password無しSecure Boot off可否、BLOCKED画面 | |
| port/接続、冷間秒数/AC、全xHCI ID/revision/BDF | |
| I225実ID/revision/subsystem又は未取得 | |
| GOP全mode/選択mode/display接続又はNOT EXPOSED | |
| viewer/input/read、別AUTORUN hash/output/exit | |
| 証拠path、PASS/FAIL/BLOCKED、検証範囲 | |

[機器対応表](device-support.md)で予定と証拠を分離します。
