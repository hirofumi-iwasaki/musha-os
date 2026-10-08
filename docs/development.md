# 開発手順と最初の実装

## 現在の実装範囲

Rust no_stdのUEFIアプリが `Hello Musha-OS!` を表示し、キー入力後に
ファームウェアへ戻る。これは起動確認用で、Boot Servicesを利用する。
GOP直接描画、ExitBootServices、RAM arena、独自USB / NIC、lwIPは未実装。
最終診断アプリの完成や0.1.0合格を意味しない。

## ビルド

Rust 1.99.0とUEFIターゲットをrust-toolchain.tomlで固定する。プロジェクト内のツールを使う場合:

```sh
export RUSTUP_HOME="$PWD/.local-tools/rustup"
export CARGO_HOME="$PWD/.local-tools/cargo"
export PATH="$CARGO_HOME/bin:$PATH"
sh tools/build-esp.sh
```

通常のrustup環境では `rustup target add x86_64-unknown-uefi` 後に
同じビルドスクリプトを実行する。Cargo.lockは追跡し依存版を固定する。
生成物は `out/esp/EFI/BOOT/BOOTX64.EFI`。
これはESP用ディレクトリで、GPT / FAT32のUSBディスクイメージではない。

## 起動確認

FAT32の試験用USBへEFIディレクトリをコピーし、Secure Bootを無効にした
試験機でUEFI起動する。USBをフォーマットする自動処理はまだ提供しない。
期待結果は挨拶と起動確認段階の表示、キー入力後のファームウェアへの復帰。
実機・QEMUでの起動試験結果はまだない。

## 次の実装

GOP情報の取得と直接描画、起動情報の型定義、メモリマップ取得、
ExitBootServicesと自前スタックへの切替を段階的に追加する。
r-efiはUEFI定義のみを利用する。[依存ライセンス](third-party.md)を参照。
