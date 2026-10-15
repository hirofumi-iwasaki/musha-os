# v0.2.0 — Read-only FAT32/exFAT file viewer

Status: record (2026-10-12)

## English

Released on 2026-10-12 as a development release. The owner approved publication with the hardware validation items below still pending.

### Main changes

- List, select, and read root files on FAT32/exFAT USB data media.
- Display hexadecimal and ASCII in 256-byte pages, with navigation, cancellation, empty-file handling, and full-name inspection.
- Support input and storage on separate USB controllers; expand key-release, disconnect, and fault tests.
- Separate initial/recovery images from update ZIPs. Normal updates back up, verify, and replace only changed managed files.
- Show a connect-and-restart message when no USB keyboard is detected.
- Explain automatic startup checks and the bundled file viewer in README.

### Downloads

- `musha-os-v0.2.0-x64-usb.img.gz`: 64 MiB GPT/FAT32 boot image for QEMU and as the initial/recovery source image.
- `musha-os-v0.2.0-x64-files.zip`: normal-update EFI, managed files, and update manifest.
- `musha-os-v0.2.0-x64-deployment.zip`: deployment tools and instructions.
- `musha-os-0.2.0.tar.gz`: complete bundle with normal EFI, boot image, synthetic exFAT test image, licenses, and documentation.
- `build-manifest.json` / `SHA256SUMS`: source commit, build conditions, and artifact hashes.

Normal updates do not generate, write, or hash an entire USB device.
Do not raw-copy the small image onto a larger USB: GPT capacity adjustment is required.
The adjustment tool supports regular files; an initial physical-device writer is not provided.
See the [deployment guide](distribution-usage.md) and [macOS guide](usb-install-macos.md).
Windows/Linux physical-device update adapters are not implemented.

### Controls and limits

An external USB Boot Keyboard is required. Up/Down selects, Enter opens, Page Up/Down navigates, Esc returns to the list, and Q ends the session.
Limits: 128 root files, 64 MiB per file, and 10 seconds per page read. See the [viewer guide](file-viewer.md).
Writing, editing, deletion, subdirectories, Japanese glyph rendering, and automatic reconnection are unsupported.
The internal MacBook keyboard, Touch Bar, trackpad, internal SSD, and direct exFAT boot are outside this release's scope.

### Validation scope

Host and QEMU tests cover FAT32/exFAT browsing, fragmentation, cancellation, disconnects, fault handling, and existing USB/input/network functionality.
Deployment tools were tested with virtual images and regular directories. Physical USB updates using the new tools remain unverified.
MacBook Pro 2018 rc1 viewer boot and FAT32 recognition were confirmed from a photo without a USB keyboard attached.
v0.2.0 hardware file selection, viewing, and exFAT reads remain unverified. The final EFI has not been retested on hardware.
v0.1.0 Lenovo input and USB-read results do not establish v0.2.0 hardware acceptance.
These remain post-release acceptance items. Internal keyboard support is not claimed.

### Final artifact verification

Source: `b17d52d5424822377c2bb554cefaaed44a2d4708` (v0.2.0 tag, merged into main).
EFI SHA-256: `b7478afc1c450d98f35b87c581334402db648add7302477f8345a9567975080a`.
The exact release EFI passed local QEMU FAT32/exFAT list-open-back tests and the no-keyboard screen check.
[PR CI passed](https://github.com/hirofumi-iwasaki/musha-os/actions/runs/38224108877). All six uploaded assets were downloaded and hash-verified before publication.

### Documentation correction after publication

On 2026-10-12, README on main and this Release description were reorganized into English-first, Japanese-second sections.
The tag, binaries, archives, manifests, and checksums are unchanged. Documents in the published archives and tagged source retain their original wording.

---

## 日本語

2026-10-12。開発版として公開。所有者は実機検証の残件を残した範囲で公開を承認しました。

### 主な変更

- FAT32とexFATのUSBデータ媒体から、ルート直下のファイルを一覧・選択・読出し。
- 内容を256バイトずつ16進数とASCIIで表示。ページ移動、取消し、空ファイル、長い名前の確認に対応。
- 入力とストレージが別USBコントローラーでも読出しを継続。入力解放・切断・異常系の検証を拡充。
- 初回・復旧用の小型イメージと通常更新用ZIPを分離。通常更新は変更ファイルのみを退避・照合して置換。
- USBキーボード未検出時に、接続して再起動する案内を表示。
- OS起動後にチェック処理と組込みファイルビューアーが動く構成をREADMEに明記。

### ダウンロード

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

### 操作と制限

外付けUSB Boot Keyboardが必要です。上下で選択、Enterで開く、Page Up/Downで移動、Escで一覧、Qで終了します。
ルート最大128ファイル、1ファイル64 MiB、1ページ読出し10秒。詳細は[操作説明](file-viewer.md)。
書込み・編集・削除、サブディレクトリー、日本語字形、再接続による自動復帰は未対応です。
MacBook内蔵キーボード・Touch Bar・トラックパッド、内蔵SSD、exFAT直接起動は対象外です。

### 検証範囲

ホスト試験とQEMUでFAT32/exFAT閲覧、断片化、取消し・切断・異常系、既存USB・入力・通信を検証しました。
配布ツールは仮想ファイル／通常ディレクトリーで試験しています。新しい配備ツールによる実USB更新は未検証です。
MacBook Pro 2018ではrc1のビューアー起動とFAT32認識を写真で確認しました。USBキーボード未接続のため、
v0.2.0での実機ファイル選択・内容閲覧・exFAT読出しは未確認です。最終公開EFIの実機再検証も未実施です。
v0.1.0でのLenovo入力・USB読出し実績は、そのままv0.2.0の実機合格とはしません。
これらは公開後の受入試験として残します。内蔵キーボード対応済みとは扱いません。


### 最終配布物の確認

ソース: `b17d52d5424822377c2bb554cefaaed44a2d4708`（v0.2.0タグ、mainへマージ済み）。
EFI SHA-256: `b7478afc1c450d98f35b87c581334402db648add7302477f8345a9567975080a`。
公開したEFIそのもので、QEMUのFAT32/exFAT一覧→開く→一覧復帰と、キーボード未接続画面を確認しました。
[PRのCIは成功](https://github.com/hirofumi-iwasaki/musha-os/actions/runs/38224108877)しています。全6配布物をアップロード後に読み戻し、ハッシュを照合してから公開しました。

### 公開後の文書修正

2026-10-12に、mainのREADMEとこのRelease本文を英語→日本語の順に整理しました。
タグ、バイナリー、アーカイブ、manifest、チェックサムは変更していません。公開済みアーカイブ内とタグ上の文書は公開時の表記を保持しています。
