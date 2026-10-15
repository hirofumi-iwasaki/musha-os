# Formatted-media quickcheck / フォーマット媒体1ページ手順

Status: current (procedure only, 2026-10-15). Six physical runs are pending.

## English

1. Owner selects an expendable external USB, identifies model/serial/capacity/logical-sector and backs up data. Record host OS/build and disk/partition identity; never choose a system disk. Download unchanged published **v0.3.0** files ZIP plus SHA256SUMS; verify ZIP and extracted EFI hashes. Use no updater, developer EFI or raw image.
2. Perform three separately recorded formatting routes: **A macOS Disk Utility MS-DOS(FAT)+MBR**, **B macOS Disk Utility MS-DOS(FAT)+GUID**, **C Windows Explorer FAT32**. In Disk Utility show all devices and select the external physical device. Verify actual **FAT32** after formatting; unavailable FAT32 means BLOCKED. For B record extra EFI partition and extract to the intended FAT32 data volume. For C record existing MBR/GPT first: Explorer formats the existing volume, not a newly chosen disk scheme. Record type/start/size/allocation/format options.
3. Extract the ZIP **contents** to volume root, retaining supplied files unchanged. Verify `EFI/BOOT/BOOTX64.EFI`, hash, root listing and absence of `AUTORUN.ELF`/legacy app ELF. Do not run deployment/update tools.
4. For each route boot **NUC5 once and Ryzen once** after ≥10 s fully off (record AC connected/disconnected): six runs. Keep keyboard and photographed port/hub topology fixed. Record firmware/UEFI/Secure Boot, boot selection and viewer arrival/failure; photograph full progression. PASS is viewer arrival; navigation response is supplementary. Do not substitute hot reboot or add a D8 requirement.
5. Retain one form per run. Only actual owner-reviewed results may enter the new formatted-media gate section of hardware status; leave historical evidence unchanged. This does not accept a v0.4 candidate or all firmware/capacities. USB preparation is owner execution only; this documentation task performs no write/format.

| Route | NUC5: cold seconds/AC, viewer/result, evidence | Ryzen: cold seconds/AC, viewer/result, evidence |
|---|---|---|
| A macOS FAT32/MBR | | |
| B macOS FAT32/GPT | | |
| C Explorer FAT32 (record existing scheme) | | |

Per run also record date/operator, host/formatter, disk/partition before/after, filesystem, ZIP/EFI hashes, CONFIG/root files, machine/firmware/topology and original photo/video paths. UNKNOWN is not PASS. [Full procedure and form](formatted-media-boot-verification.md).

## 日本語

1. 所有者が消去可能な外付けUSBを選び型番/serial/容量/論理sectorを識別、dataを退避。host OS/build、disk/partitionを記録しsystem diskを選びません。変更しない公開**v0.3.0** files ZIPとSHA256SUMSを取得しZIP/展開EFI hashを照合。updater/開発EFI/raw像は使いません。
2. **A macOS Disk Utility MS-DOS(FAT)+MBR、B同MS-DOS(FAT)+GUID、C Windows Explorer FAT32**を別々に実施/記録。Disk Utilityは全device表示から外付け物理deviceを選択し、結果が実際に**FAT32**か確認。選べなければBLOCKED。Bは付加EFI partitionも記録し対象FAT32 data volumeへ展開。Cは既存MBR/GPTを事前記録：Explorerは既存volumeのformatでdisk方式を新規選択する操作ではありません。type/start/size/allocation/optionを記録。
3. ZIPの**中身**をrootへ展開し付属fileを変更しません。`EFI/BOOT/BOOTX64.EFI`とhash、root一覧、`AUTORUN.ELF`/旧app ELF無しを確認。配備/update toolは実行しません。
4. 各経路で**NUC5 1回、Ryzen 1回**、off ≥10秒（AC接続/切断も記録）の冷間起動：計6回。同じkeyboard/撮影済port/hub接続でfirmware/UEFI/Secure Boot/boot選択/viewer到達又は失敗を撮影。PASSはviewer到達でnavigationは補足。温間再起動で代用せずD8を追加条件にしません。
5. run毎の様式と原写真/videoを保存し、実施後の所有者review済結果だけをhardware statusの新gate節へ追記。過去証拠は変更しません。v0.4候補/全firmware/全容量の受入ではありません。USB操作は所有者実施のみで、本書作成では書込み/formatしません。

| 経路 | NUC5：冷間秒数/AC、viewer/結果、証拠 | Ryzen：冷間秒数/AC、viewer/結果、証拠 |
|---|---|---|
| A macOS FAT32/MBR | | |
| B macOS FAT32/GPT | | |
| C Explorer FAT32（既存方式記録） | | |

各runに日時/実施者、host/formatter、disk/partition前後、filesystem、ZIP/EFI hash、CONFIG/root、機種/firmware/接続、原写真/video pathも記録。UNKNOWNをPASSとしません。[詳細手順/様式](formatted-media-boot-verification.md)。
