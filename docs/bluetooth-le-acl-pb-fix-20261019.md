# LE ACL transmit PB correction — 2026-10-19

Host-to-Controller LE first ACL fragments now use PB=00, with PB=01 for continuations. The production connection path calls the shared platform `le_tx_header` builder, and host tests exercise that exact builder with Pairing Request and fragmented Public Key lengths. Controller-to-Host PB=10 reception remains unchanged and covered by the existing response/reassembly fixture. Scope excludes Security Request and connection interval policy.

The first Number Of Completed Packets log now says “ACL controller buffer completion”. This event returns controller buffer credit; it does not establish a remote wireless ACK or peer SMP acceptance. Earlier acknowledged/not-host/not-dongle claims in historical handoff records are not justified. The PB defect is concrete, but whether correcting it resolves the physical peer silence remains untested.

## Validation

- Isolated default platform host suite: 126 passed, 0 failed (`../out/bt-le-pb-fix-20261019/host-tests.log`).
- Isolated platform Bluetooth security suite: 207 passed, 0 failed (`host-security-tests.log`).
- Negative control: restoring the old PB=10 value in the shared builder makes exactly two pairing-stage tests fail, eight pass (`negative-pb10-tests.log`, exit 101). Correct source restored immediately afterward.
- QEMU: five cases passed: qemu-debug normal boot (`qemu-debug-r2.log`), featureless normal EFI external ELF hello-fat32 and exit-42 (`qemu-normal-hello-r3.log`, `qemu-normal-exit42.log`), viewer exFAT normal page hashes/read-only images/DMA cleanup (`qemu-viewer.log`), cooperative traffic ARP/ICMP, 257 UDP echoes, file rechecks, ring wraps/checksums/DMA stop (`qemu-cooperative.log`).
- `git diff --check` passed. No physical Bluetooth/USB acceptance was performed. QEMU has no Bluetooth device and cannot prove pairing success.

Earlier retained attempts were harness setup failures: marker-based smoke was first given a featureless EFI (no debug marker); an initial external-ELF launch lacked toolchain PATH; archive-root QMP attempts failed due to the nested socket path/missing source/out. Final successful runs used the existing main-root harnesses with explicit isolated candidate files. Initial pre-fix host/build execution was superseded by the final patched executions; only the final counts above are claimed.

## Artifact and ownership

Normal EFI: `out/bt-le-pb-fix-20261019/normal/files/EFI/BOOT/BOOTX64.EFI`.

SHA-256: `d3c53d7634f58ff4c71303d3dd4c60bc38c5bb8570766bb7a597e77c3ce19119`.

Features: none. Source: HEAD `c1735daf82061813952d89087511300e83d745c1` plus only the three-file patch `out/bt-le-pb-fix-20261019/source.patch` (SHA-256 `86947561d6c3bfbd4c0b028f56f1bb6f417d4f60b345aaf525fe7f61f7f9b3de`). Built with `cargo build --locked --release --target x86_64-unknown-uefi -p musha-boot` from an isolated git archive. All preexisting dirty I218/owner source changes are excluded. Exact source hashes, toolchain, debug/viewer artifact hashes and configuration hash are in `normal/provenance.json`.

The staged `normal/files/SETTINGS/CONFIG.TXT` contains `log=devices`, `display=max`, `lan.echo=off`. No PAIR.ON or autorun is bundled. This is a development artifact, not a release candidate. No commit, push or USB write occurred. Existing dirty files and historical handoffs were not edited. Their initial status was recorded; an initial full diff-byte snapshot was not taken, so byte preservation cannot be independently demonstrated from a before/after hash record. All writes to tracked working files were limited to the three patch paths and this new report.

## 日本語

LE のホスト→コントローラー送信で先頭 ACL を PB=00、継続を PB=01 に修正しました。実送信とホスト試験が共通のヘッダー生成処理を使います。受信側の有効な PB=10 は変更していません。完了イベントの表示はコントローラーのバッファ完了を示す表現に修正し、相手機器の ACK や SMP 受理の証明として扱いません。

ホスト試験は通常126件、Bluetooth security有効207件が成功。旧値への負の対照試験は想定どおり2件失敗。QEMUは通常起動、通常EFIのHello/exit-42、ビューアー、協調通信の5ケース成功（UDP257往復を含む）。実機のペアリング成功は未確認です。

通常EFIはHEADに今回の3ファイル差分だけを適用した隔離ソースから生成し、既存のI218・所有者差分を除外しています。設定は `SETTINGS/CONFIG.TXT` の `log=devices`、`display=max`、`lan.echo=off`。コミット・push・USB書込みは行っていません。
