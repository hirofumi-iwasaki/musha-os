# USB build entry points / USBビルドの入口

正式配布・次版プレビューは[配布手順](distribution-usage.md)の
`tools/prepare-release.py`を使用してください。[配布方針](release-distribution-policy.md)に従い、
初回用小型イメージとmanifest付き更新ZIPを同一通常EFIから生成します。

`tools/build-usb.py`は既存の開発・T2診断用経路です。通常配布の更新ZIPとは別形式です。
既定ではファイル一式のみ生成し、ディスクイメージを作成・ハッシュ走査しません。

```sh
python3 tools/build-usb.py --output-dir out/test-usb
# QEMU用イメージが必要な開発時だけ明示指定
python3 tools/build-usb.py --output-dir out/test-usb-qemu --with-image
# 読取専用T2-D1の調査用。通常配布に混ぜない。
python3 tools/build-usb.py --output-dir out/t2-diagnostics --t2-diagnostics
```

独立した一時target-dirでビルドし、既存出力は拒否します。
`--size-bytes` / `--size-mib`は`--with-image`を伴う従来の仮想イメージ検証だけに使用します。
全容量像の生成・ハッシュ処理は通常更新では使いません。
配布用更新は[差分更新ツール](distribution-usage.md)を使い、既存FAT32・ユーザーファイルを保持します。

通常v0.2.0は[ファイルビューア](file-viewer.md)です。↑↓・Enter・Page Up/Down・Esc・Qで操作します。
旧H19の診断画面とは区別してください。
T2内蔵入力は未実装で、[T2-D1情報採取](t2-d1-diagnostics.md)は独立した調査です。
