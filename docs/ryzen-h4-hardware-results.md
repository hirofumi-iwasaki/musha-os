# Ryzen hardware H4 results, 2026-10-10

## English

Evidence: IMG_7434–IMG_7444, all 11 pages from one successful run; IMG_7432/7433 and user observation confirm left Shift E1 and A 04; Esc 29 completes the application. Current USB EFI before H5 update: SHA-256 cfb9f1584929a739475f3ce949ef1456fde4199d6840c8bf4ad00a914e77e474.

Five controllers were scanned: C0 0600 USB2/SuperSpeed hubs; C1 0801 USB2 hub and SanDisk 0781:55A9, FAT32/MUSHA.TXT 16-byte read success; route 00001/00003 children demonstrate USB2 hub enumeration; C2 0803 composite device, no supported input/storage; C3 0D02 no supported devices; C4 0F03 wired 17EF:6009 Boot Keyboard input and Esc succeeded. All recorded controller shutdowns show DMA disabled. No BAD COMPLETION/BAD TRANSFER in this run. Prior intermittent failures remain unresolved; repeat boots on unchanged topology are needed. SuperSpeed hub child traffic remains unverified. Mac DMA coverage fix requires its own re-test and was not on this USB.

H5 display correction: archive after recording result; use final controller summary on saved pages; identify saved/current detail scope and cached file owner; clear per-controller media details at scan start. Preserve application file cache and validation.

## 日本語

根拠：IMG_7434～IMG_7444の同一成功実行11ページ。IMG_7432/7433と利用者観察で左Shift E1、A 04の受信、Esc 29による終了を確認。H5更新前USBのEFI SHA-256は英語節に記載。

5コントローラーを調査。C0/0600はUSB2／SSハブ。C1/0801はUSB2ハブとSanDisk 0781:55A9、FAT32／MUSHA.TXT 16byte読出し成功。route 00001／00003でUSB2ハブ配下列挙を確認。C2/0803は複合機器（対応入力／storageなし）、C3/0D02は対応機器なし。C4/0F03で有線17EF:6009の入力・Esc終了成功。記録された停止はDMA disabled。今回BAD COMPLETION／BAD TRANSFERは出ていないが、以前の断続的失敗は未解決。同じ構成で再起動を繰返して安定性を確認する。SSハブ配下の実転送は未検証。Mac DMA修正版はこのUSBにはまだなく別途再試験が必要。

H5表示修正：結果確定後に保存し、保存ページにも最終コントローラー一覧を表示。保存／現在情報を区別し、キャッシュしたファイル結果の所有コントローラーを明示。調査開始時に機器別media表示を消去する。アプリのfile cacheと検証条件は維持する。
