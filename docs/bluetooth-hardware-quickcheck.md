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


### Primary-interface selection regression (2026-10-17)

The owner-supplied 177-byte layout (not a physical raw descriptor dump) has interface 0 with interrupt IN `81`/16 and bulk OUT `02`, IN `82`/64; interface 1 has SCO alternate 0 with two zero-size isochronous endpoints and alternates 1–5 with sizes 9/17/25/33/49. The previous selector rejected its later SCO interface as a second HCI candidate. Selection now finalizes the first complete alternate-zero HCI interface and ignores later interface/alternate endpoint semantics while still validating descriptor framing. An incomplete primary or isochronous endpoint inside it still fails. No SCO context, SET_INTERFACE or firmware command is added. This fixture demonstrates the proposed layout, not that the physical 7265 has supplied those exact bytes. Re-run standard identification before deciding controller compatibility.

### 主interface選択の回帰確認（2026-10-17）

所有者提示の177byte構成（実機raw dumpではありません）はIF0がinterrupt IN `81`/16、bulk OUT `02`・IN `82`/64、IF1はSCO alt0にsize0のisoc2本、alt1〜5にsize9/17/25/33/49を持ちます。旧selectorは後続SCOを第2HCI候補として拒否しました。最初の完全なalt0 HCI interfaceを確定後、後続interface/altのendpoint意味を選択対象から外し、descriptorの構造検査は維持します。主interfaceの不足や主内isocは引き続き拒否します。SCO context、SET_INTERFACE、firmware commandは追加しません。このfixtureは提示構成の再現であり、実7265から同一raw bytesを取得した証拠ではありません。標準識別の再試験後に互換性を判断します。


### Entropy policy update / 乱数方針更新 (2026-10-17)

Before interpreting a new pairing result, photograph the actual source/strong/DRBG decisions from the [current entropy implementation](entropy-sources-0.4.0.md). Missing strong material still refuses pairing; no weak fallback is supplied. Earlier firmware-only failure videos are not a test of the new build. / 新pairing結果の前に、現行乱数実装のsource/strong/DRBG判定を撮影してください。強源不足は引き続き拒否し、弱いfallbackは使いません。旧firmware限定版の拒否動画は新版の試験ではありません。

## Live HCI failure isolation and command inventory (development)

See [Bluetooth failure isolation](bluetooth-isolation-0.4.0.md). Use `log=devices` / `display=max`; trace is unnecessary for the persistent failure reason and HCI metadata. A failure should show `bluetooth: unavailable: <reason>`, preserve keyboard/storage, and make P report no live adapter. Record the CC/CS opcode/status/credits and timeout line before testing pairing again. A controller/Disable Slot safety failure can still end the whole device session. The operation/cleanup budgets are 2 s + at most 1 s; physical timing remains unverified.

| Command | Opcode | Live path / Bluetooth requirement |
|---|---|---|
| Reset | `0c03` | Initial warm-up, then scan reset; standard BR/EDR baseline |
| LE Rand | `2018` | Four auxiliary reads after initial Reset; LE 4.0; unsupported status degrades |
| Set Event Mask | `0c01` | Scan / connection masks; standard baseline |
| LE Set Event Mask | `2001` | Advertising / connection events; LE 4.0 |
| LE Set Scan Parameters / Enable | `200b` / `200c` | Legacy passive scan; LE 4.0 |
| Read BD_ADDR | `1009` | Connection identity; standard baseline |
| LE Read Buffer Size / Read Buffer Size | `2002` / `1005` | LE ACL capacity / shared-buffer fallback |
| LE Create Connection | `200d` | Legacy LE connection; LE 4.0 |
| LE Start Encryption | `2019` | Host-derived SC LTK; LE 4.0 controller command |
| LE Set Data Length / LE Read Maximum Data Length | `2022` / `202f` | **Not sent**; no data-length-extension requirement |
| LE Add Device To Resolving List / LE Set Address Resolution Enable | `2027` / `202d` | **Not sent**; IRK/RPA unsupported |
| Set Event Mask Page 2 | `0c63` | **Not sent** |
| Read Local Supported Commands | `1002` | **Not sent**; future capability-gate proposal only |
| LE Read Local P-256 Public Key / Generate DHKey | `2025` / `2026` | **Not sent**; P-256 is computed by the host |

This inventory does not claim that every older controller accepts the selected event-mask bits. CSR8510 and Intel7265 have responded to the five standard probe commands; that does not prove live scan/pairing or unsupported-command behavior. A future Supported Commands gate would query `1002`, validate returned length and capability bits, then refuse an unavailable mandatory command before sending it; it is not implemented here.

## 日本語: 実行時隔離と送信命令

[BT失敗隔離](bluetooth-isolation-0.4.0.md)を参照してください。`log=devices` / `display=max` でHCIメタデータと保持理由が出るためtraceは不要です。失敗時は `bluetooth: unavailable: <reason>`、キーボードとストレージの継続、Pのlive adapter不在拒否を確認します。CC/CSのopcode・status・creditsとtimeout行を記録してください。Disable Slot等の安全確認に失敗した場合は全session停止を維持します。操作2秒＋隔離最大1秒はソフトウェア予算で、実機時間は未確認です。

上表の通常経路はLE 4.0命令を使います。2022/202f、2027/202d、0c63、1002、2025/2026は送信しません。P-256はhost計算です。古いcontrollerがevent maskの全bitを受理する保証はなく、CSR8510/Intel7265の5本probe成功をscan・pairing成功へ昇格させません。1002を使う機能ゲートは将来案で、返却長と必須bitを検査して送信前拒否する設計候補です。今回実装していません。


For the next normal development retry, retain Devices logs for the distinct intent/submitted/control-completed/Event/CC stages; see [the comparison](bluetooth-transfer-alignment-0.4.0.md). A TX intent alone is not wire transmission. Warm-up now runs consecutively before other USB enumeration, with painting deferred.

日本語: 次の通常開発版では[比較表](bluetooth-transfer-alignment-0.4.0.md)に従い、意図・実送信・USB control完了・TransferEvent・HCI CCを区別して撮影してください。TX意図だけで実送信と断定しません。warm-upは他USB列挙前に連続実行し、その間の画面描画を延期します。


## 2026-10-17 HOGP discovery diagnostics (development, not acceptance)

After successful Reset/Rand, a later `BT HOGP ... REJECTED` is a discovery failure, not proof that the controller lacks Bluetooth support. With `log=devices`, photograph `ATT queue intent` and `HOGP stage=... request=... handle=... response=... len=...` through the `accepted` or `rejected error=<variant> reason=<context>` line. An ATT Error Response adds its requested opcode, handle and error code. Queue intent is not proof of transmission. AttributeNotFound may normally end characteristic/descriptor discovery; only the actual executor result labels it accepted or rejected. Record Map bytes, notification payloads, keys, passkeys and random values are never dumped.

The viewer retains the rejecting stage (services, characteristics, Report Map, descriptors, or reference/map binding). It does not establish the exact missing attribute by itself. Preserve these bounded metadata lines and use the real response to construct a regression fixture before changing compatibility rules. Discovery limits and accepted formats are unchanged. No physical Xbox HOGP acceptance or durable bond save is claimed by the local diagnostics tests.

## 日本語: HOGP探索診断（2026-10-17、開発検証）

Reset/Rand成功後の `BT HOGP ... REJECTED` は探索失敗であり、controllerのBluetooth非対応を意味しません。`log=devices` で `ATT queue intent`、`HOGP stage=... request=... handle=... response=... len=...` から `accepted` または `rejected error=<variant> reason=<context>` まで撮影してください。ATT Error Responseは要求opcode・handle・error codeだけを追加します。queue intentは実送信証明ではありません。AttributeNotFoundはcharacteristic/descriptor探索の正常終端にも使われるため、実探索結果に従ってaccepted/rejectedを表示します。Report Map本文・通知payload・鍵・passkey・乱数値は出しません。

viewerには拒否段階（service、characteristic、Report Map、descriptor、reference/map binding）が残ります。この表示だけでは不足属性を確定できません。公開メタデータを保存し、実応答を回帰fixtureにしてから互換性の修正を判断します。探索上限・受理形式は変更していません。ローカル診断試験は実XboxのHOGP受入やbond永続保存の証明ではありません。


## Initial Reset/Rand event correlation — English (2026-10-17)

The normal warm-up waits for Command Complete matching the outstanding Reset (`0c03`) or LE Rand (`2018`) opcode, within the original command deadline and 2-second operation / at most 1-second isolation soft budget. A successful matching Command Status retains the pending command; rejected status or zero credits stops Bluetooth. A structurally valid unrelated Command Complete is recorded but changes neither credits nor command state. At most 64 events are examined across warm-up; waiting never renews deadlines.

Before any connection/ACL submission, valid Number Of Completed Packets (nonzero count, unique valid handles), Disconnection Complete and LE Advertising Reports may be consumed as pre-connection events after their structural checks. Stale packet counts are not credited to a future ACL queue or replayed into a future connection. Controller Hardware Error or malformed events stop Bluetooth; unknown envelope-valid events are now explicitly logged and discarded under the same count/time caps (the Reset synchronization update supersedes the earlier unknown-event rejection) and retain the existing safe slot-isolation boundary. After warm-up the existing connection dispatcher is unchanged. New Devices metadata includes event code and parameter length, plus pending classification and expected opcode; it never includes Rand bytes or other secret payloads. The actual event code in IMG_7690 was not recorded, so this change does not identify that event as Number Of Completed Packets.

**Deployment setting path is `SETTINGS/CONFIG.TXT`**, containing `log=devices`, `display=max`, `lan.echo=off`. Root `CONFIG.TXT` is not read as boot settings. The previous deployment mistakenly added a root file; the video nevertheless directly showed the preserved settings loaded as Devices/Max/pair=false/echo=off. A future authorized USB update must inspect both paths, preserve user data and separately correct that extra root file. No USB repair is part of this source change.

## 日本語: 初回Reset/Randの応答相関（2026-10-17）

通常warm-upは未完了Reset(`0c03`)/LE Rand(`2018`)に一致するCommand Completeを、元のcommand絶対期限と操作2秒＋隔離最大1秒のソフト予算内で待ちます。一致する成功Command StatusではCC待ちを継続し、拒否statusやcredit0ではBTを停止します。構造上正しい別opcode CCは記録するだけでcredit・command状態を変えません。warm-up全体最大64events、待機による期限延長はありません。

接続・ACL送信前では、Number Of Completed Packets（非0件数・重複なしの有効handle）、Disconnection Complete、LE Advertising Reportsを構造検証後に初期化前イベントとして消費できます。古いpacket数を将来のACLへ加算したり将来接続へ再生したりしません。Hardware Error・不正形式はBT停止と既存の安全隔離へ進みます。Reset同期更新後は、外枠が正しい未知イベントだけを分類ログ付き・同じ件数/時間上限で破棄します（旧未知イベント即拒否を置換）。warm-up以後のconnection dispatcherは変更していません。Devicesにevent code/parameter長、待機分類/対象opcodeのみを追加し、Rand等の秘密payloadは出しません。IMG_7690の実event codeは未採取であり、そのイベントをCompleted Packetsと断定していません。

設定の正しい場所は **`SETTINGS/CONFIG.TXT`** です。devices/max/echo offをそこへ配置します。rootのCONFIGは起動設定として読まれません。前回配備では誤ってrootへ追加しましたが、動画では既存設定のDevices/Max/pair=false/echo off読込を直接確認しました。次の承認済みUSB更新時には両pathと利用者データを確認して余分なroot fileを別途訂正します。今回USB修正は行っていません。


## Reset synchronization retry — English (2026-10-17)

The normal discovery-only first pass sends no HCI command; the separate `bluetooth-hci-probe` does. USB slot re-enumeration is not a power cycle and does not prove that the controller event FIFO is empty. After SET_CONFIGURATION, the normal path drains events before Reset (target 100 ms, maximum 200 ms, early finish after 50 ms with no events). Every drained event is structurally checked and logged by code/length/count only. A quiet interrupt-IN TD remains owned/pending; neither its DMA buffer, mailbox identity nor ring cursor is reused or cleared by the drain timeout. Exact late completion remains correlated, and failure still requires confirmed slot isolation.

The initial sequence is Reset `0c03` → matching Read Local Version `1001` (status 0, exact return length) → four LE Rand `2018` attempts. A valid successful Reset CC arriving while Version/Rand is pending permits **one retransmission of that pending parameter-free command**, with opcode/retry metadata. Its original absolute command deadline and the overall 2-second operation plus maximum 1-second isolation budget remain unchanged. Further late Reset, invalid status/credit or expired deadline fails safely. Unknown but envelope-valid events may now be logged/discarded within the 64-event cap; malformed/Hardware Error remains fail-closed. This does not establish which of the two Reset CCs seen in IMG_7691 was old or current.

For the retry, photograph the drain summary, Reset CC, Version CC, Rand CC and any `HCI late Reset CC; retry ...` line. See [the implementation and limits](bluetooth-reset-synchronization-0.4.0.md). The operational budget is a software deadline, not an assurance that physical firmware/USB/rendering never adds latency. No USB update is included in this source change.

## 日本語: Reset同期と限定再送（2026-10-17）

通常版discovery専用の初回列挙はHCI命令を送りません。別featureのprobeは送信します。USB再列挙は電源断ではなく、controllerのevent FIFOが空である証明にもなりません。SET_CONFIGURATION後、Reset前にイベントをドレインします（目標100ms、最大200ms、無イベント50msで早期終了）。全イベントを構造検証しcode/長さ/件数だけ記録します。quiet時のinterrupt-IN TDはpending所有を維持し、DMA・mailbox相関・ring cursorを再利用/解除しません。遅着完了も正確に相関し、失敗時の安全隔離は維持します。

初回順序はReset `0c03` → Read Local Version `1001` の一致CC（status0・厳密な返却長）→ LE Rand `2018` 4回です。Version/Rand待ちに妥当な成功Reset CCが来た場合、待っている引数なし命令を **1回だけ再送** しopcode/retryを記録します。元command絶対期限、操作2秒＋隔離最大1秒の総予算は延長しません。2回目の遅着Reset・不正status/credit・期限切れは安全停止します。外枠が妥当な未知イベントは64件上限内で記録破棄し、不正形式・Hardware Errorは拒否します。IMG_7691の2つのReset CCのどちらが古い/今回のものかは断定していません。

再試験ではdrain summary、Reset CC、Version CC、Rand CC、再送行を撮影してください。[実装と限界](bluetooth-reset-synchronization-0.4.0.md)参照。これはソフト期限であり物理firmware/USB/描画の遅延を厳密に保証するものではありません。今回USB更新はしていません。


## Pairing-session retry — English (2026-10-17)

IMG_7692 lost the whole USB session during Xbox pairing without a reason. The next normal development EFI shows the reason (`Device session ended: <reason>.` and `usb: session ended: <reason>`) and confines Bluetooth-slot transfer faults, pairing-record refusal and input overflow to Bluetooth. Photograph from P through the failure or success: `LE scan enabled`, each `advertising window`, any `HID advertisement ... ignored`, `HID candidate`, `connect stage=`, `ACL buffer`, `LE create connection accepted`, `LE Connection Complete`, `ACL first TX/RX`, `L2CAP first`, each `SMP TX/RX`, `Encryption Change`, and on failure `transfer fault`, `unavailable:` or `non-local failure:` / `session ended:`. See [pairing-phase isolation](bluetooth-pairing-isolation-0.4.0.md). Values are metadata only; no keys or full addresses are shown.

## 日本語: ペアリング再試験（2026-10-17）

IMG_7692 では Xbox のペアリング中に理由が出ないまま USB セッション全体が終了しました。次の通常開発 EFI は理由（`Device session ended: <reason>.` と `usb: session ended: <reason>`）を表示し、Bluetooth slot の転送 fault・ペアリング記録の拒否・入力あふれを Bluetooth だけに閉じ込めます。P から失敗または成功まで、`LE scan enabled`、各 `advertising window`、`HID advertisement ... ignored`（出た場合）、`HID candidate`、`connect stage=`、`ACL buffer`、`LE create connection accepted`、`LE Connection Complete`、`ACL first TX/RX`、`L2CAP first`、各 `SMP TX/RX`、`Encryption Change`、失敗時は `transfer fault` と `unavailable:` または `non-local failure:` / `session ended:` を撮影してください。[ペアリング段階の分離](bluetooth-pairing-isolation-0.4.0.md)を参照。値はメタデータのみで、鍵や完全なアドレスは出ません。
