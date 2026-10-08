# USB Boot Keyboard診断

## 設定と入力

Device Descriptor取得後、configuration index 0の先頭9byteと全体を読む。
最大1024byte。descriptor length、total length、境界、HID descriptor、
対象interfaceのendpoint数、packetとintervalを検査する。
alt setting 0、class 3、subclass 1、protocol 1のBoot Keyboardを選択する。
一configuration中の入力候補は一つに限定し、複数候補は失敗とする。
非対象機器は列挙のみ行い、Set Configurationしない。

直結Low / Full / High-speedを対象とし、Interrupt IN端点をConfigure Endpointで
登録する。DCI=endpoint number*2+1、CErr=3、MaxBurst=0、DCS=1。
HS intervalはbInterval-1、FS / LSはfloor(log2(bInterval))+3。
packetはLS最大8、FS最大64、HS最大1024byte、高帯域追加transactionは非対応。
Set ConfigurationとSet Protocol(Boot)をEP0へ送信する。

Interrupt INのNormal TRBで8byte Boot Reportを読む。CPUの割込みは使わず、
event ringをpollする。pointer、slot、DCI、Success、residual 0を確認する。
short reportやstall等は診断失敗とする。rollover(usage 1〜3)は以前のキー状態を
保持し、誤った解放を生成しない。modifierと最大6個の通常usageについて押下・解放を
通知し、重複usageと並び替えで余分な通知を生成しない。reserved byteは無視する。

## 診断の寿命

KEYBOARD READY表示後、5秒または128reportまで読み取る。キー押下時のHID usageを
画面へ表示し、デバッグ出力には押下・解放を記録する。
usage表示行は高さ388pixel以上の画面で確認できる。
時間待ちは時計と最大5億poll回で制限する。
無入力でNAKが続くことは正常終了として扱う。

終了時にDisable Slotし、未完了のIN転送も破棄する。完了前のDMA領域を再利用しない。
controller停止・bus mastering解除後、RAM診断アプリへ進む。
これは起動時の入力診断であり、常時入力やアプリ向け入力queueはまだ提供しない。
文字配列、JIS / US配列変換、repeat、LED制御、ハブ、hotplug、
SuperSpeedキーボード、他configurationの探索、汎用Report Descriptor解析は未対応。

## 検証

QEMU usb-kbdのHigh-speed / Full-speedで、Shift+Aの押下・解放を
QMPで送信し、usage E1と04の両方向通知を検査した。
無入力でも診断を終え、DMA停止後にRAMアプリが完了することを確認した。
ホスト試験は不正configuration境界、packet、非対象interface、
rollover、duplicate key、endpoint IDとshort report拒否を確認する。

NUC5実機、Low-speed、CSZ=64、実物の複合キーボードは未検証。
試験方法は[開発手順](development.md)、実機準備は[NUC5試験](nuc5-bringup.md)を参照。

一次資料: [USB HID 1.11](https://www.usb.org/sites/default/files/hid1_11.pdf)
§7.2.6、Appendix B / F、
[Intel xHCI 1.2b](https://cdrdv2-public.intel.com/625472/625472_xHCI_Rev1_2b.pdf)
§4.3、4.6.5、4.11、6.2.3。
