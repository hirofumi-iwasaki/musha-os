# Preparing for NUC5 hardware tests

## English

Use the [hardware diagnostic panel](hardware-diagnostics.md) and copy the [test record template](hardware-test-record-template.md) for each configuration. Record image and EFI SHA-256, exact USB capacity, firmware, RAM, ports, and photographs. NUC5 LAN currently reports an unsupported driver; networking is not a completed hardware test.

### Hardware secured

The user has reported securing a NUC5 and a 32GB USB drive.
The exact model, RAM capacity, firmware version, and USB keyboard model are not yet recorded.
The oldest reference is NUC5i5RYH / RYK (NUC5i5RYB board).
No hardware test results are available yet.

### Boot preparation

1. Prepare a monitor, a Mini HDMI / Mini DisplayPort cable suitable for the machine,
   and a keyboard supporting USB Boot Keyboard. Connect USB devices directly without a hub.
2. Use UEFI boot in firmware and disable Secure Boot for initial tests.
3. Create a normal build using the development instructions.
   The output is `out/esp/EFI/BOOT/BOOTX64.EFI`.
4. Copy it to a FAT32 UEFI boot USB drive at `EFI/BOOT/BOOTX64.EFI`.
   If files already exist, inspect their contents first.
   Automatic formatting / USB writing tools are not provided. Do not erase a disk before confirming the target USB drive.
5. Boot from USB via UEFI. After KEYBOARD READY, press and release A or Shift.
   Input continues; finish with Esc and check USB ENUMERATED and APP COMPLETE.
   KEY CODE appears on screens at least 388 pixels high.

Writing to the internal SSD is not implemented.
No RAM or storage test rewrites the entire capacity of the 32GB USB drive.
Integrated UEFI-bootable GPT/FAT32 image generation is implemented; see [image instructions](usb-image.md). A test FAT32 image generator also exists.
Our own USB Mass Storage capacity and first / last sector reads are implemented.
Reading MUSHA.TXT from the root of MBR / superfloppy FAT32 is implemented.
GPT image reads are implemented; the initial [snapshot file API](file-api.md) is implemented; on-demand disk access remains unsupported.
The final acceptance requirement of booting the same USB image in QEMU / NUC5 / NUC8 has not been met.

On successful storage diagnostics, screens at least 444 pixels high display USB READ OK and capacity.
All initially discovered USB storage devices are probed before keyboard polling begins. Port order no longer requires ending keyboard input to start storage diagnostics.
Reads from this physical USB drive have not been validated. See [supported scope](usb-storage.md).

To check FAT32, place MUSHA.TXT of at most 4096 bytes in the root of supported USB media.
Initially use a single line containing Hello Musha-OS!.
Screens at least 476 pixels high display FAT32 READ OK.
See [supported formats and limitations](fat32.md).
Do not change an existing USB drive's partition scheme before checking it.

### Recording results

Record model, RAM capacity, firmware version, Secure Boot settings, USB device models,
display connection, and EFI-file SHA-256.
Save the boot screen and record input usages, USB ENUMERATED / XHCI FAILED,
and APP COMPLETE status.
On failure, retain the line where output stopped and the device configuration.
QEMU success alone does not establish support for hardware-specific USB routing / firmware ownership behavior.

---

## 日本語

[実機診断パネル](hardware-diagnostics.md)を確認し、構成ごとに[試験記録様式](hardware-test-record-template.md)をコピーする。imageとEFIのSHA-256、USBの正確な容量、firmware、RAM、ポート、写真を記録する。NUC5 LANは現在driver未対応表示となり、実機通信試験は完了扱いにしない。

**NUC5実機試験の準備**

### 確保済み

利用者からNUC5機と32GB USBメモリの確保報告あり。
正確な型番、RAM容量、ファームウェア版、USBキーボード型番は未記録。
最古リファレンスはNUC5i5RYH / RYK (NUC5i5RYB基板)。
実機試験結果はまだない。

### 起動の用意

1. モニター、機体に合うMini HDMI / Mini DisplayPortケーブル、
   USB Boot Keyboard対応キーボードを用意する。USB機器はハブを介さず直結する。
2. ファームウェアでUEFI起動を使用し、初期試験ではSecure Bootを無効にする。
3. 開発手順で通常ビルドを作成する。生成物は
   `out/esp/EFI/BOOT/BOOTX64.EFI`。
4. FAT32のUEFI起動用USBに `EFI/BOOT/BOOTX64.EFI` の階層でコピーする。
   既存ファイルがある場合は先に内容を確認する。
   自動フォーマット・USB書込みツールは未提供。対象USBの確認前にディスク消去しない。
5. USBからUEFI起動し、KEYBOARD READY表示後にAやShiftを押して離す。
   入力は継続するため、最後にEscを押してUSB ENUMERATED、APP COMPLETEを確認する。
   高さ388pixel以上ではKEY CODEが表示される。

内部SSDへの書込み処理は未実装。USB32GBの容量全体をRAMやストレージ試験で
書き換える処理もない。UEFI起動用の統合GPT/FAT32イメージ生成は実装済み。[生成手順](usb-image.md)を参照。試験用FAT32 imageの生成ツールもある。
独自USB Mass Storageの容量・先頭 / 末尾セクタ読出しは実装済み。
MBR / superfloppy形式のFAT32で、ルートのMUSHA.TXT読出しを実装済み。
GPTイメージ読出しは実装済み。初期の[snapshot file API](file-api.md)は実装済み。オンデマンド媒体アクセスは未対応。
同一USBイメージでQEMU / NUC5 / NUC8を起動する最終合格条件は未達。

ストレージ診断の成功時、高さ444pixel以上ならUSB READ OKと容量を表示する。
初期検出したUSBストレージの診断はkeyboard polling開始前に行う。ポート順によってEsc終了までストレージ診断を待つ構成ではなくなった。
この実物USBでの読出しはまだ検証していない。[対応範囲](usb-storage.md)を参照。

FAT32の確認には、対応形式のUSBルートへ4096byte以下のMUSHA.TXTを配置する。
内容は最初はHello Musha-OS!の一行にする。高さ476pixel以上ならFAT32 READ OKを表示する。
[対応形式と制約](fat32.md)を参照。既存USBのpartition方式を未確認のまま変更しない。

### 結果の記録

型番、RAM容量、ファームウェア版、Secure Boot設定、USB機器型番、
映像接続、EFIファイルSHA-256を記録する。
起動画面を保存し、入力usage、USB ENUMERATED / XHCI FAILED、
APP COMPLETEの状態を記載する。
失敗した場合は表示が止まった行と機器構成を残す。
実機特有のUSB routing / firmware ownership等はQEMU成功だけで対応済みとしない。
