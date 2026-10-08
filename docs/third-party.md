# 第三者依存

| 依存 | 固定版 | ライセンス | 用途 |
|---|---|---|---|
| r-efi | 5.3.0 | MIT OR Apache-2.0 OR LGPL-2.1-or-later | UEFIの型・プロトコル定義 |

r-efiはApache-2.0の条件を選択して利用する。変更は加えていない。
採用元: https://github.com/r-efi/r-efi 。配布時は本体LICENSEとともに
依存元の著作権・帰属表示を保持する。依存元ソースはCargoが取得する。
ソースをvendorする場合は元のライセンスファイルを含める。
Cargo.lockのチェックサムで取得物を固定する。lwIPはまだ取り込んでいない。
