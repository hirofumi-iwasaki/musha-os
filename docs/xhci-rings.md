# xHCI DMA・command / event ring

## 専用領域

UEFIのAllocateMaxAddressで4GiB未満に1MiBの連続LoaderDataを確保する。
BootInfoへ基点・長さを渡し、arenaと予約領域から分離する。
最終mapでLoaderData領域に収まること、ページ境界、RP / ROなし、
4GiB境界、他予約領域との非重複を検査する。
DMAのCPUマッピングはUC / RW / NX。scratchpadのnon-snoopアクセスも考慮し、
この段階ではWB最適化やcache flush APIを必要としない構成にする。
CR3切替前にMFENCE / WBINVDを実行し、旧ファームウェアのWB mappingに残った
dirty cache lineを排出する。PAT / MTRRは変更せず、旧translationも無効化する。

Poolはアドレス算術だけを扱うbump allocator。alignmentは2のべき乗を要求し、
加算overflow・容量不足では状態を変更せず失敗する。
領域は起動中に一度だけ使い、正常時も失敗時も解放・再利用しない。
デバイス書込領域へ通常のRust参照やsliceを作らず、raw pointerのvolatileアクセスを使う。
IOMMU設定とDMA mapping APIは未提供で、物理アドレスの直接DMAを前提とする。

## 固定レイアウト

| 用途 | 容量・alignment |
|---|---|
| DCBAA | 256 entries / 2048byte、64byte alignment |
| Command ring | 256 TRB / 4096byte、4096byte alignment、末尾はLink |
| Event ring | 256 TRB / 4096byte、一segment、4096byte alignment |
| ERST | 一entry、64byte確保・alignment |
| Scratchpad array | HCSPARAMS2に従う、64byte alignment |
| Scratchpad buffer | 各4096byte・alignment、最大128個 |

上限超過・不足時はDMAを開始せず失敗する。CONFIGはMaxSlotsEn=min(機器上限, 8)とする。
No-Op診断に続きUSB列挙でslotをEnableする。DCBAAのslot 0だけに必要scratchpad arrayを設定する。
AC64対応なら64bitアドレスregisterへaligned Qword write、非対応ならlow Dwordのみ。
すべてのDMAアドレスを4GiB未満に置くため、どちらでも上位アドレスは0となる。

## 初期化と所有権

停止・リセット・BME無効を確認後、DMA poolをzeroし全構造体を準備する。
構造体が揃った段階で、haltedのままBMEを有効にして読戻し確認する。
ERSTBA設定自体でtableをDMA読出しする実装があるため、BMEはその設定より先に必要。
その後DCBAAP、CRCR、CONFIG、primary interrupterのERSTSZ / ERDP / ERSTBAを設定する。
USBCMD.INTEとIMAN.IEは無効に保ち、R/Sを立ててHCHalted解除を最大100ms待つ。

Command ringは一要求ずつ発行する。payloadを先に書き、compiler release barrier、
cycle bitを含むcontrolのvolatile write、MFENCE、doorbell 0へのwriteの順に公開する。
末尾のLink TRBはTC=1とし、最後の通常TRBを公開する前に当該cycleで準備する。
完了を検査するまでcommand entryを再利用しない。

Event ringはcontrolのcycleがconsumer cycleに合うことを確認してから、LFENCEと
compiler acquire barrierの後にpayloadを読む。pointer、type、completion codeを検査する。
消費後はcursorを進め、ERDPを次entryへ更新してEHBをackする。IMAN.IPもackしIEは無効に保つ。
Command Completionはtype 33、Success、正確な提出TRB address、VFが0で、slotはcommandごとの期待値に一致することを要求する。
Enable Slotのみ、返されたslotが1からMaxSlotsEnの範囲内であることを要求する。
Port Status Changeはtype 34、port範囲とSuccessを検査して消費するが、接続処理は起動時の順次走査で行う。
未知イベント、host error、壊れた完了は診断失敗とする。

## 診断と終了

通常診断はNo-Opを600回発行する。Command ringの255 usable entriesと
Event ringの256 entriesをそれぞれ複数回周回し、cycle反転を実機相当のDMAで検査する。
各完了待ちは最大1000ms、かつ500万poll回。時計は毎周回でsampleする。
続いてUSB列挙を行い、成功時はUSB ENUMERATEDを表示する。

BME有効化以降のすべての復帰経路でR/Sを解除し、最大100msでHCHaltedを確認する。
停止待ちが失敗した場合もBMEを解除し、DMA poolは予約したまま保持する。
haltedとBME無効の両方を確認できた場合だけQUIESCEDを記録する。
診断アプリへ移る際はcontrollerを停止した状態にし、USB転送は行わない。

## 検証

- ホスト: DMA容量・alignment・overflow・失敗後のallocator状態、
  cursorの繰返しwrap、誤ったpointer・type・completion拒否、scratchpad bit配置。
- QEMU q35 / qemu-xhci: No-Op 600回、command / event wrap、停止とBME解除。
- `xhci-command-timeout`: doorbellを送らず20msで期限切れ。停止・BME解除と
  アプリの完了まで継続することをsmoke試験で確認する。

QEMUで上記を確認した。scratchpad必須controller、AC64=0、IOMMU有効環境、
BIOS ownership譲渡とNUC5 / NUC8の実機試験は未実施。
[USB列挙仕様](usb-enumeration.md)に続く処理・試験結果を記載する。

一次資料: [Intel xHCI 1.2b](https://cdrdv2-public.intel.com/625472/625472_xHCI_Rev1_2b.pdf)
§4.2、4.6.2、4.9、4.20、5.3.4、5.5.2、6.4.2、6.4.3、6.5、6.6、
[Intel SDM memory cache control](https://www.intel.com/content/www/us/en/developer/articles/technical/intel-sdm.html)。
