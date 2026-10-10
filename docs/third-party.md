# Third-party dependencies

## English

| Dependency | Pinned version | License | Purpose |
|---|---|---|---|
| FreeBSD Intel e1000 | `833d39bd2e38421a14ea489a956261a0fa993fbc` | BSD-3-Clause | I218 PHY/semaphore Rust adaptation; original C references are not compiled |
| r-efi | 5.3.0 | MIT OR Apache-2.0 OR LGPL-2.1-or-later | UEFI type and protocol definitions |
| lwIP | 2.2.1 / `77dcd25a72509eb83f72b033d219b1d40cd8eb95` | BSD-style 3-clause; retain file notices | ARP / IPv4 / ICMP / UDP |

We use r-efi under its Apache-2.0 option without modifications.
Its upstream AUTHORS and provenance are retained under `third_party/r-efi`;
include AUTHORS with binary distributions.
Source: https://github.com/r-efi/r-efi . When distributing it, retain the dependency's
copyright and attribution notices alongside the project's LICENSE.
Cargo fetches the upstream source. If vendoring the source, include its original license files.
Cargo.lock checksums pin the fetched artifacts. lwIP is vendored as selected unmodified sources and complete headers under `third_party/lwip`; `UPSTREAM`, `COPYING`, and `SHA256SUMS` record provenance, terms, and file hashes. The source is from the official [lwIP repository](https://github.com/lwip-tcpip/lwip), tag `STABLE-2_2_1_RELEASE`. The original license is retained; it does not become Apache-2.0. Include the repository `NOTICE` with binary/image distributions. The lwIP port and 82574 driver are original Apache-2.0 code; the I218 PHY/semaphore adaptation is BSD-3-Clause and retains Intel copyright/terms. Its unmodified sources, COPYING, UPSTREAM and SHA256SUMS are under `third_party/freebsd-e1000`. No GPL driver source is incorporated. `tools/build-esp.sh` stages `out/LICENSE` and `out/NOTICE`; include both with binary/image distributions. See [I218 port plan](i218-rust-port-plan.md) and [experimental probe](i218-phy-probe.md).

---

## 日本語

**第三者依存**

| 依存 | 固定版 | ライセンス | 用途 |
|---|---|---|---|
| FreeBSD Intel e1000 | `833d39bd2e38421a14ea489a956261a0fa993fbc` | BSD-3-Clause | I218 PHY/semaphoreのRust移植。C原本はcompileしない |
| r-efi | 5.3.0 | MIT OR Apache-2.0 OR LGPL-2.1-or-later | UEFIの型・プロトコル定義 |
| lwIP | 2.2.1 / `77dcd25a72509eb83f72b033d219b1d40cd8eb95` | BSD型3条項。各fileのnoticeを保持 | ARP / IPv4 / ICMP / UDP |

r-efiはApache-2.0の条件を選択して利用する。変更は加えていない。
元のAUTHORSと採用元を`third_party/r-efi`に保持し、バイナリ配布へAUTHORSを添付する。
採用元: https://github.com/r-efi/r-efi 。配布時は本体LICENSEとともに
依存元の著作権・帰属表示を保持する。依存元ソースはCargoが取得する。
ソースをvendorする場合は元のライセンスファイルを含める。
Cargo.lockのチェックサムで取得物を固定する。lwIPは変更していない選択ソースと全headerを`third_party/lwip`へ取り込んだ。`UPSTREAM`、`COPYING`、`SHA256SUMS`へ採用元・条件・file hashを記録した。[公式lwIP repository](https://github.com/lwip-tcpip/lwip)のtag `STABLE-2_2_1_RELEASE`を使用する。元のライセンスを保持し、Apache-2.0へ変更しない。binary/image配布には本体`NOTICE`を添える。lwIP portと82574 driverは自作Apache-2.0コード。I218 PHY/semaphore移植はBSD-3-ClauseでIntelの著作権・条件を保持する。変更しない原本、COPYING、UPSTREAM、SHA256SUMSは`third_party/freebsd-e1000`にある。GPL driverソースは取り込んでいない。`tools/build-esp.sh`は`out/LICENSE`と`out/NOTICE`を配置する。binary/image配布には両方を添付する。[I218移植方針](i218-rust-port-plan.md)と[実験用probe](i218-phy-probe.md)を参照。


## v0.2.0 development: Apple BCE reference

`musha-bce` adapts the wire layouts in FreeBSD commit
`7a48c3fd3e6ea097a8fa8f3ce3e25fe7c2d37d9e` under BSD-2-Clause.
The selected original header, mailbox and queue sources are retained unmodified in
`third_party/freebsd-apple-bce`, with `UPSTREAM`, `SHA256SUMS` and `COPYING`.
The Rust queue/timeout policy is deliberately a bounded host model, not a port of
FreeBSD's kernel services. Original C files are not compiled. The model is not
linked into the current boot image. NOTICE retains the terms for future use;
release bundles also carry COPYING and UPSTREAM. No Linux driver code is included.

BCEのwire形式は上記FreeBSD固定版を参考にBSD-2-Clauseで実装した。
元のCソースは参照用でcompileしない。通信・所有権モデルもまだbootへ接続していない。
入力処理の共通化`musha-input`は既存の自作Apache-2.0コードを移動したもの。
