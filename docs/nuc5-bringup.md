# NUC5実機試験の準備

## 確保済み

利用者からNUC5機と32GB USBメモリの確保報告あり。
正確な型番、RAM容量、ファームウェア版、USBキーボード型番は未記録。
最古リファレンスはNUC5i5RYH / RYK (NUC5i5RYB基板)。
実機試験結果はまだない。

## 起動の用意

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
書き換える処理もない。UEFI起動用の統合USBイメージ生成は未実装。試験用FAT32 imageの生成ツールはある。
独自USB Mass Storageの容量・先頭 / 末尾セクタ読出しは実装済み。
MBR / superfloppy形式のFAT32で、ルートのMUSHA.TXT読出しを実装済み。
GPTとアプリ向けfile APIはまだ未対応。
同一USBイメージでQEMU / NUC5 / NUC8を起動する最終合格条件は未達。

ストレージ診断の成功時、高さ444pixel以上ならUSB READ OKと容量を表示する。
ポート順によってはEscでキーボード入力を終えてからストレージ診断が始まる。
この実物USBでの読出しはまだ検証していない。[対応範囲](usb-storage.md)を参照。

FAT32の確認には、対応形式のUSBルートへ4096byte以下のMUSHA.TXTを配置する。
内容は最初はHello Musha-OS!の一行にする。高さ476pixel以上ならFAT32 READ OKを表示する。
[対応形式と制約](fat32.md)を参照。既存USBのpartition方式を未確認のまま変更しない。

## 結果の記録

型番、RAM容量、ファームウェア版、Secure Boot設定、USB機器型番、
映像接続、EFIファイルSHA-256を記録する。
起動画面を保存し、入力usage、USB ENUMERATED / XHCI FAILED、
APP COMPLETEの状態を記載する。
失敗した場合は表示が止まった行と機器構成を残す。
実機特有のUSB routing / firmware ownership等はQEMU成功だけで対応済みとしない。
