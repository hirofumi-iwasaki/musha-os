# Formatted-media boot verification / フォーマット媒体の起動確認

## English

Version boundary: this gate uses the unchanged published v0.3.0 EFI. The [new sequential boot log](boot-log.md) applies to newly built v0.4.0 only; do not require its timestamps/ready text for this six-run test or replace the published asset.

Status: procedure and blank record form, 2026-10-14. No hardware result is recorded here. Owner execution only; this planning task does not erase, format or update any USB.


### Purpose

Verify the proposed simple route using the **published v0.3.0** `musha-os-v0.3.0-x64-files.zip` unchanged: format FAT32, extract its contents to USB root, cold boot into the viewer without AUTORUN. Do not use update-usb.py, deployment tools, raw images, developer EFI or a rebuilt ZIP. Do not replace published assets/tags. This gate precedes README claims of formatter compatibility; v0.4.0 candidate acceptance is separate.


### Preparation

1. Owner selects an expendable external USB, backs up any required data, and identifies its model, serial (if available), exact capacity and logical sector size. Record host OS/build and target identity before erasing. Never select the internal/system disk.
2. Download the published ZIP and SHA256SUMS from [v0.3.0 Release](https://github.com/hirofumi-iwasaki/musha-os/releases/tag/v0.3.0). Verify the ZIP against the published checksum; record ZIP and extracted BOOTX64.EFI hashes. Hash verification is read-only, not an installation tool.
3. Use a USB/volume for which the GUI actually offers FAT32 (a modest 4–16 GB test USB is convenient); record the actual resulting FAT32 filesystem rather than assuming every “MS-DOS (FAT)” volume is FAT32. If FAT32 is unavailable, record the route as blocked; do not substitute exFAT or another formatting tool.
4. Use the same external USB keyboard and documented port/hub topology on NUC5 and Ryzen. Record firmware settings; select UEFI USB boot and Secure Boot configuration suitable for the unsigned published EFI.


### Three routes

| Route | Owner procedure | What must be recorded |
|---|---|---|
| A: macOS Disk Utility, MBR | View → Show All Devices; select the physical external USB at the top of its hierarchy. Erase with MS-DOS (FAT) and Master Boot Record. Verify the resulting volume is FAT32. | Physical disk and partition identity, MBR, partition type/start/size, FAT32 and erase-dialog/result evidence. |
| B: macOS Disk Utility, GPT | Repeat on the identified external USB, choose MS-DOS (FAT) and GUID Partition Map. Verify FAT32 and record the actual data partition and any formatter-created EFI partition. Extract to the FAT32 data volume, not automatically to an auxiliary EFI volume. | GPT, actual partition types/start/size, chosen FAT32 volume and any extra firmware partition. |
| C: Windows Explorer, FAT32 | Record the USB's **existing** partition style and volume identity first (Disk Management properties can inspect it). In Explorer, right-click that volume → Format → FAT32, record allocation unit/quick-format options, then format. Verify FAT32 and the partition style afterward. | Existing MBR or GPT, partition type/start/size and volume before/after, Windows version and format options. Explorer formatting does not initialize a disk or choose/create MBR/GPT. |


Apple documents physical-device selection and the scheme selector in [erase instructions](https://support.apple.com/en-gb/guide/disk-utility/dskutl14079/mac) and [partition schemes](https://support.apple.com/en-ie/guide/disk-utility/dsku1c614201/mac). Microsoft distinguishes formatting an existing partition in [Disk Management](https://support.microsoft.com/en-us/windows/experience/storage-filemanagement/disk-management-in-windows); [Format-Volume](https://learn.microsoft.com/en-us/powershell/module/storage/format-volume) also formats volumes on existing partitions. These describe host operations, not Musha-OS boot compatibility.

### Extraction and cold boot

1. Open the ZIP with the host's normal archive support and copy/extract its **contents** to the selected FAT32 volume root. Avoid an extra enclosing folder. Keep all supplied files unchanged, including DIAGNOSE.TXT, README.TXT, license texts and package manifest. Do not run the updater: its generated installation state is not required for this trial.
2. Confirm `EFI/BOOT/BOOTX64.EFI` exists at the correct relative path. Confirm no `AUTORUN.ELF` (and no legacy application ELF) is present. Record root listing and EFI hash. Formatter-generated hidden metadata may be recorded; do not claim the existing ZIP has the planned v0.4.0 clean root layout.
3. Safely eject. Fully power off NUC5 for at least 10 seconds, following the existing cold-boot definition; record the actual interval, whether AC power remained connected or was disconnected, and connection topology. Then select the UEFI USB entry and capture boot progression and viewer arrival. Navigation-key response is a supplementary observation, not a PASS requirement. Record firmware boot failure, no viewer arrival or unexpected application start as failures; record marker/media/keyboard observations separately.
4. Safely stop/power off, move the same route's media to Ryzen and perform one cold boot after at least 10 seconds powered off, with the same record fields including AC connection status. Do not substitute a hot reboot. Repeat steps for routes A, B and C: **six cold boots minimum, one per route per machine**.
5. Save original photos/videos and host evidence. Only after actual results and owner review add a new section titled `Formatted-media boot gate / フォーマット媒体の起動確認 (v0.3.0 extraction route)` to hardware-verification-status.md, with a bounded statement identifying route, filesystem, partition style, machines, exact ZIP/EFI and evidence. No preliminary “not run” result row is inserted there by this task. Failure does not authorize repair of published assets or a change to the test route.


### Blank record form

Copy one form per route/machine; fill observations after execution. UNKNOWN is explicit missing evidence, not a pass.

| Field | Value |
|---|---|
| Date, owner/operator | |
| Route A/B/C; formatter and host OS/build | |
| USB model/serial, bytes, logical sector | |
| Disk/partition/volume identity before erase | |
| Partition style before/after, type/start/size, extra partitions | |
| Verified filesystem, allocation unit, format options | |
| Published ZIP filename/SHA256/source | |
| Published checksum evidence; extracted EFI SHA256 | |
| Root listing; correct EFI path; AUTORUN absent | |
| NUC5 or Ryzen model, firmware, UEFI/Secure Boot settings | |
| USB port/hub, keyboard and other connected devices | |
| Cold power-off interval (at least 10 seconds), AC connected/disconnected, subsequent actions | |
| Firmware boot entry, marker/read result, viewer arrival | |
| Supplementary: navigation key and observed response (not required for PASS) | |
| Photos/videos/host evidence paths and timestamps | |
| Result PASS/FAIL/BLOCKED; limitations/UNKNOWN | |
| Owner review and proposed status-document statement | |

The owner-specified PASS condition is one cold boot (at least 10 seconds powered off) reaching the viewer on that machine for that route. Preserve checksum/extraction and FAT32/partition records to identify the tested route; navigation-key response is supplementary and is not an additional PASS condition. No D8 evidence is required for this gate. The combined gate requires all six forms; it does not prove all firmware, USB capacities or Windows partition styles are supported.

### Six-run execution sheet

Fill only after owner execution; blank cells are not results. Attach one detailed form above to each row. Store reviewed actual results in the new status-document section named above, preserving older sections.

| Route | Machine | Cold interval ≥10 s / AC status | Viewer arrival / result | Evidence / detailed record |
|---|---|---|---|---|
| A: macOS FAT32/MBR | NUC5 | | | |
| A: macOS FAT32/MBR | Ryzen | | | |
| B: macOS FAT32/GPT | NUC5 | | | |
| B: macOS FAT32/GPT | Ryzen | | | |
| C: Windows Explorer FAT32 | NUC5 | | | |
| C: Windows Explorer FAT32 | Ryzen | | | |

## 日本語

版の区分：本gateは変更しない公開v0.3.0 EFI。[新しい逐次bootlog](boot-log.md)は新v0.4.0 buildだけに適用し、この6runへtimestamp/ready新文言を要求したり公開物を置換しません。

状態：実施手順と空の記録様式、2026-10-14。実機結果は記録していません。実施者は所有者です。この計画作業ではUSBの消去・フォーマット・更新は行いません。

### 目的

**公開v0.3.0**の `musha-os-v0.3.0-x64-files.zip` を変更せず使い、「FAT32でフォーマット→中身をUSBルートへ展開→AUTORUN無しで冷間起動しビューアーへ到達」という簡易経路を検証します。update-usb.py、配備ツール、rawイメージ、開発EFI、再ビルドZIPは使いません。公開配布物・タグを置換しません。READMEにフォーマッター互換の導入経路を載せる前の確認であり、v0.4.0候補受入とは別です。

### 準備

1. 所有者が消去可能な外付けUSBを選び、必要なデータを退避し、型番・シリアル番号（取得できる場合）・正確な容量・論理セクターサイズを識別します。消去前にホストOS/ビルドと対象を記録し、内蔵・システムディスクを選ばないでください。
2. [v0.3.0 Release](https://github.com/hirofumi-iwasaki/musha-os/releases/tag/v0.3.0)から公開ZIPとSHA256SUMSを取得。公開チェックサムでZIPを照合し、ZIPと展開したBOOTX64.EFIのハッシュを記録します。ハッシュ照合は読取りだけの確認で、導入ツールではありません。
3. GUIで実際にFAT32を選べるUSB/ボリュームを使います（例えば4–16 GBの試験USB）。MS-DOS(FAT)と表示されるすべてのボリュームをFAT32と断定せず、結果のファイルシステムを記録します。FAT32を選べなければその経路をBLOCKEDとして記録し、exFATや別のフォーマットツールへ置き換えません。
4. NUC5・Ryzenで同じ外付けUSBキーボードを使い、ポート・ハブ構成を記録します。ファームウェア設定を記録し、未署名の公開EFIに対応するUEFI USB起動とSecure Boot設定を選びます。

### 3経路

| 経路 | 所有者の手順 | 記録する事項 |
|---|---|---|
| A：macOSディスクユーティリティ、MBR | 「表示→すべてのデバイスを表示」で階層最上位の物理外付けUSBを選択。MS-DOS(FAT)とMaster Boot Recordで消去し、結果のボリュームがFAT32と確認します。 | 物理ディスク・パーティションの識別、MBR、パーティション種別・開始位置・サイズ、FAT32、消去画面・結果の証拠。 |
| B：macOSディスクユーティリティ、GPT | 識別した外付けUSBで繰り返し、MS-DOS(FAT)とGUID Partition Mapを選択。FAT32と確認し、実際のデータパーティションとフォーマッターが作ったEFIパーティションも記録。補助EFIボリュームへ自動的に展開せず、FAT32データボリュームへ展開します。 | GPT、実際のパーティション種別・開始位置・サイズ、選んだFAT32ボリューム、追加のファームウェアパーティション。 |
| C：Windowsエクスプローラー、FAT32 | 先にUSBの**既存**パーティション形式とボリュームを記録（ディスクの管理のプロパティで確認可能）。エクスプローラーで対象ボリュームを右クリック→フォーマット→FAT32、割当て単位・クイックフォーマットの設定を記録して実行。結果のFAT32とパーティション形式を確認します。 | 既存MBR/GPT、前後のパーティション種別・開始位置・サイズとボリューム、Windows版、フォーマット設定。エクスプローラーのフォーマットはディスク初期化やMBR/GPTの選択・作成ではありません。 |

**Windowsのフォーマットはパーティション形式の作成・変換ではありません**。今回MBR/GPTを新設する追加操作を経路Cへ暗黙に含めません。

Appleの[消去手順](https://support.apple.com/en-gb/guide/disk-utility/dskutl14079/mac)と[パーティション方式](https://support.apple.com/en-ie/guide/disk-utility/dsku1c614201/mac)は物理デバイス選択と方式選択を説明します。Microsoftの[ディスクの管理](https://support.microsoft.com/en-us/windows/experience/storage-filemanagement/disk-management-in-windows)は既存パーティションのフォーマットを区別し、[Format-Volume](https://learn.microsoft.com/en-us/powershell/module/storage/format-volume)も既存パーティション上のボリュームをフォーマットします。これらはホスト操作の説明で、Musha-OSの起動互換性の証明ではありません。

### 展開と冷間起動

1. ホスト標準のアーカイブ機能でZIPを開き、**中身**を選択したFAT32ボリュームのルートへコピー・展開します。余分な親フォルダーを作らず、DIAGNOSE.TXT、README.TXT、ライセンス文書、パッケージmanifestを含め、配布ファイルをすべて変更せず保持。更新ツールは実行しません。この試験にツール生成の導入状態ファイルは不要です。
2. `EFI/BOOT/BOOTX64.EFI` が正しい相対位置にあり、`AUTORUN.ELF` と旧アプリELFが無いことを確認。ルート一覧とEFIハッシュを記録します。フォーマッター生成の隠しメタデータは記録可能ですが、既存ZIPを計画中のv0.4.0の整理済みルート構成と扱わないでください。
3. 安全に取り外します。既存の冷間起動定義に従いNUC5を**10秒以上完全に電源断**し、実際の電源断時間、AC電源を接続したままか取り外したか、接続構成を記録。その後UEFI USB起動項目を選び、起動経過とビューアー到達を撮影します。ナビゲーションキーの反応は補足観察でありPASS条件ではありません。ファームウェア起動失敗、ビューアー未到達、予期しないアプリ起動は失敗として記録し、マーカー・媒体・キーボードの観察は別に記録します。
4. 安全に終了・電源断し、同じ経路の媒体をRyzenへ移して**10秒以上電源断した後の冷間起動を1回**実施。AC接続状態も含め同じ項目を記録し、ホットリブートで代用しません。A・B・Cそれぞれで繰り返し、**各経路・各機種1回、合計6回の冷間起動**を行います。
5. 元の写真・動画・ホスト証拠を保存。実結果と所有者レビューの後だけ、経路・ファイルシステム・パーティション形式・機種・正確なZIP/EFI・証拠を特定した限定的記述をhardware-verification-status.mdの新節 `Formatted-media boot gate / フォーマット媒体の起動確認 (v0.3.0 extraction route)` へ追加します。今回その文書へ事前の「未実施」行は入れません。失敗は公開配布物の修復や試験経路の変更を許可するものではありません。

### 空の記録様式

経路・機種ごとに1様式を複製し、実施後に観察を記入します。UNKNOWNは証拠不足の明示で、合格ではありません。

| 項目 | 記入欄 |
|---|---|
| 日時・所有者/実施者 | |
| 経路A/B/C、フォーマッター、ホストOS/ビルド | |
| USB型番/シリアル、容量bytes、論理セクター | |
| 消去前のディスク/パーティション/ボリューム識別 | |
| 前後のパーティション形式、種別/開始位置/サイズ、追加パーティション | |
| 確認したファイルシステム、割当て単位、フォーマット設定 | |
| 公開ZIP名/SHA256/取得元 | |
| 公開チェックサム照合の証拠、展開EFI SHA256 | |
| ルート一覧、正しいEFI位置、AUTORUN無し | |
| NUC5/Ryzen型番、ファームウェア、UEFI/Secure Boot設定 | |
| USBポート/ハブ、キーボード、その他接続機器 | |
| 冷間電源断時間（10秒以上）、AC接続/取り外し、その後の操作 | |
| ファームウェア起動項目、マーカー/読出し結果、ビューアー到達 | |
| 補足：ナビゲーションキーと反応（PASSに必須ではない） | |
| 写真/動画/ホスト証拠の位置・時刻 | |
| PASS/FAIL/BLOCKED、制約/UNKNOWN | |
| 所有者レビュー、確認状況文書への記述案 | |

所有者指定のPASS条件は、**その経路・その機種で10秒以上の電源断後の冷間起動を1回行いビューアーへ到達すること**です。試した経路を識別できるよう、チェックサム・展開・FAT32/パーティション記録を保持します。キー反応は補足で、追加のPASS条件ではありません。この確認にD8証拠は要求しません。全体確認には6様式が必要で、全ファームウェア・USB容量・Windowsパーティション形式一般の互換性を証明するものではありません。

### 6回の実施一覧

所有者の実施後にだけ記入します。空欄は結果ではありません。各行に上の詳細様式を1つ添付し、レビューした実結果を指定した確認状況文書の新節へ保存。旧節は保持します。

| 経路 | 機種 | 電源断10秒以上/AC状態 | viewer到達/結果 | 証拠/詳細記録 |
|---|---|---|---|---|
| A：macOS FAT32/MBR | NUC5 | | | |
| A：macOS FAT32/MBR | Ryzen | | | |
| B：macOS FAT32/GPT | NUC5 | | | |
| B：macOS FAT32/GPT | Ryzen | | | |
| C：Windows Explorer FAT32 | NUC5 | | | |
| C：Windows Explorer FAT32 | Ryzen | | | |
