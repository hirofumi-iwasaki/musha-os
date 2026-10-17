# Stage6 LE runtime checkpoint — 2026-10-15

Status: current（2026-10-15、第3回ローカル監査。実Bluetooth未検証）。

## Current v0.4.0 scope and limitations

Earlier checkpoints retain their original evidence and pending lists. USB stream framing, shared ACL buffers and stored SC reconnect listed there have since been implemented locally; this section and the later continuation describe the current state. Earlier counts are not added to this run.

Stored-key reconnect supports only an exact address-and-type match for public or static random addresses. IRK/RPA resolution, NRPA/unresolved identities, simultaneous peers, arbitrary BLE mice, Share delivery, Classic Magic Keyboard A3118 and automatic peer-display association are **unimplemented or outside v0.4.0 scope**. Hardware testing does not implement these features. BLE keyboard host coverage remains, but keyboard procurement/hardware acceptance is outside this release.

A new fixture uses the same runtime Scan API to require Reset first in every generation, prohibit setup before successful Reset Complete, reject a prior-generation completion and stop after Reset failure. Existing hardware-error/disconnect, stream generation, stopped-owner lease and restart-budget checks remain. Viewer and Ring3 recovery require normal return through Host/file/input-loan scopes and controller halt/BME-off; uncertain stop retains quarantine. Host tests do not establish controller quiescence.

Hardware gates are MM-BTUD46 USB/revision/firmware compatibility; Xbox1914 firmware and actual Report Map; connect/authenticate/save; cold reboot stored-key reconnect; disconnect/restart/fault shutdown; and NUC5/Ryzen cold/hot operation. A captured map outside the supported parser subset would require more software work. That conditional compatibility risk is distinct from simply waiting for hardware evidence.

### Xbox fixture provenance and measured-value limits

The [published Series X|S descriptor record](https://github.com/atar-axis/xpadneo/blob/master/docs/descriptors/xbxs.md), checked on 2026-10-15, identifies 045E:0B13; it is not a capture from the owner's Xbox1914. The independently constructed fixture exercises Report ID1 with 17 bytes: four unsigned16 axes, two10-bit triggers with padding, hat, 15 buttons and Consumer Record/Share with padding. Concrete checks use axis65535, button1 and Share-only changes. It does not verify complete descriptor identity, CRC, output Report ID3 vibration or the target firmware's values. No driver or descriptor dump was copied and no dependency was added.

This audit passed **106 platform security tests and22 input tests**, including one new Reset-order test and existing lease/fault coverage. These are not added to the older412 count. Logs: `out/mac-local-preparation-040-round3/bluetooth-platform-tests.log` and `input-tests.log`. Bluetooth QEMU was not rerun during this audit.

## 現在のv0.4.0範囲と未対応機能

以下の過去checkpointの「残件」一覧には、その後ローカル実装したUSB stream組立て・shared ACL buffer・保存SC鍵再接続が含まれます。現在の状態は本節と末尾の継続記録を優先してください。過去件数は今回へ加算しません。

保存鍵再接続はpublic addressとstatic random addressについて種別・全addressの完全一致に限定します。IRK/RPA解決、NRPA/未解決identityの再接続、複数peer同時、任意BLE mouse、Shareイベント配信、Classic Magic Keyboard A3118、相手displayを選ぶ自動associationは**未実装又はv0.4.0対象外の制約**です。実機試験をすれば実装済みになる機能として扱いません。BLE keyboardはホスト検証済み部分を残しますが、v0.4.0の調達/実機受入対象ではありません。

今回のソフトウェア側再確認では、同じruntimeが使う `scan::Scan` に新世代ごとのReset先行、Reset完了前の次command禁止、旧世代Complete拒否、Reset失敗後停止のfixtureを追加しました。既存のhardware-error/disconnect判定、stream世代検査、lease所有者/停止確認拒否、再開上限を保持しています。viewerとRing3の再開はHost/file/input loanの通常終了とcontroller halt/BME-off確認を経由し、不確実停止はquarantineです。ホスト試験だけで実controllerの停止を証明した扱いにしません。

対象実機で残る確認は、MM-BTUD46のUSB/revision/firmwareの適合、Xbox1914のfirmwareと実Report Map、接続/認証/保存、cold reboot後の保存鍵再接続、入力の切断/再開、fault時停止、NUC5/Ryzen双方のcold/hot挙動です。対象実機から得たmapがparser subset外なら追加software実装が必要です。この条件付きの互換性リスクを「実機待ちだけ」と同一視しません。

### Xbox fixtureの出所と実値の境界

2026-10-15に[xpadneoのSeries X|S descriptor記録](https://github.com/atar-axis/xpadneo/blob/master/docs/descriptors/xbxs.md)を再確認。これは `045E:0B13` の公開記録で、所有者のXbox1914から採取したものではありません。fixtureは独自に構成し、入力Report ID1の17byte、unsigned16 stick4軸、10bit trigger2個と各6bit padding、4bit hatと4bit padding、15buttonと1bit padding、1bit Consumer Record/Shareと7bit paddingを検査します。fixture中のstick値65535、button1、Shareだけの変化を具体的に確認します。公開dumpのinput field形状を事実参照し、descriptor全体の同一性・CRC・出力Report ID3の振動・target firmwareの実値は検証していません。元driver/descriptor dumpのcopyや依存追加はありません。

今回の再確認はsecurity付きplatform **106**、input **22**試験が成功（以前の412件へ重ねて加算しません）。Reset順序の追加試験は1件、既存のReset先頭/lease/fault試験も含みます。ログは `out/mac-local-preparation-040-round3/bluetooth-platform-tests.log` と `input-tests.log`。今回BT用QEMUを再実行したとは主張しません。


## English

Current continuation: stored SC reconnect and multiple-report/gamepad decoding are connected in source and host/software fixtures; Bluetooth-present execution and target compatibility remain unverified. The earlier checkpoint below is retained as prior evidence; see the current continuation subsection for changes.

**Partial implementation, not stage6 completion.** Normal cooperative execution now contains USB HCI endpoint ownership, authorized scan/connect, bounded ACL/SC, conservative GATT/HOGP discovery and the existing owned-record writer connection. These Bluetooth-device branches compile but have **not run on a Bluetooth controller**: local QEMU has no USB Bluetooth device. Xbox1914/Magic Keyboard compatibility, stored-bond reconnect, multiple-report/gamepad support and hardware acceptance remain open. The earlier checkpoint was recorded before commit, based on `44895f0`.

Evidence for this continuation: `out/stage6-normal/verification.json`, `source-final.json`, `owner-start.json`/`owner-final.json`, builds, host logs and per-QEMU reports. Previous partial checkpoints remain under `out/stage6-live/` (349 host/8 QEMU) and `out/stage6-040/`; those counts are not added to the new run.

### Normal execution and trust boundary

* Viewer P or boot `pair=on` is the only scan authorization. A missing adapter/secure entropy visibly refuses the request. The selected live xHCI Host owns event IN, ACL IN/OUT and three independent completion mailboxes; TRB/slot/DCI/residual/generation checks preserve keyboard/storage completion ownership. Root/hub disconnect, command/ACL deadlines and ambiguous failures propagate to controller halt/BME-off; no timed-out DMA buffer is cleared or reused in flight.
* The normal path sends Reset/scan setup, selects connectable HID-service advertisements with keyboard/gamepad appearance, disables scanning, reads local address/LE buffer capacity, creates a central connection and verifies the returned role/address/handle. Controller-shared ACL buffers, unknown appearances, unsupported maps/formats and uncorrelated completions fail explicitly. ACL credit, 1KiB SDU and fragment limits are bounded. Current USB receive handling requires one complete HCI packet per completion; split/coalesced USB HCI packet framing remains open.
* SC is mandatory in this path: no Legacy fallback, OOB, IRK/CSRK/CTKD or Numeric Comparison implementation. Keyboard selection uses local **DisplayOnly**, displays six digits for entry on a peer KeyboardOnly/KeyboardDisplay; gamepad selection uses Just Works only if peer MITM is not requested. Explicit local EnterPasskey state/UI exists and is fixture-tested, but automatic peer-display association selection is not connected. Numeric Comparison combinations must be refused rather than relabeled passkey/JW. Targets' actual capabilities are unverified.
* The same `sc::Central` used by boot performs P-256 validation/ECDH, f4/f5/f6, 20 fresh-nonce passkey rounds, constant-time confirm/DHKey-check comparison, monotonic deadline/generation checking and correlated successful HCI Encryption Change. Only this state yields `VerifiedBond`. JW records have SC flag2, passkey records flag3; SC LTK has zero EDIV/Rand in the unchanged 4KiB codec. An unauthenticated JW bond is not an authenticated security-level4 claim.
* After encryption, bounded ATT service/characteristic/descriptor/read-blob discovery validates Report Map/reference before keyboard binding. This deliberately supports only **one Report characteristic and the existing keyboard-map subset**. Unknown/gamepad/multiple-report maps are rejected. A verified bond is handed to the selected storage Device only while BOT is idle; the actual latest recognized PAIRING.DAT is reread before merge. Existing `write_owned`/SYNCHRONIZE CACHE success must be acknowledged before CCCD enables notifications. No plaintext/unencrypted notification is accepted as input.
* USB/BT held usages are merged with independent ownership; one source's release cannot release the other's held key. Viewer and existing r2 queue receive validated key transitions. P-256/SC/GATT/bonding/HOGP host fixtures execute these same platform functions; the boot MMIO/BOT dispatcher has build and BT-absent evidence only, not an emulated radio transcript.

### Entropy, passkey and secret lifetime

**Superseded entropy paragraph:** the following firmware-only pool description records the earlier implementation. The current 2026-10-17 owner policy and multi-source DRBG are specified in [entropy-sources](entropy-sources-0.4.0.md). Existing earlier test assertions below remain historical evidence.

Before GOP selection/final map/EBS, UEFI RNG fills a CPU-owned **4096-byte** pool using an explicitly requested SP800-90 HASH/HMAC/CTR256 algorithm. No protocol pointer survives EBS. Unsupported/error/stuck output rejects the pool; there is **no default/raw, fixed-seed, clock, RDRAND or weak PRNG fallback**. This trusts the firmware's advertised algorithm, not a certification of its entropy source. QEMU evidence records `BT_ENTROPY UNAVAILABLE`, so actual provider success remains unverified.

At most **8 authorized attempts per boot**; exhaustion may occur earlier. A typical valid-scalar JW attempt needs48 bytes, displayed passkey356 bytes and entered passkey352 bytes; rejected scalars/uniform sampling retries cost more (bounded16 each). Pool bytes are erased as consumed, including failed/canceled attempts, never returned or reseeded after success; restart does not replenish them. Insufficient bytes fail safely. Pool, key/scalar, session nonce/DH/LTK, owned record scratch and secret UI reply cleanup use zeroize/volatile erase. This does not promise erasure of every compiler temporary/register copy.

Passkey entry uses the viewer's existing single keyboard consumer: six digits including leading zeros, keypad/top-row, backspace, Enter only after six digits, Esc cancel,30s deadline and stale-generation rejection. The screen shows entered **count**, not entered digits. HID/app/detach key-code diagnostics are suppressed after sensitive entry starts; the r2 queue cannot receive captured secret input. Displayed association passkeys are intentionally shown to the owner, never debug-logged. The QEMU-only UI feature also runs an official ECDH known-answer check; fixed scalar/keyboard transcript data are confined to that fixture, not production RNG.

The14,056-byte BT CPU state originally stored inline in Host/Configured exhausted the existing128KiB guarded stack (vector8 before viewer). It now occupies an image-owned fixed region; Host holds an8-byte lease (Host544 bytes). The pure lease rejects double loans, retires on Drop/secret cleanup and is reusable only after the owning controller's halt **and** BME-off, after all Host callbacks return. Wrong-owner/live/unconfirmed-stop release is refused; no stop confirmation means quarantine. CPU state is not a DMA buffer. QEMU r1/r2 recovery passes after this correction; no Bluetooth-present restart has been exercised.

### Persistence and verification limits

CONFIG.TXT remains completely read-only. Only recognized OS-owned PAIRING.DAT is writable:4096-byte directory size, ceil(4096/clusterBytes) chain, full-cluster initial zeroing and internal valid length. Successful pairing plus durable save records exact raw CONFIG SHA-256; loading the same consumed hash suppresses boot `pair=on`, and identical-content resaving does not rearm it. P remains explicit authorization. No secret/key is debug formatted. Save/sync failure does not publish RAM consumption or input: **disk state may already have changed**, so no next-boot nonconsumption/atomic rollback guarantee exists. The unchanged writer's power-loss/rollback limits still apply.

Final local checks: **361 workspace host tests**, **27 distribution/package Python tests**; representative QEMU includes r1 normal execution→viewer, r2 real TIME/WAIT/INPUT→viewer, P/config no-adapter refusal, passkey submit/cancel/timeout with UEFI-target ECDH KAT and no secret key-code logs, and generic USB OUT/short/wrap. Exact final reports/counts and EFI/source hashes are in verification.json; duplicate runs are not added. No full QEMU matrix, new Bluetooth-device write/fsck/chkdsk, physical USB, real pairing or target compatibility was performed.

Bring-up failures retained in evidence: inline-state vector8 fixed by the lease; an initial UI build omitted existing viewer mode; one r2 invocation referenced a missing old ELF and failed before guest boot; the startup-log host mock lacked the new adapter-status accessor. Corrected runs pass. Initial incremental visibility errors were compile corrections, not successful radio evidence.

Remaining: complete HCI USB stream framing/shared-buffer capability support; stored identity/bond reconnect and encryption-key reuse; broader GATT/report maps, Xbox gamepad and optional BLE mouse; automatic peer-display entry/other association policy; Bluetooth-present teardown/restart/fault and live writer failure fixtures; real UEFI RNG, exact dongle/model/firmware compatibility, full QEMU regression and NUC5/Ryzen cold/hot acceptance. See [work plan](work-plan-0.4.0.md), [crypto ledger](bluetooth-crypto-dependencies.md), [SETTINGS](settings.md). The28-package selected original license bundle is now in distribution DOCS/NOTICE; no release asset was generated. The new state machine is unaudited; p2560.13.2 is not a claim of audited ECDH/x86-64/constant-time implementation.

Owner scope0.4/scope0.5/no-shell/scenario files were preserved and excluded. That validation checkpoint preceded commit; no push, physical USB, remote CI, tag/Release or formal candidate was performed.

### Stored SC reconnect and multiple reports (current continuation)

The current continuation connects the unchanged PAIRING.DAT codec to a bounded, zeroizing SC-only key store. Public addresses and static random addresses must match both saved type and exact address. Private/resolvable addresses are refused: there is no IRK resolution or guessed identity. Only 16-byte SC keys with zero EDIV/Rand are reused. A command key is issued once; generation, connection handle, command status and successful Encryption Change must match before authenticated ATT discovery. Key rejection, timeout, cancellation or corruption does not silently start pairing or downgrade. Existing consumed CONFIG hash is retained, and reuse does not rewrite the bond or consume new CONFIG content. Codec CRC recognizes structure; it is not authenticity protection against malicious storage replacement.

Normal GATT discovery now walks up to eight Report characteristics, their own bounded descriptor ranges, Report References and CCCDs, then enables each input CCCD. The same runtime parser binds ATT value handle plus Report ID, connection and generation. Keyboard hold ownership is shared across report IDs. A bounded Game Pad Application subset supports absolute variable buttons1-32, Generic Desktop axes/hat and Simulation Controls Accelerator/Brake; malformed/unsupported fields, overflow and ambiguous maps fail closed. Gamepad transitions feed the existing SYS_INPUT record; the viewer does not acquire a mouse/gamepad UI. The actual Xbox1914 Report Map has not been captured, so this subset is not a target compatibility claim. Optional BLE mouse decoding remains unimplemented.

A subscribed disconnection schedules at most eight outer session restarts per boot. Viewer callbacks return through Host/file Drop and confirmed halt plus BME-off before scrub/restart. During an active r2 input loan, shutdown reports disconnected to the app; the app must exit/fault normally before the outer restart. It does not jump across a loan or preempt a noncooperating app. Explicit P on an active subscription requests a fresh authorized pair after this same stop boundary. Mid-pair failure/key rejection does not auto-repair. Initial-boot stored-key reconnect and post-subscription disconnect restart are separate cases. Radio-present execution remains unverified; QEMU restart probes inject only a stop request, never synthetic authentication.

New host transcripts exercise the same Store, Pending, Discovery, Reports and decoder functions used by runtime, including bad type/RPA/generation/handle, unsolicited encryption, cancel/deadline, multiple ID/ATT-handle/reference collisions, malformed maps and bounded arbitrary map input. A QEMU software map fixture feeds real SYS_INPUT and verifies button/axis records. Viewer and active-loan restart fixtures verify distinct normal return/stop paths. Fixed peer keys belong only to host/software fixtures, not production entropy. No secret-containing structure implements Debug; key/record scratch is erased on release, with the existing compiler-temporary limitation.

The frozen implementation manifest and current full-regression results are recorded under `out/stage6-regression/`; prior checkpoint evidence above is retained separately and is not added to new counts. Full-matrix completion is reported only after all current runs finish.


Final current local regression: **371 host tests, 52 Python tests and 296 QEMU case/config executions pass** (suite overlaps included; prior checkpoint/retry runs excluded). SDK relocation/four ELF identities, two virtual partition update paths and1000 native lwIP restarts pass. Exact groups, one resolved observer race, limitations, normal EFI hash and owner/source manifests are in `out/stage6-regression/results.md` and `verification.json`. This does not verify Bluetooth-present execution or complete stage6.

Owner subsequently approved one local stage6 commit. This records the validated partial implementation, not Bluetooth hardware acceptance; push remains unapproved.

### USB framing, shared buffers and cold reconnect (current local increment)

Normal runtime now uses the same bounded `stream::Stream` tested on the host to assemble HCI Event and ACL packets across USB completions and dispatch coalesced packets. Event257/ACL1028-byte CPU buffers retain no DMA pointer; partial packets have a one-second deadline, generation/clock/length checks and poisoned failure state. Completed buffers and production partial-buffer drops are zeroized. These CPU buffers belong to the existing single Device lease; controller halt/BME-off and stopped-owner confirmation still precede DMA/lease reuse. No outer Ring3/file/Host lifetime is bypassed.

LE Read Buffer Size returning zero packet count now requests standard HCI Read Buffer Size and uses the returned shared ACL pool under the existing single-LE-connection credit accounting. This is prescribed by [Bluetooth Core5.4 HCI sections4.1.1/7.4.5/7.8.2](https://www.bluetooth.com/wp-content/uploads/Files/Specification/HTML/Core-54/out/en/host-controller-interface/host-controller-interface-functional-specification.html). It does not enable Classic links, SCO, firmware downloads or unbounded buffering. Invalid status/length/capacity fails closed.

On cold boot, the existing storage reader loads a recognized SC record; `stored_any` starts a reconnect session without P or new pairing entropy. The first scan command is HCI Reset on every new generation, including a stopped restart. Previously stored peers required HID UUID/Appearance in every advertisement; that incorrectly blocked directed advertisements with empty AD data. Stored reconnect now requires connectable advertising and an exact saved public/static address **and address type**, with no AD class requirement. Unknown peers, RPA/NRPA and unresolved identity types are rejected. Connection-complete peer identity, handle/generation, key-command issuance and successful encryption must validate before authenticated GATT/HOGP binding. Only then is a single keyboard/gamepad kind selected from the validated map; mixed/unknown classes fail closed. First-time pairing retains its reference-device advertisement filter. Saved key/CONFIG consumption format and durable-save boundary are unchanged.

A valid hardware-error event now requests the existing stop-confirmed restart, as subscribed-link disconnect already did. Malformed/wrong-handle disconnects do not authorize restart. Reset is sent when an authorized pairing/reconnect session begins, not merely because an idle dongle is present. Restart is bounded by the existing eight-request budget. A USB removal, unsupported device, exhausted budget or failed stop is not silently recovered; unconfirmed halt/BME-off retains quarantine. Reboot/adapter behavior remains unverified on hardware.

The [published Xbox BLE descriptor facts](https://github.com/atar-axis/xpadneo/blob/master/docs/descriptors/xbxs.md) motivated an independently authored synthetic field fixture: four unsigned16 stick axes, two10-bit triggers, hat and15 buttons. Consumer Record/Share is recognized **only** as a one-bit absolute0/1 field and deliberately not delivered, because inventing a gamepad code would alias existing Button32. Other Consumer usages/field shapes remain rejected. Existing Button1..32→code0..31 semantics and the public ABI/schema are unchanged. Host tests also pass the known Share omission through authenticated `Reports` binding/notification. Neither xpadneo driver code nor its descriptor dump was copied. Its [license policy](https://github.com/atar-axis/xpadneo/blob/master/LICENSE.md) distinguishes GPL-2.0-only kernel code and generally GPL-3.0-or-later non-driver content; this is factual reference only, no adopted dependency/source.

The new `sdk/examples/input-monitor/main.c` observes real r2 records without fixture ordering/count assumptions, displays bounded hexadecimal event fields and exits on Q/Esc or after five minutes. Existing SDK/checker output remains `hello-c.elf`; the example README gives the exact build command. Local QEMU uses real keyboard syscalls plus explicitly synthetic gamepad injection, never a claimed real Bluetooth controller. Host412 tests (security feature enabled), including framing7, the new Realtek core4, Xbox input2 and directed-reconnect coverage, passed; the HOGP reconnect suite was additionally rechecked. Final QEMU commands/results and EFI hashes are under `out/remaining-040`.

Final local increment validation: **13/13 QEMU configurations passed**, including overlap with the earlier live-monitor trial; P/config2, passkey submit/cancel/timeout3, viewer restart1, live monitor1, r2 mouse1, r2 loan restart1, r1 LAN three-launch1, DHCP77 cooperative1, normal pixel1 and OUT stall1. The initial r1 diagnostic-feature mismatch is retained separately: app exit/scrub succeeded but its trace lacked VIEWER_FRAME; the same strict three-launch/LAN test passed with the existing viewer probe feature. Full historical QEMU matrix and all real-device tests were not repeated. Normal EFI SHA256 is recorded in `out/remaining-040/verification.json`.

Remaining hardware gates: MM-BTUD46 exact USB/revision/firmware behavior and Xbox1914 actual Report Map/firmware, pairing/save/reboot/reconnect on both reference machines. IRK/RPA resolution, multiple simultaneous peers, optional BLE mouse and Share delivery remain unsupported. BLE keyboard is implemented/host-tested but not procured or hardware-verified for v0.4.0. Magic Keyboard A3118 is Classic, deferred to v0.5.0. No real radio success or stage6 hardware completion is claimed.

## 日本語

今回の続きでは保存SC鍵再接続と複数report/gamepadをソースおよびhost/software fixtureへ接続した。Bluetooth装置経路と対象互換は未検証。以下の旧checkpointは前回証拠として保持し、変更は今回の続きの節を参照する。

**部分実装で、工程6完了ではありません。** 通常の協調実行へUSB HCI endpoint保持、承認入口からのscan/connect、上限付きACL/SC、限定GATT/HOGP探索と既存所有record writerを接続しました。ただしローカルQEMUにUSB Bluetooth装置が無く、**Bluetooth装置を通る分岐はbuildのみで実行未検証**です。Xbox1914/Magic Keyboard互換、保存bondの再接続、複数report/gamepadと実機受入は残件。この旧checkpointは44895f0を基準にcommit前に記録しました。

今回の証拠は `out/stage6-normal/verification.json`、source/owner hash、build/host logと各QEMU report。前回の349host/8QEMUは `out/stage6-live/`、初回は `out/stage6-040/` に保持し、今回件数へ加算しません。

### 通常経路と信頼境界

* scan許可はviewer P/起動pair=onのみ。adapter/安全な乱数が無ければ可視拒否。選択された生存xHCI Hostがevent IN、ACL IN/OUTと独立completion mailbox3個を所有し、TRB/slot/DCI/residual/generationを照合してkeyboard/storage completionを奪いません。root/hub切断、command/ACL期限、曖昧な失敗はhalt/BME-offへ伝播し、実行中DMA bufferを消去・再利用しません。
* Reset/scan設定後、接続可能なHID service広告のkeyboard/gamepad appearanceを選択し、scan停止・local address/LE buffer読出し・central接続を行いrole/address/handleを照合。共有ACL buffer、未知appearance/map/形式、無関係completionは明示拒否。credit、1KiB SDU/断片の上限があります。現USB受信は1 completionに完全HCI packet1個を要求し、USB上の分割/結合frame対応は残件です。
* 通常経路はSC必須でLegacy fallback無し。OOB/IRK/CSRK/CTKD/Numeric Comparisonは未対応。keyboardはOS側DisplayOnlyとして6桁を表示し、相手KeyboardOnly/KeyboardDisplayへ入力。gamepadは相手がMITMを要求しない場合のみJW。OS側EnterPasskey状態/UIはfixture合格ですが、相手displayを自動選択する通常association経路は未接続です。Numeric Comparisonをpasskey/JWへ読み替えず拒否する必要があり、対象機の実IO能力は未確認です。
* bootと同じsc::CentralでP-256/ECDH、f4/f5/f6、passkey20roundの新規nonce、定数時間confirm/DHKey比較、単調期限/世代、対応する成功HCI Encryption Changeを照合。ここだけがVerifiedBondを生成します。SC JW flag2、passkey flag3、EDIV/Rand0を従来4KiB形式へ格納。JWを認証済みsecurity level4とは主張しません。
* 暗号化後にATT service/characteristic/descriptor/read-blobでReport Map/referenceを検査。現状は**Report characteristic1個と既存keyboard map subsetのみ**で、未知/gamepad/複数reportは拒否します。選択storageのBOT idle時だけ最新の認識済みPAIRING.DATを再読しmerge。write_owned/SYNCHRONIZE CACHE成功ACK後だけCCCD通知を有効化し、未認証通知を入力へ通しません。
* USB/BT保持キーを別所有としてmergeし、一方の解放で他方の保持を解除しません。検証済み遷移をviewer/既存r2 queueへ渡します。host transcriptは通常版と同じSC/GATT/HOGP/save関数を実行しますが、bootのMMIO/BOT dispatcherはbuild/BT不在確認のみで、無線装置を模擬実行した証拠ではありません。

### 乱数・パスキー・秘密の寿命

**Current policy / 現行方針:** [entropy-sources](entropy-sources-0.4.0.md) supersedes the earlier firmware-only pool description below (2026-10-17). / 以下の旧firmware限定pool記述は過去の実装記録です。現行は多源DRBGです。

GOP選択/最終map/EBSより前に、UEFI RNGの明示SP800-90 HASH/HMAC/CTR256でCPU所有4096byte poolを確保。firmware pointerをEBS後へ残しません。未対応/error/明らかなstuck出力は拒否し、default/raw・固定seed・時計・RDRAND・弱いPRNGへのfallbackはありません。firmware申告を信頼する設計で、乱数源の認証ではありません。QEMUはBT_ENTROPY UNAVAILABLEで、本番provider成功は未検証です。

1boot最大8試行で、先にpool不足となる場合があります。正常scalar時の典型消費はJW48、表示passkey356、入力passkey352byte。scalar/均等samplingの再試行は各16回上限で追加消費。失敗/取消しも消費byteを戻さず、使用後消去し、成功初期化後の再seed/restart補充もしません。不足時は安全に失敗します。pool/key/scalar/nonce/DH/LTK、record scratch、秘密UI返信をzeroize/volatileで消去しますが、全compiler一時値/registerの消去保証ではありません。

入力はviewer既存の単一keyboard consumerで、先頭0を含む6桁、上段/テンキー、Backspace、6桁後Enter、Esc取消、30秒期限、旧世代拒否。画面には入力桁数だけを表示。秘密入力開始後はHID/app/detachのkey code診断を抑制し、r2 queueへ秘密入力を漏らしません。表示associationのpasskeyは所有者画面へ意図的に表示しdebugへ出しません。QEMU限定UI featureには公式ECDH既知解もあり、固定scalar/入力列は試験専用で本番乱数には使いません。

BT CPU状態14,056byteをHost/Configuredへ直接置いた初回は、既存128KiB guard付きstackでviewer前vector8を起こしました。image所有固定領域へ移しHost内は8byte lease（Host544byte）に修正。二重貸与を拒否、Drop/秘密消去後retireし、全callback終了と所有controllerのhalt+BME-off確認後だけ再貸与します。別owner/生存中/停止未確認は拒否し隔離を維持。CPU状態とDMA bufferは別です。修正後QEMU r1/r2復帰は合格ですが、BT装置付きrestartは未確認です。

### 保存と検証範囲

CONFIG.TXTは完全読取り専用。書けるのは認識済みOS所有PAIRING.DATのみで、directory size4096、ceil(4096/clusterBytes) chain、初回全cluster zero、内部有効長という既存方式を保持。pair成功＋durable保存後、正確なraw CONFIG SHA-256を記録し、同一消費済みhashの起動pair=onを抑止。同じ内容の保存し直しでは再有効にならず、Pは明示入口です。鍵のdebug表示はしません。write/sync失敗時はRAM消費/入力を公開しませんが、**diskは既に変更された可能性**があり、次boot未消費・原子的rollbackを保証しません。

最終ローカル検証はworkspace host361、配布/package Python27。代表QEMUは通常r1→viewer、r2実TIME/WAIT/INPUT→viewer、P/configのadapter不在拒否、passkey入力/取消/期限＋UEFI-target ECDH既知解/秘密key code非表示、汎用USB OUT/short/wrap。正確な最終件数・EFI/source hashはverification.jsonに記録し、重複再実行は加算しません。全QEMU回帰、新live BT書込みのfsck/chkdsk、実USB、実pairing/対象互換は未実施です。

失敗記録は保持：inline状態のvector8をlease化で修正、UI初回buildの既存viewer feature不足、r2の存在しない旧ELF path（guest起動前失敗）、startup-log host mockのadapter accessor不足。修正後は合格。初期の可視性compile修正も無線成功の証拠にはしません。

残件はUSB HCI stream framing/共有buffer、保存identity/bond再接続・鍵再利用、広いGATT/mapとXbox gamepad/任意BLE mouse、相手displayの自動association、BT装置付き停止/restart/faultとlive writer失敗fixture、実UEFI RNG/正確なdongle/対象firmware、全QEMUとNUC5/Ryzen冷間/hot受入です。[作業計画](work-plan-0.4.0.md)、[暗号台帳](bluetooth-crypto-dependencies.md)、[SETTINGS](settings.md)参照。28依存の採用原licenseは配布DOCS/NOTICEへ接続済みですが公開assetは生成していません。新state machineは未監査で、p2560.13.2のECDH/x86-64/定数時間が監査済みとは主張しません。

所有者のscope0.4/0.5、no-shell、scenario文書は保持・除外。この検証checkpointはcommit前の記録です。push、実USB、remote CI、tag/Release、正式candidate生成は行っていません。

### 保存SC鍵と複数レポート（今回の続き）

既存PAIRING.DAT codecを、上限付き・消去処理を持つSC専用保存鍵storeへ接続した。public addressとstatic random addressは保存した種別・全addressが一致する場合だけ使用する。IRK解決は無く、private/resolvable addressをidentityと推測せず拒否する。16byteのSC鍵・EDIV/Randゼロだけを再利用し、鍵commandの一回発行、generation、connection handle、command status、成功Encryption Changeを照合した後だけATTを許可する。鍵拒否、期限、取消、破損で自動再pairや降格をしない。既存CONFIG消費hashを保持し、再利用では保存recordの書換えや新CONFIG消費をしない。CRCは形式検査であり、悪意ある媒体交換への真正性保証ではない。

通常GATT探索は最大8個のReport characteristicと各descriptor範囲、Report Reference、CCCDを列挙し、入力CCCDを順に有効にする。同じruntime parserがATT value handleとReport ID、connection、generationを照合する。keyboardの保持状態は複数IDで共有する。Game Pad Applicationの限定subsetとして絶対variableのbutton1-32、Generic Desktop axis/hat、Simulation Controls Accelerator/Brakeを扱い、不正・非対応field、overflow、曖昧なmapは拒否する。gamepad変化は既存SYS_INPUT recordへ接続した。viewerへmouse/gamepad UIは追加しない。Xbox1914実Report Mapは未取得で、対象対応完了とは扱わない。任意BLE mouseは未実装。

subscription後の切断はboot内最大8回、外側sessionの再開を要求する。viewer callbackはHost/file Drop、haltとBME-off確認を終えてからscrub/restartする。r2 input loan中はshutdownで切断を返し、appが通常exit/faultしてloanを返した後だけ外側を再開する。loan scopeを飛び越えず、協調しないappを強制中断しない。subscription中の明示Pは同じ停止境界の後で新しい承認pairを要求する。pair途中失敗・鍵拒否では自動再pairしない。起動時保存鍵再接続とsubscription切断後再開を別caseとする。実Bluetooth装置経路は未検証で、QEMU再開probeは停止要求だけを注入し認証成功を偽装しない。

host transcriptはruntimeと同じStore/Pending/Discovery/Reports/decoder関数で、種別/RPA/generation/handle不一致、未要求暗号化、取消/期限、複数ID/ATT handle/reference衝突、不正map、上限付き任意map入力を検査する。QEMUのsoftware map fixtureから実SYS_INPUTへbutton/axisを渡す。viewer中とloan中の再開fixtureは別の通常return/停止経路を検証する。固定peer鍵はhost/software fixtureだけで、本番乱数には使わない。秘密を持つ構造体にDebugは無く、解放時に鍵/record scratchを消去するがcompiler一時copyの制限は残る。

固定実装manifestと今回の全回帰結果は`out/stage6-regression/`へ記録する。前回checkpoint証拠は保持し、新件数へ加算しない。全matrixの完了は現行runがすべて終了した後だけ報告する。

今回の最終ローカル回帰は**host371、Python52、QEMU296 case/configが合格**（suite間重複込み、前回checkpoint・再試験を加算しない）。SDK展開再現/4 ELF一致、仮想partition更新2経路、lwIP1000回も合格。内訳、解決した観測器競合1件、制限、通常EFIhash、owner/source manifestは`out/stage6-regression/results.md`と`verification.json`を参照。Bluetooth装置経路や工程6全体の完了を示さない。

所有者はその後、検証済みの工程6部分実装を1個のローカルcommitへ記録することを承認しました。実Bluetooth受入の完了は示さず、pushは未承認です。

### USB framing・shared buffer・cold再接続（今回のローカル実装）

通常runtimeにhostと同じ `stream::Stream` を接続し、USB completionをまたぐHCI Event/ACLの組立てと結合packetの順次処理を実装。CPU bufferはEvent257/ACL1028byte、DMA pointerを保持せず、部分packetは1秒期限・世代/時計/長さ検査と失敗後poisonを持ちます。完了bufferと本番の部分buffer解放時はzeroize。既存Device単一leaseへ所有させ、DMA/lease再利用前のcontroller halt/BME-off・stopped owner確認を維持し、Ring3/file/Host寿命を飛び越えません。

LE Read Buffer Sizeのpacket数0では標準HCI Read Buffer Sizeへ進み、同じ単一LE接続creditでshared ACL poolを使用します。上記Bluetooth Core5.4の規定に基づき、Classic link/SCO/firmware転送/無制限bufferを有効にする変更ではありません。status/長さ/容量不正は拒否します。

cold bootでは既存storage readerが認識済みSC recordを読み、`stored_any` がPや新pairing乱数無しに再接続を開始。停止確認後の再開を含め、新世代scanの最初はHCI Resetです。以前は保存peerにも毎回HID UUID/Appearance広告を要求し、空ADのdirected広告を拒否していました。保存再接続はconnectable広告と保存public/static address・種別の厳密一致で開始し、AD classを要求しません。unknown peer/RPA/NRPA/未解決identity種別は拒否。接続完了peer identity、handle/世代、鍵command発行と暗号化成功を確認して初めて認証済みGATT/HOGP bindingへ進み、そのmapから単一keyboard/gamepad種別を決めます。混在/不明classは拒否し、初回pairingの基準機器広告filterは保持。保存鍵/CONFIG消費形式とdurable-save境界は不変です。

正常形式のhardware-error eventでも、従来の接続済みlink切断と同じ停止確認後restartを要求します。不正/別handle切断では再開を承認しません。Resetは認可されたpair/reconnect session開始時に送り、idle dongleの存在だけでは送りません。既存restart上限8回を維持。USB取外し/未対応機器/上限超過/停止失敗を黙って回復せず、halt/BME-off未確認ではquarantineを保持します。実adapter/reboot動作は未確認です。

上記xpadneoの公開descriptor情報を事実参照し、独自synthetic fixtureでunsigned16 stick4軸、10bit trigger2個、hat、15buttonを検査。Consumer Record/Shareだけはabsolute0/1の1bit fieldとして認識して**未配信**にします。新codeを捏造すると既存Button32とaliasするためです。他のConsumer usage/field形状は拒否し、Button1..32→code0..31と公開ABI/schemaは変えません。Share無視を認証済みReports binding/notificationまでhost確認しました。xpadneo driver code/descriptor dumpはコピーしていません。同license方針はkernel codeをGPL-2.0-only、非driverを原則GPL-3.0-or-laterと分けており、今回は事実参照だけでsource/依存を採用しません。

新 `sdk/examples/input-monitor/main.c` は固定fixture件数/順序無しでr2入力を観測し、有界16進field表示、Q/Escまたは5分で終了。既存checker出力名 `hello-c.elf` と正確なbuild commandはexample READMEに記載。QEMUは実keyboard syscallと明示的software gamepad注入であり、実BT成功ではありません。security付きhost412件（framing7、Realtek core4、Xbox input2、directed再接続を含む）が合格、HOGP reconnect suiteも追加再確認しました。最終QEMU command/result/EFI hashは `out/remaining-040` に保存します。

今回の最終ローカル検証は **QEMU13/13構成合格**。先行monitor試行と重複し、P/config2、passkey submit/cancel/timeout3、viewer再開1、live monitor1、r2 mouse1、r2 loan再開1、r1 LAN3連続1、DHCP77協調1、通常pixel1、OUT stall1です。初回r1は診断featureの取り違えで、exit/scrub成功後のVIEWER_FRAME trace不足でした。既存viewer probe featureへ修正し、厳密な3連続/LAN条件で再試験合格。過去の全QEMU matrixと実機試験は今回再実施していません。通常EFI SHA256は `out/remaining-040/verification.json` に記録します。

実機gateはMM-BTUD46のUSB/revision/firmware挙動、Xbox1914実Report Map/firmware、両機種でのpair/save/reboot/reconnectです。IRK/RPA解決、複数peer同時、任意BLE mouse、Share配信は未対応。BLE keyboardは実装/host検証済みですがv0.4.0では調達/実機検証しません。Magic Keyboard A3118はClassicでv0.5.0へ延期。実電波成功や工程6実機完了は主張しません。
