# NUC5 diagnostic capture / 診断採取

Status: record (2026-10-14)

## English

Historical captures retain their original filename, MUSHA.ELF. From the renamed candidate onward, use AUTORUN.ELF.

### Next capture: routing grace and one-shot rescan (2026-10-14, not deployed)

The current source adds 150 ms ordinary / 500 ms actual-Intel-write grace and one bounded rescan only if no device enumeration was attempted. See [USB startup timing](usb-startup-timing.md). Local test artifacts under out/usb-grace are not a promoted release candidate and have not been written to USB. Older EFI hashes below identify historical captures, not this new revision.

For each NUC5 run, power off for at least 10 seconds and photograph PRE OWN D8=0 before interpreting the run as cold. Record run number, power-off seconds, PRE OWN D0/D8, deployed EFI SHA-256/provenance, all SS ROUTE+ MS observations (CCS/PED/PP/SPEED/PLS), WPR, RESCAN result and final Hello/exit or HOLD. The route timestamp starts immediately after routing writes/readback; sample resolution is 20 ms plus execution overhead. Capture every page through a complete cycle. The page header shows T+ seconds, SESSION, RESCAN; DIAGNOSTIC HOLD (no retry) cycle distinguishes page rotation from a new USB attempt. A HOLD is a failed run; any reboot starts a new run. Do not wait through repeated holds expecting retry. Keep the existing 2-second page interval.

| Run | Power-off seconds | PRE OWN D8 | EFI/provenance | Route to CCS/U0 ms (each SS port) | RESCAN | Result |
|---|---|---|---|---|---|---|
| 1 | UNKNOWN | UNKNOWN | UNKNOWN | UNKNOWN | UNKNOWN | Pending |
| 2 | UNKNOWN | UNKNOWN | UNKNOWN | UNKNOWN | UNKNOWN | Pending |
| 3 | UNKNOWN | UNKNOWN | UNKNOWN | UNKNOWN | UNKNOWN | Pending |

Validate the provisional 500 ms against physical observations before preparing a normal release candidate. NUC5 requires three consecutive cold Hello/exit successes; Ryzen requires three cold successes plus one hot reboot and READ OK-to-Hello timing. Missing old power-operation details remain unknown; the owner approved correction of the old three-cold-success claim.

### SuperSpeed capture revision (2026-10-14)

Historical SuperSpeed diagnostic EFI: `d2f5d0c039fb71f864f8e1c2b34958b35d1de8a197bd54a6a3ec73d46eb47348`, `out/nuc5-ss-recovery/BOOTX64.EFI` (diagnostics-detail only). It supersedes the preceding revisions below. No medium or absent MUSHA.ELF now returns to rotating diagnostics after quiescing devices; the viewer is not started. Claimed/rejected ELF retains its existing path.

USB3 attachment gets a shared 2000 ms budget after the existing attachment wait, sampled every 20 ms. U0 + CCS + PED is required before queueing. PLS 6/7/10 stable for 200 ms triggers WPR, at most three writes per root. Active resets, unpowered/overcurrent ports and RxDetect alone do not trigger WPR. USB2 and the existing 100 ms debounce are unchanged. All builds use the same recovery policy.

Capture SS ROOT / T (milliseconds from SuperSpeed settling start) / SAMPLE / WPR / WPR READBACK / U0 or TIMEOUT, including raw PORTSC and CCS/PED/PP/SPEED/PLS. Up to eight samples per port, including the initial sample and subsequent changes, are retained, plus every WPR/readback and final sample; this is a bounded timeline, not a complete bus trace. Existing journal capacity is 512 lines and reports JOURNAL LIMIT when exhausted. ROOT queue/skip, ENUM ROOT PORT RESET, ADDRESS ROOT SLOT, and subsequent DEVICE DESCRIPTOR / CONFIGURATION records show how far each target progressed. Root numbers in these journal lines are decimal (ROOT 12 = summary ROOT 0C); raw registers and summary root fields are hexadecimal.

Pages remain scale 3 and rotate every 2 seconds. Record all pages through wraparound after a failed cold boot. Warm-reset effectiveness on NUC5 is pending; QEMU does not reproduce the physical link fault.

Completion event layout/code reference: [Intel xHCI specification, §6.4.5.1](https://www.intel.com/content/dam/www/public/us/en/documents/technical-specifications/extensible-host-controler-interface-usb-xhci.pdf). Recovery was implemented independently; no foreign driver code was copied.

Current address-recovery diagnostic: `7eba0377ffdc39fea24544a52c8fed86bbf7254c2869d753fceebaffb948676a`, `out/nuc5-address-recovery/BOOTX64.EFI`. Both status and diagnostic text use scale 3, pages rotate every 2 seconds. Failure captures continue cycling; when an external file is claimed after a successful device session, one full diagnostic cycle precedes application entry. Normal builds do not wait. Inspect COMMAND CC, COMMAND RAW, ADDRESS FAILED ROOT, SLOT DISABLED / CONTINUE, and RESET/SLOT SPEED ID. A correlated error (codes 4/5/7/17/19/22) is consumed once; Disable Slot must succeed before clearing DCBAA and continuing. Buffers remain reserved until controller teardown. Malformed completion, timeout, ring stop/abort, or failed cleanup still stops the session. `COMMAND REJECTED` is a new internal result; the old error list below is a historical inventory with historical line numbers.


Previous compact revision: scale 6 -> 3 (half width and height), line height 50 -> 25 pixels, page interval 5 -> 2 seconds. Diagnostics-detail only. EFI SHA-256 `23b2a9266114b7c0a6f09e1017e15ea92cdd4ff33d22ec94c6d7938785aae23e`. Build, QEMU page capture (12 frames, debug 0 bytes), USB readback and safe eject passed. Current artifact/provenance: `out/nuc5-diagnostics-compact/`. Earlier hashes below describe the preceding revision.

Base: candidate source `01f6efc` (routing and bounded attachment wait retained). The delivered EFI adds **diagnostic-only instrumentation**, so it is not an unmodified-source or byte-identical normal candidate. All new observation/page code is gated by `diagnostics-detail`; no routing policy, attachment deadline, ownership deadline, ABI or USB write policy is changed. Source patch and hashes: `out/nuc5-diagnostics/manifest.json` and `diagnostics-only.patch`. Release build, features=[diagnostics-detail], no qemu-debug. Output: `out/nuc5-diagnostics/BOOTX64.EFI`. Deployed to the dedicated SanDisk on 2026-10-13; EFI readback verified and safely ejected. Provenance: `out/nuc5-diagnostics/physical-usb/provenance.json`.

After the runtime returns, diagnostic pages advance every 2 seconds and repeat, with no keyboard required. Record a full cycle (until the first page repeats). A stalled/invalid timer stops rotation on the current page instead of spinning forever. Successful external ELF execution may halt in the app path rather than return to these pages; this build is for the failing device session.

Photograph the summary USB STEP / FAIL AT / USB ERROR and journal RUN SESSION RESULT. PRE OWN port snapshots include raw PORTSC plus CCS/PED/PP/SPEED/PLS before ownership transfer, and enumeration records include the final attachment-wait sample when that stage is reached. An earlier capability/MMIO failure cannot safely provide a port snapshot. Intel PRE OWN D0/D4/D8/DC are read-only snapshots; POST ROUTE and existing BEFORE/AFTER routing records show the subsequent values when reached. Missing later records mean that stage was not reached, not zero registers.

USBLEGSUP BEFORE/AFTER shows the full register and BIOS owned(bit16)/OS owned(bit24). OWNERSHIP ELAPSED is milliseconds; failure/timeout is logged before returning the existing error. Clock failure is explicitly UNKNOWN. ABSENT / NO HANDOFF means no legacy ownership capability was found. QEMU's non-Intel controller cannot validate Intel registers or real BIOS handoff.

The generic viewer message is not a one-to-one error code: it can also follow an ordinary session end. Missing-keyboard text is selected only for ready discovery, no keyboard, successful run_session and no application input error. Match photographs against FAIL AT plus USB ERROR and RUN SESSION RESULT rather than inferring the cause from the viewer text.


Validation: host 261 passed; diagnostic and normal release builds passed. Normal .text matches the base candidate (full EFI differs; no byte-identity claim). QEMU: 12 frames, six-page automatic cycle, readable PORTSC fields, RUN SESSION RESULT OK, debug 0 bytes. Intel ownership and failure-path screen execution remain unverified on hardware. EFI SHA-256 `3f4240b77d6d6d090d4566d0b1f3236455880f5c2a0a7a2b4b7f1b13a47ff5ff`, 469504 bytes. Evidence: `out/nuc5-diagnostics/validation.json` and `qemu/`.

### Error-string catalogue

Conservative source catalogue of error constructors/mappings in run_session, nested USB modules, PCI recovery, timer and runtime callback. Includes internally handled and feature-only errors; not every entry escapes run_session in this build. Source locations allow checking that distinction. NO CONTROLLER is the initial fallback. This is a photo lookup, not a claim that every error produces the same screen.

| String | Source |
|---|---|
| `ADDRESS STATE` | `boot/src/xhci/usb.rs:422` |
| `APP FILE CACHE` | `boot/src/xhci.rs:403`, `boot/src/runtime.rs:95` |
| `APP INPUT ERROR` | `boot/src/xhci.rs:403`, `boot/src/runtime.rs:72`, `boot/src/runtime.rs:77`, `boot/src/runtime.rs:85`, `boot/src/runtime.rs:89`, `boot/src/runtime.rs:143`, `boot/src/runtime.rs:210` |
| `ASYMMETRIC SPEED` | `boot/src/xhci/usb.rs:82` |
| `BAD COMPLETION` | `boot/src/xhci.rs:562` |
| `BAD TRANSFER` | `boot/src/xhci.rs:601`, `boot/src/xhci/usb/storage.rs:62`, `boot/src/xhci/usb/storage.rs:268` |
| `BAR MEMORY CONFLICT` | `boot/src/pci.rs:1258` |
| `BAR OVERLAP` | `boot/src/pci.rs:1238` |
| `BAR RANGE` | `boot/src/pci.rs:1252` |
| `BAR RESTORE READBACK` | `boot/src/pci.rs:1280` |
| `BOT BUSY` | `boot/src/xhci/usb/storage.rs:116` |
| `BOT CANCELED` | `boot/src/xhci/usb/storage.rs:269` |
| `BOT IDLE` | `boot/src/xhci/usb/storage.rs:146`, `boot/src/xhci/usb/storage.rs:591`, `boot/src/xhci/usb/storage.rs:631` |
| `BOT NO TRANSFER` | `boot/src/xhci/usb/storage.rs:183` |
| `BOT STATUS` | `boot/src/xhci/usb/storage.rs:266` |
| `BOT TAG` | `boot/src/xhci/usb/storage.rs:118`, `boot/src/xhci/usb/storage.rs:244`, `boot/src/xhci/usb/storage.rs:558` |
| `BRIDGE AMBIGUOUS` | `boot/src/pci.rs:1070` |
| `BRIDGE DEPTH` | `boot/src/pci.rs:1098` |
| `BRIDGE DISABLED` | `boot/src/pci.rs:1086` |
| `BRIDGE LOOP` | `boot/src/pci.rs:1042` |
| `BRIDGE MISSING` | `boot/src/pci.rs:1096` |
| `BRIDGE WINDOW` | `boot/src/pci.rs:1090` |
| `BULK BUSY` | `boot/src/xhci/usb/storage.rs:21` |
| `BULK OWNER` | `boot/src/xhci/usb/storage.rs:58` |
| `BULK SIZE` | `boot/src/xhci/usb/storage.rs:24`, `boot/src/xhci/usb/storage.rs:578` |
| `CAPABILITY` | `boot/src/xhci.rs:82` |
| `CAPACITY RANGE` | `boot/src/xhci/usb/storage.rs:379` |
| `CLASS CHANGED` | `boot/src/pci.rs:1247` |
| `CLOCK OVERFLOW` | `boot/src/xhci.rs:52`, `boot/src/xhci.rs:341`, `boot/src/xhci.rs:531`, `boot/src/xhci.rs:881`, `boot/src/xhci/usb/storage.rs:265`, `boot/src/xhci/usb/storage.rs:592`, `boot/src/xhci/usb/storage.rs:618`, `boot/src/xhci/usb/storage.rs:632`, `boot/src/xhci/usb/storage.rs:641`, `boot/src/xhci/usb.rs:110`, `boot/src/xhci/usb.rs:838`, `boot/src/xhci/usb.rs:868`, `boot/src/acpi.rs:299`, `boot/src/acpi.rs:303`, `boot/src/acpi.rs:314` |
| `CLOCK STALLED` | `boot/src/xhci.rs:62`, `boot/src/xhci.rs:102`, `boot/src/xhci.rs:636`, `boot/src/xhci.rs:888`, `boot/src/xhci/usb/storage.rs:264`, `boot/src/xhci/usb.rs:123`, `boot/src/acpi.rs:309` |
| `COMMAND RESTORE READBACK` | `boot/src/pci.rs:1295` |
| `COMMAND RING` | `boot/src/xhci.rs:725` |
| `COMMAND TIMEOUT` | `boot/src/xhci.rs` |
| `CONFIG DIAGNOSTIC CHAIN` | `boot/src/xhci/usb.rs:716` |
| `CONFIG HEADER` | `boot/src/xhci/usb.rs:694` |
| `CONTEXT` | `boot/src/xhci/usb.rs:405` |
| `CURSOR` | `boot/src/xhci/usb/storage.rs:342`, `boot/src/xhci/usb/storage.rs:347`, `boot/src/xhci/usb.rs:843` |
| `DELAY RANGE` | `boot/src/acpi.rs:300` |
| `DESCRIPTOR CHANGED` | `boot/src/xhci/usb.rs:461` |
| `DEVICE DESCRIPTOR` | `boot/src/xhci/usb.rs:431`, `boot/src/xhci/usb.rs:459` |
| `DEVICE LIMIT` | `boot/src/xhci/usb.rs:355`, `boot/src/xhci/usb.rs:542` |
| `DISCONNECTED` | `boot/src/xhci/usb.rs:104`, `boot/src/xhci/usb.rs:114` |
| `DMA ADDRESS` | `boot/src/xhci.rs:31` |
| `DMA DISABLE` | `boot/src/xhci.rs:245` |
| `DMA ENABLE` | `boot/src/xhci.rs:710` |
| `DMA FULL` | `boot/src/xhci.rs:676`, `boot/src/xhci.rs:677`, `boot/src/xhci.rs:678`, `boot/src/xhci.rs:679`, `boot/src/xhci.rs:681`, `boot/src/xhci.rs:683`, `boot/src/xhci/usb/hub.rs:133`, `boot/src/xhci/usb/storage.rs:305`, `boot/src/xhci/usb/storage.rs:306`, `boot/src/xhci/usb/storage.rs:307`, `boot/src/xhci/usb/storage.rs:308`, `boot/src/xhci/usb/storage.rs:309`, `boot/src/xhci/usb.rs:386`, `boot/src/xhci/usb.rs:387`, `boot/src/xhci/usb.rs:388`, `boot/src/xhci/usb.rs:389`, `boot/src/xhci/usb.rs:752`, `boot/src/xhci/usb.rs:753` |
| `DMA POOL` | `boot/src/xhci.rs:671` |
| `DMA RANGE` | `boot/src/xhci.rs:670` |
| `DUPLICATE SHORT EVENT` | `boot/src/xhci.rs:577` |
| `EP0 PACKET` | `boot/src/xhci/usb.rs:433` |
| `EP0 SHORT PROBE` | `boot/src/xhci/usb.rs:470` |
| `EVENT PENDING` | `boot/src/xhci.rs:625`, `boot/src/xhci/usb/storage.rs:92`, `boot/src/xhci/usb.rs:915` |
| `EVENT RING` | `boot/src/xhci.rs:726` |
| `EVENT TYPE` | `boot/src/xhci.rs:615` |
| `EXT CAPABILITY` | `boot/src/xhci.rs:137`, `boot/src/xhci.rs:199`, `boot/src/xhci/usb.rs:25`, `boot/src/xhci/usb.rs:59` |
| `FAT32 READ` | `boot/src/xhci/usb/storage.rs:501` |
| `FILE API TOKEN` | `boot/src/xhci/usb/file.rs:274` |
| `FILE FINISH` | `boot/src/xhci/usb/file.rs:268`, `boot/src/xhci/usb/file.rs:269` |
| `FILE FORMAT` | `boot/src/xhci/usb/file.rs:218` |
| `FILE INDEX` | `boot/src/xhci/usb/file.rs:289` |
| `FILE LIST` | `boot/src/xhci/usb/file.rs:219` |
| `FILE MOUNT` | `boot/src/xhci/usb/file.rs:216` |
| `FILE OPERATION` | `boot/src/xhci/usb/file.rs:248` |
| `FILE OWNERSHIP` | `boot/src/xhci/usb/file.rs:258` |
| `FILE REQUEST` | `boot/src/xhci/usb/file.rs:272`, `boot/src/xhci/usb/file.rs:280` |
| `FILE SESSION LIMIT` | `boot/src/xhci/usb/file.rs:208` |
| `FILE TOKEN` | `boot/src/xhci/usb/file.rs:251` |
| `FULL` | `boot/src/xhci/usb.rs:482` |
| `HIGH` | `boot/src/xhci/usb.rs:483` |
| `HOST ERROR` | `boot/src/xhci.rs:535` |
| `HUB CONTEXT STATE` | `boot/src/xhci/usb/hub.rs:176` |
| `HUB DEPTH LIMIT` | `boot/src/xhci/usb/hub.rs:108`, `boot/src/xhci/usb.rs:545` |
| `HUB DESCRIPTOR` | `boot/src/xhci/usb/hub.rs:117`, `boot/src/xhci/usb/hub.rs:126` |
| `HUB DISCONNECTED` | `boot/src/xhci/usb/hub.rs:221`, `boot/src/xhci/usb/hub.rs:238` |
| `HUB KIND` | `boot/src/xhci/usb/hub.rs:105` |
| `HUB LIMIT` | `boot/src/xhci/usb.rs:531` |
| `HUB OVERCURRENT` | `boot/src/xhci/usb/hub.rs:224`, `boot/src/xhci/usb/hub.rs:241`, `boot/src/xhci/usb.rs:538`, `boot/src/xhci/usb.rs:858` |
| `HUB PORT CONFIG ERROR` | `boot/src/xhci/usb/hub.rs:230`, `boot/src/xhci/usb/hub.rs:244` |
| `HUB PORT DISABLED` | `boot/src/xhci/usb/hub.rs:247` |
| `HUB QUEUE` | `boot/src/xhci/usb.rs:369` |
| `HUB RESET TIMEOUT` | `boot/src/xhci/usb/hub.rs:254` |
| `HUB ROUTE` | `boot/src/xhci/usb.rs:358`, `boot/src/xhci/usb.rs:380` |
| `HUB SPEED` | `boot/src/xhci/usb/hub.rs:91` |
| `HUB SPEED ID` | `boot/src/xhci/usb/hub.rs:36` |
| `HUB STATUS` | `boot/src/xhci/usb/hub.rs:51` |
| `HUB STATUS PACKET` | `boot/src/xhci/usb/hub.rs:128` |
| `INPUT SOURCE CONNECT` | `boot/src/runtime.rs:118` |
| `KEYBOARD DISCONNECTED` | `boot/src/xhci/usb.rs:849`, `boot/src/xhci/usb.rs:865` |
| `LBA RANGE` | `boot/src/xhci/usb/storage.rs:245`, `boot/src/xhci/usb/storage.rs:563` |
| `LIVE STORAGE CONTENT` | `boot/src/xhci/usb/storage.rs:638` |
| `LOW` | `boot/src/xhci/usb.rs:481` |
| `MEMORY DISABLED` | `boot/src/xhci.rs:72`, `boot/src/pci.rs:1120` |
| `MEMORY MAP` | `boot/src/pci.rs:1251`, `boot/src/pci.rs:1254` |
| `MMIO RANGE` | `boot/src/xhci.rs:13`, `boot/src/xhci.rs:21` |
| `NESTED STORAGE LOOP` | `boot/src/xhci.rs:329` |
| `NO CONTROLLER` | `boot/src/xhci.rs` |
| `NO TIMER` | `boot/src/acpi.rs:290` |
| `NOT FOUND` | `boot/src/xhci.rs:68`, `boot/src/xhci.rs:367` |
| `OVERCURRENT` | `boot/src/xhci/usb.rs:107` |
| `OWNERSHIP` | `boot/src/xhci.rs:188` |
| `PEER BOUND CONFLICT` | `boot/src/pci.rs:602` |
| `PEER STATE CHANGED` | `boot/src/pci.rs:594` |
| `PORT DISABLED` | `boot/src/xhci/usb.rs:143` |
| `PORT EVENT` | `boot/src/xhci.rs:611` |
| `PORT RANGE` | `boot/src/xhci.rs:92` |
| `PORT SPEED` | `boot/src/xhci/usb.rs:72`, `boot/src/xhci/usb.rs:93` |
| `PROTOCOL OVERLAP` | `boot/src/xhci/usb.rs:46` |
| `PROTOCOL PORTS` | `boot/src/xhci/usb.rs:38` |
| `QUIESCE FAILED` | `boot/src/xhci.rs:301`, `boot/src/xhci.rs:403`, `boot/src/xhci.rs:802`, `boot/src/runtime.rs:217`, `boot/src/runtime.rs:231`, `boot/src/runtime.rs:240` |
| `REGISTER OFFSET` | `boot/src/xhci.rs:655` |
| `RESET STATE` | `boot/src/xhci.rs:254` |
| `SCRATCHPAD LIMIT` | `boot/src/xhci.rs:662` |
| `SCSI CAPACITY` | `boot/src/xhci/usb/storage.rs:375` |
| `SCSI COMMAND` | `boot/src/xhci/usb/storage.rs:267` |
| `SCSI NOT READY` | `boot/src/xhci/usb/storage.rs:367`, `boot/src/xhci/usb/storage.rs:372` |
| `SCSI READ` | `boot/src/xhci/usb/storage.rs:253`, `boot/src/xhci/usb/storage.rs:582` |
| `SCSI SENSE` | `boot/src/xhci/usb/storage.rs:362` |
| `SHORT CONTROL READ` | `boot/src/xhci/usb.rs:187` |
| `SLOT RANGE` | `boot/src/xhci/usb.rs:393` |
| `STATE RACE` | `boot/src/pci.rs:1264` |
| `STORAGE DISCONNECTED` | `boot/src/xhci/usb/storage.rs:604`, `boot/src/xhci/usb/storage.rs:615` |
| `SUPER` | `boot/src/xhci/usb.rs:484` |
| `TIMEOUT` | `boot/src/xhci.rs:58` |
| `TIMER WIDTH` | `boot/src/acpi.rs:293` |
| `TRANSFER RANGE` | `boot/src/xhci/usb.rs:201` |
| `TRANSFER TIMEOUT` | `boot/src/xhci/usb/storage.rs:263` |
| `UNHANDLED STORAGE LOOP` | `boot/src/runtime.rs:93` |
| `USB PROTOCOL` | `boot/src/xhci/usb.rs:31` |

## 日本語

過去の撮影記録は当時のファイル名 MUSHA.ELF のまま保持します。改名後の候補以降は AUTORUN.ELF を使用します。

### 次回採取：ルーティング後の猶予と1回限定再探索（2026-10-14、未配備）

現ソースは通常150 ms／Intelへ実際に書いた場合500 msの猶予と、機器列挙を一度も試みていない場合のみ有界な再探索1回を追加。[起動時間の説明](usb-startup-timing.md)参照。out/usb-graceはローカル試験用で、公開候補への昇格・USB書込みは未実施。以下の既存ハッシュは過去の採取版を示す。

各NUC5 runは電源断10秒以上とし、PRE OWN D8=0の写真を残す。run番号、電源断秒数、PRE OWN D0/D8、EFIハッシュとprovenance、全USB3ポートのSS ROUTE+ MS（CCS/PED/PP/SPEED/PLS）、WPR、RESCAN結果、Hello/exitまたはHOLDを記録する。時刻の起点はルーティング書込み・読戻し直後で、サンプル間隔は20 ms＋処理負荷。全ページを1周採取する。先頭のT+秒／SESSION／RESCANと「DIAGNOSTIC HOLD (no retry) cycle」で、ページ巡回とUSB再試行を区別する。HOLDは失敗、再起動は別runとして数え、巡回を待てば再試行するとは扱わない。ページ間隔2秒は維持。

| run | 電源断秒数 | PRE OWN D8 | EFI/provenance | 各SSポートCCS/U0までのms | RESCAN | 結果 |
|---|---|---|---|---|---|---|
| 1 | 不明 | 不明 | 未配備 | 未採取 | 未採取 | 未実施 |
| 2 | 不明 | 不明 | 未配備 | 未採取 | 未採取 | 未実施 |
| 3 | 不明 | 不明 | 未配備 | 未採取 | 未採取 | 未実施 |

暫定500 msの妥当性を実測確認してから通常候補へ進む。NUC5冷間3回連続Hello/exit、Ryzen冷間3回＋ホット1回とREAD OK→Helloの秒数を再採取する。以前の操作・電源断時間は不明のままとし、旧冷間3回成功という記録の訂正は所有者承認済み。

### SuperSpeed採取版（2026-10-14）

現在の診断EFIは `d2f5d0c039fb71f864f8e1c2b34958b35d1de8a197bd54a6a3ec73d46eb47348`、`out/nuc5-ss-recovery/BOOTX64.EFI`（diagnostics-detailのみ）。以下の旧版より新しい。媒体なし・MUSHA.ELF不在時は、機器停止後に診断巡回へ戻りビューアーを起動しない。検出済み・拒否されたELFの経路は維持する。

既存の接続待ちの後に、USB3全体で2000ms、20ms間隔の待機を追加。列挙キューへ入れる前にU0・CCS・PEDを要求する。PLS 6/7/10が200ms継続した場合にWPRを発行し、各ルート最大3回。リセット進行中・未給電・過電流・RxDetectのみの場合にはWPRを発行しない。USB2と既存100msデバウンスは変更しない。回復方針は通常版・診断版共通。

SS ROOT / T（USB3待機開始からのms）/ SAMPLE / WPR / WPR READBACK / U0またはTIMEOUTと、PORTSC・CCS/PED/PP/SPEED/PLSを採取する。各ポートの最初の標本と最初の8回までの変化、全WPR・読戻し・最終標本を残す。完全なバストレースではなく上限付き時系列。既存の記録上限512行を超えるとJOURNAL LIMITを表示する。ROOTのqueue/skip、ENUM ROOT PORT RESET、ADDRESS ROOT SLOT、後続のDEVICE DESCRIPTOR / CONFIGURATIONから到達段階を読む。これらのROOT番号は10進（ROOT 12＝要約のROOT 0C）、レジスター値と要約のROOT欄は16進。

文字倍率3・2秒ごとの巡回を維持。冷間起動失敗時は最初のページへ戻るまで全ページを記録する。NUC5でのWPR有効性は未確認であり、QEMUは実機のリンク障害を再現しない。

現行Address Device復旧診断版は `7eba0377ffdc39fea24544a52c8fed86bbf7254c2869d753fceebaffb948676a`（`out/nuc5-address-recovery/BOOTX64.EFI`）。左の状態文字も右のログも倍率3、2秒切替です。失敗時は循環表示し、機器セッション成功後に外部ファイルがある場合は全ページを一巡してからアプリへ進みます（通常版に待機なし）。COMMAND CC/RAW、ADDRESS FAILED ROOT、SLOT DISABLED / CONTINUE、RESET/SLOT SPEED IDを採取してください。対応する拒否応答（コード4/5/7/17/19/22）を一度消費し、Disable Slot成功後にDCBAAを解除して継続します。バッファーは制御器停止まで保持します。応答不整合・タイムアウト・リング停止/中断・後始末失敗はセッション停止を維持します。内部結果 `COMMAND REJECTED` を追加しました。従来のエラー一覧と行番号は以前の版の記録です。


最新版は文字の縦横を半分（倍率6→3）、行間50→25px、ページ切替5→2秒に変更しました。診断版のみの変更です。QEMUで表示を確認し、専用USB更新・読み戻し・安全な取外しを完了しました。成果物と配備記録は `out/nuc5-diagnostics-compact/`。以下の旧ハッシュは前版の記録です。

候補 `01f6efc` を基準に、接続待ちとルーティング処理を保持し、diagnostics-detail限定の観測と終了後の自動ページ表示を追加しています。**候補と完全に同一ソース／同一バイナリーではなく、診断専用の差分があります。** 差分・EFIハッシュは上記manifestに保存。通常版の動作方針、待機期限、ABIは変更していません。専用SanDiskへEFIのみ配備し、読戻し一致・安全な取外しを完了しました（2026-10-13）。

終了後は2秒ごとに診断ページが循環します。キーボード不要です。最初のページに戻るまで撮影してください。時計異常時は表示中のページで停止します。外部ELF成功経路はそのまま停止する場合があり、今回の機器セッション失敗の採取を対象とします。

撮影対象はUSB STEP／FAIL AT／USB ERROR、RUN SESSION RESULT、PRE OWNの各ポートPORTSCとCCS/PED/PP/SPEED/PLS、列挙まで到達した場合の待機後ROOTポート値、IntelのD0/D4/D8/DC、USBLEGSUPの前後値とBIOS/OSビット・経過msです。初期検査で失敗した場合は安全に読めない項目を採取しません。後続記録の欠落をレジスター値0と解釈しないでください。所有権機能なしはABSENT、時計異常はUNKNOWNと明示します。

上表は内部で処理されるもの・試験feature専用を含む保守的なエラー文字列一覧です。全項目が通常のrun_sessionから返るとは限りません。「Device session ended」は正常終了でも表示され得るため、写真のFAIL ATとUSB ERROR、最終RESULTの組合せで照合します。Intel実機のルーティング・所有権移行はQEMUでは未検証です。最終的な待機・切替方針は採取結果を見て決めます。
