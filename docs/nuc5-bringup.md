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
5. USBからUEFI起動し、KEYBOARD READY表示中の5秒間にAやShiftを押して離す。
   その後USB ENUMERATED、APP COMPLETEを確認する。
   高さ388pixel以上ではKEY CODEが表示される。

内部SSDへの書込み処理は未実装。USB32GBの容量全体をRAMやストレージ試験で
書き換える処理もない。現時点ではFAT32ディスクイメージ生成と、
独自USB Mass Storage / FAT32読出しは未実装である。
同一USBイメージでQEMU / NUC5 / NUC8を起動する最終合格条件は未達。

## 結果の記録

型番、RAM容量、ファームウェア版、Secure Boot設定、USB機器型番、
映像接続、EFIファイルSHA-256を記録する。
起動画面を保存し、入力usage、USB ENUMERATED / XHCI FAILED、
APP COMPLETEの状態を記載する。
失敗した場合は表示が止まった行と機器構成を残す。
実機特有のUSB routing / firmware ownership等はQEMU成功だけで対応済みとしない。
