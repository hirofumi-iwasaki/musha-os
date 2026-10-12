# v0.2.0 — 読出し専用ファイルビューアー

2026-10-12。開発版として公開。所有者は実機検証の残件を残した範囲で公開を承認しました。

## 主な変更

- FAT32とexFATのUSBデータ媒体から、ルート直下のファイルを一覧・選択・読出し。
- 内容を256バイトずつ16進数とASCIIで表示。ページ移動、取消し、空ファイル、長い名前の確認に対応。
- 入力とストレージが別USBコントローラーでも読出しを継続。入力解放・切断・異常系の検証を拡充。
- 初回・復旧用の小型イメージと通常更新用ZIPを分離。通常更新は変更ファイルのみを退避・照合して置換。
- USBキーボード未検出時に、接続して再起動する案内を表示。
- OS起動後にチェック処理と組込みファイルビューアーが動く構成をREADMEに明記。

## ダウンロード

- `musha-os-v0.2.0-x64-usb.img.gz`: 64 MiBのGPT/FAT32起動イメージ。QEMUと初回・復旧用の原本。
- `musha-os-v0.2.0-x64-files.zip`: 通常更新用のEFI・管理対象ファイル・更新manifest。
- `musha-os-v0.2.0-x64-deployment.zip`: 配備ツールと説明資料。
- `musha-os-0.2.0.tar.gz`: 通常EFI、起動像、合成exFAT試験像、ライセンス、資料を含む一式。
- `build-manifest.json` / `SHA256SUMS`: ソースcommit、ビルド条件、配布物のハッシュ。

通常更新でUSB全容量のイメージ生成・書込み・ハッシュ走査は行いません。
小型イメージを大容量USBへそのままraw書込みしないでください。GPTの容量調整が必要です。
容量調整ツールは通常ファイル向けで、実USBへの初回書込みアダプターは未提供です。
[配備手順](distribution-usage.md)と[macOS手順](usb-install-macos.md)を参照してください。
Windows/Linuxの実デバイス更新アダプターは未実装です。

## 操作と制限

外付けUSB Boot Keyboardが必要です。上下で選択、Enterで開く、Page Up/Downで移動、Escで一覧、Qで終了します。
ルート最大128ファイル、1ファイル64 MiB、1ページ読出し10秒。詳細は[操作説明](file-viewer.md)。
書込み・編集・削除、サブディレクトリー、日本語字形、再接続による自動復帰は未対応です。
MacBook内蔵キーボード・Touch Bar・トラックパッド、内蔵SSD、exFAT直接起動は対象外です。

## 検証範囲

ホスト試験とQEMUでFAT32/exFAT閲覧、断片化、取消し・切断・異常系、既存USB・入力・通信を検証しました。
配布ツールは仮想ファイル／通常ディレクトリーで試験しています。新しい配備ツールによる実USB更新は未検証です。
MacBook Pro 2018ではrc1のビューアー起動とFAT32認識を写真で確認しました。USBキーボード未接続のため、
v0.2.0での実機ファイル選択・内容閲覧・exFAT読出しは未確認です。最終公開EFIの実機再検証も未実施です。
v0.1.0でのLenovo入力・USB読出し実績は、そのままv0.2.0の実機合格とはしません。
これらは公開後の受入試験として残します。内蔵キーボード対応済みとは扱いません。

## English

Development release adding a read-only FAT32/exFAT file browser, bounded paged reads,
improved input handling, and separate initial-image/file-update packages.
An external USB boot keyboard is required. Internal Mac keyboard support is not included.
Host and QEMU validation cover browsing and fault handling. MacBook Pro 2018 rc1 boot and FAT32
recognition were observed without a USB keyboard; hardware browsing/exFAT acceptance and final EFI
hardware retesting remain pending. Physical deployment with the new update tools is also unverified.
The owner approved publication with these limitations. See the linked guides before deploying.
