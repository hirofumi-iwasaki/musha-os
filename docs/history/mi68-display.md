# Readable display / 読みやすい表示

Status: record (2026-10-15)

## English

The current display rule (owner decision 2026-10-14) replaces the earlier
MI68 fixed enlargement. Boot status, diagnostics and the external console share
`framebuffer/src/metrics.rs`: scale = clamp(round(height / 600), 1, 4),
6*scale cell width, 8*scale glyph height, 10*scale row pitch, and two-cell
margins. Labels start in column 2 and values in column 18. The right panel starts
at total columns / 2 for widths >=1200; smaller modes place it at row 20 across
the width, below boot status. Diagnostic page timing is unchanged.
The viewer preserves 80 columns and 28 rows by reducing scale when needed;
its cell proportions, line pitch and margins use the same metrics.
GOP maximum-mode selection is unchanged. Historical photographs are not edited.
See [hardware diagnostics](../hardware-diagnostics.md).

## 日本語

現行規則（所有者決定2026-10-14）は従来のMI68固定拡大を置き換えます。
起動状態・診断・外部コンソールはframebuffer/src/metrics.rsを共有し、
倍率clamp(round(高さ/600),1,4)、セル幅6×倍率、字形高8×倍率、
行ピッチ10×倍率、余白2セルとします。ラベルは2列、値は18列です。
幅1200以上では右パネルは総列数/2から、狭い画面では状態行の下の20行目から
全幅を使います。診断ページの切替周期は変更しません。
ビューアーは80列・28行を守るため必要に応じ倍率を抑え、セル比率・行間・余白は
共通メトリクスを使います。GOP最大モード選択は不変で、過去の写真は変更しません。
[ハードウェア診断](../hardware-diagnostics.md)を参照してください。
