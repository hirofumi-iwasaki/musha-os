# Prepare a hardware USB on macOS

## English

For an existing FAT32 USB, file-copy installation is sufficient: extract the
bundle's EFI directory and MUSHA.TXT at the volume root. This preserves the current
partition layout; see test-usb-build.md. The following raw-image procedure is for
an exact-capacity shared image and **erases the dedicated target USB**. It does
not write the NUC's internal SSD. Wait until the USB arrives and identify it on
this Mac before continuing.

### 1. Identify the device

Run diskutil list external physical with the USB unplugged, then plugged in.
Inspect the newly appearing whole disk. Replace diskN with that actual identifier;
never reuse an old number. Disconnect unrelated external media.

```sh
mkdir -p out
mkdir -p out
diskutil list external physical
diskutil info /dev/diskN
diskutil info -plist /dev/diskN > out/usb-device-info.plist
```

Verify the intended SanDisk model, external USB whole-disk status, exact byte
capacity and 512-byte logical sector size. Confirm it may be erased and back up
needed files. Stop if identifier, model or capacity does not match.

### 2. Generate a regular image of the exact capacity

Use the same normal EFI as the candidate, without fault/debug features. For a
source checkout build it with sh tools/build-esp.sh. In a candidate bundle skip
the build and use --efi EFI/BOOT/BOOTX64.EFI; create out with mkdir -p out. Obtain
the exact capacity from the saved plist; the Python check in the Japanese section
below rejects internal/non-whole/non-USB/non-512-byte devices. Use its output:

```sh
python3 tools/make-usb-image.py out/musha-physical-usb.img --efi EFI/BOOT/BOOTX64.EFI --size-bytes EXACT_BYTE_CAPACITY
shasum -a 256 EFI/BOOT/BOOTX64.EFI out/musha-physical-usb.img
```

Replace EXACT_BYTE_CAPACITY with the measured integer. The generator creates a
new regular file and refuses overwrites. Use sufficient local space. Changing
capacity changes the hash, so record the hardware-image hash separately. Test
that exact image in QEMU for shared-image acceptance.

### 3. Write only after rechecking

These are manual operations, never automatically run. Immediately recheck the
model/identifier/capacity and compare image bytes with target TotalSize. diskN
and rdiskN must refer to the same verified whole disk.

```sh
diskutil info /dev/diskN
stat -f %z out/musha-physical-usb.img
diskutil unmountDisk /dev/diskN
sudo dd if=out/musha-physical-usb.img of=/dev/rdiskN bs=4m
sync
sudo shasum -a 256 /dev/rdiskN
shasum -a 256 out/musha-physical-usb.img
diskutil eject /dev/diskN
```

Check dd wrote every byte without errors. Whole-disk readback must have the same
hash as the exact-capacity image. Do not test a medium with mismatching capacity
or hash. Writing and readback can take time.

### 4. Test on NUC

Use a display, compatible cable and directly attached USB Boot Keyboard. Select
UEFI boot, disable Secure Boot for initial tests and use F10 on NUC5 to select the
medium. Record RUNTIME READY, RAM, USB/FAT32 reads and A/Shift+A press/release.
Photograph before/after Esc. NUC LAN is currently unsupported, not a communication
pass. Follow hardware-test-record-template.md and nuc5-bringup.md.

## 日本語

既にFAT32のUSBなら、EFIディレクトリとMUSHA.TXTをルートへ配置する方法で起動できます。
[test-usb-build.md](test-usb-build.md)を参照してください。以下は実容量と同じrawイメージを使う手順です。


この手順は専用ブートUSBの内容を消去する。NUC内蔵SSDへの書込みは行わない。
物理USBが届き、Mac上で対象を確認してから実施する。

## 1. 対象を識別する

USBを抜いた状態で `diskutil list external physical` を実行し、挿してもう一度実行する。
新しく現れたwhole diskを調べる。下記の `diskN` は必ず実際の番号へ置換する。
古い番号を再利用しない。必要のない外付け媒体は外しておく。

```sh
mkdir -p out
diskutil list external physical
diskutil info /dev/diskN
diskutil info -plist /dev/diskN > out/usb-device-info.plist
```

型番がSanDiskの対象媒体と一致すること、外付け・USB・whole diskであること、
正確な容量byte、logical sectorが512byteであることを確認する。
消してよい媒体であることと、必要なファイルのbackupを確認する。
番号・型番・容量が一致しない場合は以後を実行しない。

## 2. 実容量の通常ファイルimageを生成する

公開候補と同じEFIを使用する。故障注入やqemu-debugのEFIを使わない。
以下は `out/esp/EFI/BOOT/BOOTX64.EFI` の通常ビルドを使う例。
配布bundleのみを使う場合はbundleのrootで作業し、buildコマンドを省略し、
generatorに `--efi EFI/BOOT/BOOTX64.EFI` を渡す。必要なout directoryは
`mkdir -p out` で作成する。下記のhash対象もbundleのEFIパスへ置き換える。

```sh
sh tools/build-esp.sh
python3 - <<'PY'
import plistlib
from pathlib import Path
with open('out/usb-device-info.plist', 'rb') as source:
    info = plistlib.load(source)
capacity = info.get('TotalSize', info.get('Size'))
if info.get('Internal') is not False or info.get('Whole') is not True:
    raise SystemExit('Refusing: must be an external whole disk')
if info.get('BusProtocol') != 'USB' or info.get('DeviceBlockSize') != 512:
    raise SystemExit('Refusing: must be USB with 512-byte logical sectors')
if not isinstance(capacity, int) or capacity < 64 * 1024**2 or capacity % 512:
    raise SystemExit('Refusing: invalid capacity')
print('Device:', info.get('DeviceIdentifier'), 'Model:', info.get('MediaName'))
print('Exact capacity bytes:', capacity)
Path('out/usb-capacity-bytes.txt').write_text(str(capacity) + '\n')
PY
python3 tools/make-usb-image.py out/musha-physical-usb.img --size-bytes "$(cat out/usb-capacity-bytes.txt)"
shasum -a 256 out/esp/EFI/BOOT/BOOTX64.EFI out/musha-physical-usb.img
```

generatorは新しい通常ファイルのみ作成する。既存imageは上書きしない。
公称32GBではなく、diskutilが示す正確なbyte数を使う。十分なローカル空き容量を確保。
容量が変わればimage hashも変わるため、実機用imageのhashを別に記録する。
QEMUと同一imageでの合格判定には、この実容量imageもQEMUで試験する。

## 3. 確認後にUSBへ書き込む

以下は自動実行しない。直前に再度 `diskutil info /dev/diskN` で番号・型番・容量を
照合し、imageのbyte数が対象USBのTotalSizeと一致することを確認する。
`diskN`と`rdiskN`は同じ確認済みwhole disk番号を指定する。

```sh
diskutil info /dev/diskN
stat -f %z out/musha-physical-usb.img
diskutil unmountDisk /dev/diskN
sudo dd if=out/musha-physical-usb.img of=/dev/rdiskN bs=4m
sync
```

ddがerrorなく全byteを書き終えたことを確認する。imageはUSB全容量と同じなので、
下記の読戻しhashをimage hashと比較する。読戻しもwhole diskを使う。

```sh
sudo shasum -a 256 /dev/rdiskN
shasum -a 256 out/musha-physical-usb.img
diskutil eject /dev/diskN
```

容量またはhashが一致しない媒体で実機試験を進めない。書込み・読戻しには時間がかかる。

## 4. NUCで確認する

モニター、対応する映像ケーブル、USB Boot Keyboardを用意し、USB機器は直結する。
UEFI起動、初期試験のSecure Boot無効を確認。NUC5はF10で起動媒体を選ぶ。
RUNTIME READY、RAM、USB/FAT32読出し、A・Shift+Aの押下/解放を記録する。
Escの前後を撮影する。NUCのLANは現在未対応で、通信合格とは扱わない。
`hardware-test-record-template.md` と `nuc5-bringup.md` に従い結果を残す。
