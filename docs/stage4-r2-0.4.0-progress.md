# Stage4 r2 runtime checkpoint —2026-10-15

## English

Status: r2 runtime calls, C/Rust wrappers and USB relative mouse are locally implemented; scope stage4 remains incomplete. Builds/tests are local development fixtures, not a formal candidate or hardware acceptance. The [first checkpoint](stage4-0.4.0-progress.md) records earlier r1 Rust execution and an unconnected proposal; this checkpoint supersedes its runtime-unconnected status. [Scope](release-scope-0.4.0.md) owner decisions are unchanged.

### Contract and lifetime

Canonical [ABI](application-abi.md): r1 revision1/minor1/features1/LOG-only NOTE is retained. r2 revision2/minor2/features3 permits LOG1/TIME2/WAIT4/INPUT8. TIME0x103, WAIT0x104, INPUT0x105 have C and Rust wrappers, checked raw error returns and generated48-byte input types. TIME is sampled monotonic PM boot milliseconds. WAIT accepts relative0–60000ms, flags0 or1 input; input wake requires both WAIT and INPUT permissions. Result0 is timeout/poll and1 input; input wins when observed with a deadline. A consecutive nonadvancing-clock watchdog returns ERR_IO. USB control work is finite but may delay a deadline by a bounded poll (each control request has a1s deadline); hard realtime is not promised. No IRQ/preemption is added; PM wraps during excessively long unsampled user execution cannot be reconstructed.

INPUT checks permissions, capacity and all48 writable supervisor aliases before polling or popping. Read-only start/text, null, overflow and cross-boundary ranges fail without modifying output or removing a queued event. Empty returns ERR_AGAIN; insufficient capacity ERR_LIMIT; invalid range/reserved args ERR_INVALID; error RDX0. It copies initialized explicit LE bytes, never casts a user pointer into a Rust reference. Queue preserves64 older events, then reports saturated drop count in an overflow record. Consumers must reset held-state assumptions; missing clicks/motion are not reconstructed. Source1/2 denotes the current keyboard/mouse session, not a persistent hardware handle. Disconnect ends the session; generation/reconnect reuse is not attempted.

The old runtime finishes file loading and stops DMA before releasing its Context/arena borrow. Only r2 with INPUT permission starts a new USB session. TIME-only/WAIT-only do not. Live Host/rings/cursors stay on OS stack; the one-step synchronous poll callback is stored as a raw context plus monomorphized function under a non-returning lifetime contract. No reference is held across callback invocation. The guest uses separately allocated user/kernel backing inside the arena; USB DMA pools remain disjoint. Terminal stop clears the loan, halts acquired controllers and clears/checks PCI BME on exit, fault, rejection or input-poll failure. A failed stop is reported unconfirmed/quarantined; stack/arena/DMA are never reclaimed. This does not promote viewer return or C3 arena reuse into the normal path.

### Mouse and remaining topology

Boot mouse configuration is parsed separately from keyboard. A bounded descriptor subset accepts only a button byte and signed8 relative X/Y, optionally wheel; IDs/absolute or unknown report layouts cannot enable wheel. Unsupported extended layouts use explicit boot protocol with wheel0. QEMU usb-mouse supplies descriptor-confirmed wheel4-byte reports; usb-tablet absolute coordinates are not converted. Viewer ignores mouse events. Keyboard-free mouse-only sessions are selected without claiming a keyboard and use the same terminal lifetime. Mouse ancestors are checked using the existing bounded hub status path.

One live input Host supports keyboard and mouse together, or mouse alone; existing storage-plus-input cross-controller ownership is retained. Simultaneous keyboard and mouse on separate input controllers (a third live controller when storage is separate) is not integrated. Generic HID/report-ID formats, gamepad radio/production and Bluetooth belong to later work. Full stage4, physical Rust/key/mouse samples on NUC5/Ryzen, release acceptance remain open. The subsequent full local regression and mixed C/Rust FP results are recorded in [local regression](stage4-local-verification-0.4.0.md).

### Evidence and regression

Evidence root: `out/stage4-r2/`; aggregated counts/hashes in `verification.json`. Related memory/platform/xHCI host tests123 pass (16+50+1+1+55); SDK Rust host2 and native C layout are separate categories. r1 and r2 SDK checkers each reject26 malformed ELFs; those runs overlap and are not52 unique malformed designs.

| QEMU case | Evidence directory |
|---|---|
| Initial C TIME/WAIT/INPUT, null/read-only/size/overflow pointer checks, A/Q down/up | `c-input` |
| C and Rust relative X17/Y−9/button/wheel and Q-release exit | `c-mouse`, `rust-mouse` |
| All new calls denied without rights | `denied` |
| User#UD with kernel-stack evidence and halt/BME-off | `fault` |
| Real USB transitions overflow the FIFO, notification delivered | `overflow` |
| Keyboard removal during WAIT gives DISCONNECTED and DMA stop | `removed` |
| TIME-only and WAIT-only, no USB restart or permission leakage | `time-only`, `wait-only` |
| Mouse without any keyboard, button-release exit and DMA stop | `mouse-only` |
| Absolute tablet coordinates produce no relative mouse event | `tablet-negative` |
| r1 C1 normal, C2 ABI/FP/copyin, feature-free normal C external ELF | `r1-c1`, `r1-c2`, `r1-normal` |

These14 QEMU scenarios are a focused set, not full regression. Early keyboard/mouse cases preceded the small mouse-only selection/watchdog refinements; later topology/capability and r1 cases exercise the final paths. Counts do not imply14 globally disjoint code paths. All observed guest/runtime checks passed; compilation caught generator quoting and a new Plan initializer during implementation, both fixed before QEMU. No new hardware/USB write, commit/push, remote CI, tag, Release or formal package candidate was performed. The development SDK archive/extracted build is separate from formal release packaging. Both relocated C r2 IO and Rust r2 Hello/start-record/IO rebuilds match repository SHA-256 byte for byte (four ELFs); source/target/core are rebuilt from paths containing spaces. Generated-definition, format and whitespace checks pass. The root README and historical hardware status are unchanged.

## 日本語

状態：r2実call・C/Rust wrapper・相対USB mouseをローカル実装済み、scope工程4全体は未完です。[初回記録](stage4-0.4.0-progress.md)のr1 Rust実行・未接続候補から進み、本書が「runtime未接続」を置換します。build/testは開発fixtureで正式候補・実機受入ではなく、[scope](release-scope-0.4.0.md)所有者決定は不変です。

### 契約と寿命

[ABI](application-abi.md)のr1 revision1/minor1/features1/LOGのみNOTEを保持。r2はrevision2/minor2/features3、LOG1/TIME2/WAIT4/INPUT8。TIME0x103・WAIT0x104・INPUT0x105にC/Rust wrapper、厳密なrawエラー結果検査、生成48byte入力型を用意。TIMEはsample済み単調PM起動ms。WAIT相対0–60000ms、flags0/入力1、入力wakeはWAITとINPUT両権限が必要です。結果timeout/poll0、input1で同時観測時input優先。時刻非進行連続watchdogはERR_IO。USB controlは有限でも1poll分（control各1秒期限）遅延し得るためhard realtimeではありません。IRQ/preemptionは追加せず、長いuser実行中に未sampleでPMが周回した時間は復元できません。

INPUTはpoll/pop前に権限・容量・全48byteの書込み可能supervisor aliasを検査。読取り専用start/text・null・overflow・境界跨ぎはoutput/queueを変更せず拒否。空ERR_AGAIN、容量ERR_LIMIT、異常範囲/予約引数ERR_INVALID、エラーRDX0。明示LE初期化済みbytesをコピーし、user pointerをRust参照へcastしません。古い64件後に飽和drop数のoverflowを通知するため、consumerは保持状態前提をリセット。欠落click/移動は復元しません。source1/2は現keyboard/mouse sessionで永続handleではなく、切断後のgeneration/再接続再利用はありません。

旧runtimeがfile読込みとDMA停止を終えContext/arena借用を返した後、r2 INPUT権限だけUSBを再開。TIME-only/WAIT-onlyは再開しません。Host/ring/cursorはOS stackに保持し、1step同期pollを非復帰寿命契約のraw context＋単相化関数として貸与。callbackを跨ぐ参照を保持せず、guest user/kernel backingはarena内、USB DMA poolは別範囲です。exit/fault/reject/poll失敗時は貸与解除・取得controller halt・PCI BME無効/確認。停止未確認は隔離扱いでstack/arena/DMAを回収しません。通常viewer復帰やC3 arena再利用は追加していません。

### mouseと残るtopology

keyboardとboot mouseを別に識別。descriptor部分形式はbutton1byte＋signed8相対XY、任意wheelだけを許可し、ID/絶対/未知形式でwheelを有効化しません。未対応拡張形式は明示boot protocolでwheel0。QEMU usb-mouseはdescriptor確認済み4byte wheel、usb-tablet絶対座標は変換しません。viewerはmouse無視。keyboard無しmouse単独Hostもkeyboardと誤報せず同じ終端寿命で動作します。mouse祖先hubは既存の上限付きstatus経路で確認します。

1個のlive入力Hostでkeyboard＋mouse、またはmouse単独を扱い、既存storage＋inputの別controller所有は保持。keyboardとmouseが別々の入力controller（storageも別なら第3live controller）の同時収集は未統合です。汎用HID/report ID、gamepad実producer/radio、Bluetoothは後続。工程4全体、NUC5/Ryzen実機Rust/key/mouse、公開受入は残件です。その後の混在C/Rust FPと全ローカル回帰は[追加検証](stage4-local-verification-0.4.0.md)に記録します。

### 証拠・回帰

out/stage4-r2/、集約verification.json参照。関連host123件（16+50+1+1+55）合格、Rust host2とnative C layoutは別分類。r1/r2 SDKは各26異常ELF拒否で、重複を含むため52固有設計ではありません。QEMUは英語表の14scenario合格。全回帰ではなく、初期keyboard/mouse後の小さいmouse単独選択/watchdog調整は後続topology/権限/r1経路で確認しています。14完全独立経路という意味ではありません。

実guest/runtime試験の不合格はなく、実装途中にgenerator quoteとPlan初期値不足をcompilerが検出し、QEMU前に修正。実機/実USB書込み・commit/push・remote CI/tag/Release/正式候補生成なし。開発SDK archive/展開buildは正式配布候補とは別です。空白付きpathへ展開したC r2 IOとRust r2 Hello/start-record/IOの計4 ELFはcore/sourceからの再buildでrepo SHA-256と完全一致。生成定義・format・差分形式検査合格。root READMEと過去実機状況は不変です。
