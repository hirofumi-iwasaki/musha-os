# USB startup timing capture

## English

### Corrected NUC5 acceptance basis (2026-10-14, independent review)

One cold-start NO MEDIUM outcome is observed in IMG_7633 (run 1, PRE OWN D0=0x300 / D8=0). One Hello / exit 0 success is observed in IMG_7634 (run 3, PRE OWN D0=0x7FF / D8=0xF already routed). Three consecutive cold-start successes are NOT proven. The owner's report of three successes is retained as an owner report, separately from video evidence. Run 2, operations after run 1, and power-off durations for runs 2/3 await owner confirmation. A restart between diagnostic hold and Hello is an interpretation, not an observed fact. The review's most likely cause is immediate empty classification after routing/PP, before attachment settles.

| Run | Evidence | PRE OWN D8 | Observed outcome | Cold acceptance |
|---|---|---|---|---|
| 1 | IMG_7633 | 0x00000000 | NO MEDIUM; diagnostic hold | Fail |
| 2 | Owner report | Unknown | Not captured | Pending |
| 3 | IMG_7634 | 0x0000000F | Hello / exit 0 | Not established |

For the next acceptance series: power off at least 10 seconds; photograph PRE OWN D8=0 for every NUC5 cold run. Record D0/D8, power-off duration, routing-to-CCS/U0 milliseconds, attachment/SS wait, WPR, rescan and Hello timing. Diagnostic hold counts as failure; any power operation begins a new run. Require three consecutive successes in the same boot per run.


NUC5 clarification and third-run evidence (2026-10-14): owner identifies IMG_7633.MOV as run 1's diagnostic loop and IMG_7634.MOV as run 3's success. The latter clip (36.967 s) directly shows MUSHA.ELF / Hello Musha-OS! / exited: status 0. At 18.483 s, PRE OWN P12 PORTSC=00001203 (CCS1 PED1 SPEED4 PLS0), D0=000007FF and D8=0000000F; ownership elapsed 0 ms. In run 1, captured ROOT12 is disconnected and DIAGNOSTIC HOLD NO MEDIUM repeats. Clip-to-run mapping is now resolved. Owner reports eventual Hello success for all three; run 1's transition from terminal hold to Hello remains unexplained by current code and is not captured. Do not classify these as three clean, uniform cold starts or dismiss the loop as cosmetic. Delayed connection after routing is a hypothesis, not a proven cause. Both clips are associated with the preceding f10672d4 diagnostic deployment. No binary/USB changes made for this observation.


Owner clarification: Ready means Hello Musha-OS! / exited: status 0. Three cold-boot Hello successes are now owner-confirmed. The association between the clip's NO MEDIUM run and those three successes remains unresolved; the clip is not direct evidence of Hello. Diagnostic hold does not retry USB, so progression from that hold to Hello within one boot is unexplained.


NUC5 IMG_7633.MOV (2026-10-14): owner reports cold 3/3 successful, with two page cycles on runs 1/2 and one on run 3; meaning of Ready/success awaits clarification. Video (44.253 s) shows DIAGNOSTIC HOLD NO MEDIUM, SCANNED K0 S0, USB STEP STOPPED DMA DISABLED and repeated 9-page diagnostics, not Hello/exit. ROOT 12-15 are CCS0/PED0/PLS5 at enumeration; ownership BIOS1/OS0 to BIOS0/OS1 completes in 0 ms. This is a captured no-medium outcome, so three Hello successes are NOT accepted. Associated EFI f10672d4a89bf9d9c56206ff24cc4f885f6b0c2df13e4c43832aa0c4c1e5ab20 follows preceding deployment, not a video hash measurement. Code diagnostic_capture calls diagnostic_pages(false) after no external app, repeating saved pages without USB retries. Suspect immediate empty classification after Intel routing may miss delayed attachment; causality remains unproven. Preserve this failure, do not promote the optimized normal candidate. Evidence: out/nuc5-7633/observation.json and sample-3/6/7.png.


### Ryzen 7: optimized diagnostic cold 3/3 and hot 1/1 (2026-10-14)

Owner reports three successful cold boots and one successful hot reboot with IMG_7632.MOV. Associated diagnostic EFI: f10672d4a89bf9d9c56206ff24cc4f885f6b0c2df13e4c43832aa0c4c1e5ab20, via the immediately preceding out/usb-empty/physical-usb/provenance.json deployment; not a hash measured from the video. The 7.385-second clip shows starting, runtime, USB READ OK (sample at 3.6925 s), GOP 3840/2160/3840, XHCI 5, and diagnostic page 1/23 at 6.885 s with C1 INPUT DONE K1 S1. It does not include the final Hello/exit screen or PM EPOCH TIMINGS pages. Repetition counts and success are owner-reported; no post-change READ OK-to-Hello duration or controller timing can be measured from this clip. Evidence: out/ryzen-7632/observation.json and frames. Ryzen diagnostic repetition check passes by owner report; NUC5 optimized-diagnostic cold 3/3 and optimized normal-candidate acceptance remain pending.


The initial 444186ec revision was measurement-only; IMG_7630.mov now supplies the measurements below. The subsequent empty-port optimization is locally implemented and awaits hardware retesting. Do not compare its total time to Hello with the normal candidate: diagnostic pages add two seconds per page. Compare the PM-epoch stage records instead. The normal candidate baseline is Ryzen USB READ OK to Hello approximately 18 seconds (owner report, IMG_7628 2.mov); NUC5 is reported shorter.

Diagnostic EFI: `444186ec095a58818f0d8f6d0491ce1ceb7918389620509f1b2690dbc1eb51c8`, `out/usb-timing/BOOTX64.EFI`; provenance manifest: `out/usb-timing/manifest.json`. Deployed to the dedicated SanDisk USB on 2026-10-14 JST: EFI-only update, backup, matching readback SHA-256 and safe eject. MUSHA.ELF and Basic Data partition unchanged. Provenance: out/usb-timing/physical-usb/provenance.json. Hardware timing capture was completed with IMG_7630.mov below. The previous normal EFI is retained in its original build directory.

Each TIME row identifies the controller BDF and phase, followed by END and elapsed MS in the same shared PM epoch. Some(n) is a valid millisecond value; None means timing unavailable, never zero. FFFF denotes app ELF I/O. DISCOVERY versus LIVE separates initial probing from selected-controller reinitialization. Nested spans overlap; do not sum DISCOVERY/LIVE totals and their constituent phases.

Recorded phases: OWNERSHIP, INITIAL HALT, RESET, ATTACH WAIT, SS WAIT, ENUMERATE, QUIESCE, ELF READ. ENUMERATE excludes the initial attach/SS waits and ends before live file/input polling. ELF READ covers the reader operation, including file discovery and read callbacks. WPR COUNT is per enumeration, including zero. Existing per-port WPR records retain the port attribution. BUDGET means the connection wait reached its bound, not necessarily a device failure. EARLY RETURN identifies a phase aborted by an error; correlate with USB ERROR and WAIT TIMEOUT's register offset, mask, target and sampled value. Ownership/reset bounds remain unchanged. Separate retained timing pages are capped at 256 rows and explicitly mark overflow.

### Local validation

Host: 269 passed. QEMU: 55 successful cases across the recorded runs: normal Basic Data 26; ESP 23 plus three isolated remaining/retry cases; diagnostic five-hub Hello, split-controller Hello, and the exact physical-capture diagnostic EFI Hello. Normal absent pixels match out/release-final-d/v030-pixels; debug output is zero. Timing checker validates 42 phase pairs in hub/split runs. Evidence: out/usb-timing/validation.json.

The first host harness lacked the timing module and was repaired. The first diagnostic QEMU attempts incorrectly required network shutdown even though diagnostics-detail exits before network startup; the explicit diagnostic-timing test mode now checks this distinction while preserving ordinary test assertions. ESP timeout initially displayed Device session did not complete instead of the expected read-timeout message; the isolated retry passed. Cause is unconfirmed; original failure is retained, not described as a clean first-pass suite.

Five-hub QEMU sample: attach 1000 ms, SS wait 2000 ms, enumeration 959 ms, ELF read 55 ms, quiesce 1 ms. These are QEMU measurements, not Ryzen results. Real-hardware stage table and optimized before/after timings remain pending.

### Ryzen measured baseline: IMG_7630.mov (2026-10-14)

The owner-supplied video shows timing pages 21–23 and successful Hello / exit 0. Associated diagnostic EFI 444186ec095a58818f0d8f6d0491ce1ceb7918389620509f1b2690dbc1eb51c8 via the preceding USB deployment. One observed execution; no new repeated-boot or cold/hot claim. Evidence: out/ryzen-7630/observation.json, detail-9.png, detail-10.png and last-0/1.png.

All times below are milliseconds; WPR is a count. Totals include unlisted work and must not be added to their constituent phases.

| BDF | Mode | Ownership | Halt | Reset | Attach | SS | WPR | Enum | Quiesce | Total |
|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| 0801 | DISCOVERY | 0 | 0 | 0 | 1000 | 2000 | 0 | 1323 | 0 | 4359 |
| 0600 | DISCOVERY | 1 | 0 | 109 | 999 | 2000 | 0 | 628 | 6 | 3864 |
| 0803 | DISCOVERY | 0 | 0 | 0 | 999 | 2000 | 0 | 173 | 2 | 3188 |
| 0D02 | DISCOVERY | 0 | 0 | 1 | 999 | 2000 | 0 | 0 | 0 | 3012 |
| 0F03 | DISCOVERY | 0 | 0 | 0 | 1000 | 2000 | 0 | 0 | 0 | 3011 |
| 0801 | LIVE | 0 | 0 | 0 | 999 | 2000 | 0 | 1324 | 0 | 4552 |

ELF READ: PM epoch 23548–23731, 183 ms. All WPR counts are zero. Ownership 0–1 ms, reset 0–109 ms, quiesce 0–6 ms: none reached its timeout. Attach plus SS waits total 17996 ms across six passes. This identifies repeated connection waits as the dominant measured delay, not quiesce or ELF I/O. It does not mean all 17996 ms occurred after the first USB READ OK. The video includes diagnostic-page display delays; do not use its 73-second duration as normal startup time. The ordinary journal ends with JOURNAL LIMIT, but the separate timing pages remain available.

Implemented after measurement: CCS=0/PLS=5 ports return Empty without waiting; attach samples before sleeping; WPR requires CCS=1. Other link states retain bounded waits, and USB2 debounce plus ownership/reset/stop limits are unchanged. An empty port can still be sampled while another port is pending. Earlier cold-boot passes are not acceptance for this change: NUC5 cold 3/3 and Ryzen cold 3/3 plus hot 1/1, and the normal READ OK-to-Hello after-time, remain pending.

## Original capture procedure (completed for IMG_7630.mov)

1. Run this diagnostic EFI on Ryzen with the same devices/ports used for the accepted normal candidate.
2. Capture the PM EPOCH TIMINGS pages through the end; preserve the preceding failure/PORTSC pages if any.
3. Tabulate all five BDFs, DISCOVERY/LIVE, ownership/reset/attach/SS/enumeration/quiesce time and WPR count; list ELF READ separately.
4. Ryzen measurements are now recorded above; QEMU values remain separate.
5. Only after those measurements, decide the empty CCS=0/RxDetect policy and CCS=1-only WPR change. Keep per-controller bounds and USB2 debounce; do not lower ownership/reset/stop deadlines without evidence.
6. After any behavioral change, run QEMU including five hubs, then NUC5 cold 3/3 and Ryzen cold 3/3 plus hot 1/1 with the new normal candidate. Record its hash and before/after READ OK to Hello seconds.

The original measurement-only step did not change attachment policy; the subsequent measured optimization is described above. Previous acceptance results apply only to the previous EFI.

### Empty-port optimization: local validation

Host: 270 passed. QEMU: 56 successful cases (normal Basic Data 23 plus three isolated remaining/retry cases; ESP 26; five-hub, split-controller, extra-empty-controller and exact diagnostic EFI Hello). Both partition types preserve absent-case pixels against out/release-final-d/v030-pixels and normal debug output remains empty. The 67 instrumented phase pairs pass the timing consistency check. QEMU attach/SS waits are 0–1 ms; the extra empty controller has both waits and enumeration at 0 ms. This is not a Ryzen after-measurement.

The Basic Data timeout case initially showed Device session did not complete instead of File read deadline exceeded; isolated retry passed. Cause remains unresolved, as with the preceding measurement build's intermittent timeout test. The failed report is retained. An initial retry command omitted the required firmware directory and was corrected before running tests.

Evidence: out/usb-empty/validation.json and manifest.json. Diagnostic EFI SHA-256: f10672d4a89bf9d9c56206ff24cc4f885f6b0c2df13e4c43832aa0c4c1e5ab20. Normal EFI SHA-256: 64cc3110266ea887718ef6c1e30d41cf3a66e458707f06703a3f97073c3da6d3. The diagnostic EFI f10672d4 was deployed on 2026-10-14 JST: EFI-only update, verified readback, Hello unchanged, Basic Data unchanged and safe eject. Evidence: out/usb-empty/physical-usb/provenance.json. Hardware acceptance remains pending; the normal EFI has not been deployed.

## 日本語

### NUC5受入記録の訂正（2026-10-14、独立レビュー反映）

冷間起動でNO MEDIUMを1回観測（IMG_7633、run 1、PRE OWN D0=0x300／D8=0）。Hello／終了0成功はD0=0x7FF／D8=0xF切替済み状態で1回観測（IMG_7634、run 3）。冷間3回連続成功は未証明。3回成功の所有者報告は報告として保持し、映像の事実と区別する。run 2、run 1後の操作、run 2／3の電源断時間は所有者確認待ち。診断ホールド後に再起動が挟まった可能性は解釈であり、観測事実ではない。最有力原因はルーティング／PP直後の即時空き判定。

| run | 証拠 | PRE OWN D8 | 映像結果 | 冷間受入 |
|---|---|---|---|---|
| 1 | IMG_7633 | 0x00000000 | NO MEDIUM・診断停止 | 失敗 |
| 2 | 所有者報告 | 不明 | 未収録 | 確認待ち |
| 3 | IMG_7634 | 0x0000000F | Hello・終了0 | 冷間条件未証明 |

次回は電源断10秒以上、各NUC5 runでPRE OWN D8=0の写真を残す。D0/D8・電源断時間・ルーティングからCCS/U0までのms・接続／SS待ち・WPR・再探索・Hello到達時間を記録。ホールドは失敗、電源操作後は別run。同じ起動内での成功が3回連続して初めて合格とする。


NUC5追記（2026-10-14）：所有者よりIMG_7633.MOVは1回目のループ、IMG_7634.MOVは3回目の成功と確認。後者36.967秒の動画でMUSHA.ELF／Hello Musha-OS!／exited: status 0を直接確認。18.483秒のPRE OWN P12 PORTSCは00001203（CCS1 PED1 SPEED4 PLS0）、D0=000007FF、D8=0000000F、所有権移行0ms。1回目ではROOT12未接続とDIAGNOSTIC HOLD NO MEDIUMの反復を確認。動画と試行番号の対応は解決済み。3回とも最終的にHello成功との所有者報告は保持するが、1回目の診断停止からHelloに至る遷移は現行コードで説明できず未収録のため、安定した冷間3回成功とは判定しない。表示だけの問題とせず、ルーティング後の接続遅延を仮説として残す。両動画は直前配備の診断版f10672d4へ関連付け。今回バイナリー・USB変更なし。


追記（所有者回答）：ReadyはHello Musha-OS!／exited: status 0を指すと確認。冷間3回すべてのHello成功は所有者確認済みとして記録する。一方、添付動画で確認したNO MEDIUMの回と、その3回との対応は未確定。映像を成功の直接証拠とはしない。現行コードの診断停止はUSB再試行を行わないため、反復後に同じ起動でHelloへ進んだという解釈は未解決。


NUC5 IMG_7633.MOV（2026-10-14）：所有者報告は冷間3回成功、1・2回目は2周、3回目は1周でReady。ただし成功画面の意味を確認中。44.253秒の動画ではDIAGNOSTIC HOLD NO MEDIUM、SCANNED K0 S0、STOPPED DMA DISABLEDと9ページの反復を確認し、Hello／終了画面は未確認。列挙時ROOT 12～15はCCS0/PED0/PLS5、所有権移行はBIOS1/OS0→BIOS0/OS1・0ms。媒体未検出の証拠があるためHello冷間3回合格にはしない。直前配備の診断EFI f10672d4…へ関連付け（動画からハッシュ測定したものではない）。外部アプリ未検出時のdiagnostic_capture→diagnostic_pages(false)は保存ページを反復表示するだけで、USB再試行は行わない。Intelルーティング直後の即時空き判定が遅れて現れる接続を逃した可能性はあるが、原因確定ではない。新通常版の受入移行は保留。証拠：out/nuc5-7633/observation.json、sample-3/6/7.png。


### Ryzen 7：待機省略後の診断版、冷間3/3・ホット1/1成功（2026-10-14）

所有者よりIMG_7632.MOVとともに冷間起動3回・ホットリブート1回の成功報告。直前のout/usb-empty/physical-usb/provenance.jsonにより診断EFI f10672d4a89bf9d9c56206ff24cc4f885f6b0c2df13e4c43832aa0c4c1e5ab20へ関連付け（動画からハッシュを測定したものではない）。動画は7.385秒で、starting、runtime、3.6925秒時点のUSB READ OK、GOP 3840/2160/3840、XHCI 5、6.885秒時点の診断1/23・C1 INPUT DONE K1 S1まで確認。Hello／終了画面とPM EPOCH TIMINGSは収録されていません。反復回数・成功は所有者申告として採用し、変更後READ OK→Helloの秒数・制御器別時間は未確認。証拠はout/ryzen-7632/observation.jsonと抽出画像。Ryzenの診断版反復確認は所有者報告により合格。NUC5の新診断版冷間3回、新通常版の受入は未完了です。


初回444186ec版は計測専用版でした。IMG_7630.movの実測を下記へ記録し、その結果に基づく空きポート待機省略を実装済みです。変更後の実機再試験は未実施です。診断ページは1ページ2秒を追加するため、Helloまでの総時間を通常版と直接比較しないでください。同一PM epochの段階別記録で比較します。通常版の基準はRyzenのUSB READ OKからHelloまで約18秒（所有者報告、IMG_7628 2.mov）。NUC5は短いとの報告です。

診断EFI：`444186ec095a58818f0d8f6d0491ce1ceb7918389620509f1b2690dbc1eb51c8`、`out/usb-timing/BOOTX64.EFI`。来歴は`out/usb-timing/manifest.json`。2026-10-14（日本時間）に専用SanDiskへEFIのみ更新し、バックアップ・読み戻しハッシュ一致・安全な取り外しを完了。MUSHA.ELFとBasic Data構成は保持。配備記録はout/usb-timing/physical-usb/provenance.json。実機時間は下記IMG_7630.movで採取済みです。前通常版EFIは元の出力先へ保存しています。

TIME行のBDF・段階・開始時刻と、次のEND・所要MSを読みます。Some(n)は有効なミリ秒、Noneは取得不能で0ではありません。FFFFはアプリのELF読込みです。DISCOVERYとLIVEで初回探索と選択制御器の再初期化を区別します。入れ子の所要時間は重複するため、全体時間と内訳を足し合わせないでください。

記録段階は所有権移行、初期停止、リセット、接続待ち、SuperSpeed待ち、列挙、quiesce、ELF読込み。列挙は接続待ち終了からライブI/O開始前まで、ELF READはファイル探索・読込みコールバックを含むReader操作です。WPR COUNTは列挙ごとの合計（0も記録）、ポート別情報は既存ログへ残します。BUDGETは接続待ち期限到達で、必ずしも機器障害ではありません。EARLY RETURNは途中エラーで、USB ERRORとWAIT TIMEOUTのレジスター・マスク・期待値・実値を照合します。所有権移行・リセットの期限は変更していません。計測専用ページは256行上限、超過時は明示します。

### 初回の採取手順（IMG_7630.movで採取済み）

1. Ryzenで受入試験と同じ接続構成を使い、この診断版を起動します。
2. PM EPOCH TIMINGSの全ページを記録。失敗時はPORTSC・エラーのページも残します。
3. 5制御器のBDFごとにDISCOVERY／LIVE、所有権移行・リセット・接続待ち・SS待ち・列挙・停止・WPR回数を表にし、ELF読込みは別記します。
4. Ryzen実測は下記へ記録済み。QEMUの数値とは区別します。
5. 実測後にCCS=0／RxDetectの即時空き判定とCCS=1限定WPRを判断。制御器単位の上限・USB2デバウンスは保持し、根拠なく所有権移行・リセット・停止の期限を短縮しません。
6. 動作変更後は5段ハブを含むQEMU回帰、新通常版でNUC5冷間3回・Ryzen冷間3回＋ホット1回。ハッシュとREAD OKからHelloまでの変更前後の秒数を記録します。

初回計測版では接続待ち動作を変更していません。採取後の変更は下記のとおりです。前版の実機受入結果を新EFIへ転記しません。

### ローカル検証

ホスト269件合格。QEMUは記録された試行で計55件合格：通常版Basic Data 26件、ESP 23件＋残り／再試験3件、診断版5段ハブHello・複数制御器Hello・実機採取用EFIそのもののHello各1件。通常版の不在時ピクセルはout/release-final-d/v030-pixelsと一致し、debug出力0 byte。ハブ／複数制御器の42区間で計測時刻整合を確認。証拠はout/usb-timing/validation.json。

初回ホスト試験は計測モジュール参照不足を修正。初回診断QEMU試験は、diagnostics-detailがネットワーク開始前に終了するにもかかわらず停止ログを要求していたため、専用のdiagnostic-timing検証モードを追加して再試験しました。通常試験の検証条件は維持。ESPの期限超過ケースは初回に期待文言ではなくDevice session did not completeを表示し不合格、単独再試験で合格しました。原因未確定で、初回の失敗記録は保持しています。

5段ハブQEMUの例：接続待ち1000ms、SS待ち2000ms、列挙959ms、ELF読込み55ms、停止1ms。Ryzenの実測ではありません。実機の制御器別表は下記。短縮後の時間は採取待ちです。


### Ryzen実測：IMG_7630.mov（2026-10-14）

計測ページ21～23とHello・終了0を確認。直前のUSB配備により診断EFI 444186ec…へ関連付け。今回は1回の実行を確認したもので、新たな反復回数・冷間／ホットの条件は申告されていません。証拠はout/ryzen-7630/observation.json、detail-9.png、detail-10.png、last-0/1.png。

上の表はミリ秒（WPRのみ回数）。DISCOVERYは順次探索、LIVEは使用制御器の再初期化です。全体時間には表外の処理も含み、内訳と合算しません。ELF読込みはPM時刻23548～23731の183ms。WPRは全て0回。所有権移行0～1ms、リセット0～109ms、停止0～6msで上限到達はありません。6回の探索／再初期化で接続待ち＋SS待ちが計17996msとなり、主要な遅延はこの待機累積でした。17996ms全てが最初のUSB READ OK後という意味ではありません。動画の73秒には診断ページ巡回時間も含むため、通常版起動時間には使いません。通常ログはJOURNAL LIMITへ達していますが、独立した計測ページは保存されています。

実測後、CCS=0・PLS=5はEmptyとして待機を省略、接続待ちをスリープ前判定へ変更し、WPRをCCS=1の場合に限定しました。他の状態の待機上限・USB2デバウンス・所有権移行／リセット／停止の期限は保持。他ポートが待機中なら空きポートも再サンプルされます。変更後のNUC5冷間3回・Ryzen冷間3回＋ホット1回と、通常版READ OK→Helloの短縮後実測は未実施です。

### 空きポート待機省略版のローカル検証

ホスト270件合格。QEMUは計56件成功（通常版Basic Data 23件＋残り／再試験3件、ESP 26件、5段ハブ・複数制御器・空き制御器追加・実機採取用診断EFIのHello各1件）。両種別の不在時ピクセルはout/release-final-d/v030-pixelsと一致、通常版debug出力0 byte。計測67区間の時刻整合も合格。接続待ち／SS待ちはQEMUで0～1ms、空き制御器では両待機と列挙が0ms。Ryzen変更後の実測ではありません。

Basic Data期限超過ケースは初回にDevice session did not completeとなり、期待するFile read deadline exceededと不一致。単独再試験は合格しましたが原因未確定で、前計測版でも発生した間欠的な試験不一致として失敗記録を保持。再試験コマンドの初回は必須firmware-dir指定不足で実行前に失敗し、補って実行しました。

証拠はout/usb-empty/validation.jsonとmanifest.json。診断EFIはf10672d4…、通常版は64cc3110…（完全なSHA-256は英語節とmanifest）。診断EFI f10672d4版を2026-10-14（日本時間）にUSBへ差分配備済み。読み戻し一致、Hello・Basic Data保持、安全な取り外しまで確認。記録はout/usb-empty/physical-usb/provenance.json。実機受入は未実施で、通常版EFIは未配備です。
