# Third-party dependencies

## English

| Dependency | Pinned version | License | Purpose |
|---|---|---|---|
| r-efi | 5.3.0 | MIT OR Apache-2.0 OR LGPL-2.1-or-later | UEFI type and protocol definitions |
| lwIP | 2.2.1 / `77dcd25a72509eb83f72b033d219b1d40cd8eb95` | BSD-style 3-clause; retain file notices | ARP / IPv4 / ICMP / UDP |

We use r-efi under its Apache-2.0 option without modifications.
Source: https://github.com/r-efi/r-efi . When distributing it, retain the dependency's
copyright and attribution notices alongside the project's LICENSE.
Cargo fetches the upstream source. If vendoring the source, include its original license files.
Cargo.lock checksums pin the fetched artifacts. lwIP is vendored as selected unmodified sources and complete headers under `third_party/lwip`; `UPSTREAM`, `COPYING`, and `SHA256SUMS` record provenance, terms, and file hashes. The source is from the official [lwIP repository](https://github.com/lwip-tcpip/lwip), tag `STABLE-2_2_1_RELEASE`. The original license is retained; it does not become Apache-2.0. Include the repository `NOTICE` with binary/image distributions. The port and NIC driver are original Apache-2.0 code; no GPL driver source is incorporated.

---

## 日本語

**第三者依存**

| 依存 | 固定版 | ライセンス | 用途 |
|---|---|---|---|
| r-efi | 5.3.0 | MIT OR Apache-2.0 OR LGPL-2.1-or-later | UEFIの型・プロトコル定義 |
| lwIP | 2.2.1 / `77dcd25a72509eb83f72b033d219b1d40cd8eb95` | BSD型3条項。各fileのnoticeを保持 | ARP / IPv4 / ICMP / UDP |

r-efiはApache-2.0の条件を選択して利用する。変更は加えていない。
採用元: https://github.com/r-efi/r-efi 。配布時は本体LICENSEとともに
依存元の著作権・帰属表示を保持する。依存元ソースはCargoが取得する。
ソースをvendorする場合は元のライセンスファイルを含める。
Cargo.lockのチェックサムで取得物を固定する。lwIPは変更していない選択ソースと全headerを`third_party/lwip`へ取り込んだ。`UPSTREAM`、`COPYING`、`SHA256SUMS`へ採用元・条件・file hashを記録した。[公式lwIP repository](https://github.com/lwip-tcpip/lwip)のtag `STABLE-2_2_1_RELEASE`を使用する。元のライセンスを保持し、Apache-2.0へ変更しない。binary/image配布には本体`NOTICE`を添える。portとNIC driverは自作Apache-2.0コードであり、GPL driverソースは取り込んでいない。
