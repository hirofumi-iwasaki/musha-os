# 読出し専用FAT32の初期実装

## 対応形式

musha-fsはno_stdの独自実装。block readerのcallbackを受け取り、書込み処理を持たない。
USB BOTのREAD(10)へ接続し、ルートディレクトリの短名MUSHA.TXTを診断で読む。

- FAT32 superfloppy(ディスク先頭がvolume boot sector)。
- MBRの一次partition type 0Bh / 0Ch。対象FAT32 partitionは一つだけ。
- block sector 512 / 4096byteと一致するBPB、1 / 2 FAT。
- 8.3短名、断片化したroot directoryとfile chain、最後のsectorの部分読出し。

GPT、extended partition、FAT12 / FAT16 / exFAT、サブディレクトリ探索、
長名の解決、アプリ向けopen / read / close、書込みは未対応。
Long File Name entryは読み飛ばすため、その短名aliasを指定することはできる。

## 検査と境界

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

## 診断と実機への配置

対応するFAT32 USBのルートに4096byte以下のMUSHA.TXTを置く。
最初は内容を `Hello Musha-OS!` の一行にする。
高画面ではFAT32 READ OKを表示し、debug出力にbyte数とFNV-1a 64bit hashを記録する。
ファイルがなければFAT32_FILE_MISSINGとし、起動・入力・RAM診断は継続する。
GPTやFAT16等はFAT32_UNSUPPORTEDとなる。
これはfile読出しの診断であり、アプリ向けfile handleはまだ提供しない。

## 検証

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
