# Musha-OS 0.1.0 アーキテクチャ設計

策定日: 2026-10-07。状態: 基本設計採用、詳細設計継続中。
基本方針は [policy-v0.1.md](policy-v0.1.md) を参照する。
独自コード・文書・設定のライセンスはApache-2.0とする。新規ソースには
実際の著作権者表示と `SPDX-License-Identifier: Apache-2.0` を付ける。
第三者コードには元のライセンス表示を保持する。
ファイル名のv0.1は旧文書名を保持したもので、初回リリース番号は0.1.0とする。

## 1. 実装単位

C17と必要最小限のx86-64アセンブリを使う。Clang / LLDを基本ツールチェーンとし、
ホストの標準ライブラリに依存しないfreestanding構成とする。
UEFIエントリだけはUEFIの呼出規約を使用し、内部はSysV AMD64 ABIとする。
ランタイムはred zoneを無効化する。FPU / SIMDの利用は状態管理を設計するまで禁止する。
UEFI関連の定義とビルド支援ライブラリは、ライセンス確認後に選定・版固定する。

0.1.0は一つのPE32+ EFI実行ファイルにランタイムと検証アプリを静的リンクする。
動的ELFロード、プロセス分離、複数アプリ切替は後続版へ送る。
以下をソース構成の予定とする。現時点で未実装である。

```text
boot/              UEFI入口、起動情報の構築
arch/x86_64/       GDT、IDT、ページテーブル、時間源
core/              メモリ、診断、協調実行
 drivers/          PCI、xHCI、Intel Ethernet
 usb/              列挙、HID、Mass Storage
 fs/               FAT32読出し
 net/              lwIP接続、アプリ向け通信
 include/musha/    内部契約とアプリAPI
 apps/             検証アプリ
 tests/            ホスト側試験、QEMU試験
 tools/            イメージ作成、試験補助
```

## 2. 起動状態遷移

`UEFI_ENTRY → PREPARE → EXIT_BOOT_SERVICES → CPU_INIT → MEMORY_INIT
→ DEVICE_INIT → APP_LOOP` とする。

PREPAREでGOPを選択し、ACPI RSDPを保存する。ランタイム用スタック、ページテーブル、
起動情報、メモリマップ保存バッファを確保する。画面情報は値として保存する。
最終GetMemoryMapとExitBootServicesの間でログ出力や新規確保を行わない。
失敗時は事前確保バッファでマップを再取得して再試行する。
不足バッファや再試行上限超過は起動失敗として処理し、通常起動を続けない。
終了後はUEFIプロトコルの関数ポインタを呼ばない。

自前スタックとGDT / IDTへ移行し、自前ページテーブルを有効化する。
その切替に必要なコード、スタック、起動情報を新旧双方のマッピングで保持する。
初期版は外部割込みをマスクし、デバイス完了をポーリングする。
例外は最小ハンドラで診断を表示して停止する。
UEFI Runtime Servicesは呼ばず、その領域は予約したまま保持する。

## 3. 起動情報とメモリ

内部boot_infoはmagic、構造体サイズ、版、UEFIマップの基点・長さ・descriptor size、
GOPの物理アドレス・サイズ・幅・高さ・stride・pixel format、RSDP物理アドレス、
予約範囲一覧を持つ。ポインタ値だけでなく各領域のサイズを必ず渡す。
UEFI descriptorは固定長配列と決めつけず、返されたstrideで走査する。

初期版は物理アドレスに対応する恒等マッピングを採用し、使用するRAMとMMIOだけを
マップする。コードは書込不可、データは実行不可を目標とし、NX対応を確認する。
MMIOは通常RAMと異なるキャッシュ属性にする。GOPもアドレス範囲を検証する。
ページ属性、PAT設定、既存MTRRとの整合は詳細設計の未完了項目である。

初期割当て元はEfiConventionalMemoryに限定し、明示予約範囲を差し引く。
Boot Services領域の回収は0.1.0では行わず、容量より安全な所有権を優先する。
カーネル用領域、DMA領域、arenaを分け、開始・終了の算術オーバーフローを検査する。
DMA用にはまず4GiB未満の連続領域を確保し、各機器の制約と必要アラインメントを適用する。
バッファを使用中の機器がある間は解放・再利用しない。

arenaはページ境界に揃った一つの連続領域を渡す。空き領域の最大候補から選び、
容量が必要量に足りなければ明示エラーとする。最低容量は検証アプリの要求から確定する。
CPUから使える仮想基点とバイト数を渡し、DMA APIとしては使用させない。
アプリ終了まで有効で、アプリは境界内で独自割当てを行える。
隔離はないため、不正書込からランタイムを保護する保証はしない。

## 4. 時間と実行モデル

ランタイムがループを所有し、各周回でUSB、NIC受信・送信完了、lwIPタイマー、
入力イベント、アプリstepを順に進める。各処理に作業量上限を設ける。
アプリstepはブロック禁止で、長い計算は複数stepに分割する。
初期予算は1step 1ms、通常ループ10ms以内を目標とし実測する。
予算超過は診断するが、強制プリエンプトは提供しない。

時間源はACPI PM timerを初期候補とし、HPETの利用可能性も検査する。
どちらも利用できない場合は未対応構成として診断する。
TSCをCPU周波数から推測して時間源にしない。カウンタのwrapを扱い、
単調増加するミリ秒値を提供する。レジスタアクセスとwrap試験は詳細設計で確定する。

## 5. アプリAPI契約

静的リンクされたapp_init(context)、app_step(context)、app_shutdown(context)を呼ぶ。
contextは版、サイズ、arena、画面情報、APIテーブルを持つ。
ヘッダー化前に構造体サイズ、呼出規約、整数幅を固定する。
APIは内部ランタイムとの契約で、0.1.0時点で将来版とのバイナリ互換を保証しない。

| API群 | 0.1.0の契約 |
|---|---|
| 画面 | 範囲確認付きピクセル・矩形描画。物理形式をAPI内で変換 |
| 入力 | 非ブロックのkey event取得。キーコードと押下・解放を返す |
| 時間 | 単調ミリ秒値を返す |
| ファイル | open / read / close。読出し専用、絶対パス、初期はASCII名 |
| UDP | bind / send / receive / close。受信はコピーして返す |
| 診断 | ログ、現在の機器状態、エラーコード |

失敗は固定整数のstatusで返す。OK、AGAIN、INVALID、UNSUPPORTED、NO_MEMORY、
IO、TIMEOUT、DISCONNECTED、NOT_FOUNDを基本候補とする。
呼出し側のバッファ所有権は移転しない。非同期処理はランタイム所有バッファへコピーする。
ファイルI/Oは状態機械として進め、完了前はAGAINを返す。
handleの世代番号で切断後やclose後の誤使用を検出する。
TCPのアプリAPIは0.1.0の必須機能には含めず、UDPまでで受入れを行う。

## 6. USBとファイル

xHCIはownership、停止、リセット、capability検査、必要scratchpad、DCBAA、
command ring、event ring、port reset、slot / endpoint contextの順に初期化する。
各待機に時間制限を設け、TRB cycle、リングwrap、DMA orderingを検証する。
register offsetやtimeout値、リング容量は詳細ドライバ仕様で定める。

初期機器は直結USB Boot Keyboardと直結USB Mass Storage BOT / SCSI transparentを
対象とする。ハブ、UAS、任意HID report descriptorは対象外。
キーボードはboot protocolを用い、修飾キーと押下・解放を差分で通知する。
BOTはCBW / data / CSWのtag、長さ、statusを検査し、stallとreset recoveryを設計する。
USB切断時は保留要求を失敗させ、古いhandleを無効化する。

イメージはGPT、単一FAT32 ESP、EFI/BOOT/BOOTX64.EFI、検証ファイルを持つ。
FAT32はMBR / GPTの基本パーティション検出とBPB検証を行い、初期は8.3名を対象とする。
セクタサイズと境界を検査し、壊れたcluster chain、循環、範囲外アクセスを拒否する。
ディスク書込APIは提供しない。同一イメージをQEMUのUSBディスクと実機USBで使用する。

## 7. Intel NICとlwIP

PCI IDによる明示allowlistを使用し、82574、I218、I219を別初期化経路にする。
BAR、bus mastering、reset、MAC取得、PHY/link、descriptor ring、送受信完了を
機種別に確認する。I218 / I219の処理を82574のレジスタ設定だけで代用しない。
未対応revisionは診断し、誤ったドライバを適用しない。

lwIPはNO_SYS=1、raw APIを内部で使用し、同一ループから入力と
sys_check_timeoutsを呼ぶ。socket / netconn APIは使用しない。
アプリにはlwIPのpbufを公開せず、受信コピーと上限付きキューを使う。
設定は静的IPv4、netmask、gatewayをビルド時に与え、DHCP / DNS / IPv6は後続対応とする。
輻輳時はドロップ数を記録し、無制限のメモリ確保を禁止する。

## 8. エラーと診断

起動・CPU・メモリの重大エラーはpanicとして画面へ段階、code、関連アドレスを表示して停止する。
個別デバイスの失敗は状態表示を残し、診断アプリを継続できるようにする。
ただし全必須機能が揃わなければ0.1.0合格とはしない。
固定容量RAMログとGOPコンソールを基本とし、QEMUのみ補助シリアルログを用いる。
実機でシリアルがないことを前提とする。書込専用USBログには依存しない。

## 9. 参照仕様

- [UEFI仕様](https://uefi.org/specs/UEFI/2.11/)
- [Intel xHCI仕様・初期化手順](https://cdrdv2-public.intel.com/625472/625472_xHCI_Rev1_2b.pdf)
- [lwIP NO_SYS実行モデル](https://www.nongnu.org/lwip/2_1_x/group__lwip__nosys.html)

参照仕様の版は実装開始時に固定し、最新UEFI版への準拠を基準機へ要求しない。
NIC、USB class、FATの一次資料と採用コードのライセンスは詳細設計時に追加する。
