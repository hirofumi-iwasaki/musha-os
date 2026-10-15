# I218 reset, transport and DHCP local checkpoint — v0.4.0

Status: current

## English

This development checkpoint has completed the full local regression; it is not hardware acceptance. The normal build still enables the QEMU82574 controller; I218-V networking is enabled only by `i218-network`. `i218-phy-probe` and `i218-power-probe` remain separate diagnostics. QEMU exposes 82574, not I218-V: enabling the I218 feature in QEMU validates shared-path regression, not I218 register behavior. NUC5 physical reset/link/traffic, OEM configuration and Realtek remain unverified.

### Implemented boundary

The optional I218 path validates firmware-provided BAR0/BAR1 resources before ExitBootServices and maps them through the existing bounded MMIO policy. BAR1 flash access issues read cycles only, validates descriptor/region/bank signatures and bounds, and verifies the NVM checksum. It does not guess a flash address, write NVM, or fall back to an invalid bank. Software LCD/OEM configuration (`FEXTNVM.SW_CONFIG`) is rejected: this limited path does not claim all NUC5 configurations work.

The reset sequence includes managed ME ULP exit or bounded software ULP wake, PHY ownership/page operations, PHY configuration gating, master-disable, Intel's reset/PCI synchronization delay, auto-read completion and host-wake cleanup. No STATUS-flushing MMIO read is inserted during the reset blackout. Unconfirmed reset stops execution rather than reusing memory. The copper-link setup requires full duplex, a valid firmware MAC and supported PHY identity. DMA is enabled only after reset/link and validated below-4GiB rings/buffers are initialized. Stop disables RX/TX and PCI bus mastering and, for I218, confirms the internal master is no longer pending before reuse. A failed stop remains fatal. Physical operation of this sequence is not established by host tests.

Shared lwIP now uses actual DHCP, including ACD; there is no static-address fallback. Startup has no address until a supplied lease exists. Renew/rebind retain a valid lease; expiry removes it. Initial/no-address timeout is 30 seconds and disables the NIC while leaving viewer input available. Runtime polling uses the boot time epoch across restart. The upstream DHCP transaction counter is correlation data, not a cryptographic security claim. DHCP remains unauthenticated.

### Validation and evidence

The local machine-readable `out/i218-regression/verification.json` records the final matrix, commands and EFI hashes. The earlier limited checkpoint remains in `out/i218-network`. The security-feature workspace passed **398 tests** (the default-only381 run overlaps); seven Python suites passed52 tests, with the smoke-consumer suite additionally rechecked. Real pinned lwIP fixtures passed seven DHCP scenarios, including dynamic address77, renew, wrong transaction ID, NAK, no server, rebind and expiry, plus50 successful lease/endpoint restarts. A separate pool/heap lifecycle test passed1,000 pending-DHCP restarts. BasicData/ESP virtual make/deploy/update both passed. Fourteen freshly rebuilt C/Rust sample ELFs matched the runtime inputs; separate archive relocation reproduced all four Rust ELFs.

The full QEMU matrix passed **298 case/configuration executions**, including overlap between suites and targeted retests, not298 unique applications. This is the preceding296-case matrix (CI54; C1/2/3; normal/debug external ELF; BasicData/ESP; viewer/cooperative/hub5; delayed/hotplug/address; resolution/hold/startup; settings writer; r2/Rust mixed-FP; LAN/exFAT repeated launch; Bluetooth UI/OUT/software restart/gamepad/no-adapter), plus two I218-feature DHCP cases. No Bluetooth adapter or I218-V is emulated. Software fixtures do not establish physical Bluetooth or I218 compatibility.

Five initial QEMU failures were consumer/diagnostic-fixture ordering conflicts introduced by real DHCP: four live-storage cases began READ or its timeout before lease acquisition; the intentional TX-timeout feature suppresses even the first DHCP DISCOVER doorbell. The storage probe now starts only after a supplied lease and delayed fixtures use existing `input-persistent`, ending through Esc or the expected error. Its four normal/hub2/disconnect/timeout cases passed without relaxing data/hash, pending input/UDP, cancellation or DMA-stop checks. The TX negative now explicitly requires DHCP-started/no-address, TX TIMEOUT, NIC DMA stop and subsequent input/file progress; it never invents a lease. CI feature composition matches the tested fixture. The five failed first attempts and298 successful executions total303 QEMU configuration attempts.

The final synchronization patch is restricted to diagnostic `live-storage-probe`; normal storage I/O is not gated. The previous runtime matrix is retained through source-impact analysis in `out/i218-regression/source-impact.json`: other features omit this probe or exclude it through `live-file-probe`, and final normal/debug rebuilds were **byte-identical** to the full-matrix inputs. All affected storage cases were rerun. The final three external-ELF timing cases ran alone after functional builds/tests ended and passed.

The no-server case confirms the real30-second deadline, NIC DMA stop and subsequent application receipt of Down. The fixture's empty viewer list need not repaint. Wire checks preserve exact Ethernet accounting, checksum/file/pixel expectations; the earlier isolated wire fixture's103 host-to-guest and106 guest-to-host frames imply exactly RX6/TX13 wraps. A proposal to replace exact wraps with lower bounds was rejected by automatic review before execution and was not applied. Existing cooperative-suite lower bounds predate this change.

Earlier checkpoint failures remain recorded: a wrong diagnostic feature lacked viewer markers; a third remount timed out, then isolated repeated launches passed (initial cause unconfirmed); delayed live-file testing exposed pre-lease ordering and the diagnostic ten-second window. Production deadlines were not enlarged. Test-launch mistakes (including UEFI boot in an Apple ARM host test and an incorrect SDK builder path) were corrected separately from runtime failures. Physical NIC/USB, remote CI and formal candidates were not run. Normal EFI SHA256: `b60789abe3a63db8007b71e395ac4265080383af8342df0a75f55e83c34d21fa`.

### Upstream and licensing

The reset/PHY/register adaptation follows the pinned FreeBSD Intel reference at commit `833d39bd2e38421a14ea489a956261a0fa993fbc`; `e1000_ich8lan.c` SHA256 is `b0b78404c057911ed0cfc698df3e5a62f498b2855f300276975446860eee5329`. Derived modules retain Intel's complete BSD-3-Clause header. The reference originals and hash manifest remain unchanged. The factual BAR1 selection was checked against `sys/dev/e1000/if_em.c` and `if_em.h` at the same revision; these files were consulted, not copied into the repository. The consulted [if_em.c](https://github.com/freebsd/freebsd-src/blob/833d39bd2e38421a14ea489a956261a0fa993fbc/sys/dev/e1000/if_em.c) SHA256 is `a79d6160c83ef359dfd241893efc0246d74db69db03a225cd535e25b2a2694c5`; [if_em.h](https://github.com/freebsd/freebsd-src/blob/833d39bd2e38421a14ea489a956261a0fa993fbc/sys/dev/e1000/if_em.h) is `c2f76d658823e0beffe3e92de293be8c42778413579ac72829dbb6a1c11eac95`.

DHCP and ACD are adopted from the already pinned lwIP commit `77dcd25a72509eb83f72b033d219b1d40cd8eb95` (STABLE-2_2_1). Their individual original BSD-3-Clause headers are preserved in the source, `COPYING-DHCP` and NOTICE; UPSTREAM/SHA256SUMS and distribution license inclusion are updated. No new Rust dependency or GPL driver code is introduced.

## 日本語

全ローカル回帰を完了した開発到達点であり、実機受入ではありません。通常版は引き続きQEMU82574を有効にし、I218-V通信は `i218-network` 指定時のみです。`i218-phy-probe` と `i218-power-probe` は別の診断です。QEMUはI218-Vを提供しないため、同feature付きQEMUの成功は共有経路の回帰でありI218 register動作の証明ではありません。NUC5実機reset/link/通信、OEM設定、Realtekは未確認です。

### 実装した境界

任意I218経路はExitBootServices前にfirmwareのBAR0/BAR1資源を検証し、既存の範囲付きMMIO方針でmapします。BAR1 flashは読取りcycleだけを発行し、descriptor/region/bank署名・範囲とNVM checksumを検証します。flashアドレスの推測、NVM書込み、無効bankへのfallbackは行いません。software LCD/OEM設定が必要な `FEXTNVM.SW_CONFIG` は拒否し、全NUC5構成で動くとは主張しません。

resetは管理ME ULP退出または上限付きsoftware ULP wake、PHY所有権/page操作、PHY設定gate、master無効化、Intelのreset/PCI同期待機、auto-read完了、host-wake解除を含みます。reset blackout中にSTATUS flushのMMIO読取りを挿入しません。reset完了不明時は停止しメモリを再利用しません。copper linkは全二重、有効なfirmware MAC、対象PHYを要求します。reset/link後に検証済み4GiB未満のring/bufferを初期化して初めてDMAを有効にします。停止ではRX/TXとPCI bus masterを無効化し、I218内部master pending解除を確認してから再利用します。停止失敗はfatalです。host試験で実機動作を保証しません。

共有lwIPはACDを含む実DHCPを使い、固定IP fallbackはありません。supplied lease成立前はアドレス無しです。renew/rebindは有効leaseを保持し、expiryは解除します。初回/アドレス喪失時の30秒期限でNICを停止し、viewer入力は継続します。再起動を含めpollの時刻epochはboot基準です。upstreamのDHCP transaction counterは対応付けであり暗号学的保証ではなく、DHCP自体も未認証です。

### 検証と証拠

ローカルの `out/i218-regression/verification.json` に最終matrix・command・EFI hashを記録し、先行する限定検証は `out/i218-network` に保持します。security feature付きworkspace **398件**合格（defaultのみ381件とは重複）。Python7suite52件合格、smoke consumer suiteは追加再確認しました。固定版の実lwIPでDHCP7シナリオ（動的address77、renew、誤transaction ID、NAK、server不在、rebind、expiry）、lease成立後のendpoint再起動50回、別のpool/heap試験でDHCP待ち再起動1,000回が合格。BasicData/ESP仮想媒体make/deploy/updateの2経路も合格です。C/Rust sample14 ELFを再生成して回帰投入物と一致し、別archiveの展開先でもRust4 ELFが一致しました。

QEMU全回帰は、suite間の重複や限定再試験を含む **298ケース/構成実行**が合格しました。固有アプリ298本という意味ではありません。先行296ケースmatrix（CI54、C1/2/3、通常/debug external ELF、BasicData/ESP、viewer/cooperative/5段hub、delayed/hotplug/address、解像度/hold/startup、settings writer、r2/Rust mixed-FP、LAN/exFAT反復起動、BT UI/OUT/software再開/gamepad/adapter不在）にI218 feature付きDHCP2条件を加えています。QEMUはBT adapterやI218-Vを提供せず、software fixtureを実機互換の証明にはしません。

初回QEMU失敗5件は実DHCP導入に伴うconsumer/診断fixtureの順序前提でした。live-storage4条件はlease成立前にREADやtimeoutを開始し、意図的TX-timeout featureは最初のDHCP DISCOVERのdoorbell自体を抑止します。診断storage probeはsupplied lease成立後に開始し、遅延fixtureは既存 `input-persistent` を使ってEscまたは期待するerrorで終了。通常/hub2/disconnect/timeoutの4条件はdata/hash、pending中の入力/UDP、取消、DMA停止を緩めず合格しました。TX負例はDHCP開始・アドレス無し、TX TIMEOUT、NIC DMA停止、その後の入力/file継続を厳密に要求し、架空のleaseを作りません。CIのfeature構成も試験と同期しました。初回失敗5回と合格298回で計303構成の試行です。

最後の同期修正は診断 `live-storage-probe` だけに限定し、通常storage I/Oをgateしません。`out/i218-regression/source-impact.json` に影響分析を記録。他featureはprobe無し、または `live-file-probe` によって当該分岐を除外し、通常/debug最終再buildは全matrix投入物と **byte一致**。影響するstorage条件は全て再実施しました。external ELF timing3件は機能試験/build終了後、単独実行で合格しています。

server不在では実30秒期限、NIC DMA停止、その後のDownのapp受領を確認。fixtureの空viewer一覧に再描画は不要です。Ethernet計数/checksum/file/pixelの厳密比較を保持し、先行する単独wire fixtureではhost→guest103、guest→host106からRX6/TX13 wrapを正確に検証。exact wrapを下限へ変える案は自動審査で実行前に拒否され、適用していません。cooperative suiteの既存下限は今回以前からのものです。

先行checkpointの失敗も保持します。診断feature取り違えによるviewer marker不足、3回目remount timeout後の隔離反復成功（初回原因未確定）、live-fileのlease前開始と診断10秒窓の不整合です。通常版期限は延長していません。Apple ARM host testへUEFI bootを含めた指定ミス、SDK builder path誤りはruntime不具合と分けて修正しました。実NIC/実USB、remote CI、正式候補は未実施です。通常EFI SHA256：`b60789abe3a63db8007b71e395ac4265080383af8342df0a75f55e83c34d21fa`。

### 原典とライセンス

reset/PHY/register移植は固定FreeBSD Intel参照commit `833d39bd2e38421a14ea489a956261a0fa993fbc` に基づき、`e1000_ich8lan.c` SHA256は `b0b78404c057911ed0cfc698df3e5a62f498b2855f300276975446860eee5329` です。派生Rust moduleはIntel BSD-3-Clause全文headerを保持し、参照原典とhash台帳は変更していません。BAR1選択は同revisionの `sys/dev/e1000/if_em.c` / `if_em.h` で事実確認し、当該file自体はrepoへコピーしていません。 同revisionの参照 `if_em.c` SHA256は `a79d6160c83ef359dfd241893efc0246d74db69db03a225cd535e25b2a2694c5`、`if_em.h` は `c2f76d658823e0beffe3e92de293be8c42778413579ac72829dbb6a1c11eac95` です。

DHCP/ACDは既存固定lwIP commit `77dcd25a72509eb83f72b033d219b1d40cd8eb95`（STABLE-2_2_1）から採用しました。各原典のBSD-3-Clause headerをsource・`COPYING-DHCP`・NOTICEへ保持し、UPSTREAM/SHA256SUMSと配布license inclusionを更新しています。Rust依存追加やGPL driver codeの導入はありません。
