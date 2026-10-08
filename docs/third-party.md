# Third-party dependencies

## English

| Dependency | Pinned version | License | Purpose |
|---|---|---|---|
| r-efi | 5.3.0 | MIT OR Apache-2.0 OR LGPL-2.1-or-later | UEFI type and protocol definitions |

We use r-efi under its Apache-2.0 option without modifications.
Source: https://github.com/r-efi/r-efi . When distributing it, retain the dependency's
copyright and attribution notices alongside the project's LICENSE.
Cargo fetches the upstream source. If vendoring the source, include its original license files.
Cargo.lock checksums pin the fetched artifacts. lwIP has not yet been incorporated.

---

## 日本語

**第三者依存**

| 依存 | 固定版 | ライセンス | 用途 |
|---|---|---|---|
| r-efi | 5.3.0 | MIT OR Apache-2.0 OR LGPL-2.1-or-later | UEFIの型・プロトコル定義 |

r-efiはApache-2.0の条件を選択して利用する。変更は加えていない。
採用元: https://github.com/r-efi/r-efi 。配布時は本体LICENSEとともに
依存元の著作権・帰属表示を保持する。依存元ソースはCargoが取得する。
ソースをvendorする場合は元のライセンスファイルを含める。
Cargo.lockのチェックサムで取得物を固定する。lwIPはまだ取り込んでいない。
