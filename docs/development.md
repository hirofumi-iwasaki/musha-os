# 開発手順と最初の実装

## 現在の実装範囲

Rust no_stdのUEFIアプリがGOP情報を取得し、フレームバッファへ直接
`Hello Musha-OS!` を描画する。メモリマップを取得してExitBootServicesを実行し、
専用64KiBスタックへ切替後に `RUNTIME READY` を直接描画する。
その後は割込みを無効にして停止する。キー入力でファームウェアへ戻る方式は終了した。

これは初期ハンドオフの実装で、完全なCPU初期化済みランタイムではない。
GDT / IDT / TSSと4段ページテーブルは自前設定へ切り替える。
予約領域を除外したRAM arenaを診断アプリへ渡し、各ページの両端を読書きする。
CPU例外は診断後に停止し、復帰しない。
独自USB / NIC、lwIP、ACPI引継ぎ、panicの画面診断は未実装。
BootInfoとメモリマップは専用LoaderDataページに保存し、回収しない。
現在はRGB / BGRの32bit GOPだけに対応し、bitmask / BLT-onlyは拒否する。
メモリマップ用バッファは128KiB固定、stale map keyの再試行は最大3回。
EFIイメージ、stack、BootInfo、map、緊急stack、ページテーブル、GOP領域を
予約し、arenaに含めない。通常stackの先頭4KiBはガードページとする。

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
cargo test -p musha-framebuffer -p musha-memory
cargo build --locked --release --target x86_64-unknown-uefi -p musha-boot --features qemu-debug
mkdir -p out/esp/EFI/BOOT
cp target/x86_64-unknown-uefi/release/musha-boot.efi out/esp/EFI/BOOT/BOOTX64.EFI
python3 tools/smoke-qemu.py --qemu /opt/homebrew/bin/qemu-system-x86_64 --firmware-dir /opt/homebrew/share/qemu
```

スクリプトはq35 / TCG、256MiB、xHCI接続USBストレージ、標準VGAで起動し、
ExitBootServices、専用stack、CPUテーブル、自前CR3への切替、arena試験と
描画を終えたマーカーを確認する。
45秒以内に確認できなければ失敗とする。成功時の画面はout/qemu-normal/screen.ppm。
QEMU終了時に試験プロセスを停止し、内部ディスクや実機にはアクセスしない。
この試験は自前xHCIドライバを確認するものではない。

2026-10-08: QEMU 11.1.2 / Rust 1.99.0で起動試験に成功し、保存画面を目視確認した。
描画レイアウト、境界とpadding、算術overflow、RGB / BGRの4試験が成功。
使用ファームウェアSHA-256:

- edk2-x86_64-code.fd: `33090cc07675baa5190d9f1e84bf5176b33bcbfa9bacac522961150cdb6dbb2a`
- edk2-i386-vars.fd: `5d2ac383371b408398accee7ec27c8c09ea5b74a0de0ceea6513388b15be5d1e`

## 次の実装

ACPI情報の保存、時間源、PCI機器の列挙を次に追加する。
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
NMIの故障注入試験は未実施。ページフォルト試験は下記を参照。
試験後は `sh tools/build-esp.sh` で故障注入を含まない実機用EFIへ戻す。
[CPU例外設計](cpu-exceptions.md)を参照。

## ページ保護とarenaの試験

2026-10-08: QEMUの256MiB RAM構成で64MiB arenaを確保し、
各4KiBページの先頭・末尾をvolatileで読書きして照合した。
メモリマップ・PE解析の7テストと描画の4テストが成功した。

| feature | case | 故障注入 | 期待する#PF error |
|---|---|---|---|
| fault-pf | pf | アドレス0への書込 | 0x02 |
| fault-ro | ro | ランタイムのコードページへの書込 | 0x03 |
| fault-nx | nx | arena上の命令実行 | 0x11 |
| fault-guard | guard | 通常stackの先頭ページへの書込 | 0x02 |

CPU例外試験と同様にfeatureを一つだけ選んでEFIを再配置し、対応caseを指定する。
いずれもQEMUで期待したvector 14、error、CR2を確認した。
自前ページテーブル上で#UD / #GP / #DFも再確認した。
[メモリ設計と制約](memory.md)を参照。
