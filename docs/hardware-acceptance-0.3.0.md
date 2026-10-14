# v0.3.0 hardware observations

## Next clean candidate: acceptance worksheet (2026-10-14)

Owner decision: include the display grid in 0.3.0 and repeat acceptance. The
local ba23cabb display checks below do not fill these gates. After approval,
build the final pre-build documentation commit; put its full hash and candidate
EFI SHA-256 in every row. Preserve diagnostic-vs-normal build identities.

Cold means power off for at least 10 seconds. NUC5: first collect diagnostic
PRE OWN D8=0 photographs for each diagnostic run, then perform three consecutive
cold boots of the normal candidate. D8=0xF is not cold evidence. A diagnostic
photograph does not measure D8 in a separate normal boot; link paired records
without conflating them. A hold is a failure. Hello and exit 0 must appear in the
same normal boot. Ryzen: cold 3 plus hot 1, recording USB READ OK-to-Hello seconds.
Mac: boot from Basic Data with AUTORUN present is the gate; storage is observation
only. On any failure, capture evidence, fix, and return to pre-build documentation.

| Machine / run | Build | EFI SHA-256 | Source commit | Date/time | Power off seconds | D8 / diagnostic evidence | READ OK to Hello seconds | Result | Evidence file |
|---|---|---|---|---|---|---|---|---|---|
| NUC5 D1 | diagnostic | `0614fbe998836f2eb5f837ca2429b922bedd8668b21db9e2c13bbfe6e65085b0` | `a1762f423516d702bc2d9e00a9425d2b173e00c9` | 2026-10-14; time unspecified | AC unplugged 10 s (owner) | PRE OWN D8=0, frame-016 | — | PASS: Hello / exit 0; diagnostic only | IMG_7645.MOV |
| NUC5 D2 | diagnostic | PENDING | PENDING | PENDING | PENDING | PENDING | — | PENDING | PENDING |
| NUC5 D3 | diagnostic | PENDING | PENDING | PENDING | PENDING | PENDING | — | PENDING | PENDING |
| NUC5 C1 | normal | PENDING | PENDING | PENDING | PENDING | paired D1; not measured in normal | — | PENDING | PENDING |
| NUC5 C2 | normal | PENDING | PENDING | PENDING | PENDING | paired D2; not measured in normal | — | PENDING | PENDING |
| NUC5 C3 | normal | PENDING | PENDING | PENDING | PENDING | paired D3; not measured in normal | — | PENDING | PENDING |
| Ryzen C1 | normal | PENDING | PENDING | PENDING | PENDING | — | PENDING | PENDING | PENDING |
| Ryzen C2 | normal | PENDING | PENDING | PENDING | PENDING | — | PENDING | PENDING | PENDING |
| Ryzen C3 | normal | PENDING | PENDING | PENDING | PENDING | — | PENDING | PENDING | PENDING |
| Ryzen H1 | normal | PENDING | PENDING | PENDING | hot reboot | — | PENDING | PENDING | PENDING |
| Mac B1 | normal | PENDING | PENDING | PENDING | record method | — | — | PENDING | PENDING |

### 次のクリーン候補の受入様式

所有者決定により表示グリッドを0.3.0へ含めて再受入します。下記のba23cabb表示確認を
上表の合格へ転記しません。承認後、ビルド前文書を確定したコミットから生成し、各行に
ソース・候補EFIの完全なハッシュ、日時、電源断秒数、結果、証拠名を記録してください。

冷間は電源断10秒以上。NUC5は診断版の各runでPRE OWN D8=0の写真を残してから、
通常版候補で冷間3回連続成功を確認します。D8=0xFは冷間の証拠に数えません。
診断版の写真は別起動の通常版D8の実測ではないため、対応する記録を紐づけつつ区別します。
ホールドは失敗、Helloと終了0は同一起動内で必要です。Ryzenは冷間3回＋ホット1回と
USB READ OK→Hello秒数、MacはAUTORUNを置いたBasic Data媒体からの起動が条件です。
Macストレージ結果は記録のみ。1回でも失敗したら採取・修正・ビルド前文書確定へ戻ります。


## English

Before accepting the renamed candidate, verify that the selected medium root contains `AUTORUN.ELF` and no legacy `MUSHA.ELF`. The legacy name alone is ignored and starts the viewer. Physical renaming requires owner approval: record the source ELF SHA-256, rename the old file (do not leave both), verify the destination hash matches, and safely eject. This user file is outside update-usb.py management; record old/new paths, EFI identity, hashes and eject result in deployment provenance. No USB operation is performed by this documentation change.

Historical captures retain their original filename, MUSHA.ELF. From the renamed candidate onward, use AUTORUN.ELF.

### Corrected NUC5 acceptance basis (2026-10-14, independent review)

Owner follow-up: exact operations after run 1 and power-off durations for runs 2/3 are not remembered and remain UNKNOWN. The owner approves the corrected record below. This resolves approval of the correction, not the missing run evidence.

One cold-start NO MEDIUM outcome is observed in IMG_7633 (run 1, PRE OWN D0=0x300 / D8=0). One Hello / exit 0 success is observed in IMG_7634 (run 3, PRE OWN D0=0x7FF / D8=0xF already routed). Three consecutive cold-start successes are NOT proven. The owner's report of three successes is retained as an owner report, separately from video evidence. Run 2 remains uncaptured; operations after run 1 and power-off durations for runs 2/3 are UNKNOWN because the owner does not remember them precisely. The owner approved this correction. A restart between diagnostic hold and Hello is an interpretation, not an observed fact. The review's most likely cause is immediate empty classification after routing/PP, before attachment settles.

| Run | Evidence | PRE OWN D8 | Observed outcome | Cold acceptance |
|---|---|---|---|---|
| 1 | IMG_7633 | 0x00000000 | NO MEDIUM; diagnostic hold | Fail |
| 2 | Owner report | Unknown | Not captured | Pending |
| 3 | IMG_7634 | 0x0000000F | Hello / exit 0 | Not established |

For the next acceptance series: power off at least 10 seconds; photograph PRE OWN D8=0 for every NUC5 cold run. Record D0/D8, power-off duration, routing-to-CCS/U0 milliseconds, attachment/SS wait, WPR, rescan and Hello timing. Diagnostic hold counts as failure; any power operation begins a new run. Require three consecutive successes in the same boot per run.


NUC5 clarification and third-run evidence (2026-10-14): owner identifies IMG_7633.MOV as run 1's diagnostic loop and IMG_7634.MOV as run 3's success. The latter clip (36.967 s) directly shows MUSHA.ELF / Hello Musha-OS! / exited: status 0. At 18.483 s, PRE OWN P12 PORTSC=00001203 (CCS1 PED1 SPEED4 PLS0), D0=000007FF and D8=0000000F; ownership elapsed 0 ms. In run 1, captured ROOT12 is disconnected and DIAGNOSTIC HOLD NO MEDIUM repeats. Clip-to-run mapping is now resolved. Owner reports eventual Hello success for all three; run 1's transition from terminal hold to Hello remains unexplained by current code and is not captured. Do not classify these as three clean, uniform cold starts or dismiss the loop as cosmetic. Delayed connection after routing is a hypothesis, not a proven cause. Both clips are associated with the preceding f10672d4 diagnostic deployment. No binary/USB changes made for this observation.


Owner clarification: Ready means Hello Musha-OS! / exited: status 0. Historical owner report: three Hello successes; cold-start qualification is not established (superseded by the corrected basis above). The association between the clip's NO MEDIUM run and those three successes remains unresolved; the clip is not direct evidence of Hello. Diagnostic hold does not retry USB, so progression from that hold to Hello within one boot is unexplained.


NUC5 IMG_7633.MOV (2026-10-14): owner reports cold 3/3 successful, with two page cycles on runs 1/2 and one on run 3; meaning of Ready/success awaits clarification. Video (44.253 s) shows DIAGNOSTIC HOLD NO MEDIUM, SCANNED K0 S0, USB STEP STOPPED DMA DISABLED and repeated 9-page diagnostics, not Hello/exit. ROOT 12-15 are CCS0/PED0/PLS5 at enumeration; ownership BIOS1/OS0 to BIOS0/OS1 completes in 0 ms. This is a captured no-medium outcome, so three Hello successes are NOT accepted. Associated EFI f10672d4a89bf9d9c56206ff24cc4f885f6b0c2df13e4c43832aa0c4c1e5ab20 follows preceding deployment, not a video hash measurement. Code diagnostic_capture calls diagnostic_pages(false) after no external app, repeating saved pages without USB retries. Suspect immediate empty classification after Intel routing may miss delayed attachment; causality remains unproven. Preserve this failure, do not promote the optimized normal candidate. Evidence: out/nuc5-7633/observation.json and sample-3/6/7.png.


### Ryzen 7: optimized diagnostic cold 3/3 and hot 1/1 (2026-10-14)

Owner reports three successful cold boots and one successful hot reboot with IMG_7632.MOV. Associated diagnostic EFI: f10672d4a89bf9d9c56206ff24cc4f885f6b0c2df13e4c43832aa0c4c1e5ab20, via the immediately preceding out/usb-empty/physical-usb/provenance.json deployment; not a hash measured from the video. The 7.385-second clip shows starting, runtime, USB READ OK (sample at 3.6925 s), GOP 3840/2160/3840, XHCI 5, and diagnostic page 1/23 at 6.885 s with C1 INPUT DONE K1 S1. It does not include the final Hello/exit screen or PM EPOCH TIMINGS pages. Repetition counts and success are owner-reported; no post-change READ OK-to-Hello duration or controller timing can be measured from this clip. Evidence: out/ryzen-7632/observation.json and frames. Ryzen diagnostic repetition check passes by owner report; NUC5 optimized-diagnostic cold 3/3 and optimized normal-candidate acceptance remain pending.


Ryzen diagnostic timing result (2026-10-14): IMG_7630.mov confirms Hello / exit 0. Six controller passes spend 17996 ms in attachment/SuperSpeed waits; ELF read 183 ms and quiesce 0–6 ms. [Measured table and subsequent optimization](usb-startup-timing.md). New revision hardware acceptance is pending.

### Ryzen 7: normal candidate cold 3/3 and hot 1/1 successful; slow startup (2026-10-14)

Owner reports cold boots 3/3 and hot reboot 1/1 successful, but a long wait for the final message. IMG_7628 2.mov (26.807 seconds) confirms one visible MUSHA.ELF (Ring 3) / Hello Musha-OS! / exited: status 0 execution; the repetition counts and boot classifications are owner-reported. Associated normal EFI 5e4af31ece007550ba4d41a8a5c58d8b7a3b493e8665f277b7817134dd8911af follows the preceding deployment/test sequence, with out/nuc5-accepted-normal/physical-usb/provenance.json; this is not a hash measured from the video.

Video-relative samples: at 4 seconds runtime / DEVICE DISCOVERY, XHCI 5 and GOP 3840/2160/3840; at 8 seconds USB READ OK / USB BYTES 72AE00000; at 24 seconds still the discovery screen; by 26.307 seconds Hello and exit 0. Thus at least 16 seconds pass after the sampled USB READ OK before the final screen. These are sample bounds, not exact transition times or power-on timings. The static PAGING / ARENA panel does not establish the active delay stage.

Code inspection identifies a likely contributor: enumerate() waits up to one second for attachment and two additional seconds for SuperSpeed readiness, including empty ports. run_session() scans controllers sequentially and subsequently reinitializes the selected controller. Five controllers can therefore accumulate waits; exact time attribution and any other delays remain unmeasured. Preserve the accepted EFI; no timeout reduction or USB rewrite was performed. Evidence: out/ryzen-7628/observation.json and frames. NUC5 and Ryzen normal-candidate cold-boot checks now pass; Mac Basic Data boot, final packaging and current-source remote CI remain pending. Startup latency is tracked separately in open-issues-0.3.0.md.

### NUC5: normal candidate cold boots 3/3 successful (2026-10-14)

Owner reports three successful cold boots on NUC5i5RYH with IMG_7627.MOV. The 7.82-second video shows GOP 1600/1200/1600 and MUSHA.ELF (Ring 3), Hello Musha-OS!, exited: status 0. The video confirms one visible successful execution; the three-boot count and cold-boot classification are owner-reported.

Associated feature-free normal EFI: 5e4af31ece007550ba4d41a8a5c58d8b7a3b493e8665f277b7817134dd8911af, via the immediately preceding USB deployment in out/nuc5-accepted-normal/physical-usb/provenance.json. This association follows the deployment/test sequence, not a hash measured from the video. Evidence: out/nuc5-7627/observation.json and sampled frames. NUC5's normal-candidate three-cold-boot check passes; Ryzen normal-candidate cold 3/3 and Mac Basic Data boot remain pending. Final packaging and current-source remote CI remain pending. NUC5 RAM/firmware UNKNOWN fields remain unchanged. The dated entries below preserve the status at the time of each earlier observation.

### NUC5: three diagnostic cold boots successful (2026-10-14)

Owner reports 3/3 cold boots successful (IMG_7626.MOV), following deployment of diagnostic EFI d2f5d0c039fb71f864f8e1c2b34958b35d1de8a197bd54a6a3ec73d46eb47348. Association is based on the current test sequence and out/nuc5-ss-recovery/physical-usb/provenance.json, not a hash measured from the video. The 28.395-second clip ends on diagnostic page 11/11: STORAGE BOT / CAPACITY, STORAGE SECTOR READ, FAT32 FILE READ, KEYBOARD POLLING, STOPPED DMA DISABLED, RUN SESSION RESULT OK. The sampled ending does not show the subsequent Hello/exit screen; three successful cold boots are owner-reported.

At 19 seconds, SS ROOT 12 T 2000 U0 TRY 0, PORTSC 00201203 (CCS1/PED1/PP1/SPEED4/PLS0). Unconnected roots 13-15 finish TIMEOUT TRY 0 with CCS0/PLS5; these are not evidence of a connected device failure. The observed root-12 success uses no WPR. Earlier sample shows D0/D4/D8/DC 7FF/4FF/F/F and ownership BIOS1/OS0 -> BIOS0/OS1, elapsed 0 ms at displayed resolution. Do not infer WPR caused the successful boots.

Together with Ryzen's owner-confirmed diagnostic cold 3/3 and hot 1/1, this clears the diagnostic retest gate. The normal candidate is rebuilt at 5e4af31ece007550ba4d41a8a5c58d8b7a3b493e8665f277b7817134dd8911af (feature-free, local uncommitted source); QEMU evidence and manifest are in out/nuc5-accepted-normal/: 26 cases per Basic Data/ESP (52 total) passed on this exact EFI, including unchanged absent pixels and debug 0 bytes. EFI-only USB update, readback and safe eject completed; physical-usb/provenance.json records the deployment. Host269 and broader QEMU99 from the same-source preceding tests were not repeated in full; C1/C2/C3 were not rerun for this change. Normal-candidate physical cold 3/3 on NUC5 and Ryzen, and Mac Basic Data boot, remain pending. Previous failure records remain historical evidence.

### Ryzen 7: three cold boots and one hot reboot successful (2026-10-14)

Owner reports **3/3 cold boots and 1/1 hot reboot successful**, with IMG_7622.mov. The 66.975-second video samples show diagnostic page 5/20 and the final MUSHA.ELF (Ring 3), Hello Musha-OS!, exited: status 0. The counts and reboot classification are owner-reported; the supplied clip establishes one visible successful execution, not four independently observed boot cycles. C1 reports INPUT DONE K1 S1 in the sampled summary.

Owner confirmed on 2026-10-14 that all four successful runs used the latest deployed diagnostics-detail EFI d2f5d0c039fb71f864f8e1c2b34958b35d1de8a197bd54a6a3ec73d46eb47348. Deployment/readback provenance: out/nuc5-ss-recovery/physical-usb/provenance.json. This association is owner-confirmed, not a hash measurement from the video. Do not assign this result to a normal release candidate or treat it as proof of the USB3 recovery's causal effectiveness. NUC5 diagnostic three-cold-boot gate and subsequent normal-candidate acceptance remain pending. Evidence: out/ryzen-7622/observation.json and frames.

### Previous-image cold reboot failure: IMG_7619.mov (2026-10-14)

Owner reports failure on a cold reboot using the previous image. Treated as the preceding Ryzen 7 test context (machine not explicitly restated). The 14.563-second video shows GOP 3840/2160/3840, PCI DEVICES 34 (hex), XHCI 5, INTEL LAN 0, CORE READY and TIMER READY. At approximately 7.3–12 seconds, USB READ OK and USB BYTES 72AE00000 are visible. By 14.1 seconds the viewer shows No medium / 0 files and No supported FAT32/exFAT medium. No Hello/exit-0 is observed.

Source correlation: USB READ OK in boot/src/xhci/usb/storage.rs follows capacity and first/last-sector reads; it does not guarantee a mounted FAT32/exFAT service. Therefore this is evidence that storage communication worked at one stage, not an all-ports-undetected result. A later re-enumeration, medium-selection or filesystem/service failure remains possible; exact stage and PORTSC are unavailable because the viewer hides diagnostics. The static PAGING / ARENA panel is not proof of a fault at that stage. Keyboard detection cannot be established from the generic viewer wording.

Previous EFI association remains provisional (preceding deployment 7eba0377..., provenance out/nuc5-address-recovery/physical-usb/provenance.json); no hash is attested by this video. Latest d2f5d0c... is not tested here. Earlier success and this failure demonstrate that the reported previous-image runs are not uniformly successful; no three-success acceptance is established. Evidence: out/previous-7619/observation.json and extracted frames. Next capture should use the already-deployed diagnostic-hold revision to retain the failed run's journal; do not assume the new USB3 recovery resolves this distinct symptom.

### Ryzen 7 previous-image cold boot: IMG_7616.mov / IMG_7618.jpg (2026-10-14)

Owner reports a cold boot on the Ryzen 7 machine using the **previous image**. The supplied photograph shows `MUSHA.ELF (Ring 3)`, `Hello Musha-OS!`, and `MUSHA.ELF exited: status 0`. Sampled video frames (0, 26.276, 52.052 seconds; duration 52.552 seconds) show the starting screen and diagnostic pages 7/20 and 20/20. Page 7 shows five xHCI controllers; controller C1 reports INPUT DONE K1 S1. The sampled end frame is still diagnostic; final application success is established by the photograph, not that video frame.

Page 20 contains `JOURNAL LIMIT / MORE OMITTED`: later journal entries were not retained. This is a capture completeness limitation, not evidence of application failure. Evidence and media hashes: `out/ryzen-7616/observation.json` and extracted frames.

The preceding deployed diagnostic EFI was `7eba0377ffdc39fea24544a52c8fed86bbf7254c2869d753fceebaffb948676a` (`out/nuc5-address-recovery/physical-usb/provenance.json`). Association to this run is provisional from "previous image"; the media do not attest an EFI hash. Latest `d2f5d0c...` diagnostic is not covered. This confirms a reported cold-boot success; three consecutive cold boots and normal-candidate acceptance remain unconfirmed.

### Intermittent cold boot: IMG_7612.jpg (2026-10-14)

Owner reports three NUC5i5RYH cold boots with the preceding diagnostic: success / no supported FAT32/exFAT medium (keyboard detected) / success. This does not pass the three-success acceptance gate. Associated previous EFI is `7eba0377ffdc39fea24544a52c8fed86bbf7254c2869d753fceebaffb948676a` via `out/nuc5-address-recovery/physical-usb/provenance.json`; association is based on deployment and owner report, not a hash measured from the photo. The photo confirms the no-medium viewer; it cannot identify PORTSC or distinguish link failure from later enumeration/read failure. ROOT 0C SanDisk SuperSpeed is the owner's suspected intermittent path.

Next diagnostic: `d2f5d0c039fb71f864f8e1c2b34958b35d1de8a197bd54a6a3ec73d46eb47348`, `out/nuc5-ss-recovery/BOOTX64.EFI`. Its USB3 recovery and diagnostic hold are implemented but not hardware-verified. First repeat three diagnostic cold boots; capture a complete diagnostic cycle on failure. Only after all three pass, rebuild the normal candidate and repeat three cold boots. Existing successful development runs do not substitute for this gate.

### NUC5 diagnostic success: IMG_7611.MOV (2026-10-14)

Owner-provided NUC5i5RYH video (25.492 s) shows all nine diagnostic pages followed by `MUSHA.ELF  (Ring 3)`, `Hello Musha-OS!`, and `MUSHA.ELF exited: status 0`. One successful diagnostic execution observed; power-off/cold-boot procedure, repeated stability and keyboard key input are not established by this clip. Normal-release candidate acceptance remains incomplete.

Associated EFI SHA-256: `7eba0377ffdc39fea24544a52c8fed86bbf7254c2869d753fceebaffb948676a`, features `diagnostics-detail`, via `out/nuc5-address-recovery/physical-usb/provenance.json`. This is deployment/owner-report association, not a hash measured by the video. Frames and video hash: `out/nuc5-address-recovery/nuc5-7611/`.

- GOP 1600/1200/1600; xHCI 8086:9CB1 / BDF 00A0. Root 2 / slot 1: Lenovo 17EF:6009, Low, speed ID 2; root 7 / slot 2: 8087:0A2A, Full, ID 1; root 12 / slot 3: SanDisk 0781:55A9, Super, ID 4. Post-reset names, Address Device speed and Slot Context speed agree. K1 S1; FAT32 file-read stage and `RUN SESSION RESULT OK` are visible.
- D0/D4/D8/DC already read `7FF/4FF/F/F` before routing and remain unchanged after it; `WRITES 3 MATCH 1`. Ports 2/7/12 were connected before ownership handoff. Thus this run does not demonstrate recovery from the earlier `300/4FF/0/F` state or isolate the routing/wait changes as the cause of success.
- USBLEGSUP `00010801` (BIOS1/OS0) -> `01000801` (BIOS0/OS1), OK, elapsed 0 ms at displayed resolution. Attachment wait reports 1000 ms.
- Root 1 is disconnected and skipped. No Address Device rejection or BAD COMPLETION is visible; the new continue-after-rejection path was not exercised on hardware in this run. The old root-1 failure's completion code remains unknown.

### Address Device recovery candidate (2026-10-13; hardware pending)

Owner decision following IMG_7609: retain Intel routing in the normal build, modify only D4/DC-selected bits, continue other roots after a safely completed Address Device rejection. Observed D0/D4/D8/DC `300/4FF/0/F -> 7FF/4FF/F/F`, roots 1/2/7 connected, root 1 slot 1 stopped at Address Device remain the historical evidence; this does not establish a controlled routing A/B comparison.

Current normal EFI `2d8b3a0669108cc6ece04c696dc84292e180a1f6bc2e85f185695ef9a168a09f`; diagnostic EFI `7eba0377ffdc39fea24544a52c8fed86bbf7254c2869d753fceebaffb948676a`. Source: `01f6efc` plus uncommitted patch; `out/nuc5-address-recovery/manifest.json` and `source.patch`. Normal QEMU 26 cases per GPT type, viewer/cooperative 38, error-injection 2 plus diagnostic-injection 2 passed; host 263. Actual NUC5 completion code and cold-boot acceptance remain pending. The prior speed listing is pre-reset; new records show before/after PORTSC, post-reset speed ID/name and Slot Context speed ID without changing the speed conversion.


### Compact diagnostic result: IMG_7609.MOV (2026-10-13)

NUC5 video (21.873 s), eight final pages. Associated deployed EFI SHA-256 `23b2a9266114b7c0a6f09e1017e15ea92cdd4ff33d22ec94c6d7938785aae23e`, via `out/nuc5-diagnostics-compact/physical-usb/provenance.json`; the video itself does not attest the hash or cold-boot procedure. Observations/frames: `out/nuc5-diagnostics-compact/nuc5-7609/`.

- Intel xHCI 8086:9CB1 / BDF 00A0. Failure: `FAIL AT USB STEP SLOT / ADDRESS`, `USB ERROR BAD COMPLETION`, `RUN SESSION RESULT BAD COMPLETION`. Target root 01, slot 1, speed High. External Hello not reached.
- USBLEGSUP `00010801` (BIOS 1 / OS 0) -> `01000801` (BIOS 0 / OS 1), result OK, elapsed display 0 ms (millisecond resolution, not proof of instantaneous transfer).
- D0/D4/D8/DC before: `00000300 / 000004FF / 00000000 / 0000000F`; after: `000007FF / 000004FF / 0000000F / 0000000F`. WRITES 3 is a two-register bitmask, not three writes. MATCH 0 is an exact-value mismatch at D0; all D4 mask bits are nevertheless set in D0. Do not equate this with routing completely failing or explain extra bits without further evidence.
- PRE OWN and immediate route snapshots: ports 1-15 show `000002A0`. After reset/start and ATTACH WAIT 1000 MS, root 1 and 7 show `000206E1` (CCS1/PED0/PP1/SPEED1/PLS7); root 2 shows `00020AE1` (CCS1/PED0/PP1/SPEED2/PLS7). Other roots are disconnected. Thus this attempt is not an all-ports-undetected failure. These speeds precede port reset; target High is the later reset result.
- Source correlation: target slot is recorded after Enable Slot; the subsequent SLOT / ADDRESS stage submits Address Device before DEVICE DESCRIPTOR. The failure is therefore in validation of the Address Device command completion. Current `command_completion` combines completion code, command pointer, slot and control-bit checks; the video does not identify which check failed. Next diagnostic needs raw event words, completion code, expected/returned command pointer and slot, plus submitted input context. Keep routing/wait policy provisional; one observation does not prove either change caused recovery.


### Owner clarification: candidate 41a61efc (2026-10-13)

Candidate `41a61efc` cold boot: failed. Waiting 30 seconds in the firmware boot menu produced the same result. Keyboard was not connected. This later owner clarification supersedes the earlier keyboard-attached statement for this attempt; it does not establish the keyboard configuration for IMG_7607.MOV.

### Intel-routing candidate: NUC5 unsuccessful attempt (IMG_7607.MOV)

Owner supplied this NUC5 result on 2026-10-13. The 8.052-second video shows the UEFI USB boot-menu entry, runtime / CORE READY / DEVICE DISCOVERY, then the viewer with `No medium | 0 files` and `Device session ended. Restart to try again.` External Hello and exit 0 are not observed. GOP is 1600/1200/1600; PCI E, XHCI 1, Intel LAN 1, TIMER READY. This does not pass candidate acceptance. Whether this was a cold boot is not established by the clip.

Associated deployed EFI: `cb0f532d24fb12c238069cec2b906c74f11fdb91b9745855ab5079c86196f23c`, source `01f6efc`, via `out/intel-routing/physical-usb/provenance.json`. This association is based on the preceding USB update and owner report; the video does not independently attest the hash. Sampled evidence: `out/intel-routing/nuc5-7607/` (start/middle/end and 4.5–7.0 seconds at 0.5-second intervals).

The generic session-ended message does not establish successful keyboard detection, all-port disconnection, or a missing MUSHA.ELF file. `runtime.rs` marks a missing keyboard only after successful ready discovery without input/application error; other endings retain this generic text. Routing register snapshots, PORTSC and the actual session error are not visible here. Routing plus bounded attachment wait has therefore not demonstrated recovery; a diagnostic build showing those records is the next discriminating test. No additional USB write was performed for this observation.

### Intel-routing candidate — preparation record (before IMG_7607.MOV)

Source `01f6efc859646fada7ef28fffa9322d266245469`, release features=[], 460,288 bytes. EFI SHA-256 `cb0f532d24fb12c238069cec2b906c74f11fdb91b9745855ab5079c86196f23c`; manifest `out/intel-routing/candidate/build-manifest.json`. Full QEMU regression passed 186 cases; USB EFI-only update/readback/safe eject completed (`out/intel-routing/physical-usb/provenance.json`). This is a new candidate after the failure below, not a claim that the cause has been identified or fixed. NUC5/Ryzen cold boots and Mac Basic Data boot must use the new hash.

### Candidate 4319b58 NUC5 cold boot failure (owner report, 2026-10-13)

EFI SHA-256 `41a61efcd96edf6142bf81a37ae2ca90ca4396ea71386da698f8cac8bccd2ae8`, associated with `out/release-display/physical-usb/provenance.json`. Owner reports a cold boot with both USB keyboard and SanDisk attached but no xHCI port detecting a device. This is a failed candidate acceptance attempt, not three successful cold boots. `IMG_7606.mov` is 1.975 seconds; sampled frames show the runtime/CORE READY stage, not the later PORTSC inventory. Thus the all-ports-disconnected detail is the owner's report, not independently visible in this short clip. Frames: `out/intel-routing/video/`. Intel shared-port routing is a hypothesis, not a confirmed cause. This result supersedes “no attempt yet” for this EFI; earlier entries below retain their historical state.

### New boot-display candidate — hardware NOT PERFORMED (2026-10-13)

Source `4319b58dda03400032c0307eaaa3b85fe8150ac0`; release profile, features=[]; EFI SHA-256 `41a61efcd96edf6142bf81a37ae2ca90ca4396ea71386da698f8cac8bccd2ae8` (455,680 bytes). Manifest: `out/release-display/candidate/build-manifest.json`. This replaces the candidate for the next acceptance session, not the historical observations below. QEMU full regression passed 186 cases (`out/release-display/report.json`); USB EFI-only deployment, readback and safe eject completed (`out/release-display/physical-usb/provenance.json`). NUC5 and Ryzen three cold boots each, and MacBook Pro 2018 Basic Data boot, must be performed after the new USB update. No successful physical boot of this EFI is recorded yet.

### Current development observations (owner report, 2026-10-13)

These results are the owner's supplied interpretation by Claude of the videos below. EFI SHA-256 for the prepared Basic Data USB: `b743ac0129a9692adf865c37c33adb2c23ca2c0238baf5b9437ebe20faee2b1a`. Each session has a retrospective provenance record under `docs/hardware-evidence/2026-10-13/{nuc5,ryzen7,macbook-pro-2018}/provenance.json`, linked to `out/physical-basic-data-a3153d2/result.json` and its verified file update. The videos do not independently attest the hash; this is the deployment/session association, not a fresh EFI measurement.

| Machine / evidence | Observations | Acceptance boundary |
|---|---|---|
| NUC5i5RYH / IMG_7602.mov | GOP 1600/1200/1600; PCI E, XHCI 1, Intel LAN 1; TIMER READY; IMAGE 0x14000000 SIZE 0x10C8000; TABLE 0x9D170000; MMIO 0 BASE 0xAA120000. MUSHA.ELF → Hello Musha-OS! → exited: status 0. Diagnostic overlap resolved. Boot menu now shows `UEFI : USB : USB : PART 0 : OS Bootloader`. | Ring 3 Hello success. Board/CPU details, RAM, firmware, Secure Boot and topology remain UNKNOWN pending owner response. Old development EFI had two No medium observations; not reproduced on this current build. |
| Ryzen 7 / IMG_7603.mov | GOP 3840/2160/3840; PCI 34, XHCI 5, Intel LAN 0; USB READ OK; USB BYTES 72AE00000; MMIO 4 BASE 0xFCC00000. Hello → exit 0 reported. | Ring 3 Hello success reported; final video is blurred, still-image supplement pending. Native-scale console is too small at 4K. Exact machine details remain unrecorded. |
| MacBook Pro 2018 / IMG_7604 2.mov, IMG_7605.MOV | GOP 3360/2100/3584; PCI 23, XHCI 3, Intel LAN 0; MMIO 2 BASE 0x8F900000. External keyboard attached: KEYBOARD READY. Without it: USB keyboard not detected. Both show No medium / No supported FAT32/exFAT medium. | Boot/display and external-keyboard detection only; no external Hello success. Internal keyboard unsupported because T2 is unimplemented. Storage cause unknown, deferred to v0.4.0 or later by owner. Historical 10/12 rc1 FAT32 success is not the current result. |

**Cold boot ×3: NOT PERFORMED; repeat on the public candidate EFI on NUC5 and Ryzen.** Mac candidate acceptance requires Basic Data boot only, not storage recovery. Current development observations do not satisfy that future candidate gate. Network detection is not physical network communication. See [open issues](open-issues-0.3.0.md).

### Historical earlier development EFI (before GOP/Basic Data)

Received 2026-10-13 JST from the owner. **One successful external Hello execution is observed; hardware acceptance is incomplete.** Two attempts were reported, not three successful cold boots. The exact power-off/reset procedure is not yet recorded.

| Field | Observation |
|---|---|
| Machine | NUC5i5RYH (owner report); exact board/CPU, RAM, firmware version UNKNOWN |
| Firmware / display | Video begins with UEFI USB boot selection; Dell display visible. Secure Boot state, cable and GOP resolution UNKNOWN |
| USB / sector size | Prepared SanDisk 3.2Gen1, 30,784,094,208 bytes, 512-byte sectors; actual port/hub topology UNKNOWN |
| Keyboard | Model and connection during each attempt UNKNOWN; input in Musha-OS not verified |
| Source / build | Prepared source `35ea93371d8a3fc4af5f2ea82d7f9c1be6c97ae7`, `release/0.3.0`, clean build, release profile, features=[] |
| EFI | 458,752 bytes; SHA-256 `fb54af2af871bd79e4dd3803c37308c686fa2e2fc3fac11299c0150a09bc9f13` |
| MUSHA.ELF | SDK C Hello, 13,312 bytes; SHA-256 `d60006f88e1e36409867546df31c778cf3d5fb7c077fba376ebb5bd3fbb2810f` |
| Deployment | Changed-file update, old EFI backed up, file readback verified and safely ejected. `out/nuc5-0.3.0-35ea933/provenance.json`, `verified-files.json`, `usb-backup/update-result.json` |
| Whole USB hash / image | Not generated or hashed; existing GPT/FAT32 retained |

The hashes identify the prepared USB files, not values measured from the photographs.

| Attempt / check | Result | Evidence |
|---|---|---|
| First reported attempt | FAIL to reach external Hello | `IMG_7596.HEIC`: viewer shows `No medium | 0 files` and `USB keyboard not detected. Connect one, then restart.` |
| Second reported attempt | PASS external file → Hello output → exit 0 | `IMG_7600.jpg`: `MUSHA.ELF`, `Hello Musha-OS!`, `MUSHA.ELF exited: status 0` |
| Video observation | Supports second result | `IMG_7598.MOV`, duration 11.67 s; sampled start shows UEFI USB selection, middle and end show Hello/exit 0 |
| Cold boot ×3 / repeatability | NOT COMPLETE | First and second outcomes differ; no three-success record |
| Keyboard input / exFAT / network / fault injection | NOT RUN for this hardware session | Do not infer these from Hello |

Original evidence is under the owner's `/Users/hirofumiiwasaki/Downloads/`; local viewing conversions are under `out/nuc5-font-review/` (not committed). The video was inspected at sampled frames, not used as a boot-time benchmark.

First-attempt diagnosis: the OS viewer had no usable medium and did not report a keyboard. This is not evidence of an ELF validation failure or of a successful filesystem read with an absent ELF. USB enumeration/controller ownership/media setup and physical topology remain possible investigation areas; this screen alone does not identify the failing layer. Do not claim a fix or stable NUC support. Next record: unchanged USB/keyboard/port configuration, whether cold or warm boot, outcome for each attempt, RAM and firmware details, and a full photograph on failure.

### Font adjustment after the observed build

The owner requested smaller log text. The local follow-up changes only external-console rendering from scale 2 to native scale 1 (6×8 cells); the 80×24 model, wrapping, replacement bytes and ABI remain unchanged. Existing viewer rendering is unchanged. This smaller-font build is **not the hardware-tested build above**, is not written to USB, and is not yet committed/pushed.

Local validation: `out/nuc5-font-review/host-tests.log` (10 framebuffer tests), `report.json` (seven normal-EFI QEMU cases: Hello, long log, UTF-8 replacement, exit 42, PF, UD, absent viewer). The absent case retains the existing pixel reference and zero debug bytes. These are local checks, not additional physical boots.

## 日本語

改名後候補の受入前に、選択媒体ルートが `AUTORUN.ELF` で旧 `MUSHA.ELF` が残っていないことを必ず確認します。旧名だけでは不在扱いでビューアーへ進みます。実USBの改名は所有者承認後に、元ELFのSHA-256を記録し、旧名を残さず改名、変更後ハッシュ一致を確認して安全に取り外します。この利用者ファイルはupdate-usb.py管理対象外です。旧／新パス・EFI識別情報・ハッシュ・取り外し結果を配備provenanceへ記録します。本変更ではUSB操作は行いません。

過去の撮影記録は当時のファイル名 MUSHA.ELF のまま保持します。改名後の候補以降は AUTORUN.ELF を使用します。

所有者の追加回答（2026-10-14）：run 1後の操作とrun 2・3の電源断時間は正確には覚えていないため不明。記録を「冷間NO MEDIUMを1回、D0/D8切替済み状態でHelloを1回確認。冷間3回連続成功は未証明」と訂正することは承認済み。記録訂正への承認と、欠けている実機証拠を区別する。

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


Ryzen診断計測（2026-10-14）：IMG_7630.movでHello・終了0を確認。6回の制御器処理で接続／SS待ち計17996ms、ELF読込み183ms、停止0～6ms。[実測表と短縮変更](usb-startup-timing.md)。新しい変更後の実機受入は未実施。

### Ryzen 7：通常版候補の冷間3/3・ホット1/1成功、起動待ち時間あり（2026-10-14）

所有者報告は冷間起動3/3・ホットリブート1/1成功。ただし最終メッセージまで長く待つ。IMG_7628 2.mov（26.807秒）でMUSHA.ELF (Ring 3)／Hello Musha-OS!／exited: status 0を確認。動画で確認できるのは1回の成功で、反復回数・起動条件は所有者申告。通常版EFI 5e4af31ece007550ba4d41a8a5c58d8b7a3b493e8665f277b7817134dd8911afへの関連付けは直前の配備・試験順序とout/nuc5-accepted-normal/physical-usb/provenance.jsonに基づき、動画から測定したハッシュではない。

動画内4秒時点でruntime／DEVICE DISCOVERY、XHCI 5、GOP 3840/2160/3840。8秒時点でUSB READ OK／USB BYTES 72AE00000、24秒時点でも探索画面、26.307秒時点でHello・終了0。USB READ OK確認後も少なくとも16秒間、最終画面へ進んでいない。これは抽出時点による範囲であり、正確な遷移時刻や電源投入からの時間ではない。固定のPAGING／ARENA表示から遅延箇所は特定できない。

コード上の有力要因はenumerate()の接続待ち最大1秒＋SuperSpeed待ち最大2秒。未接続ポートも対象で、run_session()は制御器を順次探索後、選択した制御器を再初期化する。5制御器では待ち時間が累積するが、各処理の実測内訳・他の遅延要因は未確認。受入済みEFIを保持し、待機短縮やUSB更新は行っていない。証拠はout/ryzen-7628/observation.jsonと抽出画像。NUC5・Ryzenの通常版候補冷間起動確認は合格、MacのBasic Data起動・最終梱包・現行ソースのリモートCIは未完了。起動時間はopen-issues-0.3.0.mdへ別課題として記録。

### NUC5：通常版候補の冷間起動3/3成功（2026-10-14）

所有者よりNUC5i5RYHの冷間起動3回成功とIMG_7627.MOVを受領。7.82秒の動画でGOP 1600/1200/1600、MUSHA.ELF (Ring 3)、Hello Musha-OS!、exited: status 0を確認。動画から確認できるのは1回の成功で、3回という回数と冷間起動の条件は所有者申告による。

対応するfeatureなし通常版EFI：5e4af31ece007550ba4d41a8a5c58d8b7a3b493e8665f277b7817134dd8911af。直前のUSB配備記録out/nuc5-accepted-normal/physical-usb/provenance.jsonと試験順序に基づく関連付けであり、動画からハッシュを測定したものではない。証拠はout/nuc5-7627/observation.jsonと抽出画像。NUC5の通常版候補冷間3回確認は合格。Ryzenの通常版候補冷間3回・MacのBasic Data起動、最終梱包・現行ソースのリモートCIは未完了。NUC5のRAM・ファームウェアUNKNOWN欄は保持。以下の日付付き記録は各観測当時の状態を残す。

### NUC5：診断版の冷間起動3回成功（2026-10-14）

所有者から冷間起動3/3回成功と報告（IMG_7626.MOV）。現在の試験順序とout/nuc5-ss-recovery/physical-usb/provenance.jsonに基づき診断EFI d2f5d0c039fb71f864f8e1c2b34958b35d1de8a197bd54a6a3ec73d46eb47348へ関連付ける。動画から測定したハッシュではない。動画28.395秒の終端は診断11/11で、容量取得・セクター読出し・FAT32 FILE READ・KEYBOARD POLLING・STOPPED DMA DISABLED・RUN SESSION RESULT OKを確認。その後のHello／終了画面は抽出した終端にはなく、3回成功は所有者申告として記録する。

19秒時点でSS ROOT 12 T 2000 U0 TRY 0、PORTSC 00201203（CCS1/PED1/PP1/SPEED4/PLS0）。未接続のROOT 13〜15はCCS0/PLS5のTIMEOUT TRY 0であり、接続機器の失敗とは扱わない。この回のROOT 12はWPR 0回で成功。途中でD0/D4/D8/DC 7FF/4FF/F/F、所有権BIOS1/OS0→BIOS0/OS1、表示分解能で0msを確認。WPRが成功の原因とは断定しない。

Ryzen診断版の冷間3/3・ホット1/1成功と合わせ、診断版再試験の条件を満たした。通常版候補を5e4af31ece007550ba4d41a8a5c58d8b7a3b493e8665f277b7817134dd8911afで再生成（featureなし、ローカル未コミットソース）。QEMU結果とmanifestはout/nuc5-accepted-normal/。通常版候補でのNUC5・Ryzen各冷間3回とMacのBasic Data起動は未実施。過去の失敗記録は保持する。

### Ryzen 7：冷間起動3回・ホットリブート1回成功（2026-10-14）

所有者から **冷間起動3/3回、ホットリブート1/1回成功** と報告（IMG_7622.mov）。動画66.975秒の抽出フレームで診断5/20と、最終のMUSHA.ELF (Ring 3)、Hello Musha-OS!、exited: status 0を確認。回数と再起動種別は所有者申告、動画で確認できた成功実行と区別する。4回の起動を映像で個別確認したとは扱わない。要約ではC1がINPUT DONE K1 S1。

2026-10-14、所有者が4回とも最新配備のdiagnostics-detail版 d2f5d0c039fb71f864f8e1c2b34958b35d1de8a197bd54a6a3ec73d46eb47348 での成功と確認。配備・読戻し来歴：out/nuc5-ss-recovery/physical-usb/provenance.json。所有者確認による関連付けであり、動画から測定したハッシュではない。通常版公開候補の受入には算入せず、USB3回復処理が成功の原因と断定しない。NUC5診断版の冷間起動3回と、その後の通常版候補受入は引き続き未完了。証拠はout/ryzen-7622/observation.jsonと抽出画像。

### 前版の冷間再起動失敗：IMG_7619.mov（2026-10-14）

所有者から前のイメージでの冷間再起動失敗として報告。機種の再明示はないため、直前のRyzen 7試験の文脈として扱う。動画14.563秒。GOP 3840/2160/3840、PCI DEVICES 34（16進）、XHCI 5、INTEL LAN 0、CORE READY、TIMER READYを確認。約7.3〜12秒ではUSB READ OKとUSB BYTES 72AE00000が表示され、約14.1秒にはNo medium／0 files、No supported FAT32/exFAT mediumのビューアーへ遷移。Hello・終了0は確認されない。

コード上、boot/src/xhci/usb/storage.rsのUSB READ OKは容量取得と先頭・末尾セクター読出しの後に表示されるが、FAT32/exFATのサービス確立を保証しない。したがって一段階ではストレージと通信できており、全ポート未検出とは扱わない。後続の再列挙・媒体選択・ファイルシステム／サービス確立の失敗は未切り分け。ビューアーが診断を隠すため、失敗時PORTSCや正確な段階は不明。残っているPAGING / ARENA表示をその段階の障害と断定しない。汎用文言だけではキーボード検出の有無も判定できない。

前版EFIとの関連付けは暫定（直前の配備7eba0377...、out/nuc5-address-recovery/physical-usb/provenance.json）。動画からハッシュを確認したものではなく、最新d2f5d0c...の試験には含めない。先の成功例と今回の失敗例から、報告された前版の起動は安定成功しておらず、3回連続成功の受入は未達。証拠はout/previous-7619/observation.jsonと抽出画像。次は配備済みの診断保持版で失敗時ログを採取する。USB3回復修正で今回の症状が解消したとはまだ判断しない。

### Ryzen 7 前版の冷間起動：IMG_7616.mov / IMG_7618.jpg（2026-10-14）

所有者からRyzen 7機で「前のイメージ」による冷間起動結果として提供された。写真で `MUSHA.ELF (Ring 3)`、`Hello Musha-OS!`、`MUSHA.ELF exited: status 0` を確認。動画は52.552秒で、0秒・26.276秒・52.052秒の抽出フレームから起動画面と診断7/20・20/20を確認した。7ページではxHCI 5台、C1はINPUT DONE K1 S1。抽出した終端フレームは診断画面のため、最終アプリ成功の根拠は写真とする。

20ページに `JOURNAL LIMIT / MORE OMITTED` があり、後続ログの一部は保持されていない。採取範囲の制限として記録し、アプリの失敗とは扱わない。証拠・媒体ハッシュは `out/ryzen-7616/observation.json` と抽出画像。

直前の配備診断EFIは `7eba0377ffdc39fea24544a52c8fed86bbf7254c2869d753fceebaffb948676a`（`out/nuc5-address-recovery/physical-usb/provenance.json`）。今回との関連付けは「前のイメージ」からの暫定判断であり、映像でハッシュを確認したものではない。最新の `d2f5d0c...` 診断版の結果には含めない。申告された冷間起動成功を確認したが、3回連続成功・通常版候補の受入は未確認のまま。

### 冷間起動の間欠失敗：IMG_7612.jpg（2026-10-14）

所有者から、直前の診断版でNUC5i5RYHの冷間起動3回が「成功／対応FAT32・exFAT媒体なし（キーボード検出）／成功」と報告された。3回連続成功の受入条件は未達。直前EFIは `out/nuc5-address-recovery/physical-usb/provenance.json` により `7eba0377ffdc39fea24544a52c8fed86bbf7254c2869d753fceebaffb948676a` と関連付ける。写真から測定したハッシュではない。写真は媒体なしのビューアーを確認できるが、PORTSCやリンク・列挙・読込みのどの段階で失敗したかは判定できない。ROOT 0CのSanDisk SuperSpeed経路の間欠失敗は所有者の仮説として記録する。

次の診断EFIは `d2f5d0c039fb71f864f8e1c2b34958b35d1de8a197bd54a6a3ec73d46eb47348`、`out/nuc5-ss-recovery/BOOTX64.EFI`。USB3回復処理と診断保持は実装済み、実機検証は未実施。まず診断版で冷間起動3回を行い、失敗時は診断ページ全周を採取する。3回すべて成功後に通常版候補を再ビルドし、通常版で冷間起動3回をやり直す。過去の開発版成功はこの受入を代替しない。

### NUC5診断版の成功：IMG_7611.MOV（2026-10-14）

所有者提供のNUC5i5RYH動画（25.492秒）で、診断全9ページの後に `MUSHA.ELF  (Ring 3)`、`Hello Musha-OS!`、`MUSHA.ELF exited: status 0` を確認。診断版の成功を1回観測した記録であり、電源断からの冷間起動手順、反復安定性、キーボードの実打鍵は動画から確定できない。通常公開候補の受入は未完了。

対応EFI SHA-256は `7eba0377ffdc39fea24544a52c8fed86bbf7254c2869d753fceebaffb948676a`、featureは `diagnostics-detail`。`out/nuc5-address-recovery/physical-usb/provenance.json` と所有者報告による紐付けで、動画からハッシュを測定したものではない。抽出画像と動画ハッシュは `out/nuc5-address-recovery/nuc5-7611/`。

- GOP 1600/1200/1600、xHCI 8086:9CB1 / BDF 00A0。root 2 / slot 1はLenovo 17EF:6009、Low・速度ID2。root 7 / slot 2は8087:0A2A、Full・ID1。root 12 / slot 3はSanDisk 0781:55A9、Super・ID4。リセット後の速度名、Address Device、Slot Contextの速度は一致。K1 S1、FAT32 FILE READ、`RUN SESSION RESULT OK` を確認。
- D0/D4/D8/DCは切替前から `7FF/4FF/F/F`、切替後も同値で `WRITES 3 MATCH 1`。所有権移行前からポート2/7/12は接続状態。以前の `300/4FF/0/F` からの復旧や、ルーティング・待機修正だけが成功原因であることを示す試験ではない。
- USBLEGSUPは `00010801`（BIOS1/OS0）→ `01000801`（BIOS0/OS1）、OK、表示分解能で経過0ms。接続待ちは1000ms。
- root 1は未接続としてスキップ。Address Device拒否・BAD COMPLETIONは観測されず、拒否後に列挙を継続する経路は今回の実機では未実行。以前のroot 1失敗の完了コードは依然不明。

### Address Device復旧候補（2026-10-13、実機確認前）

IMG_7609後の所有者判断に従い、Intelルーティングを通常版へ残し、D4/DCのマスク外ビットを保持します。安全に完了したAddress Device拒否は対象スロットを無効化し、他ポートへ継続します。観測値 `D0/D4/D8/DC: 300/4FF/0/F → 7FF/4FF/F/F`、root 1/2/7接続、root 1 slot 1のAddress Device停止は過去記録として維持します。ルーティングの有無を比較した対照試験ではありません。

通常版EFI `2d8b3a0669108cc6ece04c696dc84292e180a1f6bc2e85f185695ef9a168a09f`、診断版EFI `7eba0377ffdc39fea24544a52c8fed86bbf7254c2869d753fceebaffb948676a`。`01f6efc`＋未コミット差分で、`out/nuc5-address-recovery/manifest.json` と `source.patch` に記録しました。通常版はGPT両形式各26件、viewer/cooperative 38件、失敗注入2件＋診断版注入2件がQEMU合格、ホスト263件合格。NUC5での実際の完了コード・冷間起動受入は未確認です。従来の速度一覧はリセット前であり、新版はPORTSC前後・リセット後の速度ID/名前・Slot Contextの速度IDを表示します。速度変換自体は変更していません。


### 縮小診断版の実機結果：IMG_7609.MOV（2026-10-13）

21.873秒のNUC5動画、最終表示8ページ。直前の配備記録によりEFI `23b2a9266114b7c0a6f09e1017e15ea92cdd4ff33d22ec94c6d7938785aae23e` と対応付けます（動画からのハッシュ実測・冷間起動手順の確認ではありません）。画像・読取り記録は `out/nuc5-diagnostics-compact/nuc5-7609/`。

- xHCI 8086:9CB1、BDF 00A0。停止は `SLOT / ADDRESS`、`BAD COMPLETION`。対象はroot 1 / slot 1 / High。外部Helloには未到達。
- 所有権移行は `00010801`（BIOS1/OS0）→`01000801`（BIOS0/OS1）、OK。経過表示0msはミリ秒精度の値です。
- D0/D4/D8/DCは切替前 `300/4FF/0/F`、後 `7FF/4FF/F/F`（16進）。WRITES3は2レジスターのビットマスクです。MATCH0はD0の完全一致判定が外れたものですが、D4の指定ビットはD0ですべて立っています。切替全体の失敗とは断定しません。
- 所有権移行前・切替直後は全15ポート `000002A0`。リセット・開始・1000ms待機後はroot 1・2・7がCCS1で列挙対象です。root 1/7=`000206E1`、root 2=`00020AE1`。今回は全ポート未検出ではありません。初期PORTSCの速度と、ポートリセット後のHigh表示は観測段階が異なります。
- コードとの照合でEnable Slotの後、Address Deviceコマンド完了の検査で停止したことまで絞れます。完了コード・TRBポインター・slot等のどの判定で拒否されたかは、現在の診断記録では不明です。次は完了イベント4ワード、完了コード、期待値／実値、入力コンテキストの採取が必要です。接続待ち・ルーティングの最終方針は保留します。


### 所有者による条件の追記・訂正（2026-10-13）

**候補 41a61efc 冷間起動：失敗。起動メニューで30秒待機後も同様。キーボード未接続。** この後続報告を当該試行の接続条件として優先し、以前のキーボード接続済みという記録を訂正します。IMG_7607.MOVの接続条件まで確定するものではありません。

### Intelルーティング候補：NUC5で未成功（IMG_7607.MOV）

2026-10-13に所有者から受領した8.052秒の動画で、UEFI USBブート項目、runtime / CORE READY / DEVICE DISCOVERY、その後のビューアーの `No medium | 0 files` と `Device session ended. Restart to try again.` を確認しました。外部Hello・終了0は確認できず、候補受入は不合格です。GOPは1600/1200/1600、PCI E、XHCI 1、Intel LAN 1、TIMER READY。冷間起動かどうかは動画だけでは確定できません。

直前のUSB更新記録 `out/intel-routing/physical-usb/provenance.json` と所有者報告により、ソース `01f6efc`、EFI SHA-256 `cb0f532d24fb12c238069cec2b906c74f11fdb91b9745855ab5079c86196f23c` に対応付けます。動画からハッシュを実測したものではありません。抽出画像は `out/intel-routing/nuc5-7607/` に保存しました。

この終了文言はキーボード検出成功、全ポート未接続、MUSHA.ELF不在のいずれも証明しません。実装は正常な探索完了・入力なし・アプリエラーなしの場合にだけキーボード未検出を表示し、それ以外の終了では汎用文言を残します。今回の画面にはルーティングレジスター、PORTSC、セッションのエラー理由が表示されていません。ルーティングと接続待ちの追加による回復は確認できず、次は診断版でそれらを採取する必要があります。本観測のための追加USB書込みは行っていません。

### Intelルーティング候補 — IMG_7607.MOV受領前の準備記録

ソース `01f6efc859646fada7ef28fffa9322d266245469`、release・features=[]、460,288 byte。EFI SHA-256 `cb0f532d24fb12c238069cec2b906c74f11fdb91b9745855ab5079c86196f23c`。manifestは `out/intel-routing/candidate/build-manifest.json`。QEMU全186件合格。USBはEFIのみ更新し、読戻し一致・安全な取外し完了（`out/intel-routing/physical-usb/provenance.json`）。以下の失敗を受けた新候補であり、原因確定・問題解消を示すものではありません。NUC5/Ryzen冷間起動とMac Basic Data起動は新ハッシュで行います。

### 候補4319b58のNUC5冷間起動失敗（2026-10-13、所有者報告）

EFI SHA-256 `41a61efcd96edf6142bf81a37ae2ca90ca4396ea71386da698f8cac8bccd2ae8`、配備記録は `out/release-display/physical-usb/provenance.json`。USBキーボード・SanDisk接続済みでもxHCI全ポートが未検出だったとの所有者報告です。候補受入の失敗1回として記録し、冷間起動3回成功には数えません。提供動画 `IMG_7606.mov` は1.975秒で、抽出フレームはruntime/CORE READY段階です。全ポート未検出という詳細はこの動画で独立確認できず、所有者報告として区別します。Intelルーティングは未確定の仮説です。このEFIの「未試行」という旧記述は本追記で更新し、以下の過去の記録は保持します。

### 新しい起動表示の候補 — 実機未実施（2026-10-13）

ソース `4319b58dda03400032c0307eaaa3b85fe8150ac0`、release構成・features=[]。EFI SHA-256 `41a61efcd96edf6142bf81a37ae2ca90ca4396ea71386da698f8cac8bccd2ae8`（455,680 byte）。manifestは `out/release-display/candidate/build-manifest.json`。次の受入対象をこの候補へ変更します。以下の過去の実機観測は変更しません。QEMU全回帰186件合格（`out/release-display/report.json`）。USBのEFIのみの更新・読み戻し・安全な取外しを完了しました（`out/release-display/physical-usb/provenance.json`）。更新完了後にNUC5・Ryzen各3回の冷間起動とMacBook Pro 2018のBasic Data起動を行う必要があり、このEFIの実機成功はまだ記録していません。

### 現行開発版（2026-10-13、所有者報告）

以下は所有者が提供した、Claudeによる動画読取り結果です。配備済みBasic Data USBのEFI SHA-256は `b743ac0129a9692adf865c37c33adb2c23ca2c0238baf5b9437ebe20faee2b1a`。各回の `docs/hardware-evidence/2026-10-13/{nuc5,ryzen7,macbook-pro-2018}/provenance.json` に配備記録との紐付けを保存しました。動画からハッシュを測定したものではなく、配備内容と所有者の実行報告を事後に対応付けた記録です。

- **NUC5：Ring 3 Hello成功**。IMG_7602.mov、GOP 1600/1200/1600。PCI E、XHCI 1、LAN 1、TIMER READY。IMAGE 0x14000000 / SIZE 0x10C8000、TABLE 0x9D170000、MMIO 0 / BASE 0xAA120000。Hello→終了0、診断パネルの重なり解消、UEFI USBブート項目の表示を確認。RAM・UEFI版等のUNKNOWNは所有者回答待ちのままです。
- **Ryzen：Ring 3 Hello成功の報告**。IMG_7603.mov、GOP 3840/2160/3840。PCI 34、XHCI 5、LAN 0、USB READ OK、USB BYTES 72AE00000、MMIO 4 / BASE 0xFCC00000。終了0の最終映像はぼけており静止画補足待ち。4Kの等倍文字は小さすぎるためA1で改善します。
- **Mac：起動・外付けキーボード検出まで**。IMG_7604 2.mov・IMG_7605.MOV、GOP 3360/2100/3584。PCI 23、XHCI 3、LAN 0、MMIO 2 / BASE 0x8F900000。外付け接続時KEYBOARD READY、未接続時USB keyboard not detected。双方No medium／No supported FAT32/exFAT medium。内蔵キーボードはT2未実装のため未対応。10/12 rc1のFAT32成功とは区別し、退行か接続構成差かは未特定。ストレージ調査は所有者判断で0.4.0以降へ先送りします。

**冷間起動3回：未実施（公開候補EFIでNUC5・Ryzen各3回を行う）**。Macの候補確認はBasic Data起動のみです。旧開発版のNUC5 No mediumは2回観測、現行版では未再現。LAN検出は実機通信成功ではありません。[残件一覧](open-issues-0.3.0.md)を参照してください。

### 以下はGOP／Basic Data対応前の履歴

2026-10-13受領。所有者の機種報告は **NUC5i5RYH** です。**外部Helloの実行成功を1回確認しましたが、実機受入は未完了です。** 2回の試行が報告されており、冷間起動3回の連続成功ではありません。電源断／リセット方法、RAM、UEFI版、接続ポート、キーボード型番は未記録です。

- 1回目（`IMG_7596.HEIC`）：ビューアーに`No medium | 0 files`、`USB keyboard not detected. Connect one, then restart.`。外部Helloへ到達していません。
- 2回目（`IMG_7600.jpg`）：`MUSHA.ELF`、`Hello Musha-OS!`、`MUSHA.ELF exited: status 0`を確認しました。
- 動画（`IMG_7598.MOV`、11.67秒）：抽出した開始フレームでUEFI USB選択、中盤・終盤でHelloと終了0を確認しました。起動時間の計測には使用しません。

使用予定として配備した通常版EFI・Helloのハッシュ、ソースcommit、USB容量は英語表に記載しています。写真からハッシュを実測したものではありません。配備証拠は`out/nuc5-0.3.0-35ea933/`、原本は所有者のDownloads、確認用変換は`out/nuc5-font-review/`です。全容量イメージ作成・全容量ハッシュは行っていません。

1回目はOS側に利用可能な媒体がなく、キーボードも未検出という表示です。ELF不正や「ファイルがないことを正常読出しで確認した」とは解釈しません。USB列挙・所有権移行・媒体初期化や接続構成のどこで止まったかは、この写真だけでは不明です。同じ接続・同じUSBのままで電源断からの起動と再起動を区別して記録し、失敗時の全画面を保存することが次の確認です。実機キーボード操作、exFAT、ネットワーク、例外試験は今回未確認です。

文字サイズの指摘を受け、外部コンソールだけ2倍から等倍（6×8）へ変更しました。80桁×24行、折返し・代替文字・ABI、従来ビューアーは維持しています。変更版は**今回の実機成功版とは別**で、未commit・未push・USB未更新です。ホスト10件と通常版QEMU7件の結果を上記のパスに保存しました。QEMU結果を実機の追加成功回数には数えません。


## Managed USB file check (2026-10-14) / 管理ファイル確認

Before acceptance, use the matching schema-2 updater and save its verified-files
report: EFI/BOOT/BOOTX64.EFI, DIAGNOSE.TXT, README.TXT, LICENSE, NOTICE,
third_party/{lwip,freebsd-e1000,freebsd-apple-bce}/{COPYING,UPSTREAM},
third_party/r-efi/{AUTHORS,UPSTREAM}, update-manifest.json. Verify all eight
third-party files, the marker's 16 bytes/FNV 9A42A948C590F507, and absence of
retired MUSHA.TXT. AUTORUN.ELF remains a separate owner-provided program.
Historical screens used MUSHA.TXT; new candidates use DIAGNOSE.TXT.
This preparation does not count as hardware acceptance or a USB update.

受入前に対応するschema 2更新ツールを使用し、上記の全管理ファイルについて
verified-filesを保存します。third_partyの8ファイル、診断マーカー16 byte・
FNV 9A42A948C590F507、旧MUSHA.TXTの不在を確認してください。
AUTORUN.ELFは利用者が配置する別ファイルです。当時のファイル名はMUSHA.TXT、
新候補ではDIAGNOSE.TXTです。この準備は実機合格・実USB更新を意味しません。

## NUC5 owner report, 2026-10-14 / NUC5 所有者報告

The owner reports three successful cold boots with the latest deployed normal
EFI (IMG_7636.MOV). Deployment evidence: out/usb-usability-deploy/provenance.json.
EFI SHA-256: `982fa2ac30a64182a4a0c495a4f64ceea706d46c537ac28ffc88aab96d646da7`.
This is the owner-reported count; individual PRE OWN D8 photographs and power-off
durations are not supplied. It does not validate the subsequent font adjustment.

最新配備の通常版でNUC5冷間起動3回成功との所有者報告（IMG_7636.MOV）。
配備記録とEFIハッシュは上記に紐づけます。各runのPRE OWN D8写真・電源断時間は
未提供のため、所有者報告として記録します。後続のフォント調整版の実機結果には
数えません。

## Ryzen 7 owner report, 2026-10-14 / Ryzen 7 所有者報告

The owner reports three successful cold boots with the deployed normal EFI.
IMG_7639.MOV's final captured frame shows AUTORUN.ELF (Ring 3), Hello Musha-OS!,
and exited: status 0. The count of three is owner-reported, not three
independently verified video sequences. Deployment evidence:
out/usb-usability-deploy/provenance.json (source f3d2745).
EFI SHA-256: `982fa2ac30a64182a4a0c495a4f64ceea706d46c537ac28ffc88aab96d646da7`.
Frames: out/ryzen-7639/. Hot reboot and USB READ OK-to-Hello timing are not
established here. The subsequent font adjustment remains undeployed.

現在配備した通常版でRyzen 7の冷間起動3回成功との所有者報告。
動画終盤でAUTORUN.ELF (Ring 3)、Hello Musha-OS!、exited: status 0を
確認しました。3回という回数は所有者報告で、動画から3起動を独立確認した
ものではありません。配備来歴・EFIハッシュは上記に紐づけます。今回の
ホット再起動・USB READ OKからHelloまでの所要時間は未記録です。
後続のフォント調整版はUSB未配備で、この実機結果の対象外です。

## NUC5 unified-grid display check, 2026-10-14 / NUC5 共通グリッド表示確認

The owner reports that the new display is satisfactory on NUC5 (IMG_7641.MOV).
The extracted final frame confirms AUTORUN.ELF (Ring 3), Hello Musha-OS!, and
AUTORUN.ELF exited: status 0. This records a successful execution observation
and owner approval of readability, not three consecutive cold boots. Power-off
duration, PRE OWN D8 and repeat count are not established by this report.
EFI SHA-256: `ba23cabb988af8349924b78914000d030899ac3589568ce8b79fa7e3bcf21cad`.
The hash is linked through the preceding USB deployment, not measured from video:
`out/display-grid/usb-deploy/provenance.json`. Extracted frames:
`out/display-grid/nuc5-7641/`. Ryzen display validation of this EFI remains pending.

所有者からNUC5の新表示について「これで良さそう」との確認を受領しました
（IMG_7641.MOV）。抽出した終盤フレームでAUTORUN.ELF (Ring 3)、
Hello Musha-OS!、exited: status 0を確認しました。実行成功の観測と
読みやすさの所有者確認として記録し、冷間3回連続成功には数えません。
電源断時間・PRE OWN D8・反復回数は今回の報告では未確認です。
EFIは直前のUSB配備記録によって上記ハッシュに紐づけています。
動画からハッシュを実測したものではありません。このEFIのRyzen表示確認は未完了です。

## Ryzen 7 unified-grid cold boots, 2026-10-14 / Ryzen 7 共通グリッド冷間起動

The owner reports three successful cold boots (IMG_7643.MOV) with the latest
unified-grid EFI. Captured frames show GOP 3840/2160/3840, PCI DEVICES 34,
XHCI 5, INTEL LAN 0, USB READ OK, and finally AUTORUN.ELF (Ring 3),
Hello Musha-OS!, and exited: status 0. The left status and right diagnostic
panels do not visibly overlap. Three boots is the owner-reported count, not
three independently verified video sequences. Power-off durations, hot reboot
and USB READ OK-to-Hello timing are not established here.
EFI SHA-256: `ba23cabb988af8349924b78914000d030899ac3589568ce8b79fa7e3bcf21cad`.
The hash is linked through `out/display-grid/usb-deploy/provenance.json`, not
measured from video. Frames: `out/display-grid/ryzen-7643/`. This updates the
previously pending Ryzen display observation, without closing all release
acceptance requirements or establishing NUC5 repeat counts for this EFI.

共通グリッド版でRyzen 7冷間起動3回成功との所有者報告を受領しました
（IMG_7643.MOV）。抽出フレームでGOP 3840/2160/3840、PCI DEVICES 34、
XHCI 5、INTEL LAN 0、USB READ OK、最終的にAUTORUN.ELF (Ring 3)、
Hello Musha-OS!、exited: status 0を確認しました。左の状態表示と右の診断パネルに
重なりは見られません。3回という回数は所有者報告で、動画から3起動を独立確認した
ものではありません。電源断時間・ホット再起動・USB READ OKからHelloまでの
時間は今回未確認です。直前の配備記録によって上記EFIハッシュに紐づけます。
Ryzenの表示観測は確認済みとしますが、公開受入全体の完了や、このEFIでの
NUC5反復成功回数を示すものではありません。

## NUC5 diagnostic observation IMG_7644, 2026-10-14

One diagnostic boot shows AUTORUN.ELF (Ring 3), Hello Musha-OS! and exit status 0.
Source: `a1762f423516d702bc2d9e00a9425d2b173e00c9`; release + diagnostics-detail.
EFI SHA-256: `0614fbe998836f2eb5f837ca2429b922bedd8668b21db9e2c13bbfe6e65085b0`.
Identity is linked through `out/release-grid-stage3-r2/usb-diagnostic-deploy/provenance.json`,
not measured from the video. Frames and observations:
`out/release-grid-stage3-r2/nuc5-7644/`.

| Evidence / video offset | Observed value |
|---|---|
| IMG_7644.MOV, 18 s, first PRE OWN (page 5/17) | D0=0x7FF, D4=0x4FF, D8=0xF, DC=0xF; P12 PORTSC=0x1203, CCS=1, PED=1, SPEED=4, PLS=0 |
| 18 s, ownership | BIOS/OS 1/0 to 0/1, elapsed 0 ms, OK |
| 20 s, routing | WRITES 0, MATCH 1; D0/D8 unchanged |
| 22 s, attachment | ATTACH WAIT 160 MS MAX 1000; P12 U0 TRY 0; SESSION 1 RESCAN 0 |
| 28 s, enumeration | ROOT 12 SLOT 3, speed ID 4 Super; descriptor/configuration/storage sector/FAT32 file read stages |
| 40 s, diagnostic limitation | JOURNAL LIMIT / MORE OMITTED |
| 44 s, application | AUTORUN.ELF (Ring 3), Hello Musha-OS!, exited: status 0 |

This is a successful diagnostic execution, not a qualifying D8=0 cold run and
not normal-candidate acceptance. The owner subsequently reports powering off for a few seconds, then turning
the machine on again. This does not meet the required 10-second minimum.
Whether AC was disconnected remains unspecified. D8=0xF alone does not
establish what power operation the owner performed. The 500 ms post-routing-write grace path is not exercised
by this observed WRITES 0 run. Keep D1-D3 acceptance gates pending. Video offsets
are capture positions, not hardware-stage elapsed times.

### 日本語

IMG_7644.MOVで診断版の1起動を確認し、AUTORUN.ELF (Ring 3)、Hello Musha-OS!、
終了0を読み取りました。EFIは上記の直前配備記録に紐づけ、動画からハッシュを
測定したものではありません。初回PRE OWNはD0=0x7FF、D4=0x4FF、D8=0xF、DC=0xF。
P12は既にCCS=1・PED=1・SPEED=4・PLS=0でした。所有権移行は0 msで成功、
ルーティング書込み0回、接続待ち160 ms、P12はU0 TRY 0、RESCAN 0です。
ROOT 12 SLOT 3でストレージ・FAT32読出し段階へ進み、最終的にHello・終了0を確認。
途中にJOURNAL LIMIT / MORE OMITTEDがあり、全診断履歴の完全採取とは扱いません。

動作成功の観測として記録しますが、D8=0の冷間条件を満たすrunや通常版候補の
受入には数えません。所有者から「数秒電源を落として再度入れた」と追加回答を受領しました。
所定の電源断10秒以上を満たさず、ACを抜いたかは未回答です。D8=0xFだけで
所有者が行った電源操作を断定しません。書込み0回のため、切替書込み後500 ms猶予の
実機確認にもなりません。D1〜D3の受入欄は未完了を維持します。表の秒数は動画位置で、
各ハードウェア処理の所要時間ではありません。

## NUC5 D1: AC-disconnected cold diagnostic boot, 2026-10-14

The owner reports unplugging AC for 10 seconds before IMG_7645.MOV. The first
PRE OWN snapshot shows D0=0x300, D4=0x4FF, D8=0, DC=0xF (16 s video offset).
Routing changes D0 to 0x7FF and D8 to 0xF; WRITES=3 is a bitmask for the two
register writes, not a count of three writes. MATCH=1. Hello and exit status 0
appear at 44 s. This qualifies as diagnostic D1, one successful cold observation;
D2/D3 and all normal-candidate acceptance runs remain pending.

EFI SHA-256: `0614fbe998836f2eb5f837ca2429b922bedd8668b21db9e2c13bbfe6e65085b0`.
Source: `a1762f423516d702bc2d9e00a9425d2b173e00c9`, release + diagnostics-detail.
Identity is linked through `out/release-grid-stage3-r2/usb-diagnostic-deploy/provenance.json`,
not measured from the video. Frames and structured readings are in
`out/release-grid-stage3-r2/nuc5-7645/`.

Important limitation: first ATTACH WAIT shows 500 ms (PM timing 499 ms), but
P12 remains CCS=0/PLS=5 at ROUTE+505 ms and is classified Empty. A later controller
pass in SESSION 1 shows P12 CCS=1/PED=1/SPEED=4/PLS=0 and queues enumeration;
RESCAN remains 0. This does not demonstrate that 500 ms alone is sufficient to
establish the cold SuperSpeed link. ROOT 1 Address Device reports completion
code 4; its slot is disabled and enumeration continues. Final timing records show
a second attach wait of 159 ms, WPR count 0 for both passes, ELF read 217 ms and
quiesce 0 ms. Preserve this successful end-to-end result together with the
intermediate failure and delayed P12 observation; do not claim a clean first-pass
storage discovery or a RESCAN recovery. Video offsets include diagnostic page
presentation and are not the device operation timings.

### 日本語

所有者報告は「ACアダプターを抜いて10秒」。IMG_7645.MOVの初回PRE OWNで
D0=0x300、D4=0x4FF、D8=0、DC=0xFを確認しました（動画16秒）。
切替後はD0=0x7FF、D8=0xF、MATCH=1。WRITES=3は2レジスターへの書込みを示す
ビットマスクで、書込み3回という意味ではありません。44秒でHello・終了0を確認。
診断版D1の冷間成功1回として記録し、D2/D3と通常版候補の受入は未完了です。
EFI・ソースは上記配備記録に紐づけます。

ただし初回ATTACH WAITは500 ms（PM計測499 ms）を実行しても、P12は
ROUTE+505 msでCCS=0/PLS=5のままEmpty判定でした。同じSESSION 1の後続パスでは
P12がCCS=1・PED=1・SPEED=4・PLS=0になり列挙へ進み、RESCANは0です。
この結果だけで500 msの猶予単独が十分とは判断しません。ROOT 1ではAddress Deviceの
completion code 4が出ましたが、slotを無効化して残りの列挙を続行しています。
最終時間記録は後続の接続待ち159 ms、両パスWPR 0、ELF読出し217 ms、停止0 ms。
最終成功と途中の拒否・P12接続遅延を両方残し、初回探索ですべて成功した、または
RESCANで回復したとは扱いません。動画位置の秒数は診断ページ表示時間を含み、
デバイス処理時間ではありません。
