# Single boot-console implementation — v0.4.0 results

## English

Status: local implementation and regression record, 2026-10-16. No physical acceptance, publication, remote CI, USB write or push is implied. Contract: [boot log](boot-log.md); development EFI hashes: [acceptance worksheet](hardware-acceptance-0.4.0.md).

The old panels, headings, left column, PM footer, colored failure band and separate 80×24 application console were removed. One full-GOP white/black console serves the kernel and unchanged `SYS_LOG` bytes; the ABI and 4,096-byte limit are unchanged. Startup binding paints nothing. Cells and CPU-shadow scrolling repaint only the text they own. A 65,536-byte reserved-image ring retains whole records at every level, including pre-EBS settings/GOP decisions. Existing PAT WC entries may be used for GOP only; absent WC or disabled caches explicitly fall back to strong UC without changing PAT/MTRRs. Store batches and viewer ownership transitions issue SFENCE.

### Verification and counting

The selected matrix below contains **296 case configurations**, with semantic overlap between suites. It counts a C3/return configuration once even when that configuration executes three launches. It does not add builds, observer retries, repeated startup cases, pixel comparisons or native restart iterations to the QEMU count. Original failed attempts remain in their directories; replacement evidence below determines final status. Final status: **296/296 selected case configurations passed**, with the additional diagnostics-detail large-file limitation below.

| Matrix / 範囲 | Cases / 件数 | Local evidence / ローカル証拠 |
|---|---:|---|
| CI workflow matrix | 54 | `out/bootlog-regression/final-ci/ci.json` |
| External ELF debug | 26 | `out/bootlog-regression/external-{debug-stable,debug-retry}/report.json` |
| External ELF normal | 25 | `out/bootlog-regression/external-{normal-stable,normal-retry}/report.json` |
| C1 / C2 / C3 | 42 | `out/bootlog-privilege-regression/verification.json` |
| Startup configuration / four levels | 16 | `out/bootlog-startup-regression/verification.json` |
| Diagnostic safe stop | 3 | `out/bootlog-independent/diagnostic-stop-final2/report.json` |
| SETTINGS disposable-image writer | 15 | `out/bootlog-regression/write.json` |
| C/Rust r2 input | 11 | `out/bootlog-regression/r2-final.json` |
| Bluetooth software / OUT fixtures | 10 | `out/bootlog-regression/stage6.json` |
| USB / LAN variants | 13 | `out/bootlog-regression/variants.json` |
| Basic Data / ESP partitions | 38 | `out/bootlog-regression/partition/report.json` |
| Display / hold / address / hotplug | 26 | `out/bootlog-regression/display-extra.json` |
| Rust / return / absent / quarantine | 14 | `out/bootlog-regression/last-groups.json` |
| ELF timing observations | 3 | `out/bootlog-regression/timing-base-final.json` |

Final host workspace: **428 passed, zero failed/ignored**, 41 test-result groups, with `musha-platform/bluetooth-security`; the firmware-only boot binary is excluded from the native target. Evidence: `out/bootlog-final-startup5/verification.json`. The earlier 433 total included five tests for the removed obsolete 80×24 helper; the final 428 contains the replacement console tests. Python: **57 tests in eight suites**, plus fresh SDK checks, native DHCP packets, and 1,000 native lwIP teardown/reinitialize iterations (`out/bootlog-regression/tools.json`). Extracted C/Rust SDK archives build in a path containing spaces; all four Rust ELF bytes match their fresh builds (`relocated/result.json`). Fresh C10/Rust4 identities are also recorded. These are host checks, not hardware results.

The final-source repeated subset includes quiet/ready/phases/devices/quiet-failure (5), r2 (11), normal app return/pixels, media lifetime rejection/recovery, and fatal exceptions (3), all passed. Earlier unaffected matrix binaries retain their own source/EFI hashes; they are not represented as one byte-identical build. See `out/bootlog-regression/final-ci/*-source.json`, `out/bootlog-final-startup5/verification.json`, and the final aggregate. Removing the unused old console module was separately proven byte-identical for an external-app EFI (`helper-removal.json`).

### Failures, observer migration and final boundaries

- Real defect fixed: `files=Stopped` after confirmed USB shutdown was mistaken for never having found boot media. A separate atomic observation is now retained only after successful marker verification or explicit ambiguous-AUTORUN rejection; it retains no file handle, DMA buffer or media generation. A missing/invalid marker still cannot claim readable media. Duplicate ELF names remain unexecutable; the rejection may return to the viewer.
- Fatal logging uses the last cached PM time rather than reentering a clock borrow. A reentrant console fault cannot obtain a second mutable console reference. Recoverable Ring 3 faults append the kernel outcome before rollback; viewer frames are fenced before ownership transfers.
- Old panel/literal/footer expectations were replaced with the new sequential transcript. C3 receives a valid read-only FAT32 fixture so that the new required-media rule is exercised rather than bypassed. r2 mouse values remain exact (DX 17, DY -9, buttons and wheel); its observer separates primary log records from the legacy raw debug channel instead of relaxing values.
- Hotplug at 200/800 ms can arrive during the initial bounded attach wait. The observer distinguishes that verified read path from at most one bounded rescan; reset counts, read-before-execution, read-only hashes and safe DMA stop remain required. Address-recovery kept its real failure/next-device ordering and only changed the displayed app name.
- The 4K baseline was initially a firmware fallback to 2048×2048; it is retained as such. A separate VGA-64MiB fixture proves actual 3840×2160. Independent renderer comparisons at 1024×768, 1600×1200 and actual 3840×2160 match every pixel using real preconsole baselines plus the known ready line and all 4,096 application bytes (`console-pixels.json`). Startup ready and quiet failure have separate independent full-pixel comparisons. Four-level tests compare primary and retained ring text with only variable timestamps normalized.
- The cooperative no-keyboard run initially timed out under concurrent host load. Its final isolated run checks the same 257 UDP echoes, ARP/ICMP, file rereads, strict ring wraps/checksums and DMA stop. No packet/wrap threshold was reduced.
- Existing ELF timing observations use the original feature configuration. A 5,000-ms reader-deadline rejection for a large payload is an observed bounded result, not successful execution or a throughput claim (also present before this change). The extra `diagnostics-detail` configuration passed 16KiB but hit USB transfer timeout at 1MiB/16MiB; that additional combination remains a known limitation. It is not hidden by the baseline matrix or fixed by increasing deadlines.
- Final exception-runner preparation first used an invalid option (no VM), then omitted the private EFI and reached no runtime marker using a stale shared staging area. These were invocation errors; the separately built, explicitly selected three fault EFIs provide the evidence. Other early missing-fixture/old-marker observer failures remain recorded and were not treated as firmware successes.

Normal final EFI: `out/bootlog-regression/external-normal-retry/target/x86_64-unknown-uefi/release/musha-boot.efi`, SHA256 `f9372997aeed48120f7217c7b13bf08583d0841321b6f7c38d89ff85dc505e06`. Normal pixel runs require an empty debug port. The three development feature EFIs have separate hashes/features/source provenance in `out/bootlog-development-20261016/registry.json`; they are not this normal binary or formal candidates.

No physical WC measurement, NUC5/Ryzen log photograph, Realtek TxConfig value, physical I218/NVM/link/DHCP/UDP result or real Bluetooth pairing/reconnect compatibility is established. Existing I218 safety refusals and opt-in policy remain. Owner documents for v0.5.0/no-shell/scenario and newly added POSIX notes were preserved outside this implementation; concurrent owner changes were recorded rather than reverted.

## 日本語

状態：2026-10-16のローカル実装・回帰記録です。実機受入・公開・remote CI・USB書込み・pushは行っていません。[boot log仕様](boot-log.md)、[開発EFI受入表](hardware-acceptance-0.4.0.md)を参照してください。

旧panel／見出し／左列／PM下端行／色付き例外帯／独立80×24アプリconsoleを撤去しました。kernelと変更しないSYS_LOG byteを全GOPの白黒consoleで表示し、ABIと4,096byte上限は維持します。起動時bindは描画せず、cellとCPU shadowのscroll領域だけ再描画します。予約image内65,536byte ringに全レベルとEBS前の設定・GOP判断をレコード単位で保持します。GOPだけ既存PAT WCを使えますが、WC無し又はcache無効時は理由付きstrong UCへfallbackし、PAT/MTRRは変更しません。描画batchとviewer所有移行でSFENCEします。

### 検証と件数

次表は**296ケース構成**であり、suite間の意味上の重複を含みます。C3やreturnの1構成内3launchを3ケースには加算しません。build、観測器再試験、startup再試験、pixel比較、native再初期化反復もQEMU件数へ加算しません。失敗した初回証拠は保持し、下記再試験で最終合否を判定します。最終状態：**選択した296／296ケース構成が合格**。追加diagnostics-detail大payloadの制約は下記に分けて記録します。

| Matrix / 範囲 | Cases / 件数 | Local evidence / ローカル証拠 |
|---|---:|---|
| CI workflow matrix | 54 | `out/bootlog-regression/final-ci/ci.json` |
| External ELF debug | 26 | `out/bootlog-regression/external-{debug-stable,debug-retry}/report.json` |
| External ELF normal | 25 | `out/bootlog-regression/external-{normal-stable,normal-retry}/report.json` |
| C1 / C2 / C3 | 42 | `out/bootlog-privilege-regression/verification.json` |
| Startup configuration / four levels | 16 | `out/bootlog-startup-regression/verification.json` |
| Diagnostic safe stop | 3 | `out/bootlog-independent/diagnostic-stop-final2/report.json` |
| SETTINGS disposable-image writer | 15 | `out/bootlog-regression/write.json` |
| C/Rust r2 input | 11 | `out/bootlog-regression/r2-final.json` |
| Bluetooth software / OUT fixtures | 10 | `out/bootlog-regression/stage6.json` |
| USB / LAN variants | 13 | `out/bootlog-regression/variants.json` |
| Basic Data / ESP partitions | 38 | `out/bootlog-regression/partition/report.json` |
| Display / hold / address / hotplug | 26 | `out/bootlog-regression/display-extra.json` |
| Rust / return / absent / quarantine | 14 | `out/bootlog-regression/last-groups.json` |
| ELF timing observations | 3 | `out/bootlog-regression/timing-base-final.json` |

最終host workspaceは**428合格、失敗／ignore 0**、41結果群で、bluetooth-security有効、firmware専用bootはnative対象外です。証拠は `out/bootlog-final-startup5/verification.json`。先の433件には撤去した旧80×24helperの5試験が含まれ、最終428件には新console試験を含みます。Pythonは**8suite・57件**、別途SDK fresh、native DHCP packet、lwIP停止→再初期化1,000反復が合格しました。空白を含む展開先でもC/Rust SDKをbuildし、Rust4 ELF全byte一致、fresh C10/Rust4一致も確認しました。いずれもhost試験です。

最終sourceでは4レベル＋quiet失敗の5件、r2の11件、通常app復帰／pixel、媒体寿命の拒否／復帰、fatal3件も再確認し、すべて合格しました。非影響の先行matrixは各source／EFI hashを保持し、全件が1つの同一binaryとは主張しません。旧console module撤去前後はexternal-app EFIのbyte一致を別途確認しました。

### 失敗、観測移行、残る境界

- 実装不具合を修正：USB正常停止後のfiles=Stoppedを媒体未検出と誤認していました。marker検証成功又は曖昧AUTORUNの明示拒否後だけ媒体観測atomicを保持します。file handle／DMA／media generationの寿命は延ばしません。marker欠落／不正は成功扱いせず、重複ELFも実行せずviewerへ拒否復帰します。
- fatal時刻はPM clockの借用へ再入せず直前cacheを使います。console再入時も二重mutable参照を作りません。Ring3回復faultはrollback前にkernel結果を記録し、viewer描画完了をfenceして所有を渡します。
- 旧panel／literal／footer期待を逐次ログへ移行し、C3には必須媒体条件を満たすread-only FAT32を接続しました。r2 mouseはDX17／DY-9／button／wheelの厳密値を維持し、primary LOGと旧raw debugの二重channelだけ分離します。
- 200／800ms hotplugは初回bounded attach wait中に認識される場合があるため、その実読取り経路と最大1回のbounded rescanを区別します。reset回数・実行前read・媒体hash不変・DMA停止は維持します。address-recoveryも実拒否→次deviceの順序を保ち、表示app名だけ更新しました。
- 最初の4K指定は実際2048×2048へのfirmware fallbackでした。これを保持し、VGA64MiBの別fixtureで実3840×2160を確認しました。1024／1600／実4Kは実preconsole背景と既知ready行＋4,096byte全量による独立rendererで全pixel一致です。ready／quiet失敗も別の独立全pixel比較を実施し、4levelでは可変timestampだけ正規化してprimaryとretained ringの本文を検査します。
- 協調no-keyboard初回は並行負荷でtimeoutしました。最終単独試験も257 UDP echo／ARP／ICMP／file再読／厳密wrap／checksum／DMA停止を要求し、閾値を下げません。
- 既存ELF timingは元のfeature構成で実施します。大payloadの5,000ms reader期限による拒否は以前にもあるbounded観測結果であり、実行成功やthroughputとは扱いません。追加diagnostics-detail構成は16KiB合格、1MiB／16MiBでUSB transfer timeoutとなり、この組合せは既知制約として残します。既存matrixへ隠したり期限延長で通したりしません。
- 最終例外runnerはoption誤り（VM未起動）の後、private EFI指定不足で古いshared stagingを使いruntime markerへ到達しませんでした。いずれも起動操作の誤りとして区別し、各fault専用EFIを別build・明示指定した3件を証拠とします。他の初期fixture不足／旧marker観測失敗も成功には数えず保持します。

通常最終EFIは `out/bootlog-regression/external-normal-retry/target/x86_64-unknown-uefi/release/musha-boot.efi`、SHA256 `f9372997aeed48120f7217c7b13bf08583d0841321b6f7c38d89ff85dc505e06`。通常pixel試験ではdebug出力空を要求します。開発3EFIは別hash／features／source来歴をregistryへ記録し、通常版や正式候補とは扱いません。

実機WC測定・NUC5／Ryzen新ログ写真・Realtek TxConfig実測・I218実NVM／link／DHCP／UDP・実Bluetooth pair／再接続互換は未確認です。I218安全拒否とopt-inを維持します。所有者のv0.5／no-shell／scenarioと新POSIX文書はcommit対象外で保持し、並行変更を戻していません。
