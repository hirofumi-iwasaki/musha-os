# Initial read-only FAT32 implementation

## English

### Supported formats

musha-fs is an original no_std implementation.
It accepts a block-reader callback and has no write operations.
It connects to USB BOT READ(10) and reads the short-name root file MUSHA.TXT for diagnostics.

- FAT32 superfloppy (the disk starts with a volume boot sector).
- MBR primary partition type 0Bh / 0Ch; exactly one target FAT32 partition.
- A BPB matching 512 / 4096-byte block sectors, with one / two FATs.
- 8.3 short names, fragmented root-directory and file chains, and partial reads of the last sector.

GPT, extended partitions, FAT12 / FAT16 / exFAT, subdirectory traversal,
long-name resolution, application open / read / close, and writes are unsupported.
Long File Name entries are skipped, so their short-name aliases can be specified.

### Validation and boundaries

Check the MBR signature, boot indicators, partition start / length, and disk boundaries.
Treat GPT protective MBRs as Unsupported.
Validate BPB sector size, power-of-two sectors per cluster, reserved sectors,
FAT count, total sectors, FAT length, root cluster, FAT32 version, and active FAT.
Determine FAT type from the data-cluster count; do not mount as FAT32 if it is below 65525.
Verify that the FAT can hold entries for all clusters and the data region fits within the partition.

Read FAT 0 when mirroring is enabled; otherwise read the active FAT specified by ExtFlags.
FAT-copy consistency checks and FSInfo use are not yet implemented.
Check LBA boundaries before every block read; mask the upper four bits of FAT entries.
Reject free / bad / reserved / out-of-range clusters, premature EOC, and cycles.
Also reject chains extending beyond the file length as Corrupt.
Error output buffers may contain partial writes; do not expose them until success.

Limit each operation to 1024 sector reads.
Track up to 128 clusters each for root and file chains, detecting cycles with visited clusters.
Every USB transfer also has a deadline.
The diagnostic file buffer is 4096 bytes. Report larger files as TooLarge without displaying contents or publishing hashes.
Exclude deleted entries, volume labels, directories, and LFNs when matching short names.
Continue diagnostics for NotFound files and Unsupported media.
Corrupt target FAT32 media, I/O errors, and exceeded limits lead to controller shutdown and DMA disablement.

### Diagnostics and hardware deployment

Place MUSHA.TXT of at most 4096 bytes in the root of a supported FAT32 USB drive.
Initially use a single line containing `Hello Musha-OS!`.
On sufficiently tall screens, display FAT32 READ OK;
record byte count and FNV-1a 64-bit hash in debug output.
If the file is absent, report FAT32_FILE_MISSING and continue boot, input, and RAM diagnostics.
GPT, FAT16, and similar media produce FAT32_UNSUPPORTED.
This is a file-read diagnostic; application file handles are not provided yet.

### Validation

Host tests covered MBR / superfloppy, fragmented root and file chains,
512 / 4096-byte sectors, active FAT, malformed BPBs / partitions, cycles,
short chains, insufficient buffers, missing files, and I/O errors.
In QEMU, attach an approximately 34MiB FAT32 fixture as USB media and read fragmented,
1186-byte MUSHA.TXT, verifying its hash against the host payload.
Both MBR and superfloppy succeeded. Attach fixtures read-only;
also check that the image SHA-256 remains unchanged after exit.
A cyclic file-chain fixture verified diagnostic failure, controller shutdown,
DMA disablement, and RAM completion.

The [fixture generator](../tools/make-fat32-fixture.py) creates only new regular files.
It does not overwrite existing paths or format physical disks.
Test fixtures contain no UEFI boot code or EFI files and are not hardware boot images.
The NUC5 32GB USB drive and physical FAT32 media with 4096-byte sectors remain untested; GPT is unsupported / untested.

Primary source:
[Microsoft FAT specification v1.03](https://www.pcjs.org/documents/papers/microsoft/MS_FAT_OVERVIEW_103-2000-12-06.pdf)
Boot Sector and BPB, FAT Type Determination, FAT Directory Structure.
Do not copy the specification text or existing driver code into the repository.

---

## 日本語

**読出し専用FAT32の初期実装**

### 対応形式

musha-fsはno_stdの独自実装。block readerのcallbackを受け取り、書込み処理を持たない。
USB BOTのREAD(10)へ接続し、ルートディレクトリの短名MUSHA.TXTを診断で読む。

- FAT32 superfloppy(ディスク先頭がvolume boot sector)。
- MBRの一次partition type 0Bh / 0Ch。対象FAT32 partitionは一つだけ。
- block sector 512 / 4096byteと一致するBPB、1 / 2 FAT。
- 8.3短名、断片化したroot directoryとfile chain、最後のsectorの部分読出し。

GPT、extended partition、FAT12 / FAT16 / exFAT、サブディレクトリ探索、
長名の解決、アプリ向けopen / read / close、書込みは未対応。
Long File Name entryは読み飛ばすため、その短名aliasを指定することはできる。

### 検査と境界

MBRのsignature、boot indicator、partition開始・長さ・ディスク境界を確認する。
GPT protective MBRはUnsupportedとして扱う。
BPBのsector長、power-of-twoのsectors per cluster、reserved sector、FAT数、
total sector、FAT長、root cluster、FAT32 version、active FATを検査する。
FAT種別はdata cluster数から判定し、65525未満はFAT32としてmountしない。
FATが全clusterのentryを収容することと、data領域がpartitionに収まることを確認する。

mirror有効時はFAT 0、無効時はExtFlags指定のactive FATを読む。
FAT copy間の一致検査やFSInfo利用はまだ行わない。
各block読出し前にLBA境界を検査し、FAT entryは上位4bitをmaskする。
free / bad / reserved / 範囲外cluster、途中のEOC、循環を拒否する。
file長を超えて続くchainもCorruptとして拒否する。
error時の出力bufferは部分書込みを含み得るため、成功するまで公開しない。

一処理の上限は1024sector read。rootとfileの各chainは最大128clusterを追跡し、
visited clusterで循環を検出する。USBの各転送にも期限がある。
診断のfile bufferは4096byte。大きなfileはTooLargeとし、表示・hash公開をしない。
deleted entry、volume label、directory、LFNを除外して短名を一致比較する。
見つからないfileはNotFound、非対象媒体はUnsupportedとして診断を継続する。
対象FAT32の破損、I/O error、上限超過はcontroller停止・DMA無効化へ進む。

### 診断と実機への配置

対応するFAT32 USBのルートに4096byte以下のMUSHA.TXTを置く。
最初は内容を `Hello Musha-OS!` の一行にする。
高画面ではFAT32 READ OKを表示し、debug出力にbyte数とFNV-1a 64bit hashを記録する。
ファイルがなければFAT32_FILE_MISSINGとし、起動・入力・RAM診断は継続する。
GPTやFAT16等はFAT32_UNSUPPORTEDとなる。
これはfile読出しの診断であり、アプリ向けfile handleはまだ提供しない。

### 検証

ホストでは、MBR / superfloppy、rootとfileの断片化、512 / 4096byte sector、
active FAT、不正BPB / partition、循環、短いchain、buffer不足、file欠落、I/O errorを確認した。
QEMUでは約34MiBのFAT32 fixtureをUSBとして接続し、断片化した1186byteの
MUSHA.TXTを読み、ホスト側payloadのhashと一致することを確認した。
MBRとsuperfloppyの両形式で成功。fixtureはreadonly接続し、
終了後のimage SHA-256も不変であることを検査する。
循環file chainのfixtureでは診断失敗、controller停止、DMA無効化とRAM完了を確認した。

[fixture作成ツール](../tools/make-fat32-fixture.py)は新しい通常ファイルだけを作成する。
既存pathの上書きや実機ディスクのformatは行わない。
試験用fixtureはUEFI起動コードやEFIファイルを持たず、実機起動用imageではない。
NUC5の32GB USB、4096byte sectorの実物FAT32、GPTは未検証 / 未対応。

一次資料:
[Microsoft FAT仕様 v1.03](https://www.pcjs.org/documents/papers/microsoft/MS_FAT_OVERVIEW_103-2000-12-06.pdf)
Boot Sector and BPB、FAT Type Determination、FAT Directory Structure。
仕様本文・既存ドライバのコードはrepositoryに転載しない。
