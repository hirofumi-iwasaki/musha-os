# USB controller diagnosis D2 / USBコントローラー診断 D2

## English

This document preserves the D2 diagnostic milestone. Current H2 behavior and validation supersede it: see [multi-controller and hub implementation](usb-multi-controller-hub-plan.md).

### Problem and evidence

On the user's Ryzen 7 system, photographs IMG_2817–IMG_2819 show successful runtime, 64MiB arena and PM timer initialization, five xHCI controllers, and session completion. The selected controller is `1912:0014`, packed BDF `0600`. Its two observed descriptors are `045B:0210` (SuperSpeed) and `045B:0209` (HighSpeed). Moving external devices between rear ports did not change this list. These observations do not establish keyboard input or FAT32 file-read success, nor do VID/PID values alone establish USB device class. CPU vendor is not sufficient to explain the failure.

### Implemented design

D2 is included in the normal build and keeps the current single-controller ownership and DMA lifecycle. PCI inventory reads configuration space without resetting additional controllers, changing their BARs or enabling their DMA. The first eight xHCI controllers are displayed in PCI scan order with VID/PID and packed BDF. `*ACTIVE` identifies the actual firmware-selected BAR/BDF, rather than the first controller encountered during the runtime PCI scan. The label denotes selection, not successful driver initialization. Other controllers are explicitly `NOT PROBED`. The total controller count remains available; debug builds also retain the complete PCI log.

For the selected controller, the first eight enumerated devices each occupy two rows: port/slot/VID/PID/speed, then device class/subclass/protocol and configuration-interface classification. A bounded descriptor walker checks lengths and configuration total size before classifying interfaces. Device class zero means classification resides in interfaces; it is not evidence of keyboard support. Hubs report `HUB UNSUPPORTED`. HID and mass-storage classification does not imply driver compatibility: only successful Boot Keyboard/BOT parser matches are labeled `BOOT KEYBOARD MATCH / CHECK INPUT` and `BOT STORAGE MATCH / CHECK READ`. Matched-device counts likewise do not prove input, sector-read or file-read success; those results are separate.

The panel uses 42 fixed 64-byte ASCII lines (384×420 pixels), with non-overlapping slots for eight two-line device records, eight controller records, match counts, storage/file results and network counters. It is shown fully at width ≥1024 and height ≥444, or below the main output at width ≥408 and height ≥980. Smaller modes retain the compact phase/error line. No heap allocation or new firmware calls are introduced after ExitBootServices.

### Hardware test and next decisions

Boot a newly built normal D2 EFI; the heading ends in `HARDWARE D2`. Photograph the whole panel. If `HUB UNSUPPORTED` appears for the observed devices, hub support becomes a concrete next requirement. If the desired keyboard/storage is not visible, the other controllers' ports remain unknown: this diagnostic does not enumerate all five controllers or prove which external socket belongs to each one. The next implementation must either select a different controller explicitly or introduce bounded multi-controller ownership, mappings, DMA allocation, independent failure isolation and shutdown. Changing the selected BDF alone without preserving validated BAR mappings is insufficient.

Do not infer a USB-port map from socket position. Hardware results remain pending until the updated EFI is booted. D2 does not implement hubs, new NIC support, or multiple simultaneously active xHCI controllers.

## 日本語

本書はD2時点の診断記録。現在のH2実装・検証は[複数コントローラーとハブ対応](usb-multi-controller-hub-plan.md)を参照し、本書の当時の制限より優先する。

### 問題と観測結果

Ryzen 7機の写真IMG_2817〜IMG_2819では、実行環境・64MiB arena・PMタイマーの初期化とセッション終了が確認できた。xHCIは5個検出され、選択先は`1912:0014`、packed BDF `0600`。認識されたDevice Descriptorは`045B:0210`（SuperSpeed）と`045B:0209`（HighSpeed）の2件で、背面ポートを変更しても同じだった。これだけではキーボード入力やFAT32ファイル読出しの成功、機器クラスは確定しない。CPUがAMD製という理由だけで原因を判断しない。

### 実装設計

D2は通常ビルドに含め、既存の1コントローラー制御とDMA停止手順を維持する。PCI設定空間の読取りだけで全xHCIを数え、先頭8個のVID/PIDとBDFを表示する。他のコントローラーのリセット・BAR変更・DMA有効化は行わない。`*ACTIVE`はUEFIで選択した実際のBAR/BDFとの一致を示し、初期化成功を意味しない。未選択のコントローラーは`NOT PROBED`と明示する。8個を超えた場合も総数は表示し、debug版には全PCI記録を残す。

選択先で列挙した先頭8機器は各2行とし、port・slot・VID/PID・速度に加えてDevice Class/Subclass/ProtocolとConfiguration内のInterfaceクラスを表示する。記述子の長さと総サイズを検証する有界走査で分類する。Device Classが0ならInterface側で分類し、キーボードとは断定しない。ハブは`HUB UNSUPPORTED`と表示する。HID／Storageクラスだけでは対応ドライバーの存在を意味しない。既存のBoot Keyboard／BOTパーサーで適合した機器を`MATCH / CHECK INPUT`または`MATCH / CHECK READ`と表示し、件数を集計する。適合件数と実際の入力・読出し成功は分けて読む。

パネルは固定長64-byte ASCIIの42行、384×420ピクセル。USB機器8件×2行、xHCI8件、適合件数、storage/file結果、network countersの表示領域を分離する。幅1024以上・高さ444以上なら右側、幅408以上・高さ980以上なら主表示の下側に全体を表示する。それ未満では簡易phase/error行を使う。ExitBootServices後のヒープ確保やUEFI呼出しは追加しない。

### 実機確認と次の判断

新しい通常版EFIで起動し、見出し末尾の`HARDWARE D2`と全パネルを撮影する。観測された機器が`HUB UNSUPPORTED`ならハブ対応が具体的な次の課題になる。目的の機器が現れない場合、他のコントローラー配下は未調査であり、D2だけでは全5個の配下や外部ポートとの対応を確定できない。次は選択先の明示変更、または複数コントローラーのBAR mapping・DMA確保・所有権・個別の失敗処理・停止を設計する。検証済みBAR mappingを保持せずBDFだけ差し替えてはいけない。

ポート位置から接続関係を推測しない。更新EFIの実機試験は未実施。D2ではハブ対応、新規LANドライバー、複数xHCI同時制御を追加しない。

## Validation / 検証

Validation results and artifact paths are recorded after testing below. / 検証後に結果と生成物を下記へ記録する。

### 2026-10-09

- Host tests: 67 passed, including classification of class-zero devices/hubs, malformed and mixed-interface configurations, and panel bounds. / ホスト試験67件成功。class-zero・ハブ、破損／複合Interface、パネル境界を含む。
- QEMU 11.1.2, q35/TCG, EDK2, 256MiB: two xHCI controllers, a direct Boot Keyboard, two storage devices and a root-port hub. Verified selected/unprobed BDF labels, hub rejection classification, keyboard press/release/Esc, GPT/FAT32 application file reads, and DMA shutdown. / xHCI2個、直結キーボード、storage2件、root-port hub構成で、選択先／未調査表示、ハブ未対応、入力・Esc、GPT/FAT32読出し、DMA停止を確認。
- Descriptor transfer-timeout regression passed; failure stage and shutdown remain observable. / 記述子transfer timeoutの失敗段階と停止表示を確認。
- Fresh normal package (`features: []`) booted from its read-only 64MiB GPT image with two controllers and one hub. Screenshot confirms D2, input, 16-byte MUSHA.TXT read and session completion. Image SHA-256 unchanged after QEMU. / featuresなし通常配布物を読出し専用64MiB GPTから起動。D2表示・入力・16-byte MUSHA.TXT読出し・終了を目視確認。イメージhash不変。
- Base commit: `df75d0f8b4d37dbbd9c0b33c4b7cf5e7e0bee028`; D2 is currently an uncommitted local change on `release/mi68-preparation`. No branch switch, reset, push or physical-USB write. / 同commitを基に同branch上の未コミット変更として実装。branch変更・reset・push・実物USB書込みなし。

Artifacts / 生成物:

- Normal EFI: `out/usb-diagnosis-d2-normal-20261009/files/EFI/BOOT/BOOTX64.EFI`
- FAT32 root file bundle: `out/usb-diagnosis-d2-normal-20261009/musha-os-fat32-files.zip`
- QEMU-only 64MiB image: `out/usb-diagnosis-d2-normal-20261009/musha-os.img` — do not raw-write this undersized image onto the 32GB USB. / 32GB USBへこの小容量イメージを全体書込みしない。
- Manifest and checksums: same output directory. EFI SHA-256: `38a586d5ee639fadc04cfa8d90a11cd027d4238dc1c9c0a622cd19ec4f06a6f3`.
- Evidence: `out/usb-diagnosis-d2-tests/multi-controller-hub.log`, `descriptor-timeout.log`, `normal-screen.ppm`.

Reproduce / 再検証:

```sh
cargo test --workspace --exclude musha-boot
cargo build --locked --release --target x86_64-unknown-uefi -p musha-boot --features qemu-debug
mkdir -p out/esp/EFI/BOOT
cp target/x86_64-unknown-uefi/release/musha-boot.efi out/esp/EFI/BOOT/BOOTX64.EFI
python3 tools/smoke-qemu.py --firmware-dir /opt/homebrew/share/qemu --fat-fixture gpt --keyboard-exit --extra-xhci --usb-hub
python3 tools/build-usb.py --output-dir out/usb-diagnosis-d2-new
```

Adjust toolchain/firmware paths for your environment; output directory must be new. The final command creates normal artifacts without fault/probe features. / 環境に応じてtoolchain・firmwareパスを変更する。出力先は新規にする。最後のコマンドは故障注入・probeなしの通常配布物を生成する。
