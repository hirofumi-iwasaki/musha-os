# MI68 readable display / MI68向け表示

> 最新の実機確認状況（2026-10-10）は[配布資料用の実機確認状況](hardware-verification-status.md)を参照。以下の過去の準備・検証記録は、その記録日時点の内容です。

## English

H10 doubles normal output glyphs from scale 3 (15x21 pixels) to scale 6 (30x42). Existing application text coordinates are transformed at rendering time; vertical spacing fits the GOP height up to twice the original spacing. Visitor-panel hexadecimal values omit leading zeros so labels and values remain readable. Key-value clearing follows the transformed rectangle. H11 diagnostic glyphs use the same scale 6 (30x42 pixels) as normal output, with 50-pixel lines, wrap to the available width, and paginate without a keyboard every eight seconds. At widths below 1200 the diagnostic pages use the full width. No GOP mode switch is performed. At small resolutions, the doubled visitor output can exceed the available width; MI68 should use at least 1280x768.

## 日本語

H10で通常出力の字形をscale 3（15x21ピクセル）からscale 6（30x42）へ倍増。既存アプリの文字座標は描画時に変換し、行間をGOPの高さに合わせて最大2倍にする。左側の16桁hex値は先頭ゼロを省略してラベルと値の視認性を確保。キー値の消去矩形も変換後の位置に合わせる。H11では診断文字も通常出力と同じscale 6（30x42ピクセル）、行間50ピクセルとし、表示幅で折り返して8秒毎に自動ページ切替する。幅1200未満では診断ページは全幅で表示。GOPのモード変更は行わない。小さな解像度では倍増した通常出力が幅を超えるため、MI68では1280x768以上を使用する。

H11 uses a short page header to reserve exactly one line. Larger diagnostic text increases the number of pages; all saved diagnostics remain available through the eight-second automatic rotation.

H11のページ見出しは1行に収まる短い表記を使用する。診断文字の拡大によりページ数は増えるが、保存した診断内容は8秒毎の自動切替で表示する。
