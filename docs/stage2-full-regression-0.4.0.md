# Stage2 full local regression / 工程2 全ローカル回帰

## English

Status: local regression complete before the stage2 implementation commit, source base `35545a6`. No physical acceptance, formal release candidate, clean two-build reproducibility, remote CI, push, tag, Release or USB operation is claimed. Evidence: `out/stage2-full/verification.json`, runner ledgers `ci.json`, `privilege.json`, `extra.json`, `variants.json`, `retry-final.json` and `final-render.json`. Each tool retains commands, logs, screens and relevant image hashes. [Earlier stage2 checks](stage2-0.4.0-results.md) remain a separate checkpoint.

The 11 suite categories cover **229 suite-labelled scenarios after corrective reruns**. This sum includes overlapping conditions between CI, partition and mode suites; it is not a count of globally unique tests. There were **238 successful QEMU run checks and six initial failed runs** in this full-regression checkpoint. The nine extra successful runs are two superseded128-byte probe checks, two384-byte cancel rechecks, four final normal-pixel/fast-API rechecks and one final devices capture. Host results: **285 passed**. Build/fixture commands are not counted as QEMU scenarios: the CI runner has107 top-level steps, privilege3, extra53, variants17, final-render8 and final retry5; nested tools have their own command ledgers.

| Suite category | Covered scenarios |
|---|---:|
| CI QEMU scenarios | 54 |
| C1 | 8 |
| C2 | 14 |
| C3 | 20 |
| BasicData/ESP | 38 |
| external debug functional | 27 |
| external explicit hotplug/timing/hub | 7 |
| external normal functional | 26 |
| address recovery | 2 |
| resolution/hold scenarios | 20 |
| additional USB/network/phase variants | 13 |

Coverage reproduces local CI through USB, README-marker negative3, ARP/ICMP/UDP, input idle/removal, multiple controllers, five-level hubs, live storage/files, cancellation/timeout/disconnect, required exceptions, FAT corruption, FAT32/exFAT viewer navigation and no-feature normal pixels including a larger virtual image. Formal candidate preparation is deliberately replaced by a normal development build/image. C1/C2/C3 cover all8/14/20 existing cases. Basic Data and ESP cover38 cases. External ELF covers27 debug functional cases,26 feature-free normal cases and7 explicit hotplug/timing/hub cases. Additional USB coverage includes512/4096-byte storage sectors, high-speed storage, MBR/superfloppy, unsupported NIC, hub removal, EP0 ring validation and read-only T2 diagnostics. QEMU xHCI is not physical T2 evidence.

Mode coverage is1024×768,1600×1200 and3840×2160 for Hello/long log/absent viewer, new same-mode FAT32 references and independent Hello pixel rendering; starting/runtime holds also cover default2048², for20 QEMU scenarios. Reference generation first verifies known names, contents, page hashes and navigation with the viewer harness. Feature-free comparisons still require exact full-frame equality, zero debug bytes and unchanged virtual images. The Hello checker independently renders the known font/grid/content plus the explicitly selected ready footer. Actual frames were not unconditionally copied into expected results.

### Failures and corrective reruns

- Four CI scenarios (live-file FAT32, exFAT, split exFAT and close/reopen) exposed the old7-byte first-file probe assumption after README grew to1356 bytes. An intermediate128-byte probe passed FAT32 but remained too slow with repeated exFAT metadata and200ms artificial sector delays. Final384-byte partial requests preserve multiple requests, sector crossings, exact byte/hash/EOF checks and input/UDP concurrency. The production API,10-second session limit and200ms delay were not relaxed. Five final FAT32/exFAT/cancel checks plus `close-final` passed; fast transport was also rechecked on both filesystems.
- External absent-viewer initially used the old v3 reference. `external-debug-absent-retry` uses a newly verified current FAT32 reference; full-frame equality was retained. The26 preceding debug cases passed and were not needlessly repeated.
- The +5000ms hotplug test needs diagnostic hold but the old default tool selected only external-ELF instrumentation. That case now explicitly selects diagnostics-detail; `hotplug5000-verified` passes. A separate invocation missing the local toolchain environment failed before QEMU and is excluded from QEMU counts.
- Code inspection found that suppressing Loading at ready/quiet left the rejection console unbound. Runtime now prepares the console model without drawing. The full26 normal cases, including malformed/CPU/size/duplicate/deadline rejection, passed. The host policy test also proves zero-paint preparation and visible quiet rejection.
- Devices startup initially showed the pre-discovery panel. It now refreshes existing recorded values at the ready boundary, preserving hidden state and leaving four diagnostic footer rows; application/viewer geometry is unchanged. `devices-final/devices/ready.png` shows names/VID:PID, ports/speeds, blocks/sector and the PM summary without overlap. Both normal pixel filesystems were rechecked. No full-suite repetition was needed for this devices-only rendering change.

Timing16KiB completed in107ms in the recorded virtual run;1MiB and16MiB reached the existing5000ms deadline without completion. Their timing assertions passed as censored observations, not successful large-file reads or hardware throughput. Firmware-error injection remains unperformed. All source changes and final checks remain development evidence, not NUC5/Ryzen/Mac acceptance.

Final tested feature-free EFI SHA-256: `364fc51c13a920c9dd08d6f0fd53c631eae9ce222dfbabb1a5a7af2afb5c833a` (`out/stage2-full/external-normal/target/x86_64-unknown-uefi/release/musha-boot.efi`). This normal suite was built after the no-paint console fix. Later mode builds only test explicit GOP selection; they are not distribution binaries.

Stage1 remains the four-asset contract `musha-os-<ver>-x64.zip`, `-x64-qemu.img.gz`, `-c-sdk.zip`, `-rust-sdk.zip`, plus separate build-manifest/SHA256SUMS. Root EFI/DOCS/empty SETTINGS/README, stateless trusted-prior-hash updates, retirement/migration of both install JSON names and build-derived README length/FNV are implemented. Rust SDK remains a source scaffold and keeps publication gates closed. Formal two-build/assets comparison, Rust guest SDK, remaining goals and physical formatter/settings/key/display/timing acceptance remain open. Image-embedded documents/licenses still require renewed acceptance; full comparison is retained.

## 日本語

状態：工程2実装commit前の全ローカル回帰完了。基準commitは `35545a6`。実機受入、正式候補、clean二回build再現性、remote CI、push／tag／Release／USB操作は主張しない。証拠は `out/stage2-full/verification.json` と各runner台帳 `ci.json`／`privilege.json`／`extra.json`／`variants.json`／`retry-final.json`／`final-render.json`。各工具にコマンド・log・画像・媒体hashが残る。[先行工程2検証](stage2-0.4.0-results.md)とは別の時点を記録する。

**11区分・suite名付き229シナリオを修正後の再試験まで含めてカバー**。CI／partition／mode間で同条件が重複するため、世界全体で一意のテスト229件という意味ではない。今回の全回帰では**QEMU成功チェック238回・初回失敗6回**。229以外の成功9回は旧128byte試行2、384byte cancel再確認2、最終通常pixel／高速API4、devices最終描画1。hostは**285件合格**。build／fixture生成はQEMUケースに数えず、runner上位stepはCI107・privilege3・extra53・variants17・final-render8・final retry5。各工具内部のコマンドは別台帳に記録する。

| suite区分 | 対象シナリオ |
|---|---:|
| CI QEMU scenarios | 54 |
| C1 | 8 |
| C2 | 14 |
| C3 | 20 |
| BasicData/ESP | 38 |
| external debug functional | 27 |
| external explicit hotplug/timing/hub | 7 |
| external normal functional | 26 |
| address recovery | 2 |
| resolution/hold scenarios | 20 |
| additional USB/network/phase variants | 13 |

CIのUSB、README負例3、ARP／ICMP／UDP、入力待機／取外し、複数controller、5段hub、live storage／file、cancel／timeout／disconnect、例外、FAT破損、FAT32／exFAT viewer操作、通常版pixelと拡大仮想像までローカル再現した。正式候補生成は通常開発build／imageに置換。C1／C2／C3は既存全8／14／20、Basic Data／ESPは38、externalはdebug機能27・feature無し通常26・明示hotplug／timing／hub7。追加USB分岐は512／4096byte sector、高速storage、MBR／superfloppy、未対応NIC、hub切断、EP0 ring、読取り専用T2診断。QEMU xHCIを実T2証拠にはしない。

1024×768／1600×1200／3840×2160でHello／long log／absent viewer、同modeの新FAT32参照、独立Hello pixelを確認。starting／runtime holdは既定2048²も含め計20 QEMUシナリオ。参照作成前にviewer工具が既知名・内容・page hash・操作を検査し、通常版比較では全frame完全一致・debug0・媒体不変を維持する。Hello checkerは字形／grid／既知内容と明示ready footerを独立描画する。実装画像を無条件に期待値へコピーしただけではない。

### 失敗と再試験

- CI4ケース（FAT32 live-file、exFAT、split exFAT、close/reopen）はREADME1356byte化に旧7byte部分要求が合わず期限内に終わらなかった。中間128byteはFAT32合格でもexFAT反復metadata＋sector200ms人工遅延には不足。最終384byteで部分要求・複数要求・sector跨ぎ・正確なbyte/hash/EOF・入力/UDP並行を保持し、本番API／10秒期限／200ms遅延は緩めていない。最終5ケース＋close-final合格、高速経路も両filesystemで再確認。
- external absentは旧v3参照を使って失敗。新しい現行FAT32参照で `external-debug-absent-retry` 合格、完全一致条件保持。先行debug26件は合格済みなので無用な再反復はしなかった。
- +5000ms hotplugは診断holdが必要なのに旧工具既定featureに不足。当該caseでdiagnostics-detailを明示し `hotplug5000-verified` 合格。別のtoolchain環境指定漏れはQEMU前に失敗したためQEMU件数に含めない。
- ready／quietでLoadingを抑止すると拒否consoleが未初期化になる経路をコードで発見し、runtime開始時に無描画で準備した。異常ELF／CPU／サイズ／重複／期限を含む通常版全26件合格。hostでもquiet準備無描画・拒否可視を確認。
- devicesが探索前panelのままだったため、ready境界で既存記録値を再描画する局所修正を実施。hidden状態保持、診断footer4行の余白を残し、アプリ／viewer geometryは不変。`devices-final/devices/ready.png` に名称／VID:PID／port／speed／blocks／sector／PM要約を重なりなく表示。通常pixel両filesystemも再試験した。devices専用描画修正で全suiteは再反復していない。

timing16KiBは当該仮想実行107msで完了。1MiB／16MiBは既存5000ms期限で未完了であり、打切り観測の検査合格を大容量読出し成功や実機速度へ読み替えない。firmware error注入は未実施。いずれも開発証拠でNUC5／Ryzen／Mac受入ではない。

最終通常EFI SHA-256: `364fc51c13a920c9dd08d6f0fd53c631eae9ce222dfbabb1a5a7af2afb5c833a`（`out/stage2-full/external-normal/target/x86_64-unknown-uefi/release/musha-boot.efi`）。この通常suiteは無描画console修正後にbuildした。その後の明示GOP mode用buildは配布binaryではない。

工程1はv無しの4配布物契約（`musha-os-<ver>-x64.zip`／`-x64-qemu.img.gz`／`-c-sdk.zip`／`-rust-sdk.zip`）と別build-manifest／SHA256SUMS、EFI／DOCS／空SETTINGS／README、信頼した前版hashで保護するステートレス更新、両install JSON廃止移行、ビルド由来README長/FNVまで実装済み。Rust SDKはsource scaffoldで公開gate閉鎖。正式二回build／全asset比較、Rust guest SDK、残目標と実機formatter／設定／キー／display／timing受入は未完。像内文書/license変更の再受入と完全比較も維持する。
