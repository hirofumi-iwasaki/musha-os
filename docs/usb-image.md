# Shared GPT/FAT32 USB image

## English

### Scope and layout

Stage 1 of the [pre-hardware work plan](pre-hardware-work-plan.md) is implemented and verified in QEMU. Physical NUC5 / NUC8 boot tests remain pending.
`tools/make-usb-image.py` uses only Python's standard library and creates a new regular file with exclusive creation. Existing files, symlinks, and device paths are refused. It does not format or write a physical USB drive.

The default image is 64MiB, with 512-byte sectors, a protective MBR, primary and backup GPT headers / 128-entry arrays, and one FAT32 EFI System Partition starting at LBA 2048. It contains `EFI/BOOT/BOOTX64.EFI` and root file `MUSHA.TXT` with `Hello Musha-OS!` followed by a newline. Two FATs, backup boot sectors, and FSInfo are included. UUIDs derive from the EFI hash and image size; directory timestamps are fixed at zero. Identical EFI input and size produce identical bytes. This does not yet promise identical EFI binaries across toolchain installations.

### Build and boot

Use the Rust environment described in [development instructions](development.md).

```sh
sh tools/build-esp.sh
python3 tools/make-usb-image.py out/musha-usb.img
```

The output path must not already exist. Choose another name to retain an earlier image. `--size-mib` accepts 64MiB–128GiB. `--size-bytes` instead accepts the exact medium capacity in bytes, as a multiple of 512; use that value for a physical USB drive. Larger volumes use larger clusters. This image can be used as QEMU USB storage or copied to a physical USB drive after identifying the target. Its GPT backup header is at the end of the image; on a larger physical USB drive, partition metadata must be expanded / relocated before the strict runtime validator can accept it. Raw copying the 64MiB image onto a 32GB drive alone is therefore not yet an accepted deployment method. Exact-capacity generation is implemented; the physical writing procedure requires drive identification after arrival.

For automated diagnostics, generate the image from a `qemu-debug` EFI build:

```sh
cargo build --locked --release --target x86_64-unknown-uefi -p musha-boot --features qemu-debug
cp target/x86_64-unknown-uefi/release/musha-boot.efi out/esp/EFI/BOOT/BOOTX64.EFI
python3 tools/make-usb-image.py out/musha-usb-debug.img
python3 tools/smoke-qemu.py --usb-image out/musha-usb-debug.img --qemu /opt/homebrew/bin/qemu-system-x86_64 --firmware-dir /opt/homebrew/share/qemu
```

The actual image is attached read-only. The script checks UEFI handoff, RAM, keyboard transitions, runtime storage reads, the exact MUSHA.TXT length / hash, DMA shutdown, and unchanged image SHA-256. Do not include `qemu-debug` in physical builds.

### GPT reader contract

The FAT32 reader now accepts one GPT ESP as well as existing MBR / superfloppy layouts. It supports GPT revision 1.0, exactly 128 entries of 128 bytes, and 512 / 4096-byte logical sectors. Other array formats are Unsupported.
Validate protective MBR coverage without hybrid entries, both header CRCs, array CRC, identical primary / backup arrays, disk GUID and usable-range consistency, metadata bounds, partition bounds, nonzero unique partition GUIDs, and partition overlap. Require exactly one ESP. Corrupt metadata returns Corrupt; no repair, backup-only recovery, or disk writes occur. The existing 1024-read budget includes metadata reads.

### Verification

2026-10-08: 40 Rust host tests and four image-tool tests passed. GPT host tests include both sector sizes, corrupted CRCs, hybrid MBR, invalid ranges, overlap, duplicate GUIDs, metadata overflow, and header mismatch. Image-tool tests independently verify CRCs, FAT copies, nested EFI contents, text contents, reproducibility, invalid-input rejection, and overwrite / symlink refusal, exact capacity, and large-volume cluster geometry.
QEMU booted the generated 64MiB GPT image and read its root text file successfully; the saved screen showed FAT32 READ OK. Fragmented GPT fixtures also passed. A cyclic file chain was rejected with controller shutdown, DMA disablement, and continued RAM diagnostics. A normal hardware build was also booted from the actual image in QEMU; visual inspection confirmed keyboard input, Esc termination, RAM completion, and FAT32 READ OK. Its SHA-256 was unchanged: `fd3bfad53063e3f2f405fa9ed20e636c613566cdce28a87f3f2fb56a99a8f53e`. Physical media, firmware ownership, and 4096-byte-sector GPT boot remain unverified.

Reference: [UEFI 2.11 GPT format](https://uefi.org/specs/UEFI/2.11/05_GUID_Partition_Table_Format.html).

## 日本語

### 対象範囲と構成

[実機到着前の作業方針](pre-hardware-work-plan.md)の第1段階を実装し、QEMUで確認した。NUC5 / NUC8の実機起動試験は未実施。
`tools/make-usb-image.py`はPython標準ライブラリだけで新しい通常ファイルを排他的に作成する。既存ファイル、symlink、device pathは拒否する。実物USBのformatや書込みは行わない。

既定は64MiB、512byte sector。protective MBR、主・副GPT header / 128-entry array、LBA 2048から始まる単一FAT32 ESPを持つ。`EFI/BOOT/BOOTX64.EFI`と、Hello Musha-OS!に改行を加えたルートの`MUSHA.TXT`を含む。2つのFAT、backup boot sector、FSInfoを配置する。UUIDはEFI hashとイメージ容量から生成し、directory timestampは0に固定する。同じEFI入力と容量なら同じbyte列になる。ツール環境間でEFIバイナリ自体が一致する保証はまだない。

### 生成と起動

Rust環境は[開発手順](development.md)に従う。

```sh
sh tools/build-esp.sh
python3 tools/make-usb-image.py out/musha-usb.img
```

出力先は存在しないpathを指定する。以前のイメージを残す場合は別名を使う。`--size-mib`は64MiB～128GiBに対応する。`--size-bytes`で512の倍数の正確な媒体byte容量を指定できる。実物USBにはこの容量を使う。大容量ではclusterも拡大する。QEMUのUSB媒体として使え、対象を特定後に実物USBへ配置できる。ただしGPT backup headerはイメージ末尾にあるため、大きい実物USBではpartition metadataの拡張・移動が必要。64MiBイメージを32GB USBへ単純にraw copyするだけでは、厳密なランタイム検証に合格する配置にはならない。正確な容量に合わせた生成は実装済み。実機への書込みには到着後の対象USB特定が必要。

自動診断は`qemu-debug`版EFIからイメージを生成する。

```sh
cargo build --locked --release --target x86_64-unknown-uefi -p musha-boot --features qemu-debug
cp target/x86_64-unknown-uefi/release/musha-boot.efi out/esp/EFI/BOOT/BOOTX64.EFI
python3 tools/make-usb-image.py out/musha-usb-debug.img
python3 tools/smoke-qemu.py --usb-image out/musha-usb-debug.img --qemu /opt/homebrew/bin/qemu-system-x86_64 --firmware-dir /opt/homebrew/share/qemu
```

実際のイメージをreadonly接続し、UEFI引継ぎ、RAM、キー押下・解放、ランタイムのストレージ読出し、MUSHA.TXTの長さ・hash、DMA停止、イメージSHA-256不変を検査する。実機版に`qemu-debug`を含めない。

### GPT読出し契約

既存のMBR / superfloppyに加えてGPTの単一ESPを扱う。GPT revision 1.0、128byte entryが128個の配列、512 / 4096byte logical sectorに対応する。他の配列形式はUnsupported。
protective MBRの範囲とhybridなし、主・副header CRC、array CRC、主・副array一致、disk GUID・usable range一致、metadata境界、partition境界、非ゼロで重複しないpartition GUID、partition非重複を検査する。ESPは一つ必要。破損はCorruptとし、修復、副だけでの復旧、ディスク書込みは行わない。metadata読出しも既存の1024回上限に含める。

### 検証

2026-10-08: Rustホスト40試験と生成ツール4試験に合格。GPTは両sector長、CRC破損、hybrid MBR、不正範囲、重複、GUID重複、metadata overflow、header不一致を検査した。生成ツールはCRC、FATコピー、階層内EFI内容、text内容、再現性、不正入力、上書き・symlink拒否、正確な容量、大容量cluster構成を独立に確認する。
QEMUで生成した64MiB GPTイメージから起動し、ルートtextを読出した。保存画面でFAT32 READ OKを確認。断片化GPT fixtureも合格。循環file chainは拒否し、controller停止・DMA無効化とRAM診断継続を確認。通常実機版も実際のイメージからQEMU起動し、キー入力・Esc終了・RAM完了・FAT32 READ OKを目視確認した。SHA-256は不変: `fd3bfad53063e3f2f405fa9ed20e636c613566cdce28a87f3f2fb56a99a8f53e`。実物媒体、firmware ownership、4096byte sector GPT起動は未検証。

参照: [UEFI 2.11 GPT仕様](https://uefi.org/specs/UEFI/2.11/05_GUID_Partition_Table_Format.html)。
