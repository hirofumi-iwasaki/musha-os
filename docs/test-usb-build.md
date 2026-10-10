# Build a test USB / テストUSBのビルド

## English

Install Git, Python 3.9 or newer, Rust via rustup, and Clang with the Windows x86-64
cross-target available. macOS and Linux can use the commands below; Windows users
can use WSL. No proprietary SDK or FAT-formatting utility is required.
The repository pins Rust and its UEFI target in `rust-toolchain.toml`; rustup
installs them when Cargo first runs. The first build needs network access for
Rust and the locked Cargo dependencies. Run from a checkout of this repository:

```sh
python3 tools/build-usb.py --output-dir out/test-usb
```

The output directory must be new. The script builds the **normal** application
without experimental PHY probes or QEMU debug/fault features. It creates:

- `musha-os-fat32-files.zip`: EFI/BOOT/BOOTX64.EFI, MUSHA.TXT, LICENSE, NOTICE and r-efi-AUTHORS.
- `musha-os.img`: a 64MiB GPT/FAT32 image for QEMU by default.
- `SHA256SUMS` and `build-manifest.json`: checksums, image capacity, source commit,
  working tree state and compiler versions.
- `files/`: the same uncompressed USB files, and license copies beside the image.

For an **existing FAT32 USB**, extract the ZIP contents at the USB root, so
`EFI/BOOT/BOOTX64.EFI` and `MUSHA.TXT` are directly below that root. Preserve other
files as needed. Boot the NUC in UEFI mode with Secure Boot disabled, a display
and a directly attached USB keyboard. Expect `Hello Musha-OS!` and the diagnostic
panel; Esc ends the session. This procedure does not format or write a raw disk.
Physical NUC operation remains unverified, and I218-V networking is not implemented.

**Do not copy the default 64MiB raw image onto a 32GB USB.** Our GPT reader expects
the backup GPT at the physical medium's last sector. To generate a raw image,
obtain the exact byte capacity of the intended USB and use that value:

```sh
python3 tools/build-usb.py --output-dir out/test-usb-exact --size-bytes EXACT_BYTE_CAPACITY
```

Replace `EXACT_BYTE_CAPACITY` with the measured integer, not an estimate from the
32GB label. Raw disk writing is a separate operation that erases the target;
this build script never performs it. See [image layout and QEMU testing](usb-image.md)
and [NUC5 preparation](nuc5-bringup.md). Large images are sparse when supported by
the filesystem, but checksum calculation reads their full logical size.

## 日本語

Git、Python 3.9以上、rustup経由のRust、Windows x86-64向けにコンパイルできるClangを
用意してください。macOSとLinuxで以下を実行できます。WindowsではWSLを利用できます。
専用SDKやFATフォーマットツールは不要です。Rustの版とUEFIターゲットは
`rust-toolchain.toml`で固定され、Cargo初回実行時にrustupが導入します。
初回はRustと固定されたCargo依存の取得にネットワークが必要です。リポジトリ内で実行します。

```sh
python3 tools/build-usb.py --output-dir out/test-usb
```

出力先には新しいディレクトリを指定します。実験的PHYプローブやQEMU専用の
デバッグ・障害注入を含まない**通常ビルド**を生成します。

- `musha-os-fat32-files.zip`：EFI/BOOT/BOOTX64.EFI、MUSHA.TXT、LICENSE、NOTICE、r-efi-AUTHORS。
- `musha-os.img`：既定ではQEMU用の64MiB GPT/FAT32イメージ。
- `SHA256SUMS`と`build-manifest.json`：チェックサム、容量、ソースのコミット、未コミット変更の有無、コンパイラの版。
- `files/`：展開済みのUSB用ファイル。イメージの隣にもライセンスを配置します。

**既にFAT32のUSB**にはZIPの中身をルートへ展開します。
ルート直下に`EFI/BOOT/BOOTX64.EFI`と`MUSHA.TXT`が配置されるようにしてください。
必要な既存ファイルは保持してください。NUCをUEFI起動、Secure Boot無効に設定し、
画面とUSB直結キーボードを接続します。`Hello Musha-OS!`と診断画面を確認し、Escで終了します。
この方法ではフォーマットやディスク全体への書き込みを行いません。
実機NUCの動作は未検証で、I218-Vのネットワーク通信は未実装です。

**既定の64MiBイメージを32GB USBへそのまま書き込まないでください。**
現在のGPT読み取りはバックアップGPTが実媒体の最終セクタにあることを要求します。
ディスク全体へ書き込むイメージはUSBの正確なバイト容量を調べて生成します。

```sh
python3 tools/build-usb.py --output-dir out/test-usb-exact --size-bytes EXACT_BYTE_CAPACITY
```

`EXACT_BYTE_CAPACITY`を実測した整数に置き換えます。32GBという表示から推測しないでください。
実ディスクへの書き込みは対象を消去する別作業であり、このスクリプトは実行しません。
[イメージ構造とQEMU検証](usb-image.md)、[NUC5準備](nuc5-bringup.md)も参照してください。
大きなイメージは対応ファイルシステムでは疎ファイルになりますが、ハッシュ計算は全容量を読みます。

## T2 internal keyboard investigation / T2内蔵入力の調査

For the opt-in read-only T2-D1 image, add `--t2-diagnostics`.
It does not start an internal keyboard driver. See [T2-D1 diagnostics](t2-d1-diagnostics.md).

読取専用のT2-D1診断版は`--t2-diagnostics`を付けて作成する。
内蔵キーボードドライバーはまだ開始しない。[実機撮影手順](t2-d1-diagnostics.md)を参照。
