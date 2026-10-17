# Bluetooth hardware quickcheck / Bluetooth実機1ページ手順

Status: current (procedure only, 2026-10-15). No physical Bluetooth pass is recorded.

## English

For a v0.4.0 EFI implementing [boot-log](boot-log.md), use `log=devices` and photograph continuous white-on-black lines, with PM timestamps and scrolling. Devices now gives concise per-device decisions; use `log=trace` (or the F8/8 startup override) for detailed transfer/stage/timing and inventory evidence. The ring retains all detail at every display level. No rotating diagnostic pages, old colored status words or PM footer are required. Capture adapter/peer IDs, pairing/save/reconnect decisions and HCI response/failure as bluetooth/usb events in the same timestamped log. Photograph actual input-monitor app output separately from pair success. Existing debug-port markers are auxiliary developer evidence, not a substitute for a visible HCI response or physical input. Hardware acceptance is still pending.

1. Record candidate commit, EFI SHA-256/features, NUC5 model/firmware, MM-BTUD46 label/revision and Xbox Series X|S model1914 firmware. Connect the dongle **directly to a photographed NUC5 rear USB port**, plus the boot USB and external keyboard. Use `log=devices` in the existing CONFIG; retain its exact bytes/hash. No USB is written by this document.
2. Cold boot (power off ≥10 s; record AC state). Photograph devices log lines: controller/BDF/root/slot, actual dongle VID:PID, revision, USB class/subclass/protocol and endpoints. `0A12:0001`, `E0/01/01` are expectations, not measured identity or proof of CSR firmware compatibility. Internal AC7265 Bluetooth is a separate path.
3. Enter pairing with viewer **P**, or a deliberately prepared `pair=on` CONFIG. Put Xbox1914 into pairing mode. Capture scan/connection, address type, pairing result, HOGP report-map/descriptor identity and errors/timeouts; never publish LTK/passkey. Public/static addresses only: IRK/RPA resolution is not implemented. Pair persistence is successful only after durable record save; unchanged consumed CONFIG hash does not rearm pairing.
4. Use **live input-monitor** (`sdk/examples/input-monitor/main.c`) from the viewer and record its ELF hash. Build-only: `python3 tools/check-sdk.py --c-only --abi-revision 2 --source sdk/examples/input-monitor/main.c --output out/input-monitor`; output `out/input-monitor/hello-c.elf`. The sample displays kind/device/code/flags/value/buttons/modifiers/ms in hexadecimal; Q/Esc exits and there is a five-minute limit. Existing `gamepad.elf` (IO_CASE8) is a fixed software-fixture checker, not manual hardware acceptance. Host/build and QEMU checks do not prove physical Bluetooth. Press/release A/B, move sticks and triggers, and record observed `SYS_INPUT` values, neutral/release and disconnect behavior. Photograph sample output; mere pairing is not input acceptance. Preserve keyboard/viewer/storage operation.
5. Stop safely, cold boot ≥10 s with the **same** dongle, port, media and controller; do not press P. Record saved pairing load, reconnect, sample events and elapsed time. Record FAIL/BLOCKED explicitly. The wider release cold3/Ryzen hot1 gate is separate; this first NUC5 run is not full acceptance.
6. Separate **ASUS USB-BT50LE experiment**: record exact identity first (label alone does not identify firmware). Use an explicitly labelled development EFI built with `bluetooth-hci-probe`; do not load firmware or substitute this for the CSR reference test. Capture USB descriptors and HCI Reset opcode `0x0C03` Command Complete/status or exact failure/timeout. Success marker is `BT_HCI_RESET_TRANSPORT_OK` in development debug output; photograph the corresponding devices/error log lines too. This reset probe does not pair or establish runtime/HOGP support.

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

[bootlog](boot-log.md)実装v0.4.0 EFIは`log=devices`でPM時刻付き白黒逐次log/scrollを撮影。devicesは機器判断を簡潔に表示し、詳細transfer/stage/timing・一覧は`log=trace`又はF8/8起動overrideで採取。ringは全表示levelで詳細を保持。診断巡回頁・旧色status語・PM footerは条件にしません。adapter/peer ID、pair/保存/再接続判断、HCI応答/失敗を同timestamp logのbluetooth/usb行へ記録。input-monitorの実出力はpair成功と別に撮影。既存debug markerは補助証拠で画面応答/実入力の代用にしません。 実機受入は未完です。

1. 候補commit、EFI SHA-256/features、NUC5型番/firmware、MM-BTUD46ラベル/revision、Xbox Series X|S model1914 firmwareを記録。dongleは**NUC5背面の撮影したUSBポートへ直挿し**し、起動USB・外付けkeyboardも接続。既存CONFIGの`log=devices`と正確な内容/hashを記録。本書ではUSBへ書込みません。
2. 電源off ≥10秒（AC接続状態も記録）から起動。devicesのcontroller/BDF/root/slot、実VID:PID/revision、class/subclass/protocol、endpointを撮影。`0A12:0001`、`E0/01/01`は見込みで、実測値・CSR firmware互換の証拠ではありません。内蔵AC7265 BTと区別します。
3. viewer **P**、または意図して用意した`pair=on`でpairを開始しXbox1914をpairing modeへ。scan/接続、address種別、pair結果、HOGP report-map/descriptor、error/timeoutを撮影。LTK/passkeyは公開しません。public/static addressのみでIRK/RPA解決は未実装。永続record保存成功まで保持成功とせず、消費済み同一CONFIG hashでは再pairしません。
4. **live input-monitor**（`sdk/examples/input-monitor/main.c`）をviewerから開きELF hashを記録。buildのみ：`python3 tools/check-sdk.py --c-only --abi-revision 2 --source sdk/examples/input-monitor/main.c --output out/input-monitor`、出力`out/input-monitor/hello-c.elf`。kind/device/code/flags/value/buttons/modifiers/msを16進表示しQ/Esc終了・5分上限。既存`gamepad.elf`（IO_CASE8）は固定software fixtureで実機受入には使いません。build/QEMU成功は実BT成功の証拠ではありません。A/B押下・解放、stick/trigger、neutral・切断時の`SYS_INPUT`値を記録/撮影。pair成功だけで入力合格にしません。keyboard/viewer/storageの継続も確認します。
5. 安全に停止し同じdongle/port/media/controllerで再度off ≥10秒の冷間起動。Pを押さず保存pairing読出し・再接続・sample入力・所要時間を記録。失敗/保留を明記。release全体の冷間3/Ryzen温間1とは別のNUC5初回試験です。
6. **ASUS USB-BT50LE別実験**：ラベルだけでfirmwareを断定せず実IDを採取。`bluetooth-hci-probe`付きと明示した開発EFIを用い、firmwareを投入せずHCI Reset `0x0C03`のCommand Complete/status、または失敗/timeoutを採取。debug成功markerは`BT_HCI_RESET_TRANSPORT_OK`。devices/errorの逐次log行も撮影。reset応答はpair/runtime/HOGP対応の証明ではありません。

| 記録欄 | 値 |
|---|---|
| 日時/実施者、EFI/features/hash、CONFIG hash、sample hash | |
| 接続写真、dongleラベル/VID:PID/revision/class/endpoint | |
| Xbox firmware/address種別、descriptor/report-map証拠 | |
| pair/保存→入力値/解放→冷間秒数/AC→再接続 | |
| ASUS probe EFI/hash、reset応答/status又は失敗、証拠path | |
| PASS/FAIL/BLOCKED、検証範囲・残件 | |

実BT未検証。HOGP keyboard実機はv0.5.0、Magic A3118 ClassicはLE対象外。[工程6](stage6-0.4.0-progress.md)参照。

Development EFI identity / 開発EFIの識別: [three diagnostic artifacts and original provenance](hardware-acceptance-0.4.0.md#development-efi). Select the purpose-specific artifact and record CONFIG; these are not normal-candidate acceptance. / 用途別artifactとCONFIGを識別し、通常候補受入とは区別してください。


## Standard HCI identification round — English (2026-10-17)

Use a development `bluetooth-hci-probe,diagnostics-detail` EFI with `log=devices`, `display=max`, and no `pair=on`. This probe accepts the standard E0/01/01 USB HCI interface regardless of VID. It probes each matched interface during enumeration; the normal stack instead retains the first successfully prepared controller. A label is not controller identity. Record VID/PID, bcdDevice, interface count/HCI interface, physical port and every command status.

1. **Remove the external dongle first**, cold boot the NUC5 with this probe and photograph the internal 7265 identity. The probe sends only Reset `0x0C03`, Read Local Version `0x1001`, Read Local Supported Features `0x1003`, LE Read Local Supported Features `0x2003`, and Read BD_ADDR `0x1009`. Record HCI/LMP versions, revisions, manufacturer, feature masks and address. Reset failure or malformed/transport failure stops that probe; a nonzero status on an information command is recorded and later commands continue. An unsupported LE-feature command alone is not proof of unusable ROM. The COMPLETE marker means the bounded sequence finished, not that every information command succeeded. Fragmented/coalesced or unsolicited events are rejected by this probe; a transport/framing failure alone does not establish ROM incompatibility. Each command has a bounded event wait; there is no firmware download, Intel `0xFC11`/`0xFC05`, patch command, flash programming or pairing in this probe. Standard Reset does change volatile controller state; it is not a passive USB-descriptor read. Unknown/ROM firmware can reject or time out; hardware recovery/power procedures remain owner-controlled.
2. **Test the external dongle with the normal stack and Xbox1914**, using the existing P → live input-monitor → durable save → cold reconnect procedure above. Record the selected controller identity; an internal adapter may win the present first-arrival selection. Controller selection is not changed by this procedure.
3. **Only after recording both results, test 7265 pairing separately** using the normal stack and a BLE peer. Identification success is not pairing/HOGP compatibility. Do not claim an untested device or firmware patch is supported.

Secure Connections P-256 public-key generation/ECDH runs in the host (`sc_crypto.rs`, fixed p256 dependency); no LE Read Local P-256 Public Key `0x2025` or LE Generate DHKey `0x2026` command is used. Thus a controller lacking those optional key commands is not rejected for that reason alone. This does not establish controller/peer interoperability or provide an audit claim for the cryptographic library.

Owner decision2026-10-17: **A is adopted for v0.4**, retaining the first controller whose prepare succeeds. Root/port traversal does not universally guarantee dongle-first selection. Record actual selected identity and remove the dongle for built-in identification. **B is a v0.5 candidate**, with proposed CONFIG syntax `bt=dongle`, `bt=internal` or `bt=VVVV:PPPP`; it is unimplemented and not a supported key. Identity matching and absent/ambiguous handling remain design work. C (vendor preference) is not adopted.

Source: [Bluetooth Core 5.4, Vol 4 Part E HCI commands](https://www.bluetooth.com/wp-content/uploads/Files/Specification/HTML/Core-54/out/en/host-controller-interface/host-controller-interface-functional-specification.html). Test outcomes and exact EFI provenance are recorded under `out/bluetooth-standard-probe/verification.json`; physical values remain uncollected until owner testing.

## 標準HCI識別ラウンド — 日本語（2026-10-17）

開発用 `bluetooth-hci-probe,diagnostics-detail` EFIを使い、`log=devices`、`display=max`、`pair=on`なしとします。標準E0/01/01 USB HCI interfaceをVID制限なしで選び、probe版は列挙中の各適合interfaceを調べます。通常stackは最初にprepare成功したcontrollerを保持します。ラベルを実IDとせず、VID/PID・bcdDevice・interface数/HCI interface・物理port・各command statusを撮影します。

1. **外付けdongleを外して**NUC5をprobe版で冷間起動し、内蔵7265を採取します。送信するのはReset `0x0C03`、Local Version `0x1001`、Local Supported Features `0x1003`、LE Local Supported Features `0x2003`、BD_ADDR `0x1009`だけです。HCI/LMP版・revision・manufacturer・feature mask・addressを記録します。Reset失敗、形式不正、transport失敗はそのprobeを中止。情報commandの非zero statusは表示して後続へ進み、LE feature非対応だけでROM全体が使用不能とは断定しません。COMPLETE markerは試行列の完了で全command成功を意味しません。このprobeは分割/結合/非要求eventを安全拒否し、そのtransport/framing失敗だけでROM非互換と断定しません。待機は有界で、firmware download、Intel `0xFC11`/`0xFC05`、patch、flash program、pairingは行いません。ただし標準Resetは揮発状態を変更し、受動的なUSB descriptor読取りとは異なります。未知/ROM firmwareの拒否・timeoutの可能性があり、電源等の復旧操作は所有者判断です。
2. **外付けdongle＋通常stack＋Xbox1914**を、上記のP→live input-monitor→永続保存→冷間再接続の順で試します。先着選択で内蔵側が選ばれる可能性があるため実際のcontroller IDを記録します。本手順では選択方式を変更しません。
3. **両結果の記録後に7265のpairingを別試験**します。識別成功はpairing/HOGP互換性を証明せず、未試験機器やfirmware patch対応を主張しません。

SCのP-256公開鍵生成/ECDHはhost側`sc_crypto.rs`と固定版p256で計算し、controllerのLE P-256 Public Key `0x2025`/Generate DHKey `0x2026`は使いません。この任意commandが無いことだけで拒否しませんが、実機互換性や暗号libraryの監査範囲を保証する意味ではありません。

所有者決定2026-10-17：**0.4は案A採用**、最初にprepare成功したcontrollerを保持。root/port探索順でdongle優先を普遍保証しません。実選択IDを記録し、内蔵識別はdongleを外して実施。**案Bは0.5候補**でCONFIGの`bt=dongle`・`bt=internal`・`bt=VVVV:PPPP`構文を提案しますが、未実装で現行keyではありません。ID一致・不在・曖昧時処理は未設計です。vendor優先の案Cは採用しません。

一次仕様は上記Bluetooth Core 5.4 Vol4 PartE。試験とEFI来歴は`out/bluetooth-standard-probe/verification.json`へ記録し、実機値は所有者採取まで未確認です。
