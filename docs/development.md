# 開発手順と最初の実装

## 現在の実装範囲

Rust no_stdのUEFIアプリがGOP情報を取得し、フレームバッファへ直接
`Hello Musha-OS!` を描画する。メモリマップを取得してExitBootServicesを実行し、
専用64KiBスタックへ切替後に `RUNTIME READY` を直接描画する。
その後は割込みを無効にして停止する。キー入力でファームウェアへ戻る方式は終了した。

これは初期ハンドオフの実装で、完全なCPU初期化済みランタイムではない。
GDT / IDT / TSSは自前設定へ切り替える。ページテーブルはまだ
ファームウェアの設定を保持する。CPU例外は診断後に停止し、復帰しない。
RAM arena、独自USB / NIC、lwIP、ACPI引継ぎ、panicの画面診断は未実装。
BootInfoとメモリマップは専用LoaderDataページに保存し、回収しない。
現在はRGB / BGRの32bit GOPだけに対応し、bitmask / BLT-onlyは拒否する。
メモリマップ用バッファは128KiB固定、stale map keyの再試行は最大3回。
後続の予約範囲管理でEFIイメージ、stack、BootInfo、mapを保護する必要がある。

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
期待結果は挨拶と `RUNTIME READY` の表示。その後は停止し、UEFIへ戻らない。
NUC5 / NUC8の実機起動試験はまだない。

## QEMUによる起動試験

QEMUとEDK2ファームウェアが必要。Homebrew版は次の手順で試験できる。
`qemu-debug` はQEMU専用I/Oポートに成功マーカーを出すための機能で、実機版には含めない。

```sh
cargo test -p musha-framebuffer
cargo build --locked --release --target x86_64-unknown-uefi -p musha-boot --features qemu-debug
mkdir -p out/esp/EFI/BOOT
cp target/x86_64-unknown-uefi/release/musha-boot.efi out/esp/EFI/BOOT/BOOTX64.EFI
python3 tools/smoke-qemu.py --qemu /opt/homebrew/bin/qemu-system-x86_64 --firmware-dir /opt/homebrew/share/qemu
```

スクリプトはq35 / TCG、256MiB、xHCI接続USBストレージ、標準VGAで起動し、
ExitBootServices成功後のスタック範囲検査と描画を終えたマーカーを確認する。
45秒以内に確認できなければ失敗とする。成功時の画面はout/qemu-normal/screen.ppm。
QEMU終了時に試験プロセスを停止し、内部ディスクや実機にはアクセスしない。
この試験は自前xHCIドライバを確認するものではない。

2026-10-08: QEMU 11.1.2 / Rust 1.99.0で起動試験に成功し、保存画面を目視確認した。
描画レイアウト、境界とpadding、算術overflow、RGB / BGRの4試験が成功。
使用ファームウェアSHA-256:

- edk2-x86_64-code.fd: `33090cc07675baa5190d9f1e84bf5176b33bcbfa9bacac522961150cdb6dbb2a`
- edk2-i386-vars.fd: `5d2ac383371b408398accee7ec27c8c09ea5b74a0de0ceea6513388b15be5d1e`

## 次の実装

ページテーブル・予約範囲管理、ACPI情報の保存、
RAM arenaの確保を次に追加する。
r-efiはUEFI定義のみを利用する。[依存ライセンス](third-party.md)を参照。

## CPU例外試験

`fault-ud`、`fault-gp`、`fault-df` はQEMU専用の故障注入機能。
いずれか一つだけを有効にしてビルドし、EFIを再配置してから試験する。

```sh
cargo build --locked --release --target x86_64-unknown-uefi -p musha-boot --features fault-ud
cp target/x86_64-unknown-uefi/release/musha-boot.efi out/esp/EFI/BOOT/BOOTX64.EFI
python3 tools/smoke-qemu.py --case ud --qemu /opt/homebrew/bin/qemu-system-x86_64 --firmware-dir /opt/homebrew/share/qemu
```

GPではfeatureをfault-gp、caseをgpへ、DFではfault-df / dfへ変更する。
2026-10-08: 通常起動、#UD（vector 6 / error 0）、#GP（vector 13 / error 0x28）、
#DF（vector 8 / error 0）をQEMUで確認した。例外のRIPも非ゼロであることを検査する。
#DFではハンドラのスタックが専用IST領域内であることを実行時に検査した。
NMIとページフォルトの故障注入試験は未実施。
試験後は `sh tools/build-esp.sh` で故障注入を含まない実機用EFIへ戻す。
[CPU例外設計](cpu-exceptions.md)を参照。
