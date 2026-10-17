# Probe / live HCI transfer comparison

Status: development implementation, physical retry pending. The owner-supplied 177-byte descriptor layout is a host fixture, not a captured raw descriptor.

IMG_7688 shows `HCI TX opcode=0c03` followed by `BT INITIALIZATION TIMEOUT`, then a functioning FAT32 viewer and P/no-adapter response. The old TX line preceded the control-transfer budget check: it proves intent, not Reset submission. No controller CC/CS rejection or LE Rand invocation was observed. The precise original wire failure remains unconfirmed.

| Property | Successful older standard probe | Updated live path |
|---|---|---|
| CSR/owner-layout interrupt IN | address 81, DCI 3, type 7, packet 16; xHCI interval 3 for bInterval 1 | Same shared context-builder words |
| Bulk endpoints | Not configured | OUT 02 / DCI 4 / type 2 / packet 64; IN 82 / DCI 5 / type 6 / packet 64 retained |
| Add Context Flags / Context Entries | 09 / 3 | 39 / 5; only the selected HCI interface, no SCO isochronous context |
| Event context | CErr 3, Average TRB Length 257, Max ESIT Payload 257 | Same existing values; not changed as a speculative fix |
| Admission sequence | Configure Endpoint → SET_CONFIGURATION 1 → Reset | Same, then contiguous Reset + four LE Rand warm-up before enumerating other devices |
| SET_INTERFACE | None | None; SCO alternate settings are not activated |
| HCI Reset setup | 20/00, value 0, index 0, length 3 for device-class HCI | Same `Transport::command_setup`; interface-class composite uses 21/00 and its selected interface index |
| Interrupt TD | Normal TRB, length 257, IOC=1, ISP=1, Chain=0, initial Cycle=1 | Same; control completion precedes interrupt-IN arm/doorbell during Reset/Rand, matching probe order |
| Completion delivery | Direct exact TRB/slot/DCI wait | Same identity check plus controller mailbox; other USB completions remain owned by their consumers |
| Initial HCI credit | One outstanding command | 1; CC/CS credits replace the local count, zero/rejection handled fail-closed |
| Timeout | Independent bounded waits for each probe operation | Fixed 2 s preparation/warm-up operation budget + maximum 1 s slot isolation |

The confirmed implementation weakness was that the live deadline began before preparation, but warm-up did not run until after unrelated device enumeration. Devices-screen painting also consumed that deadline, and the TX-intent line could be followed by a pre-submission budget failure. The new path completes warm-up immediately after preparation. An RAII screen guard postpones only framebuffer painting, retains every ring/debug record and restores the original visible/hidden state after success or isolation. No timeout is renewed per poll. The 3 s figure is a Bluetooth software-work/cleanup budget, not a bound for all USB enumeration, deferred repaint or physical wall-clock timing.

New Devices metadata distinguishes endpoint context, Configure Endpoint completion code, initial credits, TX intent, setup/wait cap, control submission, USB control completion/incompletion, queued interrupt/ACL slot/DCI/length and receive TransferEvent completion code/length. HCI CC/CS metadata remains distinct from xHCI TransferEvent completion codes. No entropy bytes, keys or command parameters are logged. An initial isolation latches Bluetooth disabled for that controller session, preventing another adapter from reusing the retired CPU lease. The established whole-controller safety stop remains mandatory if slot isolation is not confirmed.

The real probe and live paths now use the same pure context builder. Host tests start from the owner 177-byte and CSR/SCO layouts and verify exact Event words, three live endpoint contexts, Flags/Entries, Reset setup, malformed alignment and isochronous exclusion. A separate test invokes the actual boot-log implementation and proves nested/early-return screen deferral preserves ring/debug records and restores the prior framebuffer state. QEMU has no local HCI device; its USB/input/storage regressions do not prove physical Reset/CC success.

## 日本語

状態: 開発実装、実機再試験待ち。177 byte descriptorは所有者提示のlayoutを合成したhost fixtureで、実機raw dumpではありません。

IMG_7688の確定観測はTX意図 `0c03` → `BT INITIALIZATION TIMEOUT` → FAT32 viewerとP入力継続です。旧TXはcontrol予算検査より前に出るため、Reset実送信・CC拒否・LE Rand到達の証拠にはなりません。元のwire上の失敗原因は未確定です。

上表の両経路はevent DCI3/type7/packet16/interval3とTRB length257・IOC/ISP・Chain無しを共有します。通常版は必要なbulk OUT DCI4/type2/64、IN DCI5/type6/64を保持し、Flags39/Entries5です。probeはeventだけなので09/3です。SCOやSET_INTERFACEは使わず、Configure Endpoint→SET_CONFIGURATION→Resetの順です。Reset/Randのinterrupt-IN armはcontrol完了後へ合わせました。HCI setupはdevice classなら20/00・value0・index0・length3、compositeなら選択interface宛21/00です。初期credit1から、実CC/CSで更新します。

実装上確認できた弱点は、prepare開始から2秒の期限が、後続USB列挙や画面描画を挟んだ後のwarm-upまで進み続ける点です。今回prepare直後にReset+Randを連続完了させ、RAIIで画面描画だけを延期します。ring/debugは全保持し、成功/隔離後に元の表示状態へ戻します。pollごとに期限を延長しません。2秒操作＋最大1秒隔離はBTソフト予算で、全USB列挙・延期描画・実機の厳密な実時間保証ではありません。

Devicesにはendpoint実構築値・Configure Endpoint完了code・credit初期値・TX意図・setup/待機上限・control送信/完了・queued slot/DCI/length・TransferEvent code/受信長を出します。xHCI完了codeとHCI CC/CS statusは別表示です。秘密値や命令parameterは出しません。初回隔離後はcontroller session内のBTを無効化し、退役leaseへ別adapterを再prepareしません。隔離未確認なら全体停止を維持します。

hostは本番共通context builderへowner177/CSR layoutを通し、DCI/packet/flags/entries/event words/setup/isoc除外を検証します。別試験は本番bootlogを呼び、入れ子/早期return/元hidden状態とring保持・描画復元を確認します。ローカルQEMUにHCI装置が無い制限を保ち、USB回帰を実機Reset/CC成功と扱いません。

Local verification: 155 platform + 59 xHCI host tests passed (214 total, including two new context/layout cases and one new real-console deferral case). Five targeted QEMU configurations passed: adapter-absence P, r2 keyboard and mouse with viewer return, SETTINGS creation, and feature-free Hello/viewer return. No real HCI is emulated. The first new defer test omitted the Devices configuration and therefore observed no drawing; after correcting that fixture, the complete host run passed. Evidence: `out/bt-transfer-alignment/verification.json`. Older checkpoint/repeated attempts are not added to these counts.

日本語: hostはplatform155＋xHCI59＝214件（新context/layout2件・実console defer1件を内包）、限定QEMU5構成が合格しました。実HCIはエミュレートしていません。新defer試験の初回はDevices設定漏れで描画を期待できないfixtureだったため失敗し、fixture訂正後の全hostで合格しました。証跡は `out/bt-transfer-alignment/verification.json`、旧checkpointや反復は件数へ加算しません。
