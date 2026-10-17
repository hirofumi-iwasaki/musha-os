# Bluetooth failure isolation in v0.4.0

Status: development implementation; physical HCI failure/recovery acceptance is pending.

An optional Bluetooth failure must not disable a working USB keyboard or storage device. The live preparation/poll paths share one slot-stop sequence: retain the exact outstanding Event/ACL/EP0 TRB identities, await a correlated xHCI Disable Slot completion, then clear that slot's DCBAA entry and drop its CPU state. The shared event cursor and keyboard/storage/mouse completions are retained. Old Bluetooth completion metadata is consumed only for the recorded TRB, slot and endpoint identity. Allocated DMA buffers and the retired CPU lease remain reserved until the entire controller's halt and BME-off are confirmed; slot isolation does not permit buffer or lease reuse.

Preparation starts a common 2,000 ms operation budget, including endpoint configuration, SET_CONFIGURATION, the initial HCI Reset and four LE Rand commands. Each blocking operation is limited by the remaining budget (and its existing 1,000 ms per-operation limit). Isolation has a separate maximum 1,000 ms Disable Slot wait. Enumeration of other devices can consume elapsed time within the operation budget. Thus a slow optional Bluetooth initializer can be disabled rather than extending boot indefinitely. These are software deadlines, not a measured physical real-time guarantee.

BT protocol, status rejection, disconnect and command-timeout failures are isolated. Shared MMIO/clock/event-identity/command-ring faults, a retirement-identity failure or an unconfirmed Disable Slot still propagate to whole-controller stop. A normal re-pairing request is not a failure: it keeps the existing controller-stop/restart path. The mailbox retains one isolated slot per controller session; a second retirement is refused safely rather than widening identities or reclaiming old memory.

After successful isolation, the Ready-level log retains `bluetooth: unavailable: <reason>` and the viewer retains the reason. P rejects with `Bluetooth pairing unavailable: no live USB HCI adapter.` BT keyboard state is cleared independently through the two-source merger; held USB keys survive. Previously held BT gamepad buttons are released and nonzero axes are neutralized. No new press is synthesized.

Devices logging includes HCI TX/CC/CS opcode, status and command credits, plus timeout opcode and elapsed milliseconds. Missing status is never replaced with success. LE Rand payloads, keys, nonces, passkeys and DRBG state are not logged. Unsupported LE Rand status makes the auxiliary source unavailable while preserving an existing strong DRBG source. Ambiguous transport failure still requires slot isolation (or controller stop if isolation fails).

The IMG_7687 observation proves RDSEED strong-source and DRBG initialization success, followed by a stopped device session. It does not identify the previous precise USB/HCI error or establish that a claimed ten-second delay came from one command. The previous Devices filter hid `usb_failure` detail; the new persistent unavailable reason and HCI metadata address that observation gap. QEMU has no HCI controller in the local setup: host injection exercises the production isolation orchestration, while QEMU verifies existing USB services and Bluetooth absence. Neither establishes physical CSR8510/Intel7265 pairing acceptance.

## 日本語

状態: 開発実装。実HCI失敗時の隔離・回復は実機受入待ちです。

任意機能のBluetooth失敗で、動作中のUSBキーボードやストレージを停止しない設計です。実行時のprepare/pollは同じ停止手順を使い、Event/ACL/EP0の未完了TRB識別を保持し、対応するxHCI Disable Slot完了を確認してからDCBAAの当該slotを外し、CPU状態を破棄します。共有event cursorと他USB機器の完了は保持します。遅着完了を捨てるのは、記録したTRB・slot・endpointが一致する場合だけです。DMA領域と退役CPU leaseは、コントローラー全体のhalt/BME無効を確認するまで予約したままで、slot隔離だけでは再利用しません。

prepare開始から2,000 msを共通操作予算とし、endpoint設定、SET_CONFIGURATION、初回HCI Reset、LE Rand 4回を含めます。各待機は残予算と従来の1,000 ms上限の短い方です。隔離のDisable Slot待機は別に最大1,000 msです。他機器の列挙中も経過時間に含まれるため、遅い任意BT初期化は起動を無期限に延ばさず無効化されます。実機で測ったハードリアルタイム保証ではありません。

BT固有のプロトコル異常・拒否status・切断・命令timeoutは隔離します。共有MMIO・時計・event識別・command ring異常、退役識別失敗、Disable Slot未確認は従来どおり全体停止へ伝播します。正常な再ペアリング要求は失敗扱いせず既存の全体停止・再開経路を保ちます。退役mailboxは1 sessionにつき1 slotまでで、2件目は安全拒否します。

隔離後はReadyレベルにも `bluetooth: unavailable: <reason>` を記録し、viewerへ理由を残します。Pはlive adapter不在として拒否します。BTキーボードの押下状態だけを二系統mergeで解除し、USB側の押下は維持します。押下済みBTゲームパッドbuttonはrelease、非zero axisは中立化し、新規pressは作りません。

DevicesにはHCI TX/CC/CSのopcode・status・creditsとtimeoutのopcode・実経過msだけを出します。欠落statusを成功へ補完せず、乱数・鍵・nonce・passkey・DRBG状態は出力しません。LE Randの非zero statusは補助源だけを無効化し、既存strong源を保持します。曖昧なtransport異常は隔離確認が必要です。

IMG_7687はRDSEED strong源とDRBG初期化成功、その後のsession終了を確認した証拠です。前版の正確なHCI/USB失敗理由や「10秒」の単一原因は確定していません。旧Devices表示で隠れた失敗理由を、新しい理由保持とHCIメタデータで切り分けます。ローカルQEMUにはHCI装置がなく、host注入は本番共通の隔離手順、QEMUは既存USB機能とBT不在を検証します。実CSR8510/Intel7265のペアリング合格とは区別します。

## Local verification

`out/bt-isolation/verification.json` records 152 platform and 59 xHCI host tests (211 total; the six new isolation/metadata/merge tests and one new retirement test are included). Seven distinct QEMU configurations cover P/config adapter absence, r2 keyboard/mouse syscalls and viewer return, SETTINGS creation, the existing normal Bluetooth loan-restart fixture, and feature-free Hello/viewer return. Repeated attempts are not additional distinct configurations. One mouse run failed before QEMU because the ELF path was wrong; the corrected existing ELF passed. One initial SETTINGS invocation used the host transport; a separate explicit-QEMU run passed. Physical HCI failure isolation and its timing remain unverified, and this is not a complete release regression.

日本語: 証跡は `out/bt-isolation/verification.json` です。platform 152件＋xHCI 59件＝211件に、新規の隔離・メタデータ・merge 6件と退役1件を内包します。QEMUはP/設定経由のadapter不在、r2 keyboard/mouseとviewer復帰、SETTINGS作成、既存BT loan再開fixture、feature無しHello復帰の7構成です。反復を別構成へ加算しません。mouse初回はELFパス誤指定でQEMU前に失敗し、修正後合格しました。SETTINGS初回はhost経路だったため、明示QEMU版を別実施し合格しました。実HCI隔離と実機時間は未確認で、全リリース回帰とは区別します。
