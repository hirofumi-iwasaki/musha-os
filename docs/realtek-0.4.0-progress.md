# Realtek local prototype / Realtekローカルprototype

Status: current

## English

Implemented locally, not committed: original Apache-2.0 `net/src/realtek.rs` provides a bounded reset/register interface, explicit hardware-admission denial, 16-byte no-offload TX/RX descriptors, ownership/length/address-overflow/FCS checks and conservative stop. Reset requires BME already off and polls at most1,000 times. Stop attempts BME-off even when register writes fail; without a reviewed silicon-specific drain sequence it returns `DrainUnverified`, never permission to reuse DMA memory. A private permission constructor is exercised only by host fixtures; runtime cannot obtain a permit. No normal-build selection or hardware writes are enabled.

This is a host-tested minimum core, **not a completed RTL8111H driver**. MAC/PHY/OCP initialization, measured identity admission, mapped hardware frame adapter, ring service, link and lwIP attachment remain blocked/incomplete. Existing lwIP DHCP/UDP/restart can be reused after that adapter exists; current82574/I218 support is not reclassified as Realtek evidence. See the [normal-enable proposal](lan-default-enablement-0.4.0.md) for exact owner capture and review requirements.

Research checked2026-10-15: [FreeBSD if_re.c](https://raw.githubusercontent.com/freebsd/freebsd-src/releng/14.3/sys/dev/re/if_re.c), [if_rlreg.h](https://raw.githubusercontent.com/freebsd/freebsd-src/releng/14.3/sys/dev/rl/if_rlreg.h), and [Linux v6.18 r8169_main.c](https://raw.githubusercontent.com/torvalds/linux/v6.18/drivers/net/ethernet/realtek/r8169_main.c). Register/descriptor facts were consulted; no driver code, tables, PHY sequences or headers were copied/translated/vendored. FreeBSD individual files retain their BSD-4-Clause/advertising obligations for any future adoption; Linux is GPL-2.0-only. No source adoption, new dependency, firmware redistribution or new upstream-license claim is made by this original prototype. The existing [license boundary](lan-plan-0.4.0.md) continues to apply.

Local evidence is recorded under `out/remaining-040`; host tests are software/register fixtures, not silicon acceptance. Physical identity and communication remain unverified. Do not use the expected PCI ID as a firmware or PHY compatibility guarantee.

## 日本語

未コミットの独自Apache-2.0実装 `net/src/realtek.rs` に、上限付きreset/register interface、明示的hardware admission拒否、16byteのoffload無しTX/RX descriptor、所有権/長さ/アドレスoverflow/FCS検査、保守的停止を追加しました。resetはBME-offを要求し、pollは最大1,000回。register書込み失敗時もBME-offを試みますが、silicon別drain順が未reviewなので `DrainUnverified` を返し、DMA再利用許可を捏造しません。private permission構築はhost fixtureのみで、runtimeはpermitを取得できません。通常選択やhardware書込みは有効化していません。

host検証済み最小coreであり **RTL8111H driver完了ではありません**。MAC/PHY/OCP初期化、実測識別admission、hardware frame adapter、ring service、link、lwIP接続は未完/blockedです。adapter成立後に既存DHCP/UDP/再起動を共用できますが、現在の82574/I218成功をRealtek証拠にはしません。必要採取項目とreview順は[通常有効化案](lan-default-enablement-0.4.0.md)に記載しています。

2026-10-15に上記FreeBSD if_re.c/if_rlreg.hとLinux v6.18 r8169_main.cを一次参照しました。register/descriptorの事実照合に限定し、driver code/table/PHY sequence/headerのコピー・翻訳・vendorはしていません。将来FreeBSD採用時は個別BSD-4-Clauseと広告条項、LinuxはGPL-2.0-onlyの境界を守ります。今回の独自prototypeで原典採用・依存追加・firmware再配布・新license適用を主張しません。既存[license境界](lan-plan-0.4.0.md)を維持します。

ローカル証拠は `out/remaining-040` に保持。host register fixtureでありsilicon受入ではありません。実機識別と通信は未確認、見込みPCI IDをfirmware/PHY互換保証として使用しません。


## Stage 0 identification branches — English (2026-10-16)

This is a research decision table, not runtime admission. Preserve the complete measured `TxConfig40` DWORD and PCI/subsystem identity; expected `10ec:8168`, board branding or PCI revision alone selects no profile. The diagnostic path remains one read after resource/UC/RO validation, with no reset, PHY/OCP write, BME change or DMA enablement.

For the nearby gigabit candidates below, compare `(raw >> 20) & 0x7cf`. These are masked patterns, not required equality of all 32 bits. The pinned [Linux v6.18 classification/MAC startup source](https://raw.githubusercontent.com/torvalds/linux/v6.18/drivers/net/ethernet/realtek/r8169_main.c) and [PHY dispatch source](https://raw.githubusercontent.com/torvalds/linux/v6.18/drivers/net/ethernet/realtek/r8169_phy_config.c) provide these reference branches:

| Masked value | MAC version | Candidate | MAC / PHY reference profile |
|---|---|---|---|
| `0x541` | 46 | RTL8168h/8111h candidate | `rtl_hw_start_8168h_1` / `rtl8168h_2_hw_phy_config` |
| `0x6c0` | 46 | RTL8168M, separate identity | same H profiles; not proof of RTL8111H |
| `0x4c0` | 40 | RTL8168g/8111g | `rtl_hw_start_8168g_1` / `rtl8168g_1_hw_phy_config` |
| `0x509` | 42 | RTL8168gu/8111gu | `rtl_hw_start_8168g_2` / `rtl8168g_2_hw_phy_config` |
| `0x502` | 51 | RTL8168ep/8111ep | `rtl_hw_start_8168ep_3` / `rtl8168ep_2_hw_phy_config` |
| `0x54a`, `0x54b` | 52 | RTL8168fp/RTL8117 | `rtl_hw_start_8117` / `rtl8117_hw_phy_config` |
| `0x540`, other patterns | unresolved here | no H admission | stop classification; investigate measured variant |
| raw `0xffffffff`, unavailable/skipped read | invalid evidence | no classification | preserve refusal reason; no extra MMIO probe |

Linux additionally distinguishes the 100-Mbit PHY case: a version-46 match without GMII becomes version48/RTL8107e. Its H reference names therefore do not establish gigabit PHY compatibility. The H entry references `rtl_nic/rtl8168h-2.fw`; this is an upstream dependency indication, not firmware bundled or accepted by Musha-OS.

FreeBSD uses a different granularity: [the register header](https://raw.githubusercontent.com/freebsd/freebsd-src/releng/14.3/sys/dev/rl/if_rlreg.h) defines the lookup mask `0x7cc00000` and H family value `0x54000000`; [the attach path](https://raw.githubusercontent.com/freebsd/freebsd-src/releng/14.3/sys/dev/re/if_re.c) separately keeps MAC revision bits `raw & 0x00700000`. Its displayed chip mask `0x7c800000` is not the lookup mask. A coarse FreeBSD H family label cannot replace the finer measured raw value or Linux branch check.

No initialization profile is implemented from this table. After capture, review the exact MAC/PHY/firmware combination, bounded polling, DMA ownership and silicon-specific drain independently before authorizing hardware writes. Source/license policy remains the earlier paragraph and [LAN plan](lan-plan-0.4.0.md): original host code only, no copied/translated driver table or sequence, no new dependency/firmware. Any future source adoption needs its own fixed provenance and applicable notices; GPL-2.0-only and the individual FreeBSD advertising-license obligations do not become Apache-2.0 through citation.

## 段階0の識別分岐 — 日本語（2026-10-16）

上表は一次資料に照らす調査表で、runtime有効化表ではありません。実測した `TxConfig40` の32bit全値とPCI/subsystem情報を保存し、見込みの `10ec:8168`、board名、PCI revisionだけでprofileを決めません。近接候補の比較は `(raw >> 20) & 0x7cf` であり、raw全値が上表の値そのものになるという意味ではありません。

`0x541` はH候補ですが、`0x6c0` は同じMAC46でもRTL8168Mという別の識別です。G/GU/EP/FPをHへ寄せず、`0x540` だけの値や未知値は未確定として停止します。GMII無しではMAC46候補が100MbitのMAC48/RTL8107eへ分かれるため、H系profile名が同じでもgigabit PHY適合の証明にはなりません。参照されたH用firmware名は依存調査の情報で、Musha-OSへの同梱・受入ではありません。

FreeBSDのlookup maskは `0x7cc00000`、H family値は `0x54000000` で、MAC revisionは別に `raw & 0x00700000` を保持します。表示用chip mask `0x7c800000` と混同しません。粗いfamily表示だけでLinux側の細分類や実測全値を置き換えません。上表のprofileは実装せず、実測後にMAC/PHY/firmware、poll上限、DMA所有権と停止根拠を別途reviewします。診断はresource/UC/RO検証後の1回読出しのまま、reset・PHY/OCP書込み・BME変更・DMA有効化は追加しません。既存license方針を維持し、コード・初期化列・第三者tableの取込みや新firmware依存はありません。
