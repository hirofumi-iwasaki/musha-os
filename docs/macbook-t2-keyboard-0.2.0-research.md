# MacBook Pro 2018 internal keyboard: v0.2.0 investigation

調査日: 2026-10-10。対象ブランチ: `release/0.2.0`（v0.1.0のmainから開始）。
状態: ソース調査と実装方針。T2ドライバーはまだ実装・実機検証していない。

## 結論

FreeBSDのApple T2 BCE / VHCI実装を主な参考にできる。現行mainには
`sys/dev/apple_bce/`があり、各対象ファイルのSPDXはBSD-2-Clause。
同じ版の`apple_bce(4)`はMacBook Pro 2018–2020を対象に挙げ、初出をFreeBSD 16.0とする。
これは現在のmainにある実装の確認であり、既存のFreeBSD安定版すべてに含まれるという意味ではない。

[AppleのT2搭載機種一覧](https://support.apple.com/en-us/103265)とFreeBSDの実装から、
今回の2018年モデルでは `PCI Apple BCE → T2通信キュー → VHCI → USB HID → キーイベント`
を実装する方針が妥当。既存のIntel xHCIへ内蔵キーボードを追加登録するだけでは足りない。
実機の正確なモデル識別子、BCEのPCI位置、内蔵HID descriptorは次の診断で確認する。

## 固定した一次資料

FreeBSD調査版: `7a48c3fd3e6ea097a8fa8f3ce3e25fe7c2d37d9e`。

| 資料 | 確認内容 |
|---|---|
| [apple_bce.c / .h](https://github.com/freebsd/freebsd-src/tree/7a48c3fd3e6ea097a8fa8f3ce3e25fe7c2d37d9e/sys/dev/apple_bce) | PCI、BAR、mailbox、DMA queue、周期時刻通知、停止処理 |
| [apple_bce_vhci.c](https://github.com/freebsd/freebsd-src/blob/7a48c3fd3e6ea097a8fa8f3ce3e25fe7c2d37d9e/sys/dev/apple_bce/apple_bce_vhci.c) | 仮想USB bus、port/device管理、control/interrupt/bulk転送 |
| [apple_bce(4)](https://github.com/freebsd/freebsd-src/blob/7a48c3fd3e6ea097a8fa8f3ce3e25fe7c2d37d9e/share/man/man4/apple_bce.4) | 対象機種とドライバーの役割 |
| [VHCI導入commit](https://github.com/freebsd/freebsd-src/commit/9f90536c74b8172fc67cd977e5451f37a12462d5) | 2026-06の導入。記載された実機試験はMacBookPro16,2とMacmini8,1。今回の2018実機での成功を保証する資料ではない |
| [FreeBSD ukbd.c](https://github.com/freebsd/freebsd-src/blob/7a48c3fd3e6ea097a8fa8f3ce3e25fe7c2d37d9e/sys/dev/usb/input/ukbd.c) | USB keyboard入力処理の補助資料。取り込む場合はこのファイル自身の条件を確認 |
| [t2linux apple-bce-drv](https://github.com/t2linux/apple-bce-drv/tree/c7448ca0a58299a78836969ddd3b43761ffac6b1) | Linux側のBCE/VHCI比較資料。apple_bce.cはMODULE_LICENSE("GPL")を宣言 |
| [t2bce](https://github.com/deqrocks/t2bce/tree/a973d53c8278e9db5ff8314b816d6880309ed39e) | Linux側の後継構成。core / DMA / VHCIを分離している |

今回は第三者ドライバーコードをリポジトリへ取り込んでいない。
FreeBSD実装を翻案する段階で、原本、固定commit、ファイルhash、著作権表示、
BSD-2-Clauseの条件を保存し、NOTICEと第三者依存一覧へ追加する。
Linux実装の閲覧を、そのコードの無条件な転用許可と扱わない。

## ソースから確認した実装条件

- BCEのPCI IDは`106b:1801`。DMA用はBAR2、mailbox用はBAR4。
  xHCIのBAR0前提やH19のIntel専用復旧条件を流用しない。
- Firmware protocol値は`0x20001`。mailboxは返信の種類と値、timeoutを検証する。
- FreeBSDはMSIを8本確保し、mailboxにvector 0、DMAにvector 4を使う。
  Mushaの既存xHCIはCPU割込みを使わずpollするため、同等の通知基盤が未整備。
- FreeBSDのmailboxにはearly boot用の有界pollingがある。ただし、これだけでは
  **BCE/VHCI全体が割込みなしで動くことは証明できない**。
  completionの回収・ack・再通知条件を確認し、polling試作かMSI実装かを決定する。
- Firmwareへ150ms間隔で時刻を通知する実装がある。初期化中も通常入力中も
  長い同期waitでこの処理を止めない設計が必要。
- VHCIは専用message/event/data queueを使う。xHCI TRB ringとは別物。
  FreeBSDのbus_dma、taskqueue、callout、mutex、USB stackへの依存を
  Rustの有界状態機械・専用DMA領域へ置き換える必要がある。
- VHCIソースにはinterrupt/bulk未実装という古いコメントが残るが、直後に
  それらの転送処理が存在する。コメントだけで対応可否を判定しない。

## v0.1.0との接続点

実機H19で確認済みなのは外付けLenovo `17EF:6009`のA / Shift+A / Escと、
SanDisk `0781:55A9`のFAT32ファイル読出し。内蔵入力の実証にはならない。
C1 `0700`の非boot-ownerに対する復旧skipは、T2対応のために解除しない。

| 現在の構成 | 再利用／変更方針 |
|---|---|
| `boot/src/pci.rs` | 読取専用のBCE inventoryを追加。xHCI controller一覧とは別に保持 |
| `xhci/src/keyboard.rs` | 既存はboot keyboard interfaceと8-byte report向け。輸送に依存しないキー状態処理を共通化する候補 |
| `boot/src/xhci/usb.rs` | USB要求・descriptor検査の考え方を利用。転送backendをVHCIへ直結コピーしない |
| `boot/src/app.rs` | 内蔵／外付け入力の選択とfallbackを統合。現在の単一keyboard session前提を明示的に扱う |
| timer / DMA shutdown | 既存の期限管理と終了時の停止方針を利用。BCE queue固有の所有権・停止確認を追加 |

内蔵HIDがboot protocolで動くかは未確認。device/configuration/HID report descriptorを
採取してから判断する。必要ならreport ID、modifier、key array / bitmapを
有界に解釈する限定HID parserを追加する。最初から全HID仕様を実装する必要はない。

Touch BarのEscは内蔵の文字キーと同じ経路・初期化だけで利用できると仮定しない。
初期段階の合格条件をA / Shift+A / key releaseとし、終了操作は外付けEscまたは
診断timeoutを確保する。Touch Bar、Fn、バックライト、trackpad、audio、
suspend/resumeは別の追加範囲として評価する。

## 推奨する実装順序と合格条件

1. **読取専用の実機診断**: SMBIOSモデル、全PCIのvendor/device/class、BCEのBDF、
   BAR2/4、Command、PM、MSI capability、UEFI終了前後の状態を記録。
   BAR sizing書込みやDMA有効化はしない。BAR範囲・上流bridge・予約領域を検証する。
2. **BCE通信の最小試作**: 検証済みMMIOに限定してprotocol handshakeと時刻通知。
   timeout・予期しない返信を処理し、失敗しても外付け入力とUSB読出しを継続できること。
3. **DMAと通知**: 専用arena、alignment、queue index、長さ、所有権、barrierを実装。
   通知方式を確定し、queue登録・完了・取消・停止を確認する。
   停止確認前にDMAメモリを再利用しない。
4. **VHCI最小host**: port discovery、device生成、endpoint 0、descriptor取得、
   configuration設定、keyboard interrupt INに絞る。audio/bulkの汎用対応は後回し。
5. **内蔵キー入力**: descriptorに基づくreport解釈と共通入力イベント化。
   A、Shift+A、押下／解放、連続入力、無入力、終了を実機で検証。
   外付けとの同時接続では入力元の選択と重複・stuck modifierを確認する。
6. **回帰と公開判定**: 既存host tests、QEMU USB/HID/BOT、H19のMac実機USB読出しと
   Lenovo入力を維持。異常descriptor、queue wrap、遅延completion、timeout、
   未完了転送中の終了を検証する。通常QEMUのxHCI成功だけでT2成功と扱わない。

次の実装単位は1の診断と、MMIOを含まないBCE protocol/queueモデルのhost tests。
この調査だけでv0.2.0完成や内蔵キーボード対応済みとはしない。

## 実装進捗

読取専用の情報採取を[T2-D1診断](t2-d1-diagnostics.md)として追加した。
次はMac実機の結果を確認し、BAR範囲検証とBCE通信の最小試作へ進む。
