# Bluetooth hardware quickcheck / Bluetooth実機1ページ手順

Status: current (procedure only, 2026-10-15). No physical Bluetooth pass is recorded.

## English

1. Record candidate commit, EFI SHA-256/features, NUC5 model/firmware, MM-BTUD46 label/revision and Xbox Series X|S model1914 firmware. Connect the dongle **directly to a photographed NUC5 rear USB port**, plus the boot USB and external keyboard. Use `log=devices` in the existing CONFIG; retain its exact bytes/hash. No USB is written by this document.
2. Cold boot (power off ≥10 s; record AC state). Photograph devices pages: controller/BDF/root/slot, actual dongle VID:PID, revision, USB class/subclass/protocol and endpoints. `0A12:0001`, `E0/01/01` are expectations, not measured identity or proof of CSR firmware compatibility. Internal AC7265 Bluetooth is a separate path.
3. Enter pairing with viewer **P**, or a deliberately prepared `pair=on` CONFIG. Put Xbox1914 into pairing mode. Capture scan/connection, address type, pairing result, HOGP report-map/descriptor identity and errors/timeouts; never publish LTK/passkey. Public/static addresses only: IRK/RPA resolution is not implemented. Pair persistence is successful only after durable record save; unchanged consumed CONFIG hash does not rearm pairing.
4. Use **live input-monitor** (`sdk/examples/input-monitor/main.c`) from the viewer and record its ELF hash. Build-only: `python3 tools/check-sdk.py --c-only --abi-revision 2 --source sdk/examples/input-monitor/main.c --output out/input-monitor`; output `out/input-monitor/hello-c.elf`. The sample displays kind/device/code/flags/value/buttons/modifiers/ms in hexadecimal; Q/Esc exits and there is a five-minute limit. Existing `gamepad.elf` (IO_CASE8) is a fixed software-fixture checker, not manual hardware acceptance. Host/build and QEMU checks do not prove physical Bluetooth. Press/release A/B, move sticks and triggers, and record observed `SYS_INPUT` values, neutral/release and disconnect behavior. Photograph sample output; mere pairing is not input acceptance. Preserve keyboard/viewer/storage operation.
5. Stop safely, cold boot ≥10 s with the **same** dongle, port, media and controller; do not press P. Record saved pairing load, reconnect, sample events and elapsed time. Record FAIL/BLOCKED explicitly. The wider release cold3/Ryzen hot1 gate is separate; this first NUC5 run is not full acceptance.
6. Separate **ASUS USB-BT50LE experiment**: record exact identity first (label alone does not identify firmware). Use an explicitly labelled development EFI built with `bluetooth-hci-probe`; do not load firmware or substitute this for the CSR reference test. Capture USB descriptors and HCI Reset opcode `0x0C03` Command Complete/status or exact failure/timeout. Success marker is `BT_HCI_RESET_TRANSPORT_OK` in development debug output; photograph available devices/error pages too. This reset probe does not pair or establish runtime/HOGP support.

| Record | Value |
|---|---|
| Date/operator; EFI/features/hash; CONFIG hash; sample hash | |
| Physical topology/photo; dongle label/VID:PID/revision/class/endpoints | |
| Xbox firmware/address type; descriptor/report-map evidence | |
| Pair/save → input values/release → cold interval/AC → reconnect | |
| ASUS probe EFI/hash; reset response/status or failure; evidence paths | |
| PASS/FAIL/BLOCKED, exact boundary and unresolved observations | |

Physical BT is unverified. HOGP keyboard hardware is deferred to v0.5.0; Magic Keyboard A3118 Classic is outside this LE path. See [stage6](stage6-0.4.0-progress.md).

## 日本語

1. 候補commit、EFI SHA-256/features、NUC5型番/firmware、MM-BTUD46ラベル/revision、Xbox Series X|S model1914 firmwareを記録。dongleは**NUC5背面の撮影したUSBポートへ直挿し**し、起動USB・外付けkeyboardも接続。既存CONFIGの`log=devices`と正確な内容/hashを記録。本書ではUSBへ書込みません。
2. 電源off ≥10秒（AC接続状態も記録）から起動。devicesのcontroller/BDF/root/slot、実VID:PID/revision、class/subclass/protocol、endpointを撮影。`0A12:0001`、`E0/01/01`は見込みで、実測値・CSR firmware互換の証拠ではありません。内蔵AC7265 BTと区別します。
3. viewer **P**、または意図して用意した`pair=on`でpairを開始しXbox1914をpairing modeへ。scan/接続、address種別、pair結果、HOGP report-map/descriptor、error/timeoutを撮影。LTK/passkeyは公開しません。public/static addressのみでIRK/RPA解決は未実装。永続record保存成功まで保持成功とせず、消費済み同一CONFIG hashでは再pairしません。
4. **live input-monitor**（`sdk/examples/input-monitor/main.c`）をviewerから開きELF hashを記録。buildのみ：`python3 tools/check-sdk.py --c-only --abi-revision 2 --source sdk/examples/input-monitor/main.c --output out/input-monitor`、出力`out/input-monitor/hello-c.elf`。kind/device/code/flags/value/buttons/modifiers/msを16進表示しQ/Esc終了・5分上限。既存`gamepad.elf`（IO_CASE8）は固定software fixtureで実機受入には使いません。build/QEMU成功は実BT成功の証拠ではありません。A/B押下・解放、stick/trigger、neutral・切断時の`SYS_INPUT`値を記録/撮影。pair成功だけで入力合格にしません。keyboard/viewer/storageの継続も確認します。
5. 安全に停止し同じdongle/port/media/controllerで再度off ≥10秒の冷間起動。Pを押さず保存pairing読出し・再接続・sample入力・所要時間を記録。失敗/保留を明記。release全体の冷間3/Ryzen温間1とは別のNUC5初回試験です。
6. **ASUS USB-BT50LE別実験**：ラベルだけでfirmwareを断定せず実IDを採取。`bluetooth-hci-probe`付きと明示した開発EFIを用い、firmwareを投入せずHCI Reset `0x0C03`のCommand Complete/status、または失敗/timeoutを採取。debug成功markerは`BT_HCI_RESET_TRANSPORT_OK`。画面のdevices/errorも撮影。reset応答はpair/runtime/HOGP対応の証明ではありません。

| 記録欄 | 値 |
|---|---|
| 日時/実施者、EFI/features/hash、CONFIG hash、sample hash | |
| 接続写真、dongleラベル/VID:PID/revision/class/endpoint | |
| Xbox firmware/address種別、descriptor/report-map証拠 | |
| pair/保存→入力値/解放→冷間秒数/AC→再接続 | |
| ASUS probe EFI/hash、reset応答/status又は失敗、証拠path | |
| PASS/FAIL/BLOCKED、検証範囲・残件 | |

実BT未検証。HOGP keyboard実機はv0.5.0、Magic A3118 ClassicはLE対象外。[工程6](stage6-0.4.0-progress.md)参照。
