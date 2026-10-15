# Stage5 local regression — 2026-10-15

## English

Status: local implementation and the planned full host/QEMU regression are complete, based on `47c8a9c`; this record accompanies the owner-authorized stage5 implementation commit. Physical and formal release acceptance remain open. Evidence: `out/stage5-regression/verification.json`, per-suite commands/logs/reports and `plan.md`. The earlier `out/stage5-040/` checkpoint is historical preliminary evidence.

### Implemented boundary

Initial AUTORUN executes once. Exit0/nonzero, recoverable synchronous user faults (divide/UD/GP/PF), and loader refusal return through the C3 continuation to a fresh viewer. Kernel/NMI/#MC/#DF and invariant failures remain fatal. r2 saves its continuation around Ring3 inside the existing USB callback, without jumping across an outer Drop-bearing scope. Detach callbacks/input queues, confirm xHCI halt plus PCI BME-off, restore CPU/page tables/MSRs/segments/FPU, return normally through callback/session Drop, retire canceled file handles, then scrub application arena/staging. Unconfirmed stop halts without reuse. Fresh discovery/mount generations invalidate stale IDs. Boot PM epoch remains monotonic across launches.

Enter on a complete catalog name ending in ASCII case-insensitive `.ELF` submits the opaque current-mount ID through the same bounded reader, close, ELF/NOTE and permission checks as AUTORUN. Display-name truncation never selects the file. Other files remain read-only; viewer mouse UI and pairing are not added. Outcome/status or fault vector/refusal is visible after return, including the no-keyboard fallback.

LAN now initializes the lwIP core/timers once and recreates only the session endpoints. After confirmed NIC DMA shutdown, teardown removes UDP PCB, brings link/netif down, clears ARP-held pbufs, removes netif and clears queued frames. Persistent ARP timers use the boot epoch, not a reset per-session clock. No new NIC driver is included.

### Verification

Workspace host: **319 passed**. Existing Python tools: **52 tests in7 suites passed**. Native actual-lwIP stress: one program with **1000 init/ARP/stop cycles**, checking UDP/netif/frame removal and heap/memp baselines plus monotonic sys_now. SDK r1/r2 layout and26 malformed ELF mutations per check pass; generated files synchronize. Four Rust ELFs and C r2 I/O ELF reproduce identically from independently extracted SDKs in paths containing spaces; final C/Rust development archives exclude generated targets and private evidence links. Four final virtual writer images pass isolated dosfstools `fsck.fat -n`. Repository links828 and standalone SDK links34 each have no missing targets. Existing references to13 omitted source-only documents become explicit checkout-path text in the archive; source documents are unchanged. The27 related package tests pass again (overlap with52 above).

| QEMU suite | Successful cases/configurations |
|---|---:|
| CI | 54 |
| C1 | 8 |
| C2 | 14 |
| C3 | 20 |
| partition | 38 |
| external_debug | 27 |
| external_normal | 26 |
| hotplug_timing_hub | 7 |
| address_recovery | 2 |
| resolution_hold | 20 |
| variants | 13 |
| startup | 16 |
| diagnostic_hold | 3 |
| settings_writer | 15 |
| r2_runtime | 11 |
| mixed_fp | 1 |
| rust_r1 | 2 |
| stage5_lifecycle | 9 |

Total **286 successful case/configuration executions, including overlap between suites**. This is not286 distinct features or command invocations. Build/package commands, failed attempts/retries, three offline pixel comparisons and1000 native iterations are not added. External debug27/normal26 use the successful case union documented in verification.json; early stopped aggregate reports retain failures and are not represented as whole-suite passes. CI54 includes three corrected socket-path replays. C3 final evidence is `c3-stable`.

Stage5 lifecycle functional runs execute and return **12 apps** (r1 FAT32 three, status42 one, UD fault one, r2 Rust one, r2 C three, r1 exFAT three), plus **three pre-execution invalid-ELF refusals**. LAN UDP echo succeeds after each return in all three FAT32/r2/exFAT series (nine echoes). Normal feature-free recovery pixels match the verified viewer frame without debug output. One separate stop-failure gate injection forbids scrub/reuse; it runs after real shutdown and does not simulate a physical controller refusing to halt.

Normal EFI SHA-256: `3dfc0a40f69572e1c9af1e8a71114d0b682ad8752b06e2b1da0bf7cf906fbfbb` (`ci-final/normal.efi`, identical to external-normal-tail). Final runtime manifest and per-file hashes: `runtime-final.json`; initial frozen manifest: `runtime-frozen.json`. After that freeze, only duplicate-catalog claim handling changed in `api/src/bootfile.rs` and `boot/src/external.rs` (plus its API test): duplicate rejection is published once during automatic Searching, never during returned viewer scanning. Duplicate cases and the subsequent debug/normal tails were rerun with the final EFI. Earlier nonduplicate viewer/input/storage/LAN/startup tests cannot take that branch; C1/C2/embedded C3 bypass external catalog claims. Their successful evidence remains applicable. Later consumer-only changes do not change runtime binaries. Reports retain each EFI hash.

### Failures, corrected expectations and remaining gates

The first checkpoint's LAN second-init refusal is fixed by lifecycle cleanup above. Local regression also found a return notice missing without a keyboard, and repeated duplicate AUTORUN rejection poisoning fresh-viewer readiness; both are fixed and directly retested. Earlier compilation during editing is excluded; C3 was rerun after source freeze. Prior InputPoll-stop and idempotent-render fixes remain covered.

Old terminal exit/fault/refusal assertions now require exact signed status/vector/refusal, rollback, stop-before-scrub and fresh viewer. Invalid ELF/NOTE/permissions/memory/CPU, timeout and no-read refusal checks remain. Transient4096-byte LOG/wrapping and UTF-8/control sanitization remain checked: debug raw bytes and normal bounded observation fixture (test-only hold). The independent active-LOG pixel renderer passes1024×768,1600×1200,3840×2160 from matching captured active screens; it retains fixed font, grid origin, clipping, scale and ready-footer assertions. Three obsolete final-exit pixel commands in display-extra.json failed before their commands were migrated; the replacement offline results are `console-final-*`, not copied reference pixels. Existing normal viewer golden references are unchanged. Three CI QMP socket paths exceeded macOS limits; short hashed paths and saved matching EFIs pass their replays. CR normalization in a debug assertion and a wrong SDK tool path were runner errors, corrected without relaxing content. Timing16KiB/1MiB/16MiB passes with no other QEMU load.

Physical NUC5/Ryzen three-launch/LAN/USB acceptance, chkdsk, BCE and Bluetooth integration, third-controller simultaneous mouse input and formal candidate reproducibility remain unverified. No push, physical USB write, remote CI, tag/Release or formal candidate generation was performed. Owner scope0.4/scope0.5/scenario/no-shell-policy files are excluded; start/final hashes are saved. All four hashes match this task’s starting snapshot; earlier task snapshots are not substituted for that baseline.

## 日本語

状態：`47c8a9c` を基点とする工程5実装と計画した全host/QEMU回帰を完了。本記録は所有者承認済み工程5実装commitに伴うもので、実機・正式公開受入は未完了です。証拠は `out/stage5-regression/verification.json`、各suiteのcommand/log/reportと `plan.md`。旧 `out/stage5-040/` は初期到達点の歴史記録です。

### 実装境界

初回AUTORUNを1回だけ自動実行。exit0/非0、回復可能な同期user fault（divide/UD/GP/PF）、loader拒否後はC3保存継続から新viewerへ戻ります。kernel/NMI/#MC/#DF・不変条件違反は停止。r2は既存USB callback内のRing3だけを囲み、外側のDrop scopeを飛び越しません。callback/input queue解除、xHCI haltとPCI BME-off確認、CPU/page table/MSR/segment/FPU復元、callback/session Dropの通常return、取消しhandle無効化を済ませてからarena/stagingを消去。停止未確認は再利用せず停止します。fresh mount世代で旧IDを無効化し、起動PM epochは単調性を保持します。

完全catalog名の末尾 `.ELF` をASCII大文字小文字無視で判定し、同mountのopaque IDを自動起動と同じ上限付きreader/close/ELF/NOTE/権限検査へ渡します。省略表示名は選択に使いません。他fileは読出し専用、viewer mouse UI・pairing追加なし。復帰後はstatus/fault vector/拒否を表示し、keyboard無しfallbackでも観測可能です。

LANはlwIP core/timerを一度だけ初期化し、session endpointだけを再生成。NIC DMA停止確認後にUDP PCB削除、link/netif down、ARP保持pbuf解放、netif削除、frame queue消去を行います。ARP timerはsessionごとの0ではなくboot epochで進みます。新NIC driver追加はありません。

### 検証

workspace host **319件合格**、既存Python **7suite/52件合格**。実lwIP native stressは1programで初期化/ARP/停止1000反復を実施し、UDP/netif/frame参照・heap/memp基準復帰・sys_now単調性を確認。SDK r1/r2 layoutと各checkの不正ELF26変種、生成同期が合格。空白入り独立展開SDKからRust4 ELFとC r2 I/O ELFが一致。最終開発C/Rust archiveはtarget生成物と私的証拠リンクを除外。writer仮想像4件で隔離dosfstoolsの `fsck.fat -n` が合格。repoリンク828件・各SDK内リンク34件は欠落0。archiveの非同梱source文書13種類への既存参照はcheckout path付き非リンク表現へ変換し、source文書は不変。関連package27試験も再合格（上記52件との重複）です。

QEMU内訳は上表のとおり、**suite間重複を含む286成功case/config実行**です。286独立機能・command数ではありません。build/package、失敗/再試験、offline pixel3件、native1000反復を加算していません。external debug27/normal26はverification.jsonに各成功caseの採用証拠を記録し、途中停止したaggregate全体を合格とは扱いません。CI54はsocket修正再試験3件を含み、最終C3は `c3-stable` です。

工程5機能試験は**実行後復帰12回**（FAT32 r1三、status42一、UD一、r2 Rust一、r2 C三、exFAT r1三）と**実行前不正ELF拒否3回**を区別。FAT32/r2/exFATの各3回復帰後、合計9回LAN UDP echoに成功。feature無し通常版の復帰pixelは検証済みviewerと一致しdebug出力なし。別の停止失敗gate注入は消去/再利用を禁止しますが、実停止後の注入なので実controller halt失敗の証明ではありません。

通常EFI SHA-256は `3dfc0a40f69572e1c9af1e8a71114d0b682ad8752b06e2b1da0bf7cf906fbfbb`。`ci-final/normal.efi` とexternal-normal-tailが一致します。最終runtime/file hashは `runtime-final.json`、初期凍結は `runtime-frozen.json`。凍結後のruntime変更は `api/src/bootfile.rs` と `boot/src/external.rs` の重複catalog claimのみ（関連API試験も追加）。自動Searching時に一度だけ拒否し、復帰viewer再走査では通知しません。影響するduplicateと後続debug/normal tailは最終EFIで再試験。先行した非重複viewer/input/storage/LAN/startupは該当分岐に入らず、C1/C2/埋込みC3はexternal catalog claimを通らないため採用。consumerのみの後続変更はruntimeを変えず、reportに各EFI hashを保持します。

### 不合格経緯・期待変更・残件

初期到達点のLAN二回目初期化拒否は上記cleanupで解消。keyboard無し復帰通知欠落、重複AUTORUN拒否の再発行でfresh viewer readinessを壊す不具合も修正・直接再試験済み。編集中のcompile失敗は採用せずC3を凍結後再実施。既存InputPoll停止と冪等描画修正も維持します。

旧terminal終了/fault/拒否期待は、正確な符号付きstatus/vector/拒否とrollback/停止後消去/fresh viewerへ更新。ELF/NOTE/権限/memory/CPU拒否・timeout・読出し前拒否の検査は保持。消えるLOG4096 byteの折返しとUTF-8/control置換もdebug raw byte・通常版の有限観測fixtureで検証。独立active-LOG pixelモデルは対応する実画面で1024×768/1600×1200/3840×2160が合格し、固定font/grid原点/clip/scale/ready footerを検査。display-extra.jsonの旧終了画面pixel command3失敗は `console-final-*` のoffline結果で置換し、実装画面の参照コピーではありません。既存通常viewer goldenは不変。CI3件はmacOS QMP socket長制限を短いhash pathで解消し同EFI再試験合格。debug CR正規化とSDK tool位置誤りはrunner修正で内容条件は維持。timing16KiB/1MiB/16MiBは他QEMU負荷なしで合格しました。

実機NUC5/Ryzen連続3回/LAN/USB、chkdsk、BCE/Bluetooth、第3controller同時mouse、正式候補再現は未確認。push・実USB・remote CI・tag/Release・正式候補生成なし。所有者scope0.4/scope0.5/scenario/no-shell-policyは対象外で開始/終了hashを保存。4件とも今回の開始snapshotとhash一致です。過去taskのsnapshotを今回の基準に置き換えていません。
