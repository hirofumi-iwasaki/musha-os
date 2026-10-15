# I218 managed-power checkpoint / I218管理電源到達点

## English

Status: partial local implementation,2026-10-15; not I218 networking or hardware acceptance. After stage6, the work plan has no numbered stage7: its required parallel tracks remain. This checkpoint starts goal13's NUC5 I218-V track, following the existing proposed Intel-first order. Realtek remains separate and requires measured revision; Mac storage/T2, physical timing and documentation gates remain open. Stage6's Bluetooth limitations are unchanged. The owner-approved stage6 commit `1f4d121` was pushed normally before this work; this work was uncommitted at that earlier checkpoint.

### Implemented vertical slice

`net/src/i218/power.rs` adapts only the firmware-managed branch of FreeBSD `e1000_disable_ulp_lpt_lp(force=true)`. The opt-in `i218-power-probe` includes the existing `i218-phy-probe`: exact PCI8086:15A3 discovery, validated BAR and memory decoding, PCI bus mastering disabled, then ME ULP exit followed by the existing bounded PHY identity read. It still returns **no network session**. The normal build's supported NIC selection is unchanged.

The same function used by runtime accepts only I218-V3, requires FWSM.FW_VALID, rejects a preexisting H2ME.ENFORCE request, clears H2ME.ULP and sets ENFORCE while preserving other bits. It observes FWSM.ULP_CFG_DONE with at most30 delays of10ms, rejecting firmware loss, floating all-ones MMIO, IO/clock error or timeout. The underlying IO/delay implementation is also finite. Once a request write is attempted, every ordinary error attempts to clear its ENFORCE bit and read back confirmation; cleanup failure supersedes the earlier error. Cleanup preserves unrelated bits changed during the handshake. It cannot undo a firmware power transition, guarantee cleanup after panic/power loss or establish that a timed-out transition completed.

Only the optional power feature adds H2ME to the existing runtime write whitelist. This slice does not perform PHY data writes, MAC/PHY reset, NVM writes, RX/TX setup, link negotiation or DMA. Missing ME is explicitly `I218 SOFTWARE ULP REQUIRED`, not a guessed fallback or successful initialization. The upstream software ULP/LANPHYPC/SMBus/K1 sequence, PHY gating and reset workarounds must be ported before full initialization. The full reset has PCI-config synchronization and other device dependencies; this partial handshake is not a replacement for that sequence.

### Provenance and deliberate differences

Primary source: [FreeBSD pinned e1000_ich8lan.c](https://github.com/freebsd/freebsd-src/blob/833d39bd2e38421a14ea489a956261a0fa993fbc/sys/dev/e1000/e1000_ich8lan.c), managed branch around lines1435–1475; constants are in the captured `e1000_ich8lan.h`. Original sources, complete BSD-3-Clause terms and SHA256SUMS remain unchanged under `third_party/freebsd-e1000`. Source SHA256: `b0b78404c057911ed0cfc698df3e5a62f498b2855f300276975446860eee5329`. The new Rust module retains Intel's complete copyright/terms/disclaimer; tests are original Apache-2.0. No Linux code, new dependency or firmware is incorporated. Existing NOTICE/distribution already includes the retained e1000 license.

Differences from upstream are explicit: V3 only; no cached ULP state; no software branch/reset; a hard300ms requested-delay limit rather than upstream post-increment timeout; fail closed on firmware disappearance/all-ones reads; reject an already-owned request; attempt ENFORCE cleanup and confirm it on ordinary error as well as success. Cleanup is a bounded safety policy, not an atomic rollback guarantee. The adaptation uses existing checked MMIO and PM-timer delays.

### Local evidence and remaining work

Evidence: `out/post-stage6/verification.json`, host/build logs and separate `power-traffic`/`default-traffic` QEMU captures. The net crate's28 host tests pass, including12 new handshake cases: reserved-bit ordering, ME absence, unsupported variants, foreign request, timeout, firmware loss, clock/read/write failures, ambiguous latched write and unconfirmed cleanup. Both ordinary QEMU82574 and the power-feature build pass ARP/ICMP,257 UDP echoes, cached-file rechecks, ring wrap/checksums and USB/NIC DMA stop. These QEMU devices are82574, **not I218**: they verify preserved integration, not the managed handshake on hardware. Current lwIP has `LWIP_DHCP=0`; no DHCP success is claimed. Full QEMU regression is not repeated for this opt-in slice; the ordinary debug and feature-free EFI bytes are compared with the previous full-regression binaries.

Remaining: software ULP recovery, exact revision/PHY confirmation, complete power/reset/workarounds, link/autoneg, I218 descriptors/DMA/stop, DHCP and physical NUC5 link/address/echo. Ryzen Realtek needs its own identified backend. Actual ME behavior, IO delays, link and DMA on I218 are unverified. No physical USB, remote CI, formal candidate, new commit/push, tag or Release is performed. Owner scope/no-shell/scenario documents are preserved separately.

Later implementation: [reset/transport/DHCP checkpoint](i218-network-0.4.0-progress.md). This page retains the earlier managed-power-only scope.

## 日本語

状態：2026-10-15の部分的なローカル実装で、I218通信や実機受入の完了ではありません。作業計画には工程6後の番号付き工程7はなく、必須並行trackが残っています。既存のIntel優先提案に従い目標13のNUC5 I218-Vへ着手しました。Realtekはrevision実測を前提とする別実装、Mac storage/T2・実機timing・文書gateも残件。工程6のBluetooth制限は維持します。承認済み工程6の`1f4d121`を通常pushした後の新作業で、この先行checkpoint時点では未commitでした。

### 実装した縦断

`net/src/i218/power.rs`はFreeBSD `e1000_disable_ulp_lpt_lp(force=true)`のME管理分岐だけを移植します。任意の`i218-power-probe`は既存`i218-phy-probe`を含み、PCI8086:15A3・BAR/memory decode・PCI bus master無効を確認後、ME ULP退出から既存の上限付きPHY ID読出しへ接続します。**通信sessionは返しません**。通常版の対応NIC選択は変更しません。

runtimeと同じ関数がV3のみを受け付け、FWSM.FW_VALIDを要求し、既存H2ME.ENFORCE requestは拒否します。他bitを保持してULPをclear・ENFORCEをsetし、最大30回×10msでFWSM.ULP_CFG_DONEを待ちます。firmware消失・全bit1の浮遊MMIO・IO/clock error・期限超過を拒否し、下位IO/delay自体も有限です。request書込みを試みた後は通常errorでもENFORCEの解除と読戻しを試み、cleanup不確認は元errorより優先します。途中で変わった無関係bitを保持します。firmware電源遷移を元に戻したり、panic/電源断後のcleanupやtimeout後の遷移完了を保証しません。

runtime書込許可へH2MEを追加するのは任意power featureだけ。PHY data書込み・MAC/PHY reset・NVM書込み・RX/TX・link交渉・DMAは実装しません。ME無しは`I218 SOFTWARE ULP REQUIRED`と明示拒否し、推測fallbackや初期化成功にしません。全初期化には上流software ULP/LANPHYPC/SMBus/K1、PHY gating、reset workaroundの移植が必要です。全resetはPCI config同期などにも依存し、今回のhandshakeで代用しません。

### 出典と意図的な差

一次原本は上記の固定FreeBSD commitの`e1000_ich8lan.c`1435–1475行付近と保存済み`e1000_ich8lan.h`定数。`third_party/freebsd-e1000`の原本・完全BSD-3-Clause・SHA256SUMSを変更せず、原典hashは英語節の通りです。新Rust moduleはIntel著作権・全条件・免責を保持し、試験は自作Apache-2.0。Linuxコード・新依存・firmwareは取り込みません。既存NOTICE/配布へe1000原licenseは接続済みです。

上流との差はV3限定・ULP cache無し・software分岐/reset無し・post-increment式ではなく要求delay最大300ms・firmware消失/全1の拒否・既存request拒否・通常error時もENFORCE解除と確認を試みること。cleanupは上限付き安全方針であり、原子的rollback保証ではありません。既存の検査済みMMIOとPM timer delayを利用します。

### ローカル証拠と残件

証拠は`out/post-stage6/verification.json`、host/build logと別出力の`power-traffic`/`default-traffic`。net crate host28件が合格し、新12件で予約bitと順序・ME無し・非対象機・他者request・timeout・firmware消失・clock/read/write失敗・書込みが反映済みか不明なerror・cleanup不確認を検査しました。通常QEMU82574とpower featureの2条件でARP/ICMP・257 UDP echo・cached file再読・ring wrap/checksum・USB/NIC DMA停止が合格。ただし仮想NICは82574で**I218ではなく**、実ME handshakeの証明ではありません。現lwIPは`LWIP_DHCP=0`でDHCP成功を主張しません。この任意sliceで全QEMUは繰り返さず、通常debug版とfeature無しEFIは前回全回帰binaryとのbytes一致を確認します。

残件はsoftware ULP、実revision/PHY、完全なpower/reset/workaround、link/autoneg、I218 descriptor/DMA/停止、DHCP、実NUC5 link/address/echo。Ryzen Realtekは特定後に別backendが必要です。実I218のME・delay・link・DMAは未確認。実USB・remoteCI・正式候補・新commit/push・tag/Releaseは実施せず、所有者scope/no-shell/scenario文書を保持します。


後続実装：[reset/transport/DHCP到達点](i218-network-0.4.0-progress.md)。本pageは先行する管理電源のみの到達点を保持します。
