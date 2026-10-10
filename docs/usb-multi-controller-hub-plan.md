# Multi-controller and hub implementation plan / 複数コントローラー・ハブ対応設計

## English

### Evidence and scope

The user's D2 hardware photograph IMG_7422.jpg establishes two class-09 hubs (`045B:0210` SuperSpeed, `045B:0209` HighSpeed) on controller `1912:0014`, BDF `0600`. Controllers `1022:149C` at `0801`, `0803`, `0F03` and `1002:7446` at `0D02` were not probed. This is evidence about this machine, not all AM4 boards. Endpoint locations behind hubs or other controllers remain unknown.

Support bounded, sequential xHCI discovery and bounded hub topology; preserve UEFI/GOP, read-only BOT/FAT32, one Boot Keyboard input session, existing application snapshot API, and shutdown checks. Do not introduce USB storage writes, arbitrary class drivers, hot-plug management, SMP or parallel active controllers in this work.

### M1: sequential controllers

Before ExitBootServices, collect up to eight unique segment-zero xHCI PCI I/O BAR resources. Copy only scalar BAR/BDF records, sort by BDF and report overflow explicitly. Reject duplicates/invalid or conflicting mappings. Do not obtain a BAR by guessing from PCI IDs. Keep the NIC's separate selection contract.

Reserve a dedicated 1MiB below-4GiB DMA slice per collected controller in one permanent LoaderData allocation; never share DMA addresses across different controllers. Validate and map all collected BAR ranges UC/RW/NX before switching page tables. The handoff must fit its allocated page. Firmware pointers never enter runtime.

For multiple controllers, run an initial discovery pass: reset, own, enumerate direct devices, read storage snapshots, record keyboard availability, then halt and disable PCI bus mastering. Do not start a long keyboard session or report application discovery complete during this pass. File events copy data into the existing application-owned cache, which keeps its first successful snapshot even if the storage controller is later stopped.

After scanning every controller, reinitialize the first successfully scanned controller containing a compatible keyboard for the final input session. Do not require keyboard and storage on the same controller. Report Ready exactly once after discovery. For a single controller retain the current one-pass behavior. If no keyboard exists, complete discovery once and use the bounded no-input application path.

Ordinary isolated reset/enumeration failures are recorded per BDF and the scan continues. A controller's DMA memory stays reserved even after failure. Failure to verify controller quiescence or bus-master disable stops the scan rather than initializing another controller. Never reuse a failed controller's pool for a retry. The selected input controller may reuse only its own slice after a successful quiesce. Fatal application errors abort scanning. Keep normal and injected-fault builds distinct.

Diagnostics must distinguish candidate, probing, discovery result, failure and input selection. Device rows must identify their controller; per-controller summaries must survive later scans. The visible device table is bounded and may show only the current controller; the debug log retains the full sequence. Probe matches do not imply successful input or file reads.

### H1/H2: hub topology and enumeration

Build pure checked parsers and topology arithmetic first. Maintain bounded records containing controller index, root port, route string, depth, parent slot/downstream port, speed and transaction-translator ancestry. Route strings exclude the root port and are limited to five four-bit tiers. Reject excess topology explicitly; never silently truncate a path. The initial supported bound is 15 downstream ports per hub and eight simultaneously allocated slots including hubs. Device count and DMA allocation remain independently bounded.

H1 integrates USB 2 hubs: validate configuration and class-specific hub descriptor; select the supported configuration/interface; set the xHCI Slot Hub/Number-of-Ports/TT fields via the proper context command. Choose single-TT operation unless multi-TT alternate-interface selection is explicitly implemented. Power ports as specified by the hub descriptor, wait power-good time, read class GET_STATUS, debounce connection, reset a connected port, wait for enable/reset completion, clear only the defined change features, determine low/full/high speed and construct the child's route/root-port/TT context before Address Device. Full/low-speed devices behind a high-speed hub need the correct TT hub slot and port; nested full-speed hubs retain that ancestry. Keep the parent hub slot alive until all descendants are disabled.

H2 integrates SuperSpeed hubs separately: descriptor type `0x2A`, applicable hub depth/setup requests, USB3 port power/reset/link-status semantics and upstream-compatible speed IDs. Do not reuse USB2 status-bit interpretation. USB2 and USB3 companion hubs may represent the same physical sockets; do not merge records by VID/PID. A controller's Supported Protocol/PSI data remains authoritative for speed encoding.

Control transfers need bounded reusable producers/cycle handling: the current hard-coded EP0 TRB indices are insufficient for arbitrary hub port requests. Read and zero-length requests must retain correct Setup/Data/Status directions. Handle short descriptor replies by validated actual length, not by assuming every requested byte arrived. Never replay W1C/change bits or reset an occupied port twice inadvertently.

Use a bounded topology work queue rather than unbounded recursion. Initially enumerate devices present at boot, with status polling during discovery; dynamic attach/detach and hub interrupt endpoints require a later design. Diagnose unsupported hub layouts, limits, overcurrent, reset timeouts and disconnects separately. On transfer/host errors that leave endpoint state uncertain, stop that controller safely rather than continuing transfers through a poisoned ring. During final input, verify the keyboard's upstream path remains connected. Disable child slots before parent slots on normal teardown; on controller failure halt and disable DMA while retaining all buffers.

### Acceptance and milestones

- M1: QEMU with storage on controller A and keyboard on B; an empty first controller; all controllers without keyboard; bounded list/duplicate/DMA separation checks; isolated failures and quiesce failure policy; existing single-controller keyboard/file/network and fault regressions.
- Hub foundations: malformed/truncated USB2/USB3 hub descriptors, port limits, status-bit differences, five-tier routes, excessive depth, TT ancestry and DMA-independent context arithmetic.
- H1: QEMU direct and nested USB hubs with keyboard/storage below the hub, mixed-speed TT cases where the emulator supports them, disconnect/reset/overcurrent errors and slot/DMA limits. QEMU's generic hub does not establish SuperSpeed compatibility.
- H2: a suitable SuperSpeed hub model or physical hardware; verify the observed `045B:0210` and `045B:0209` independently, including normal boot, keyboard events, MUSHA.TXT reads and DMA shutdown.

Implement M1 and the pure hub foundations first, then integrate H1 and H2. A stage is complete only after its checks; document remaining runtime limitations and rebuild a normal image before physical tests. USB writing remains a separate explicitly requested operation. Do not change branch, reset existing work or push as part of this work.

References: [Intel xHCI 1.2b](https://cdrdv2-public.intel.com/625472/625472_xHCI_Rev1_2b.pdf), sections 4.3/4.6/6.2; [USB-IF USB 2.0 specification](https://www.usb.org/document-library/usb-20-specification), chapter 11; [USB-IF USB 3.2 specification](https://www.usb.org/document-library/usb-32-specification-released-september-22-2017-and-ecns), hub/routing definitions. Check exact field and request definitions against these before runtime integration.

## 日本語

### 観測結果と範囲

D2写真IMG_7422.jpgで、`1912:0014`／BDF `0600`配下のclass 09ハブ2件（`045B:0210` SuperSpeed、`045B:0209` HighSpeed）が確認できた。`1022:149C`の`0801`・`0803`・`0F03`、`1002:7446`の`0D02`は未調査だった。この機体の結果であり、AM4基板全体の共通構成とは断定しない。目的の機器がどの配下にあるかは未確定。

有界の順次xHCI調査とハブ経路を追加する。UEFI/GOP、読出し専用BOT/FAT32、Boot Keyboard1台の入力、既存snapshot API、停止検証を維持する。USB書込み、任意class driver、hot-plug管理、SMP、複数コントローラー同時稼働は追加しない。

### M1：複数コントローラーの順次調査

ExitBootServices前にsegment 0のxHCI PCI I/O BARを最大8個収集する。scalar BAR/BDFだけを保存し、BDF順に並べ、超過・重複・不正mappingを明示する。PCI IDからBARを推測しない。NICは別の選択契約を維持する。

専用LoaderDataを4GiB未満に確保し、各コントローラーへ独立した1MiB DMA領域を割り当てる。異なるコントローラーでDMA addressを共有しない。全BARの範囲・競合を検証しUC/RW/NXでmappingしてからページテーブルを切り替える。handoffが確保した1ページに収まることを確認し、UEFI pointerをruntimeへ持ち込まない。

複数ある場合は最初に全候補を順にリセット・所有・列挙し、storage snapshotとkeyboard適合情報を得て、毎回haltとbus mastering無効化を確認する。この調査中は長時間の入力sessionやアプリへの探索完了通知を開始しない。file eventは既存アプリcacheへコピーし、最初の成功snapshotをstorage側停止後も保持する。

全調査後、成功した候補のうち最初の適合keyboardを持つコントローラーを再初期化して入力用にする。keyboardとstorageが別コントローラーでも扱える。Ready通知は探索完了後に1回だけ行う。1個の場合は従来の1回処理を維持する。keyboardがなければ探索完了を1回通知し、既存の時間制限付き無入力処理へ進む。

通常の局所的な初期化／列挙失敗はBDF別に記録して次へ進む。失敗時もDMA領域は予約したままにする。quiesce／bus mastering無効化を確認できなければ順次調査を止め、別候補を開始しない。失敗候補のpoolは再試行に使わない。入力用候補の再初期化は、成功した停止確認後に自身の領域だけを再利用する。アプリ重大エラーは調査を中断する。通常版と故障注入版を区別する。

表示は候補・調査中・結果・失敗・入力選択を区別し、機器行にcontrollerを含める。後の調査で前の候補の結果を消さない。機器の詳細表は現在の候補を表示する有界領域とし、debug logには全経過を残す。適合件数を実際の入力／読出し成功とは扱わない。

### H1/H2：ハブ配下の列挙

まず安全なparserと経路計算を実装する。controller、root port、route string、深さ、parent slot／port、速度、TT祖先を有界のrecordとして管理する。root portはrouteに含めず、4-bitの5階層までとする。深さ超過を黙って切り詰めない。初期対応はhub当たり15 port、hubを含め同時slot8個。列挙件数とDMA容量にも独立の上限を設ける。

H1はUSB2ハブを対象に、configuration／hub descriptorを検証して設定し、正しいcontext commandでHub・port数・TT情報を反映する。multi-TTのalternate interfaceを明示対応するまではsingle-TTを選ぶ。記述子に従ってport電源を入れ、power-good待機、class GET_STATUS、接続debounce、port reset、enable／reset完了待ち、定義されたchange featureだけをclearする。low/full/high速度を決め、子のroute・root port・TT contextを作ってからAddress Deviceを行う。高速hub配下の低速／full-speed機器はTT slot／portを必要とし、途中のfull-speed hubでも祖先情報を保持する。子が終了するまで親hub slotを維持する。

H2はSuperSpeed hubを別処理とし、descriptor `0x2A`、必要なhub depth／setup request、USB3のpower／reset／link状態を扱う。USB2のstatus bitを流用しない。USB2／USB3 companion hubが同じ外部socketを表していてもVID/PIDだけで統合しない。速度IDはcontrollerのSupported Protocol／PSI情報に合わせる。

EP0は有界のproducer／cycle管理に拡張する。現在の固定TRB indexでは任意数のhub port requestに対応できない。read／zero-length requestのSetup/Data/Status方向を維持し、短い応答は実転送長を検証する。W1C／change bitの再書込みや、意図しないoccupied portの再resetを防ぐ。

無制限の再帰ではなく有界queueを使う。まず起動時に存在する機器をstatus pollingで列挙し、動的attach/detachとhub interrupt endpointは後の設計とする。未対応記述子、上限、過電流、reset timeout、切断を分けて診断する。endpoint/ring状態が不確かになるtransfer／host errorでは、壊れた経路の転送を続けず当該controllerを停止する。入力中はkeyboardの上流経路も確認する。通常終了は子slotから親slotの順に無効化し、controller失敗時はhalt＋DMA無効化を行ってbufferを保持する。

### 検証と実装順

- M1：storage A／keyboard B、先頭が空、全候補keyboardなし、上限・重複・DMA分離、局所障害と停止失敗、従来の単一候補入力／file／network／故障注入をQEMUで確認する。
- hub基礎：USB2／USB3の破損・短い記述子、port上限、status bit差、5階層route、深さ超過、TT祖先、context計算をホスト試験する。
- H1：QEMUの直結／多段hub配下でkeyboard／storageを試し、モデルが対応する混在速度、切断・reset・過電流・slot／DMA上限を確認する。generic hubだけではSuperSpeed対応の証拠にならない。
- H2：適したSuperSpeedモデルまたは実機で、今回の2hubを別々に確認し、通常起動・入力・MUSHA.TXT・DMA停止を確かめる。

M1とhub基礎から開始し、H1、H2の順に統合する。検証後に段階の完了と残る制限を記録し、実機用は通常版を再生成する。USB書換えは別途の明示依頼で行う。branch変更・reset・pushを行わない。参照仕様は英語節のIntel xHCI／USB-IF資料とし、runtime統合前にfield／requestの定義を確認する。

## Implementation results / 実装結果

### English — M1 stage, 2026-10-10

The first implementation stage is in place: collect up to eight validated controllers, sort by BDF, map every retained BAR, reserve independent 1MiB DMA slices, and scan controllers sequentially before selecting a keyboard controller. Storage snapshots survive controller shutdown. The single-controller path remains a single pass. Ordinary controller failures continue discovery; unverified quiescence stops it.

Host validation passed 72 tests, including bounded controller collection, DMA separation, USB2/SuperSpeed hub descriptor parsing, port-status interpretation, five-tier routes, and TT ancestry. Normal and debug UEFI builds passed. QEMU verified storage on controller A with keyboard on B, and an empty first controller with both devices on B; file contents, key transitions, one Ready notification, and shutdown were checked. The two-controller no-keyboard case and single-controller regression also passed. Cooperative traffic and no-keyboard runs each verified ARP/ICMP, 257 UDP echoes, cached-file rechecks, ring wraps, checksum rejection and DMA shutdown. Formatting and whitespace checks passed. A normal build with no diagnostic features was generated under `out/usb-m1-normal-20261010` (64MiB QEMU image; not sized for the physical USB).

At the M1 milestone (superseded by the H1 progress below), hub helpers were implemented in `xhci/src/hub.rs`, but runtime hub enumeration was **not yet implemented**: attached hubs still report `HUB UNSUPPORTED`. H1 requires reusable EP0 rings, hub setup/status/reset requests, bounded child traversal and parent-slot lifetime management. H2 follows H1. Local-failure fallback and quiesce-failure injection tests remain pending; physical Ryzen/NUC validation remains pending. This is an implementation milestone, not a complete USB compatibility claim. No USB device was rewritten and no changes were pushed.

### 日本語 — M1段階、2026-10-10

初段階を実装した。最大8個の検証済みコントローラーをBDF順に保持し、全BARをmapping、各1MiBの独立DMA領域を確保する。順次探索の完了後にキーボード用コントローラーを選び、ストレージ停止後も取得済みファイルを保持する。単一コントローラーは従来どおり1回処理する。局所的な失敗では次へ進み、停止を確認できない場合は探索を中断する。

ホスト試験72件と通常／debug UEFIビルドが成功した。コントローラー収集・DMA分離、USB2／SuperSpeedハブ記述子、port状態、5階層route、TT祖先を試験した。QEMUではstorage A／keyboard Bと、先頭が空で両機器がBにある構成を確認し、ファイル内容・キー入力・Ready通知1回・停止を検証した。2コントローラーでキーボードなしの構成と、従来の単一構成も成功した。通信と無入力の協調試験では、それぞれARP／ICMP・257回UDP echo・cache済みfile再読出し・ring wrap・checksum拒否・DMA停止を確認した。format／差分空白検査も成功。追加診断機能なしの通常版を`out/usb-m1-normal-20261010`に生成した（64MiBのQEMU用イメージであり、実USB容量には合わせていない）。

M1段階では（下記H1進捗で更新）、`xhci/src/hub.rs`に基礎処理を追加したが、**ハブ配下のruntime列挙は未実装**で、当時は`HUB UNSUPPORTED`と表示する。次はH1のEP0 ring拡張、ハブ設定・状態確認・reset、有界の子探索、親slot寿命管理を実装し、その後H2へ進む。局所障害後の継続と停止失敗の故障注入試験、Ryzen／NUC実機確認は未完了。USB書換え・pushは行っていない。


### English — H1 milestone (superseded by H2 below), 2026-10-10

USB2 hubs now support boot-time child discovery: validated alternate-zero single-TT configuration, hub descriptors, port power and power-good delay, status/debounce/reset/change acknowledgement, child speed IDs from the root's Supported Protocol/PSI, routed Slot/EP0 contexts, and retained parent slots. A FIFO bounds total devices at 32, hub ports at 15, route depth at five and simultaneous slots at eight. Children are reset immediately before addressing, rather than leaving several devices at default address zero. Parent hubs are released in reverse traversal order after child devices and the keyboard.

EP0 now has an independent producer/cycle per slot and commits advancement after Status completion. Short Data Stage events validate pointer, slot, endpoint and residual; the final Status is still required. Exact-length descriptor/status callers reject short replies. A dedicated `usb-control-probe` feature verifies 300 repeated requests plus an 18-byte reply to a 64-byte request; normal builds exclude it. Upstream keyboard hub ports are checked every 100ms. A single pending keyboard completion can be deferred while a hub control request consumes events; errors still stop the controller. New attach/re-enumeration remains unsupported.

The panel heading is `HARDWARE H1`; USB records show controller, root port and route (`RT`). A configured USB2 hub shows `USB2 HUB / BOOT ENUMERATION`; SuperSpeed or unsupported interfaces still show `HUB UNSUPPORTED`. Matching a hub is not a child input/storage pass.

QEMU tests use four-port full-speed `usb-hub` models with port-power enabled. The eight-port model's descriptor advertises a bitmap length inconsistent with the strict USB2 parser, so it is not used to weaken descriptor validation. QEMU's Configure Endpoint updates Context Entries/Slot State without copying hub fields; input contexts use the specification's Configure Endpoint command and an allocated but unarmed interrupt status endpoint. Missing hub-field readback emits `HUB_CONTEXT_FIELDS_NOT_REPORTED`, not proof of hardware TT support. See [QEMU hub source](https://raw.githubusercontent.com/qemu/qemu/master/hw/usb/dev-hub.c) and [QEMU xHCI source](https://raw.githubusercontent.com/qemu/qemu/master/hw/usb/hcd-xhci.c). This implementation is original Rust; emulator source was consulted for model behavior, not copied.

Validation: 76 host tests passed. QEMU verified one/three/five-level hubs, keyboard press/release/Esc and 160-report ring wrap, GPT/FAT32 child storage, idle EP0 wrap, child-keyboard disconnect with DMA shutdown, 257 UDP echoes concurrent with three-level hub input, separate-controller storage/input regression, and descriptor-timeout cleanup. Normal/debug builds and formatting/whitespace checks passed. Logs are under `out/usb-h1-tests`; the normal 64MiB image and file-copy bundle are under `out/usb-h1-normal-20261010` (not physical-capacity images).

H1 is an initial boot-time implementation: physical high-speed TT traffic, low-speed children, CSZ=64, overcurrent/reset/slot-exhaustion fault matrices and Ryzen/NUC execution remain unverified. H2 SuperSpeed hubs are not implemented. No USB rewrite, branch change, commit or push was performed.

Primary command/field references: [Intel xHCI specification](https://www.intel.com/content/dam/www/public/us/en/documents/technical-specifications/extensible-host-controler-interface-usb-xhci.pdf), §4.5.2, §4.6.5, §6.2.2.2–3 and control-transfer Short Packet rules; [USB-IF USB2 specification](https://www.usb.org/document-library/usb-20-specification), chapter 11.

### 日本語 — H1段階の記録（下記H2で更新）、2026-10-10

USB2ハブ配下の起動時列挙を追加した。alternate zeroのsingle-TT構成・hub descriptorを検証し、電源投入・power-good待機・状態確認・debounce・reset・change解除を行う。子機器の速度IDをrootのSupported Protocol／PSIから決め、route／TTを含むSlot・EP0 contextを構成する。FIFOの総機器上限32、hub port上限15、route深さ5、同時slot8とする。複数の子を先にresetせず、各子をaddressする直前にresetする。子とキーボードの終了後に、親hubを逆順で解放する。

EP0をslot別producer／cycle管理へ変更し、Status完了後に進める。短いData応答はpointer・slot・endpoint・residualを検証し、最後のStatus完了も必要とする。所定の長さを必要とする記述子・status読出しは不足を拒否する。試験専用`usb-control-probe`で300回の転送と64byte要求に対する18byte応答を検証し、通常版には含めない。入力中は100msごとに上流hubのportを監視し、hub制御転送中に来たキーボード完了1件を保留して後で処理する。転送異常時はcontroller停止。新規接続の再列挙は未対応。

見出しは`HARDWARE H1`。機器行にcontroller・root port・route（`RT`）を表示する。設定成功したUSB2 hubは`USB2 HUB / BOOT ENUMERATION`、SuperSpeedなど未対応hubは`HUB UNSUPPORTED`とする。hub設定成功だけで子の入力／読出し成功とは判定しない。

QEMUは電源制御付き4-port full-speed hubを使用した。8-portモデルはbitmap長が厳密なUSB2記述子検証と合わないため、検証を緩める対応はしない。QEMUのConfigure EndpointはHub関連fieldを出力contextへ反映しないため、仕様どおりConfigure Endpointと未投入のstatus interrupt ringを構成し、読戻し不足は`HUB_CONTEXT_FIELDS_NOT_REPORTED`と記録する。実機TT対応の証明とはしない。モデル挙動の確認先は英語節のQEMU一次ソース。実装コードは独自Rustであり、エミュレーター実装のコピーは行っていない。

ホスト試験76件成功。QEMUで1／3／5段hub、キー押下・解放・Esc・160回のring wrap、hub配下GPT／FAT32、無入力中のEP0周回、子キーボード切断とDMA停止、3段hub入力と257回UDP echoの同時進行、別controllerのstorage／keyboard回帰、記述子timeout時の停止を確認した。通常／debugビルドとformat・差分空白検査が成功。ログは`out/usb-h1-tests`、通常64MiBイメージとファイル配置bundleは`out/usb-h1-normal-20261010`（実USB容量イメージではない）。

H1は起動時列挙の初期実装。高速hubのTT経由通信、low-speed機器、CSZ=64、過電流／reset／slot不足の故障試験、Ryzen／NUC実機は未検証。H2のSuperSpeed hubは未実装。USB書換え・branch変更・commit・pushは行っていない。field／commandの一次資料は英語節に記載する。


### English — H2 implementation detail, saved before coding, 2026-10-10

Start with USB3.0 / 5Gbps hubs (device class/subclass/protocol 09/00/03, bcdUSB 0300), keeping USB2 companion hubs independent. Newer USB3.1/3.2 hubs require extended port-status/rank handling before acceptance; skip them with an explicit unsupported diagnostic. This avoids interpreting Enhanced SuperSpeed as necessarily 5Gbps.

Accept a single alternate-zero hub interface, validate its IN interrupt status endpoint and immediately following SuperSpeed companion (packet 2, burst 0, attributes 0, bytes per interval 2). Support periodic or notification usage; notification interval must be 8–16. Keep the status ring allocated but unarmed while polling class status. Fetch exactly 12 bytes of hub descriptor type 2A. After Set Configuration and Configure Endpoint, issue Set Hub Depth (request 0C, class/device OUT, wValue equal to route depth 0–4). Reject reserved power-switching modes and invalid depth before sending requests.

USB3 status uses power bit 9 and link-state bits 5–8. Use a warm child-port reset (feature 28), wait for the warm-reset change bit plus reset-complete and ready U0/enabled status, then acknowledge only defined USB3 change features: connection 16, overcurrent 19, reset 20, warm-reset 29, link 25 and configuration-error 26. USB2 change features 17/18 must not leak into this path. Reserved speed encodings, port configuration errors and timeout must not become successful child records. USB2 retains its existing reset path.

QEMU 11.1.2 exposes only the full-speed usb-hub model. Test request/status/configuration policy on the host and run USB2, multi-controller, network, timeout and normal-build regressions. These tests cannot validate SuperSpeed wire transactions or high-speed TT. Physical evidence remains required; retain all existing uncommitted work and do not rewrite USB or push.

### 日本語 — H2実装詳細、実装前に保存、2026-10-10

初期対象はUSB3.0・5Gbps hub（Device 09/00/03、bcdUSB 0300）。USB2 companion hubと別々に扱う。USB3.1／3.2 hubは拡張port status／速度rankの対応まで未対応表示でskipし、Enhanced SuperSpeedを一律5Gbpsと解釈しない。

alternate zeroの単一hub interface、IN interrupt status endpointと直後のSuperSpeed companionを検証する（packet 2、burst 0、attributes 0、bytes per interval 2）。Periodic／Notificationを区別し、Notificationのintervalは8〜16。status ringは確保するが転送投入せず、class statusをpollする。hub descriptorは型2A・12byte。Set ConfigurationとConfigure Endpoint後にSet Hub Depth（要求0C、class/device OUT、wValueはroute深さ0〜4）を送る。予約済み電源切替モードと不正深さは要求前に拒否する。

USB3 statusでは電源bit 9、link状態bit 5〜8を使う。子portはwarm reset（feature 28）を行い、warm-reset change・reset-complete・U0／enabledを確認する。解除する変更featureはconnection 16、overcurrent 19、reset 20、warm-reset 29、link 25、configuration-error 26だけ。USB2用17／18を流用しない。予約速度値、port設定エラー、timeoutを成功扱いにしない。USB2の既存resetは保持する。

QEMU 11.1.2にはfull-speed usb-hubだけがある。ホスト試験で要求・状態・記述子を検証し、USB2／複数controller／通信／timeout／通常ビルドを回帰試験する。SuperSpeedの実転送と高速TTの証明にはならず、実機確認が必要。未コミット変更を保持し、USB書換え・pushは行わない。


### English — H2 initial implementation results, 2026-10-10

The normal driver now contains the USB3.0/5Gbps hub branch described above. Configuration parsing checks periodic/notification interrupt usage and the SS companion; setup reads descriptor 2A and sets hub depth after configuring the slot. Child reset and change acknowledgement use protocol-specific policy; SS requires both reset and warm-reset completion plus ready U0 status, and rejects configuration errors. USB3.1/3.2 device descriptors are skipped with `HUB USB0310 UNSUPPORTED` (actual version is displayed), allowing other candidates to continue. Supported Protocol/PSI still selects the controller's speed ID. No VID/PID-based companion merging is performed.

The panel heading is `HARDWARE H2`. A configured SS hub is explicitly labeled `SS HUB / BOOT ENUMERATION UNVERIFIED`; only actual child file/input results count as success. Initial supported generation is bcdUSB 0300 with Device 09/00/03 and Interface 09/00/00. Extended port status, faster links/rank conversion and SuperSpeed keyboards remain unsupported. The SS status endpoint stays unarmed; hotplug re-enumeration remains outside this stage.

Validation: 81 host tests passed, including USB2/SS change selectors, depth 0–4 and rejection of depth 5, warm-reset completion and U0 requirements, invalid power/speed states, SS companion/truncation/interval rejection, and device-generation filtering. QEMU USB2 five-hub input/storage/ring-wrap, three-hub concurrent ARP/ICMP and 257 UDP echoes, split-controller file/input, idle child disconnect and descriptor-timeout cleanup all passed. The SS runtime path itself has **not** been exercised: the installed QEMU exposes only a full-speed hub. High-speed TT, low-speed children and Ryzen/NUC hardware are also unverified. The normal image was booted separately in QEMU with USB2 hub input and two controllers: Esc/session completion, H2 heading, root/route records, cached 16-byte MUSHA.TXT and DMA stop were checked in the screenshot; its read-only image hash stayed unchanged. H2 is an initial implementation, not completed hardware support.

A normal build and 64MiB QEMU image are generated under `out/usb-h2-normal-20261010`, with features `[]`; logs are under `out/usb-h2-tests`. Physical-capacity image generation and USB rewriting are separate actions. Existing uncommitted work is retained; no branch change, commit or push was performed.

Primary source used for the SS requests and descriptors: USB 3.0 Promoter Group, [USB 3.2 Revision 1.1 (June 2022)](https://e2e.ti.com/cfs-file/__key/communityserver-discussions-components-files/138/USB-3.2-Revision-1.1.pdf), §9.6.6, §10.15 and §10.16. Local source extraction was used for specification review, not copied implementation code.

### 日本語 — H2初期実装の結果、2026-10-10

通常ドライバーへ上記USB3.0／5Gbps hubの分岐を追加した。Periodic／Notification interruptとSS companionを検証し、descriptor 2Aの読出しとslot設定後のhub depth要求を実装した。子resetと変更解除はprotocol別に選択する。SSはreset・warm-reset両方の完了とU0を要求し、設定エラーを成功扱いにしない。USB3.1／3.2は`HUB USB0310 UNSUPPORTED`のように実際のUSB世代を表示してskipし、別の候補の調査を続ける。速度IDは引き続きSupported Protocol／PSIを使い、VID／PIDだけでcompanion hubを統合しない。

見出しは`HARDWARE H2`。SS hub設定後も`SS HUB / BOOT ENUMERATION UNVERIFIED`と明記し、実際の子の入力・file結果を別に判定する。初期対象はbcdUSB 0300、Device 09/00/03、Interface 09/00/00。拡張port status、より高速なlink／rank変換、SuperSpeed keyboardは未対応。SS status endpointへの転送は投入せず、hotplug再列挙も対象外。

ホスト試験81件成功。USB2／SSの変更selector、深さ0〜4と5の拒否、warm-reset完了とU0条件、不正電源／速度状態、SS companion・短い記述子・interval・USB世代判定を含む。QEMUでUSB2の5段hub入力／storage／ring wrap、3段hubとARP／ICMP・257回UDP echoの同時進行、別controllerのfile／keyboard、無入力中の子切断、記述子timeout時の停止が成功した。**SS runtimeの実転送は未検証**。QEMUにfull-speed hubしかなく、高速TT、low-speed機器、Ryzen／NUC実機も未検証。通常版を別途QEMUで起動し、USB2ハブ経由入力・2コントローラー構成でEsc終了、H2見出し、root／route、16byte MUSHA.TXT、DMA停止を画面確認した。読出し専用イメージのhashは不変。H2は初期実装であり、実機対応完了ではない。

通常ビルドと64MiB QEMUイメージを`out/usb-h2-normal-20261010`に生成し、featuresは`[]`。ログは`out/usb-h2-tests`。実USB容量に合わせた生成とUSB書換えは別作業。既存未コミット変更を保持し、branch変更・commit・pushは行っていない。SS要求・記述子の一次資料は英語節のUSB3.2仕様（§9.6.6、§10.15、§10.16）。
