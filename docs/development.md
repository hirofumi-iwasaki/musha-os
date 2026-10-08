# Development instructions and initial implementation

## English

For an end-to-end normal build and USB artifacts, see [test USB build](test-usb-build.md).

### Current implementation scope

The Rust no_std UEFI application obtains GOP information and draws
`Hello Musha-OS!` directly into the framebuffer.
It obtains the memory map, executes ExitBootServices, switches to a dedicated 64KiB stack,
and directly draws `RUNTIME READY`.
It then stops with interrupts disabled. The earlier approach of returning to firmware on key input has ended.

This implements the initial handoff, rather than a runtime with complete CPU initialization.
GDT / IDT / TSS and four-level page tables are switched to our own configuration.
A RAM arena excluding reserved regions is passed to the diagnostic application,
which reads and writes both ends of every page.
CPU exceptions display diagnostics and halt without returning.
ACPI PM timer information is carried over; diagnostics check 100ms elapsed time and PCI device detection.
xHCI stop / reset, dedicated DMA / rings, and 600 No-Op diagnostics are implemented.
USB enumeration through Device Descriptor reads has been added, as have boot-time Boot Keyboard input
and storage read diagnostics. Initial QEMU 82574/lwIP diagnostics are implemented; see [network diagnostics](network.md). NUC NIC initialization, HPET, and on-screen panic diagnostics remain pending.
The application uses cooperative steps to test 64 RAM pages at a time.
BootInfo and the memory map are stored in dedicated LoaderData pages and are not reclaimed.
Only 32-bit RGB / BGR GOP formats are supported; bitmask / BLT-only formats are rejected.
The minimum screen size is 320×356 to accommodate additional diagnostic lines.
The memory-map buffer is fixed at 128KiB; stale map keys are retried at most three times.
Reserve the EFI image, stack, BootInfo, map, emergency stack, page tables, and GOP regions;
exclude them from the arena. The first 4KiB of the normal stack is a guard page.

### Build

rust-toolchain.toml pins Rust 1.99.0 and the UEFI target.
When using tools installed within the project:

```sh
export RUSTUP_HOME="$PWD/.local-tools/rustup"
export CARGO_HOME="$PWD/.local-tools/cargo"
export PATH="$CARGO_HOME/bin:$PATH"
sh tools/build-esp.sh
```

In a normal rustup environment, run `rustup target add x86_64-unknown-uefi`,
then use the same build script. Track Cargo.lock to pin dependency versions. Clang is also required for the lwIP C port.
The output is `out/esp/EFI/BOOT/BOOTX64.EFI`.
This is an ESP directory. Use the [shared USB image generator](usb-image.md) to create a GPT / FAT32 disk image.

### Boot check

Copy the EFI directory to a FAT32 test USB drive and boot it via UEFI on a test machine with Secure Boot disabled.
No automatic USB-formatting procedure is provided yet.
The expected result is the greeting and `RUNTIME READY`, followed by a halt without returning to UEFI.
NUC5 / NUC8 hardware boot tests have not been performed.

### QEMU boot tests

QEMU and EDK2 firmware are required. With the Homebrew version, use the following procedure.
`qemu-debug` outputs success markers to a QEMU-only I/O port; do not include it in hardware builds.

```sh
cargo test -p musha-framebuffer -p musha-memory -p musha-platform -p musha-api -p musha-xhci
cargo build --locked --release --target x86_64-unknown-uefi -p musha-boot --features qemu-debug
mkdir -p out/esp/EFI/BOOT
cp target/x86_64-unknown-uefi/release/musha-boot.efi out/esp/EFI/BOOT/BOOTX64.EFI
python3 tools/smoke-qemu.py --qemu /opt/homebrew/bin/qemu-system-x86_64 --firmware-dir /opt/homebrew/share/qemu
```

The script boots q35 / TCG with 256MiB, xHCI-connected USB storage and keyboard,
and standard VGA. It checks markers for ExitBootServices, the dedicated stack,
CPU tables, switching to our own CR3, arena tests, and completed drawing.
Fail if the checks do not complete within 45 seconds.
The successful screen is saved to out/qemu-normal/screen.ppm.
Stop the test process when QEMU exits; do not access internal disks or physical hardware.
Also check our xHCI driver's ring wraparound, enumeration of two devices, shutdown, and DMA disablement.

2026-10-08: Boot tests passed with QEMU 11.1.2 / Rust 1.99.0; the saved screen was visually inspected.
Four framebuffer tests passed, covering drawing layout, boundaries and padding, arithmetic overflow, and RGB / BGR.
Firmware SHA-256 values used:

- edk2-x86_64-code.fd: `33090cc07675baa5190d9f1e84bf5176b33bcbfa9bacac522961150cdb6dbb2a`
- edk2-i386-vars.fd: `5d2ac383371b408398accee7ec27c8c09ea5b74a0de0ceea6513388b15be5d1e`

### Next implementation work

Key input is connected to the API version 3 FIFO.
USB BOT capacity / sector read diagnostics and MBR / FAT32 root-file reads have been added.
GPT and shared-image generation are implemented. The initial bounded [application file API](file-api.md) is implemented. Initial [cooperative input/file/network progress](cooperative-io.md) is implemented. Next, prepare hardware diagnostics and test records.
See [HID input diagnostics](usb-keyboard.md) and [NUC5 test preparation](nuc5-bringup.md).
See [USB enumeration specifications](usb-enumeration.md), [DMA / ring specifications](xhci-rings.md),
the [Rust application API](app-api.md), [xHCI initialization](xhci.md),
and the [ACPI / time-source contract](acpi-timer.md).
Only UEFI definitions are used from r-efi. See [dependency licenses](third-party.md).

### CPU exception tests

`fault-ud`, `fault-gp`, and `fault-df` are QEMU-only fault-injection features.
Enable only one at a time, build, and copy the EFI output into place before testing.

```sh
cargo build --locked --release --target x86_64-unknown-uefi -p musha-boot --features fault-ud
cp target/x86_64-unknown-uefi/release/musha-boot.efi out/esp/EFI/BOOT/BOOTX64.EFI
python3 tools/smoke-qemu.py --case ud --qemu /opt/homebrew/bin/qemu-system-x86_64 --firmware-dir /opt/homebrew/share/qemu
```

For GP, change the feature to fault-gp and the case to gp; for DF, use fault-df / df.
2026-10-08: Normal boot, #UD (vector 6 / error 0), #GP (vector 13 / error 0x28),
and #DF (vector 8 / error 0) were verified in QEMU.
Also check that the exception RIP is nonzero.
For #DF, runtime checks confirmed that the handler stack was within the dedicated IST region.
NMI fault-injection tests have not been performed. See below for page-fault tests.
After testing, use `sh tools/build-esp.sh` to restore a hardware EFI build without fault injection.
See [CPU exception design](cpu-exceptions.md).

### Page protection and arena tests

2026-10-08: Allocated a 64MiB arena with QEMU configured for 256MiB RAM,
and verified volatile reads / writes at the beginning and end of each 4KiB page.
Seven memory-map / PE parsing tests and four framebuffer tests passed.

| feature | case | Fault injection | Expected #PF error |
|---|---|---|---|
| fault-pf | pf | Write to address 0 | 0x02 |
| fault-ro | ro | Write to a runtime code page | 0x03 |
| fault-nx | nx | Execute instructions in the arena | 0x11 |
| fault-guard | guard | Write to the first page of the normal stack | 0x02 |

As with CPU exception tests, select only one feature, copy the EFI output into place,
and specify the corresponding case.
All cases produced the expected vector 14, error, and CR2 in QEMU.
#UD / #GP / #DF were also rechecked with our own page tables.
See [memory design and limitations](memory.md).

### xHCI timeout tests

Build with the `xhci-timeout` feature, copy the EFI output into place,
and pass `--case xhci-timeout` to the smoke script.
Diagnose the timeout and continue through application execution and boot completion.
Rebuild without the feature for the normal version. Do not combine dedicated test features.

### Command ring timeout tests

Build with `xhci-command-timeout`, copy the EFI output into place,
and pass `--case xhci-command-timeout` to the smoke script.
Omit the doorbell to check a 20ms timeout, then verify controller shutdown,
BME disablement, application completion, and boot completion.
After testing, restore the normal build with `sh tools/build-esp.sh`.

Validation for this change: 20 host tests passed, as did QEMU's normal 600 No-Op test,
command timeout test, and page protection against a null write.

### USB enumeration tests

The normal smoke test enumerates USB storage and a High-speed keyboard.
With the same normal build, `--keyboard-usb-version 1` checks Full-speed.
Build with `qemu-debug,usb-descriptor-timeout` and pass `--case usb-descriptor-timeout`
to smoke to omit the doorbell for the 18-byte transfer.
Check a 20ms timeout, controller shutdown, DMA disablement, and application completion.
The normal build does not include injected nonresponse.

### Keyboard diagnostics

After HID_READY, normal smoke sends Shift+A through QMP and checks presses / releases.
Use `--keyboard-usb-version 1` for Full-speed and `--no-keyboard-input` to check termination without input.
The normal hardware build continues input after KEYBOARD READY and ends with Esc.

For continuous-input smoke tests, build with `--features qemu-debug,input-persistent`
and specify `--keyboard-exit`.
Adding `--keyboard-wrap` checks 160 presses / releases and ring wraparound before sending Esc.

### USB storage tests

Normal smoke also checks successful USB storage reads.
Use `--storage-fixture 512` or `--storage-fixture 4096` to add a 4MiB test raw USB disk,
then check capacity, first / last sector hashes, and an unchanged test-file SHA-256.
Add `--storage-high-speed` to test storage on a directly connected USB 2.0 port.
Build with `--features qemu-debug,storage-timeout` and pass `--case storage-timeout`
to smoke to test nonresponse during read Data IN.
See [storage specifications and limitations](usb-storage.md).

### FAT32 tests

With a normal qemu-debug build, pass `--fat-fixture mbr` or `--fat-fixture superfloppy` to smoke.
This adds known-content FAT32 USB media and checks file length / hash and an unchanged image SHA-256.
`--fat-fixture mbr --case fat-corrupt` checks rejection of a cyclic file chain and DMA shutdown.
Create a fixture in a regular file with `python3 tools/make-fat32-fixture.py out/test-fat32.raw`.
The fixture is not for UEFI boot on physical hardware. See [supported scope](fat32.md).

---

## 日本語

通常ビルドからUSB用ファイルまでの一括生成は[テストUSBビルド](test-usb-build.md)を参照してください。

**開発手順と最初の実装**

### 現在の実装範囲

Rust no_stdのUEFIアプリがGOP情報を取得し、フレームバッファへ直接
`Hello Musha-OS!` を描画する。メモリマップを取得してExitBootServicesを実行し、
専用64KiBスタックへ切替後に `RUNTIME READY` を直接描画する。
その後は割込みを無効にして停止する。キー入力でファームウェアへ戻る方式は終了した。

これは初期ハンドオフの実装で、完全なCPU初期化済みランタイムではない。
GDT / IDT / TSSと4段ページテーブルは自前設定へ切り替える。
予約領域を除外したRAM arenaを診断アプリへ渡し、各ページの両端を読書きする。
CPU例外は診断後に停止し、復帰しない。
ACPIのPM timer情報を引継ぎ、100msの経過とPCI機器の検出を診断する。
xHCIは停止・リセットに加え、専用DMA / ringとNo-Op 600回の診断まで実装済み。Device DescriptorまでのUSB列挙を追加済み。Boot Keyboardの起動時入力診断も追加済み。ストレージは読出し診断まで追加済み。QEMU 82574/lwIPの初期診断も実装した。[通信診断](network.md)を参照。NUC向けNIC初期化、
HPET、panicの画面診断は未実装。アプリは64ページずつRAMを試験する協調step方式。
BootInfoとメモリマップは専用LoaderDataページに保存し、回収しない。
現在はRGB / BGRの32bit GOPだけに対応し、bitmask / BLT-onlyは拒否する。
診断行の追加に合わせ、最小画面サイズは320×356とする。
メモリマップ用バッファは128KiB固定、stale map keyの再試行は最大3回。
EFIイメージ、stack、BootInfo、map、緊急stack、ページテーブル、GOP領域を
予約し、arenaに含めない。通常stackの先頭4KiBはガードページとする。

### ビルド

Rust 1.99.0とUEFIターゲットをrust-toolchain.tomlで固定する。プロジェクト内のツールを使う場合:

```sh
export RUSTUP_HOME="$PWD/.local-tools/rustup"
export CARGO_HOME="$PWD/.local-tools/cargo"
export PATH="$CARGO_HOME/bin:$PATH"
sh tools/build-esp.sh
```

通常のrustup環境では `rustup target add x86_64-unknown-uefi` 後に
同じビルドスクリプトを実行する。Cargo.lockは追跡し依存版を固定する。lwIP C portのためClangも必要。
生成物は `out/esp/EFI/BOOT/BOOTX64.EFI`。
これはESP用ディレクトリ。[共通USBイメージ生成](usb-image.md)でGPT / FAT32ディスクイメージを作成できる。

### 起動確認

FAT32の試験用USBへEFIディレクトリをコピーし、Secure Bootを無効にした
試験機でUEFI起動する。USBをフォーマットする自動処理はまだ提供しない。
期待結果は挨拶と `RUNTIME READY` の表示。その後は停止し、UEFIへ戻らない。
NUC5 / NUC8の実機起動試験はまだない。

### QEMUによる起動試験

QEMUとEDK2ファームウェアが必要。Homebrew版は次の手順で試験できる。
`qemu-debug` はQEMU専用I/Oポートに成功マーカーを出すための機能で、実機版には含めない。

```sh
cargo test -p musha-framebuffer -p musha-memory -p musha-platform -p musha-api -p musha-xhci
cargo build --locked --release --target x86_64-unknown-uefi -p musha-boot --features qemu-debug
mkdir -p out/esp/EFI/BOOT
cp target/x86_64-unknown-uefi/release/musha-boot.efi out/esp/EFI/BOOT/BOOTX64.EFI
python3 tools/smoke-qemu.py --qemu /opt/homebrew/bin/qemu-system-x86_64 --firmware-dir /opt/homebrew/share/qemu
```

スクリプトはq35 / TCG、256MiB、xHCI接続USBストレージとキーボード、標準VGAで起動し、
ExitBootServices、専用stack、CPUテーブル、自前CR3への切替、arena試験と
描画を終えたマーカーを確認する。
45秒以内に確認できなければ失敗とする。成功時の画面はout/qemu-normal/screen.ppm。
QEMU終了時に試験プロセスを停止し、内部ディスクや実機にはアクセスしない。
自前xHCIドライバのリング周回と2機器の列挙、停止・DMA無効化も検査する。

2026-10-08: QEMU 11.1.2 / Rust 1.99.0で起動試験に成功し、保存画面を目視確認した。
描画レイアウト、境界とpadding、算術overflow、RGB / BGRの4試験が成功。
使用ファームウェアSHA-256:

- edk2-x86_64-code.fd: `33090cc07675baa5190d9f1e84bf5176b33bcbfa9bacac522961150cdb6dbb2a`
- edk2-i386-vars.fd: `5d2ac383371b408398accee7ec27c8c09ea5b74a0de0ceea6513388b15be5d1e`

### 次の実装

キー入力をAPI版3のFIFOへ接続済み。USB BOTの容量・セクタ読出し診断も追加済み。MBR / FAT32のルートファイル読出しも追加済み。GPTと共通イメージ生成は実装済み。初期の上限付き[アプリfile API](file-api.md)も実装済み。初期の[協調I/O](cooperative-io.md)も実装済み。次は実機診断・試験記録を整える。
[HID入力診断](usb-keyboard.md)と[NUC5試験準備](nuc5-bringup.md)を参照。
[USB列挙仕様](usb-enumeration.md)を参照。
[DMA / ring仕様](xhci-rings.md)を参照。
[RustアプリAPI](app-api.md)と[xHCI初期化](xhci.md)を参照。
[ACPIと時間源の契約](acpi-timer.md)を参照。
r-efiはUEFI定義のみを利用する。[依存ライセンス](third-party.md)を参照。

### CPU例外試験

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

### ページ保護とarenaの試験

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


### xHCI timeout試験

`xhci-timeout` featureでビルドしてEFIを再配置し、smokeスクリプトに
`--case xhci-timeout`を指定する。期限切れを診断し、アプリ実行と起動完了まで継続する。
通常版はfeatureを外してビルドし直す。専用feature同士は組み合わせない。


### Command ring timeout試験

`xhci-command-timeout` featureでビルドしてEFIを再配置し、smokeスクリプトへ
`--case xhci-command-timeout`を渡す。doorbellを省略して20msの期限切れを検査し、
controller停止、BME解除、アプリ完了と起動完了まで確認する。
試験後は `sh tools/build-esp.sh` で通常版へ戻す。

今回の検証: ホスト20テスト、QEMUの通常600 No-Op・command timeout・null書込のページ保護が成功。

### USB列挙の試験

通常smokeでUSBストレージとHigh-speedキーボードを列挙する。
同じ通常ビルドで `--keyboard-usb-version 1` を指定するとFull-speedを確認できる。
`qemu-debug,usb-descriptor-timeout` featureでビルドし、smokeへ
`--case usb-descriptor-timeout` を渡すと、18byte転送のdoorbellを省略する。
20msの期限切れ、controller停止、DMA無効化、アプリ完了を確認する。
通常ビルドには応答停止の注入を含めない。

### キーボード診断

通常smokeはHID_READY後にQMPでShift+Aを送信し、押下・解放を検査する。
`--keyboard-usb-version 1` でFull-speed、`--no-keyboard-input` で無入力の終了を確認する。
通常の実機ビルドはKEYBOARD READY後も入力を続け、Escで終了する。

継続入力のsmokeは `--features qemu-debug,input-persistent` でビルドし、
`--keyboard-exit` を指定する。`--keyboard-wrap` を併用すると160回の押下・解放と
ring周回を確認してからEscを送る。

### USBストレージの試験

通常smokeはUSBストレージの読出し成功も検査する。
`--storage-fixture 512` または `--storage-fixture 4096` で4MiBの試験raw USBを追加し、
容量、先頭・末尾セクタhash、試験ファイルのSHA-256不変を確認する。
`--storage-high-speed` を併用するとUSB 2.0側の直結ポートでストレージを検査する。
`--features qemu-debug,storage-timeout` でビルドし、smokeへ
`--case storage-timeout` を渡すと読出しData INの応答停止を検査する。
[ストレージ仕様と制約](usb-storage.md)を参照。

### FAT32の試験

通常のqemu-debugビルドで `--fat-fixture mbr` または
`--fat-fixture superfloppy` をsmokeへ渡す。USBに既知内容のFAT32を追加し、
file長・hashとimageのSHA-256不変を検査する。
`--fat-fixture mbr --case fat-corrupt` は循環file chainの拒否とDMA停止を確認する。
通常ファイルへのfixture作成は `python3 tools/make-fat32-fixture.py out/test-fat32.raw`。
fixtureは実機のUEFI起動用ではない。[対応範囲](fat32.md)を参照。
