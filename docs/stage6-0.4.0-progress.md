# Stage6 LE runtime checkpoint — 2026-10-15

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
