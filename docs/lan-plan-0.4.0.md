# v0.4.0 LAN integration plan / LAN接続計画

## English

Design reference only,2026-10-15. No Realtek driver or third-party source is copied, ported, vendored or compiled by this change. [Scope goal13](release-scope-0.4.0.md), [device matrix](device-support.md), [owner configuration](hardware-verification-status.md) and [identification procedure](ryzen-realtek-identification.md) govern acceptance.

### Separate controller work

1. Capture Ryzen Ethernet PCI vendor:device:revision/subsystem and board revision. The owner reports ASUS TUF GAMING X570-PLUS and L8200A/RTL8111H-class;10EC:8168 is expected, not measured. Read-only devices inventory now shows the first Ethernet ID/revision independent of vendor, xHCI ID/revision and bounded Ethernet journal observations. This is identification, not driver support. PCI revision alone does not identify every Realtek MAC/PHY initialization variant.
2. Proposed priority: implement known NUC5 I218-V networking first from the existing Intel foundation, while separately identifying/planning Realtek. `boot/src/net.rs` runs QEMU82574 by default and a limited optional `i218-network` path; the separate PHY probe is diagnostic, and physical I218 networking remains unverified. Keep Intel PHY/semaphore/reset/ULP and Realtek MAC/PHY/OCP/firmware initialization separate. NUC11 I225-LM is another controller family and only a future device trial, not inherited I218 support.
3. Introduce an internal bounded frame-transport boundary around the existing lwIP input/tx/poll calls. Preserve cooperative polling, buffers, deadlines, network counters and DHCP/UDP echo behavior; do not add an application API just for Realtek. Each controller owns validated BAR ranges, DMA descriptors/buffers, reset/link deadlines and stop/PCI bus-master-disable before memory reuse. Reuse network protocol tests, never Intel register constants or DMA descriptor layouts for Realtek.
4. Realtek host fixtures should check chip-family classification, MMIO widths/bounds, descriptor ownership/ring wrap, cache/order, timeout/link loss, malformed lengths and DMA shutdown. Linux register definitions provide factual cross-checks such as ChipCmd0x37, TxConfig0x40, RxConfig0x44 and RxDescAddrLow0xE4. Its RTL8168h/8111h table selects a MAC variant and firmware name; a generic8168 PCI ID does not establish firmware requirements or a safe initialization sequence. Decide exact silicon/firmware support only after identity capture.
5. Preserve QEMU82574 DHCP/UDP and concurrent USB/input regression. Available emulated Realtek devices do not prove the target RTL8111H revision. Physical completion requires separate NUC5 and Ryzen link/DHCP/OS-echo samples, failure cleanup and startup timing. No physical LAN pass is claimed here.

### Primary references and licensing boundary

Checked2026-10-15 against the official upstream moving revisions `freebsd/freebsd-src:main` and `torvalds/linux:master`; these are research references, not pinned imported dependencies. Pin an exact commit and file hashes before any future source adoption under the [third-party policy](third-party.md).

| Individual file | Checked header / purpose |
|---|---|
| [FreeBSD re(4) source](https://raw.githubusercontent.com/freebsd/freebsd-src/main/share/man/man4/re.4) | Four-condition BSD notice, including advertising acknowledgement; family/media overview, manual date2026-07-13 |
| [FreeBSD if_re.c](https://raw.githubusercontent.com/freebsd/freebsd-src/main/sys/dev/re/if_re.c) | SPDX BSD-4-Clause; controller/variant behavior reference |
| [FreeBSD if_rlreg.h](https://raw.githubusercontent.com/freebsd/freebsd-src/main/sys/dev/rl/if_rlreg.h) | No SPDX in checked opening header; full four-condition BSD notice with advertising clause; register reference |
| [Linux r8169_main.c](https://raw.githubusercontent.com/torvalds/linux/master/drivers/net/ethernet/realtek/r8169_main.c) | SPDX GPL-2.0-only; register and hardware-version cross-check |
| [Linux r8169.h](https://raw.githubusercontent.com/torvalds/linux/master/drivers/net/ethernet/realtek/r8169.h) | SPDX GPL-2.0-only; variant type reference |
| [Linux r8169_firmware.h](https://raw.githubusercontent.com/torvalds/linux/master/drivers/net/ethernet/realtek/r8169_firmware.h) | SPDX GPL-2.0-only; firmware interface reference |

A future FreeBSD-derived port must retain the individual copyright, complete conditions/disclaimer and satisfy the advertising acknowledgement when applicable; adding NOTICE alone is not a claim of full compliance. Existing e1000 BSD-3-Clause terms do not cover re files. Linux GPL code/header implementations are not incorporated into Apache-2.0 files. Any future derivative or firmware redistribution needs its own reviewed provenance/license decision; reference links do not grant a different license. A source adoption must record UPSTREAM, exact commit, COPYING, SHA256SUMS and distribution notices, following repository practice. This change adds no new third-party dependency or firmware.

### Current I218 local slice

The [reset/transport/DHCP checkpoint](i218-network-0.4.0-progress.md) adds optional `i218-network` reset, software ULP, flash-read validation, link and DMA sequencing. Normal82574 now uses DHCP without a fixed-IP fallback. Host/QEMU evidence does not verify physical I218; software LCD/OEM configuration is rejected. No Realtek implementation is included.

## 日本語

2026-10-15時点の設計参照です。この変更でRealtek driverや第三者ソースのコピー・移植・vendor・compileは行いません。[scope目標13](release-scope-0.4.0.md)、[機器表](device-support.md)、[所有者構成](hardware-verification-status.md)、[特定手順](ryzen-realtek-identification.md)を受入基準とします。

### controller別の作業

1. Ryzen EthernetのPCI vendor:device:revision/subsystemとboard revisionを採取します。所有者申告はASUS TUF GAMING X570-PLUSとL8200A/RTL8111H系で、10EC:8168は見込み・未実測です。読取り専用devices一覧はvendorを限定せず先頭Ethernet ID/revision、xHCI ID/revision、上限付きEthernet journalを表示します。識別機能であってdriver対応ではありません。PCI revisionだけで全Realtek MAC/PHY初期化variantは特定できません。
2. 優先順の提案：既存Intel基盤を使う既知NUC5 I218-V通信を先に実装し、Realtek特定・計画は別に進めます。`boot/src/net.rs` は通常QEMU82574、任意指定時に限定 `i218-network` 経路を使用します。別のPHY probeは診断であり、実機I218通信は未確認です。Intel PHY/semaphore/reset/ULPとRealtek MAC/PHY/OCP/firmware初期化を分離します。予定NUC11 I225-LMも別controllerで、将来の機器試行でありI218対応を継承しません。
3. 既存lwIP input/tx/poll呼出しの周囲に内部の上限付きframe transport境界を設けます。協調poll・buffer・期限・counter・DHCP/UDP echoを保持し、Realtek専用のアプリAPIは追加しません。各controllerが検証済みBAR範囲・DMA descriptor/buffer・reset/link期限を所有し、メモリ再利用前に停止とPCI bus master無効化を行います。protocol試験は共用してもIntel register値やdescriptor配置をRealtekへ流用しません。
4. Realtek host fixtureはchip分類、MMIO幅/範囲、descriptor所有権/ring周回、cache/order、timeout/link断、異常長、DMA停止を検査する計画です。Linux register定義のChipCmd0x37・TxConfig0x40・RxConfig0x44・RxDescAddrLow0xE4などを事実照合に使います。同ソースのRTL8168h/8111h表はMAC variantとfirmware名を選ぶため、一般的な8168 PCI IDだけでfirmware要否や安全な初期化順は確定しません。実silicon/firmware対応は特定後に判断します。
5. QEMU82574 DHCP/UDPとUSB/input並行回帰を保持します。別のRealtek仮想機器の成功は対象RTL8111H revisionの証明になりません。完了にはNUC5/Ryzenそれぞれのlink/DHCP/OS echo、失敗時停止、起動timingが必要です。今回は実機LAN成功を主張しません。

### 一次資料とライセンス境界

上表の各URLを2026-10-15に公式upstreamの移動revision `freebsd/freebsd-src:main`、`torvalds/linux:master` で確認しました。調査参照であり取込み依存の固定版ではありません。将来採用前に厳密commit/file hashを固定し[第三者方針](third-party.md)へ従います。

| 個別file（URLは上表と同一） | 確認したheader / 用途 |
|---|---|
| FreeBSD re(4) source | 広告謝辞を含む4条件BSD notice、family/media説明、manual日付2026-07-13 |
| FreeBSD if_re.c | SPDX BSD-4-Clause、controller/variant動作参照 |
| FreeBSD if_rlreg.h | 確認した冒頭にSPDXなし、広告条項を含む完全4条件BSD notice、register参照 |
| Linux r8169_main.c | SPDX GPL-2.0-only、register/hardware version照合 |
| Linux r8169.h | SPDX GPL-2.0-only、variant型参照 |
| Linux r8169_firmware.h | SPDX GPL-2.0-only、firmware interface参照 |

将来FreeBSD由来の移植を行う場合は個別著作権・全条件・免責を保持し、適用時に広告謝辞も満たす必要があります。NOTICE追加だけで全条件充足とはしません。既存e1000のBSD-3-Clauseはre fileを覆いません。Linux GPLのコード/header実装はApache-2.0 fileへ取り込みません。派生物やfirmware再配布は別途、来歴・license判断が必要で、参照リンクが別licenseを許す意味ではありません。採用時は既存運用どおりUPSTREAM・厳密commit・COPYING・SHA256SUMS・配布noticeを記録します。今回は新依存・firmwareの追加なしです。

### 現在のI218ローカル縦断

[reset/transport/DHCP checkpoint](i218-network-0.4.0-progress.md)で任意 `i218-network` のreset・software ULP・flash読取り検証・link・DMAを接続。通常82574も固定IP fallback無しのDHCPへ移行。host/QEMUは実I218の証明ではなく、software LCD/OEM設定は拒否します。Realtek実装は含みません。
