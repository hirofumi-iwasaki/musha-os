# Shared display grid / 共通文字グリッド

Status: record (2026-10-14)

## Current applicability / 現在の適用範囲

English: The v0.3.0 record below retains its original layout, tests, screenshots and hashes. New v0.4.0 startup/application text follows [boot-log](boot-log.md): one full-frame console at the top-left, white on black, no startup fill/panels/footer/80x24 app box. The base font, height-derived scale and row pitch remain; the viewer keeps its own layout. This note adds no new hardware validation.

日本語：以下v0.3.0のlayout/試験/写真/hashは原記録を保持。新v0.4.0の起動/app表示は[bootlog](boot-log.md)へ移行し、左上起点の全画面白黒console、起動全消去/panel/footer/80x24 app枠無し。基本font/高さ由来scale/行ピッチを保持しviewerは独自layout。本注記は新実機合格ではありません。

## English

Owner decision (2026-10-14): include the shared display grid in 0.3.0 and
repeat acceptance on the rebuilt candidate. This changes boot status, fatal
messages, diagnostics, the console and viewer plus tests; it is not a single
font constant adjustment. Historical results remain tied to their old EFI hashes.
The local normal test EFI was deployed for readability review; this is distinct
from the next clean-source distribution candidate and its release acceptance.

`framebuffer/src/metrics.rs` defines scale = clamp(round(height/600),1,4),
cell width 6*scale, glyph height 8*scale, row pitch 10*scale and two-cell margins.
The full grid is floor(width/cell width) by floor(height/row pitch). Label/value
columns are 2/18. Named status rows replace legacy pixel positions. Diagnostic
wrapping and pagination share these metrics and retain bounded 64-byte lines
and 42-row storage. At width >=1200 the panel starts at total columns/2, row 2.
At narrower widths it occupies full width starting at row 20, below status
rows, so the two displays cannot overwrite one another. Clear rectangles also
use the grid. Exception/failure messages use the same coordinate system.

The console retains its 80x24 limit and begins at cell (2,2). The viewer retains
80x28 content and existing names/navigation. Its scale is capped further by
width/(6*84) and height/(10*32) to preserve that layout on smaller displays.
Its glyph proportions, margins and line pitch come from the same Metrics type.
A full responsive viewer redesign is deliberately outside this display fix.
At 320x356, glyphs and rows remain bounded; long viewer text retains the existing
horizontal clipping limitation. No ABI, syscall, filenames, USB exploration,
reset/deadline policy or distribution format changes.

### Evidence (2026-10-14)

Evidence root: `out/display-grid/` (local ignored test outputs).

- Host: 53 passing tests across framebuffer, viewer and API packages, including
  the requested resolution table and a 320x356 render test guarding margins,
  rejected out-of-grid rows and framebuffer stride padding (`host-final.log`).
- Feature-free release build and diagnostics-detail build succeed.
- Feature-free QEMU: 26 external-ELF cases pass, including exit, fault, rejection,
  long log, high resolution and absent viewer (`normal-final/report.json`).
- Exact 1024x768, 1600x1200 and 3840x2160: Hello and absent viewer each pass;
  absent includes list/open/back full-pixel comparisons with debug output zero
  (`modes-report.json`). These builds use **display-grid-probe**, which selects
  only the requested GOP mode for testing; production maximum-mode selection
  remains unchanged. They are not release candidates.
- Starting and runtime/diagnostic screenshots at those three modes plus default
  2048x2048: eight passing cases (`stages-report.json`). The test-only
  MUSHA_GRID_HOLD stops the stage for capture, and is absent from normal builds.
- Four Hello screens match independently rendered expected pixels byte for byte
  (`hello-reference-*/result.json`). Viewer references are derived from old
  verified `out/usb-usability/pixels` glyph pixels, resampled onto the new grid
  (`ref-*/reference-provenance.json`, `reference-default/`). No new guest capture
  was accepted as its own expected reference. The old integer-only resize tool
  cannot move separated rows or downscale glyphs; grid-viewer-reference.py does
  both. Existing references are preserved.
- Visual inspection: 1600x1200 and 1024x768 diagnostics and 3840x2160 console
  have aligned rows/values and no overlap. NUC5 readability was approved by the owner (IMG_7641.MOV). Ryzen
  IMG_7643.MOV shows separated panels at 3840x2160 and Hello/exit 0;
  three cold boots are owner-reported. See hardware-acceptance-0.3.0.md.
  Deployment provenance: out/display-grid/usb-deploy/provenance.json.
- Boot-source scan: no remaining `.text(` calls; the sole boot `text_scaled`
  call is the diagnostic renderer with metric-derived x/y/scale. RGB literals
  are colors, not coordinate literals. Formatting and diff checks pass.

Commands: cargo test -p musha-framebuffer -p musha-viewer -p musha-api;
cargo build --locked --release --target x86_64-unknown-uefi -p musha-boot
(with separate target directories; additionally diagnostics-detail and test-only
probe variants). QEMU commands and results are retained in the report files.
The test helpers are tools/smoke-display-grid.py, grid-viewer-reference.py,
check-grid-console.py and the existing external/normal-viewer smoke tools.

Normal test EFI SHA-256:
`ba23cabb988af8349924b78914000d030899ac3589568ce8b79fa7e3bcf21cad`.
This is a local test build, not a newly approved distribution candidate.
The prior intermittent debug timeout result is not reclassified by this display
validation; the entire USB/C1/C2/C3 release matrix was not rerun here.

## 日本語

所有者決定（2026-10-14）：共通グリッドを0.3.0へ含め、新候補で再受入します。
起動状態・致命的エラー・診断・コンソール・ビューアーと試験にまたがる変更です。
過去の実機記録は当時のEFIに対して保持します。通常版のローカル試験EFIは読みやすさ
確認用にUSB配備済みですが、次のクリーンソース配布候補と公開受入とは区別します。

framebuffer/src/metrics.rsに倍率clamp(round(高さ/600),1,4)、セル幅6×倍率、
字形高8×倍率、行ピッチ10×倍率、余白2セルを集約しました。総列・行数は画面寸法
をセル幅・行ピッチで割り切り捨てます。ラベル2列、値18列。起動表示は名前付きの
連番行へ移行し、消去矩形・エラー表示もグリッドを使います。診断は64バイト・
42行の記録上限を保持。幅1200以上は総列数/2・2行目から右半分、狭い画面では
状態行の下の20行目から全幅に配置し、状態表示との重なりを防ぎます。

コンソールは原点(2,2)、上限80×24を維持します。ビューアーは既存の80列・28行と
名前表示・操作を維持するため、画面幅/(6×84)、高さ/(10×32)でも倍率を制限します。
字形比率・行間・余白は共通メトリクスです。320×356でも行と描画境界を守り、
長いビューアー行の横方向クリップは従来の制約として残します。ABI・syscall・
ファイル名・USB探索・リセットや待機期限・配布形式は変更しません。

証拠はout/display-grid/へ保存しました。関連ホスト53件合格、通常版と診断版の
ビルド成功、最終通常版QEMU26件合格。指定3解像度のHello・ビューアー6件と、
既定2048×2048を含む起動・診断画面8件も合格です。指定モードと画面停止は
試験専用display-grid-probeでのみ有効で、通常版のGOP最大モード選択は不変です。
Hello4解像度を独立描画の期待値と全画素比較し、ビューアーlist/open/backは
旧検証済み字形から新グリッドへ移した期待値と比較しました。debug出力は0 byte。
既存の拡大専用resizeツールは行の移動・縮小に対応しないため、新しい変換ツールを
使用し、参照更新理由と元・新ハッシュを保存しています。旧参照は保持しています。

1600×1200・1024×768の診断と3840×2160のコンソールを目視確認し、重なりは
ありません。NUC5は所有者が読みやすさを確認済み（IMG_7641.MOV）。Ryzenは
IMG_7643.MOVで4Kのパネル分離とHello・終了0を確認し、冷間3回は所有者報告として
記録しました。詳細はhardware-acceptance-0.3.0.md、配備記録は
out/display-grid/usb-deploy/provenance.jsonです。
通常版試験EFIのSHA-256は上記。過去のdebug timeout不安定事象の解決を意味せず、
USB・C1/C2/C3を含む公開用全回帰を今回再実行したものでもありません。
