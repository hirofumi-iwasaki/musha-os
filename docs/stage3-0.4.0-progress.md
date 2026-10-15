# Stage 3 implementation checkpoint — 2026-10-15

## English

The approved commits through `ce25a16a989c2e99ed5150c8b5992b41f1e8318b` were pushed normally to `origin/release/0.4.0`. This document accompanies the owner-authorized stage3 implementation commit. Local verification is complete; release and hardware acceptance remain open.

### Owner decisions and scope coordination

The owner confirmed fixed directory size4096, a chain of ceil(4096/cluster bytes), internal logical length, and “size0” meaning zero logical payload. All bytes in newly allocated clusters are initialized before allocation metadata is published; slack is zero. Only recognized OS-created records are writable. CONFIG.TXT is user-owned and completely read-only: no padding, resize, key deletion or rewrite.

After successful pairing, stage6 will save the exact raw CONFIG SHA-256 in the OS-owned pairing record. `pair=on` with the same consumed hash is suppressed; any content change rearms it. Saving identical content again does not rearm it. Pairing failure or failed durable save leaves the request unconsumed. SHA-256 makes accidental/adversarial content collisions impractical; it is not proof of file origin. CRC32 is accidental-corruption detection, not authentication.

At this checkpoint the owner has not yet edited scope goals4/5/12 to incorporate these additions. The previous scope wording permits general SETTINGS writes and says CONFIG is cleared after pairing; those statements are superseded by the additional owner decision. The concurrently owned scope file was preserved. [Work plan](work-plan-0.4.0.md) and [configuration guide](startup-configuration.md) reflect the decision.

### Implemented

- BOT WRITE(10), SYNCHRONIZE CACHE(10), DATA OUT direction, fixed-format REQUEST SENSE and DATA PROTECT interpretation. MODE SENSE(6) checks protection before WRITE because a protected USB device can stall OUT instead of completing with a failed CSW. Failed/aborted transport retains DMA ownership until existing controller shutdown.
- The internal FAT32 writer accepts only `PAIRING.DAT` under existing `/SETTINGS/`; there is no application write syscall. New and existing records must pass the supplied full format recognizer. Existing same-named user data, unknown versions or damaged records are refused. Magic/version/CRC recognition cannot distinguish a deliberately forged or copied valid record from one created by the OS; filename alone is never sufficient.
- Bounded preflight checks short names, geometry, mirrored FAT equality, primary/backup FSInfo signatures/count/hint, directory/chain limits and selected-cluster cross-links. Directory traversal is limited to64 directories/4096 entries, each chain to128 clusters; metadata reads have finite budgets. Larger or ambiguous layouts fail closed. No active-FAT-only layout, LFN modification, deletion, rename or directory creation is supported. Inserting at an end marker is refused if it would revive nonzero bytes in the following entry, including a cluster boundary; deleted entries are handled separately.
- First creation initializes the entire allocated chain, updates both FATs, primary/backup FSInfo and one directory record, then synchronizes. Overwrite validates existing ownership and fixed size, modifies only the4096-byte file on the original chain, and allocates nothing. exFAT and CONFIG writes are refused.
- A separate4096-byte LE bond record: magic `MUSHALTK`, version1, 64-byte header, explicit payload length0–320 (excluding the header)/count, up to8 fixed40-byte bond records, consumed-CONFIG SHA-256 flag/value, reserved/padding validation and whole-file CRC32. Key size7–16, address type, security flags and duplicate identity are checked before any output is published. Zero logical payload means no bond records; the physical file remains4096 bytes.
- Firmware CONFIG reading computes SHA-256 from exact bounded raw bytes only after successful parsing; a close failure clears it. Only a32-byte CPU-owned digest survives protocol/file-buffer lifetimes. Pure pairing-request/acknowledgement functions are implemented. Actual Bluetooth, SMP success and production pairing-save calls remain stage6 work; normal startup does not write pairing records. The QEMU-only adapter exercises the writer over real BOT commands.

### Validation and evidence

Full workspace host306 tests and existing Python7 suites/52 tests passed. The focused filesystem/platform/xHCI subset is167 of those306 tests, not additional coverage. Tests include fixed4096-byte files, logical length0/maximum320,4097-byte refusal, same-named user-file protection, cross-links/end-marker garbage,4096-byte sectors, full initialization of8KiB clusters, recoverable metadata rollback at every write position, explicit permanent-recovery failure, every BOT cancel phase, SHA-256 vectors/raw-byte distinctions, consumption durability, and corruption at every one of4096 byte positions without output publication.

QEMU scenario totals below include cross-suite overlap; they are not globally unique cases or command counts. Corrective/limited reruns are recorded separately.

| Suite | Scenarios |
|---|---:|
| Existing CI | 54 |
| C1 / C2 / C3 | 8 / 14 / 20 |
| Basic Data / ESP | 38 |
| External ELF debug / normal / explicit hotplug-timing-hub | 27 / 26 / 7 |
| Address recovery | 2 |
| Resolution / hold | 20 |
| Additional USB / network | 13 |
| Startup / diagnostic hold | 16 / 3 |
| SETTINGS writes | 15 |
| Total, including overlap | 263 |

All scenarios passed after the documented reruns. SETTINGS includes twelve behavior cases plus three added geometry variants. Four geometries (512-byte sector/1 or16 sectors per cluster;4096-byte sector/1 or2) exercise logical length0, maximum eight bonds and4097-byte overflow refusal. Free-cluster bytes are deliberately nonzero before creation; resulting slack is zero. Other cases cover protected/missing SETTINGS/unowned/full directory/no space/fragmentation/timeout/removal/exFAT/cancel/existing CONFIG. All observe CONFIG-write refusal and post-probe DMA shutdown. FAT32 cases pass the specified `fsck.fat -n` before and after, with independent untouched-sector/directory/FAT comparisons. exFAT is byte-identical and is not passed to a FAT checker. Fragmented overwrite preserves allocation metadata. Final payload-length encoding received six limited reruns (four geometries, fragmented and unowned), all passed.

`fsck.fat4.2` was obtained from the official checksum-verified Homebrew macOS bottle and extracted into `.local-tools/` without system installation. It exposed a pre-existing generator mismatch: BPB label `MUSHA-OS` had no root volume-label entry. The generator now emits the matching metadata entry, preserving the documented FAT volume label and GPT partition name `Musha-OS`. Volume-label metadata is not a file, so the four root file/directory elements remain unchanged. Image bytes change; this is a runtime-source correction outside docs-only provenance. Existing image/package/distribution tests and viewer/pixel regressions passed. README marker remains1356 bytes/FNV64 `AC9B8160200C6A04`; README banner remainsv0.3.0.

Final evidence: `out/stage3-regression/verification.json`, `host-workspace-final.log`, `python-tests.json`, `ci.json`, `privilege.json`, `extra-final.json`, `variants.json`, `write-variants.json`, `write-final.json`, per-case reports/debug logs/fsck logs, and `fsck-tool.json` (tool/source checksums). Earlier checkpoint evidence in `out/stage3-040/` is retained. The regular-image harness is [test-settings-write.py](../tools/test-settings-write.py).

Resolved initial issues are retained, without weakening checks: the startup orchestrator initially supplied a bootable data USB and firmware selected the wrong EFI; marker-only data fixed the setup and all16 cases passed. The parallel1MiB ELF timing attempt hit a BOT timeout; isolated retry with the exact same EFI and unchanged assertions/deadlines passed with a5000ms deadline-censored result. Its exact transient cause is not established, and successful1MiB loading is not claimed. Earlier harness shutdown matching/protected OUT issues are retained in the previous checkpoint. No unresolved failure remains.

The feature-free EFI is byte-identical between CI and the final-source normal external suite: SHA-256 `e83c20359f48eb352b4bd4443b793c4d18ca97a4e091680ce539a0e6d706b27c`. Normal release builds, formatting, Python syntax, whitespace and local links are recorded in verification.json.

### Limits and remaining gates

The writer restores changed metadata after a recoverable I/O error, including the failed sector. Content may be undefined, as scoped. Host tests inject each creation-write failure and verify restored metadata. A permanently failing device, removal during metadata publication, failed rollback or power loss cannot guarantee recovery; the writer returns `RecoveryRequired` when restoration fails. QEMU timeout/removal/cancel stop before metadata publication with DATA doorbell withheld; they prove unchanged images and DMA shutdown at that boundary, not arbitrary removal atomicity during FAT publication. Filesystem integrity after irreversible metadata loss is not claimed.

Remaining gates are physical USB/NUC5/Ryzen write–cold-reboot–readback and macOS/Windows mount/chkdsk, plus actual Bluetooth/SMP and production successful-pairing save calls in stage6. Local tests, including the specified checker and dedicated QEMU size/geometry variants, are complete. No physical USB, formal candidate, remote CI, tag/Release or additional push was performed. This local implementation commit does not claim release/hardware acceptance.

## 日本語

承認済み `ce25a16a989c2e99ed5150c8b5992b41f1e8318b` までを `origin/release/0.4.0` へ通常pushした。本書は所有者承認済みの工程3実装commitに伴う記録。ローカル検証は完了し、公開・実機受入は未完。

### 所有者判断とscope調整

directory size4096固定、ceil(4096/cluster bytes)のchain、内部有効長、「size0」は論理payload0と確定した。初回割当てmetadata公開前に全clusterの全bytesを初期化し、slackは0にする。OS自身が作成し形式検証できるrecordだけを書き込む。CONFIG.TXTは利用者所有で完全読取り専用とし、padding/resize/key削除/書換えはしない。

工程6でpair成功後、正確なraw CONFIGのSHA-256をOS所有pairing recordへ保存する。同じ消費済みhashの `pair=on` は抑止し、内容変更で再有効にする。同一内容を保存し直すだけでは再有効にならない。pair失敗・永続化失敗は未消費のまま。SHA-256は偶発的・意図的衝突を現実的に困難にするが、出自認証ではない。CRC32も偶発破損検出であり改ざん認証ではない。

この区切りではscope目標4/5/12の所有者改訂はまだない。旧scopeの一般的SETTINGS書込みとpair後CONFIG消去は追加判断で置換される。同時編集対象のscopeは保持した。[作業計画](work-plan-0.4.0.md)と[設定文書](startup-configuration.md)は新判断へ整合した。

### 実装

- BOT WRITE(10)/SYNCHRONIZE CACHE(10)/DATA OUT、固定形式REQUEST SENSE/DATA PROTECT。保護USBは失敗CSWでなくOUTをstallする場合があるため、WRITE前のMODE SENSE(6)で保護を調べる。失敗・中断時は既存controller停止までDMA所有権を維持する。
- 内部FAT32 writerは既存 `/SETTINGS/PAIRING.DAT` だけを許可し、アプリ書込みsyscallは追加しない。新旧recordとも完全な形式認識が必要。同名利用者file・未知版・破損recordは拒否する。意図的に偽造・コピーされた正常recordの出自はmagic/version/CRCでは識別できず、ファイル名だけで所有性を判断しない。
- 上限付きpreflightは8.3名/geometry/両FAT一致/primary・backup FSInfo署名・count・hint、directory/chain上限と対象clusterのcross-linkを検査する。directory64個・entry4096個、各chain128cluster、metadata読取りbudgetも有限。大きすぎる・曖昧な構造は拒否。active FAT限定構造、LFN変更、削除/改名/mkdirは非対応。終端への作成で次entryの非zeroを復活させる場合はcluster跨ぎでも安全拒否し、削除entry再利用と区別する。
- 初回は全chain初期化→両FAT・両FSInfo・directory1record更新→同期。上書きは既存の所有形式と固定sizeを確認し、同一chainの4096bytesだけ変更、追加割当て無し。exFAT/CONFIG書込みは拒否。
- 独立4096byte LE bond recordはmagic `MUSHALTK`、version1、header64byte、headerを除くpayload有効長0–320/count、最大8個の40byte bond、消費済みCONFIG SHA-256のflag/value、予約部/padding検証、全file CRC32。key size7–16/address種別/security flag/identity重複を、出力公開前に検証する。論理payload0はbond無しで、物理fileは4096byteのまま。
- firmware読込み時に上限付きraw CONFIGをparse成功後SHA-256化し、close失敗なら破棄する。protocol/file bufferの寿命後にはCPU所有32byte digestだけ残す。pair要求/成功ackは純粋関数として実装した。実Bluetooth/SMP成功と本番pairing保存呼出しは工程6で、通常起動はpairing recordを書かない。QEMU専用adapterで実BOT経由writerを検証する。

### 検証と証拠

全workspace host306件、既存Python7suite/52件が合格。filesystem/platform/xHCIの167件は306件の内数で、重ねて加算しない。4096byte固定file、有効長0/最大320、4097byte拒否、同名利用者file、cross-link/終端garbage、4096byte sector、8KiB cluster全初期化、write全位置の回復可能metadata rollback、回復不能の明示、BOT全phase cancel、SHA-256既知値/raw差分、消費と永続化、全4096位置の破損時出力無変更を確認する。

QEMUの次の合計はsuite間重複を含み、全体の一意case数やcommand数ではない。修正・限定再試験は別記する。

| Suite | Scenario数 |
|---|---:|
| 既存CI | 54 |
| C1 / C2 / C3 | 8 / 14 / 20 |
| Basic Data / ESP | 38 |
| 外部ELF debug / normal / 明示hotplug・timing・hub | 27 / 26 / 7 |
| Address recovery | 2 |
| 解像度 / hold | 20 |
| 追加USB / network | 13 |
| Startup / diagnostic hold | 16 / 3 |
| SETTINGS書込み | 15 |
| 重複を含む合計 | 263 |

記載の再試験後は全合格。SETTINGSは12動作caseと追加geometry3種。512byte sector×cluster1/16sector、4096byte sector×cluster1/2sectorの4構造で、有効長0、最大8bond、4097byte拒否を検証する。作成前にfree clusterを非zeroにし、slack全0を確認。他は保護/SETTINGS無し/非所有file/entry満杯/空き無し/断片化/timeout/取外し/exFAT/cancel/利用者CONFIG保持。全probeでCONFIG拒否と終了後DMA停止を観測。FAT32全caseの前後に指定 `fsck.fat -n` と独立sector/directory/FAT比較を行い合格。exFATは全bytes不変でFAT checkerを適用しない。断片上書きでは割当metadataも不変。payload有効長field確定後はgeometry4種・断片・非所有の6caseを限定再試験し全合格。

`fsck.fat4.2` は公式Homebrew macOS bottleのchecksumを検証し、system install無しで `.local-tools/` へ隔離展開した。既存生成器の「BPB label MUSHA-OSに対応するroot volume label無し」を検出し、同じlabelのmetadata entry追加で修正。文書どおりFAT volume label `MUSHA-OS` とGPT partition name `Musha-OS` を保持。volume labelはfileではなく、rootの4要素は変わらない。像bytesは変わるためdocs-only外のruntime source修正。画像/package/distribution試験とviewer/pixel回帰は合格。README markerは1356byte/FNV64 `AC9B8160200C6A04`、README冒頭v0.3.0も保持。

最終証拠は `out/stage3-regression/verification.json`、`host-workspace-final.log`、`python-tests.json`、`ci.json`、`privilege.json`、`extra-final.json`、`variants.json`、`write-variants.json`、`write-final.json`、各caseのreport/debug/fsck log、tool/source checksumの `fsck-tool.json`。先行 `out/stage3-040/` は保持。通常file像だけを扱う[test-settings-write.py](../tools/test-settings-write.py)を使う。

初期問題も保持し判定を緩めない。startup orchestratorは最初にデータUSBも起動可能にしたため別EFIが起動し、marker-only dataへ直して全16合格。並行時の1MiB timingはBOT timeoutとなり、同一EFI・同じ判定/期限での単独再試験は5000ms deadline-censoredとして合格。正確な一過性原因は未確定で、1MiB読込み成功とは主張しない。以前のshutdown誤認/保護OUT問題は先行checkpointに保持。未解決failureは無い。

feature無しEFIはCIと最終sourceのnormal external suiteで完全一致し、SHA-256は `e83c20359f48eb352b4bd4443b793c4d18ca97a4e091680ce539a0e6d706b27c`。通常release build、整形、Python構文、空白、リンク検査もverificationへ記録する。

### 制限と残gate

回復可能I/O失敗では失敗sectorを含むmetadataを復元する。内容不定はscopeどおり。hostは作成write全位置に障害を入れてmetadata復元を比較する。永続故障・metadata公開中の媒体消失・rollback失敗・停電では回復を保証できず、復元不能は `RecoveryRequired`。QEMU timeout/removal/cancelはDATA doorbellを保留しmetadata前で止める試験で、その境界の像不変とDMA停止を示す。FAT公開中の任意取外しの原子性や、不可逆metadata消失後の構造正常は主張しない。

残gateは実USB/NUC5/Ryzen書込み・冷間・読戻しとmacOS/Windows mount/chkdsk、工程6の実Bluetooth/SMP成功と本番pairing保存呼出し。指定checker・専用QEMUサイズ/geometryを含むローカル検証は完了。実USB、正式候補、remote CI、tag/Release、追加pushは未実施。このローカル実装commitを公開・実機受入完了とはしない。
