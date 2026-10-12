# macOS USB installation and updates / macOSのUSB配備

## Current verified route / 現在の確認済み経路

既存のMusha-OS FAT32ボリュームへEFI/BOOT/BOOTX64.EFIを配置する経路を維持します。
H19の実USBでファイル更新・読戻しが確認されています。
この実績は今回追加した更新ツールやv0.2.0の実機合格を意味しません。
[機種別実績](hardware-verification-status.md)を参照してください。

## Next-version tooling / 次版ツール

[配布方針](release-distribution-policy.md)と[実行コマンド・復旧手順](distribution-usage.md)を参照。
新しい配布物はまだ公開していません。

通常更新はmanifest付きfiles.zipを使います。対象USBを接続して、
`diskutil list external physical`と`diskutil info`で型番・容量・接続方式を確認し、
対象のFAT32をマウントします。番号は毎回確認し、以前のdiskNを再利用しません。
update-usb.pyのdry-runでUUIDと変更ファイルを確認してから、UUIDを指定してapplyします。
今回の作業では実USBに対してこれらを実行していません。

更新は変更ファイルのみ、ホスト側バックアップ、一時配置、同期、照合、差し替え、
明示指定で安全な取り外しを行います。全容量raw書込み・ハッシュ走査は不要です。
通常更新失敗時の再フォーマットは行いません。
取り外し・再接続後のdry-runで全payloadがskippedになることを確認すれば、再マウント後にも照合できます。

初回・復旧用イメージは64 MiBの固定ESPとし、容量調整は通常ファイルでのみ検証します。
実USBに小さいイメージを単純コピーすることも、32GB全量イメージを通常更新に使うこともありません。
実デバイス用初回書込み処理は未提供です。必要なデータの救出と対象識別・消去承認を伴う別作業になります。

UEFI x64、GOP、対応xHCI、外付けキーボード、未署名EFIを許可するSecure Boot設定が必要。
v0.2.0は一覧・Enter・ページ切替・Escで戻る・Q終了を確認します。
Mac内蔵キーボード、NUC実機通信、exFAT実機の確認状態は各実機受入記録に従います。
