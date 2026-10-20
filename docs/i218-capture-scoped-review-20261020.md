# I218 capture stage 1 scoped review — 2026-10-20

## English

This review describes the existing uncommitted capture change. It is a proposed independent checkpoint, not a commit or physical acceptance. The working tree was preserved before review at `/private/tmp/musha-offline-040-20261020/` (`start.patch`, `untracked.tgz`, `start-status.txt`). Do not stage the other owner, filesystem, viewer, handoff or documentation changes with it.

The exact capture source/test scope is `boot/Cargo.toml`, `boot/src/net.rs`, `boot/src/net/i218.rs`, `boot/src/net/i218_capture.rs`, `boot/src/runtime.rs`, `net/src/i218.rs`, `net/src/i218/capture.rs`, `net/src/i218/capture_policy.rs`, `net/tests/i218_capture.rs`, and `platform/tests/i218_capture_display.rs`. The related procedure is `docs/i218-swconfig-read-controls.md`; `docs/i218-hardware-quickcheck.md` and `docs/i218-network-0.4.0-progress.md` contain small references. `boot/src/runtime.rs` also carries owner changes from other work, so a future commit must select its capture hunks only. Other dirty docs are not implicitly included.

The saved `out/i218-swconfig-capture/verification.json` reports net 66 (including core 13 and policy 3), display 1, capture QEMU 1 and normal 82574 QEMU 1. Current hashes match that record for eight of the ten listed capture source/test files. `boot/Cargo.toml` and `boot/src/runtime.rs` differ from the saved build source; the latter includes mixed owner work. The saved EFI hash therefore identifies the earlier checkpoint. The [current exact-file manifest](i218-capture-source-manifest-20261020.json) records the present 10 hashes and feature-specific EFI hashes. Fresh current-tree validation passed: capture core 13, policy 3, display 1; UEFI feature check and release build; capture QEMU UI/admission/Esc DMA-stop; and separate normal 82574 QEMU DHCP, ARP, ICMP, 97 UDP echoes, wrap/checksum and DMA-stop. The normal QEMU run used `qemu-debug`; an initial build without it supplied no diagnostic log to that harness and did not complete DHCP, so it is not counted as a pass. QEMU exercises UI/admission only for I218; it contains no I218 NVM/PHY.

The read-only-in-effect capture path is opt-in through `i218-swconfig-capture` and still performs bounded control writes: SWFLAG acquire/release, MDIC read commands, PHY page selector 0 and restoration, and flash read-cycle commands. The adapter refuses other MAC writes, records register/value/result and restoration target, reserves the original 5-second deadline for cleanup, and does not enter reset/BME/ring initialization in capture mode. A failed selector operation may already have changed the page; the core attempts restoration and SWFLAG release and reports both outcomes. A failed restore is an incomplete capture, not proof of safe PHY state. Physical ownership and cleanup behavior remain unverified.

The LCD `data/address` pairs are NVM table entries only. The capture reads three encoded PHY offsets (page 768, registers 25/26/30) after selecting page 0, but does not read every LCD pair's target PHY register/current value. It cannot establish that the current PHY matches the NVM table, and does not complete any requirement that calls for all target values. Do not extend PHY writes or assume a target page from an address word before physical values and owner review. Software LCD/OEM application remains stage 2 and unimplemented.

The source lineage is the pinned FreeBSD e1000 `releng/14.3` commit `833d39bd2e38421a14ea489a956261a0fa993fbc`, under BSD-3-Clause. `third_party/freebsd-e1000/UPSTREAM` and `COPYING` retain the source and terms; the adapted capture module retains the BSD notice. No new external source was imported for this review.

## 日本語

既存の未コミットI218採取差分を独立checkpointとして査読しました。commit・実機受入はしていません。開始時の全差分は上記`/private/tmp/`へ退避しています。上記10 source/testと採取手順を対象にし、混在するruntimeの他用途hunk、fs/viewer/HANDOFF/所有者文書は一括stageしません。

保存済み検証記録のnet 66件（core13/policy3を内包）、表示1件、採取QEMU1件、通常82574 QEMU1件は過去checkpointの結果です。現行10ファイル中8件のhashが一致し、`boot/Cargo.toml`と`boot/src/runtime.rs`は異なります。現行treeの[正確なmanifest](i218-capture-source-manifest-20261020.json)を生成し、採取core13/policy3/表示1、UEFI check/release build、採取QEMUの画面/停止、別の通常82574 QEMU DHCP/ARP/ICMP/UDP97件・停止を再確認しました。通常QEMUは`qemu-debug`付きの試験専用buildです。I218のNVM/PHY実機操作はQEMUで検査できません。

採取にはSWFLAG、MDIC、PHY page選択と復元、flash read-cycleの制御writeがあります。許可外MAC writeを拒否し、書込値・結果・元pageを記録し、同じ5秒の期限内に復元とreleaseを試みます。復元失敗は未完採取であり、安全なPHY状態の証明ではありません。実機での所有権・cleanupは未確認です。

LCDの`data/address`はNVM表の値です。採取コードはpage0選択後のpage768 reg25/26/30を読みますが、各LCD pairが示すPHY対象番地の現在値は全件読んでいません。現行PHYとNVM表の一致は未証明です。実値・所有者確認前にPHY write範囲を広げたりstage2のsoftware LCD/OEM設定を実装したりしません。参照元・BSD-3-Clause表示は英語節の通り保持されています。
