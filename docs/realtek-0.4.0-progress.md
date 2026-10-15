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
