# 実機確認状況（配布用PDFの参照資料）

更新日: 2026-10-10（日本時間）。写真・利用者の報告・ローカル検証記録に基づく。別スレッドでPDFを作成する場合は、本ファイルを最新状況の参照元とする。過去の日付の準備記録にある「実機未確認」「実物USBへの書込み未実施」を現在の全体状況として転記しない。

## 配布資料に使用できる確認状況

| 項目 | 確認状況 | 範囲・留意点 |
| --- | --- | --- |
| Ryzen 7機のUSB起動・画面表示 | 実機で確認 | 利用者所有機の特定構成。Ryzen全般の対応保証ではない。マザーボード・CPUの詳細型番は未記録 |
| Ryzen機のUSBメモリー読出し | 実機で確認 | SanDiskの容量取得、FAT32、MUSHA.TXT 16byte、アプリ側読出しを確認 |
| Ryzen機の有線キーボード入力 | 実機で確認 | H4でLenovo 17EF:6009のShift・A入力、Escによる終了を確認。任意のHID機器対応ではない |
| Ryzen機のUSB2ハブ配下列挙 | 実機で一部確認 | H4でハブ配下routeの機器列挙を確認。ハブ全種・高速TT・SuperSpeed配下転送の確認とは区別する |
| 複数入力機器の同時接続 | 既知の制約あり | H15でLenovo有線とRazer無線ドングルを同時接続すると、選択されたRazer側だけを監視しLenovo入力を受け付けない状況を確認。未修正 |
| MacBook Pro 2018のUSB起動・画面表示 | 実機で確認 | H19全10ページとruntime/arena/timer到達を確認。利用者が2018年モデルと明示 |
| MacBook Pro 2018のOS独自USBストレージ読出し | H19で実機成功 | 7D00の条件付き復旧、SanDisk容量取得、FAT32、MUSHA.TXT 16 byteのアプリ読出しまで確認 |
| MacBook Pro 2018の外付けキーボード入力 | H19で実機成功 | Lenovo 17EF:6009、A・Shift+A・Escの動作を利用者が確認。C0/00A0でINPUT DONE K1、KEY CODE 29、終了を確認 |
| MacBook Pro 2018の内蔵キーボード入力 | 成功未確認 | 外付けLenovoの成功とは区別する |
| 専用SanDisk USBの更新 | Mac実機で実施・照合済み | 既存GPT/FAT32を保った起動EFIの更新、バックアップ、読み戻し、ディスク検査、安全な取り外しを実施。ホスト側の更新作業であり、Musha-OSの書込み機能ではない |
| H17の実機動作 | Macの診断完了・USB未成功 | 全11診断ページを受領。BAR読取りは安定し、資源番号の混同を特定。RyzenのH17結果は未取得 |
| NUC5 / NUC8 | 実機結果未取得 | I218-V / I219-Vの実機LAN通信は未実装・未確認 |
| ネットワーク通信 | QEMUで確認 | Intel 82574経路のARP/ICMP/UDP echo。実機通信成功の記録なし |
| AHCI/NVMe、Wi-Fi、Bluetooth、音声、GPU直接制御等 | 実機確認なし | 実装範囲は各設計書を参照。対応済みと表示しない |

## 配布用の短い記載例

実機では、Ryzen 7機の特定構成でUSB起動、USBメモリー内のFAT32ファイル読出し、有線キーボード入力とEscによる終了を確認しています。MacBook Pro 2018ではH19でUSB制御器の復旧とOS独自ドライバによるFAT32ファイル読出しまで確認しました。同じMacで外付けLenovoキーボードのA・Shift+A入力とEscによる終了も確認しました。内蔵キーボードの動作は未確認です。NUC実機と実機ネットワーク通信は未確認です。

「実機検証はすべて未実施」とは記載しない。一方、「Ryzen対応済み」「Mac対応済み」「全機種対応」「実機で安定動作」とも記載しない。特定構成での成功記録と、製品としての一般的な互換性・再起動時の安定性は別である。

## 実機記録の根拠

### Ryzen成功実行 H4

[既存の詳細記録](ryzen-h4-hardware-results.md)。IMG_7434〜IMG_7444の同一実行11ページとIMG_7432/7433、利用者の入力観察が根拠。5コントローラーを走査。C1/0801でSanDisk 0781:55A9とFAT32/MUSHA.TXT読出し、C4/0F03でLenovo 17EF:6009の入力とEsc終了を確認。記録された停止はDMA disabled。当該実行でBAD COMPLETION/BAD TRANSFERは出ていないが、以前の断続的失敗と繰返し起動の安定性は未解決。

### Ryzen H15の入力問題

IMG_7494ではKEYBOARD READY、USB READ OK、FAT32 READ OK、APP FILE READ OKを表示。利用者はLenovo有線とRazer無線ドングルの両方が接続されていたと報告。コードと表示の照合では、先に選択されたC1のRazer 1532:0296を入力対象とし、別のコントローラー上のLenovo入力を監視しない。写真のPROBING表示だけでCPU全体の停止を断定しない。複数入力機器対応と表示改善は残件。この写真はH15であり、H16/H17のRyzen回帰試験ではない。

### MacBook Pro H13〜H16

H13: IMG_7470/7472/7473/7474。H14: IMG_7476〜IMG_7484。H15: IMG_7485〜IMG_7493。H16: IMG_7495〜IMG_7503。起動・表示は継続しているが、USB入力・ストレージは未成功。

H16のIMG_7502は、復旧候補7D00の資源検査でpeer segment 0、01:00.0、BAR2がDESCRIPTOR TYPEで拒否されたことを示す。rawの先頭wordからUEFI記述の種別はI/Oと読み取れるが、config側はmemoryとして扱われていた。真の資源重複や不一致の原因はまだ確定していない。機種固有例外で拒否を回避せず、H17で全BAR値・取得前後・PCIIOとCF8/CFCの照合を追加した。[H17設計と実装記録](pci-h17-bar-consistency-inspection-plan.md)。

### MacBook Pro H17（2026-10-10受領）

IMG_7504〜IMG_7515で全11ページを確認（9/11は2枚）。起動、runtime/arena/timer、診断セッション完了を確認。USB NO KBD/STORAGEは継続。詳細は[H17結果記録](pci-h17-hardware-results.md)を参照。

peer 01:00.0のBAR0とBAR2は64bitメモリーBAR、BAR1とBAR3は上位32bit。BAR4はI/O base 0x3000。GetBarAttributesに2を渡した結果はI/O base 0x3000、length 0x100であり、設定領域のBAR4と対応する。BEFORE/AFTER/REPEATは一致し、CF8/CFCの10項目も一致。共通処理が設定領域スロット番号とUEFI資源番号を混同していることが拒否の原因と判断した。修正後に別の安全条件で拒否される可能性があり、MacのUSB復旧成功は未確認。実際の資源重複はこの結果から確定できない。

APP COMPLETE / STATE SESSION COMPLETEは、エラーを含む診断セッションの終了も表す。USB成功の根拠には使用しない。

## 過去媒体 H17と検証の区分

専用媒体: SanDisk 3.2Gen1、容量30,784,094,208byte。H17通常版（features=[]）のEFIは294,400byte、SHA-256は `dbbac7d975fd4c25cb297bd76f6ab258300c532689ae35747e063f8ea0a74021`。

2026-10-10に旧H16をバックアップしてEFIのみ更新。読み取り専用で再マウント後に全byte一致、GPT検査成功、FAT32 fsck終了コード0、MUSHA.TXT保持、安全な取り外しを確認。記録: `out/physical-usb-h17-minimal-20261010/verification.json`。成果物: `out/usb-h17-bar-consistency-20261010/`。これらはローカルGit除外対象であり、一般公開済みリリースを意味しない。

H17のplatform単体検査22件、通常UEFIビルド、H17 qemu-debug版の単一・複数USBコントローラー構成で読出し・入力・終了を確認。QEMU結果をH17実機成功として表示しない。MacBookのH17全ページを受領し、起動・診断完了とUSB未成功を確認。RyzenのH17再試験は未受領。

## 次に確認すること

- Mac H19の読出し・外付け入力成功構成で、同じ接続の反復起動と長時間安定性を確認する。内蔵キーボード対応は別件。
- RyzenのLenovo有線単独でのH19起動・読出し・入力・終了、同一構成での繰返し起動。
- 複数キーボードの共通入力選択・監視の改善と再検証。
- NUC5/NUC8の実機試験、実機LAN通信の実装・検証。

## 前回媒体 H18（2026-10-10）

共通BAR番号対応を修正し、platform 23件と単一・複数USBコントローラーのQEMU回帰検査が成功。通常版成果物を作成。H18のMac実機全11ページを受領。起動・診断完了を確認したがUSBは未成功。専用SanDiskの旧H17を保存してH18通常版へ更新。読み取り専用再マウント後の全byte一致、GPT/FAT32検査、MUSHA.TXT保持、安全な取り外しを確認。EFI 295,936byte、SHA-256 `34e3a982244e14cca2976b3353170d7501f539b113cd6b8b4bce9501fe3adac4`。完了記録: `out/physical-usb-h18-minimal-20261010/verification.json`。H17は前回媒体の記録。[設計・検証記録](pci-h18-bar-index-plan.md)。

## Mac H18実機結果

IMG_7516〜IMG_7526の全11ページを確認。旧01:00.0の種別拒否を通過し、00:1f.5 / BAR0のBAR BASE MISMATCHへ進んだ。設定値0xFE010000に対し記述基底は0x81617000、長さ0x1000。取得前後・追加読取りとCF8/CFCは一致し、原因の詳細は未確定。USB復旧は拒否を維持し未成功。[詳細と次の検査設計](pci-h18-hardware-results.md)。

## H19実装・回帰検証（2026-10-10）

H18の直接の拒否原因は、USB以外の00:1f.5（8086:A324、Intel SPI制御器）の設定BARとUEFI記述の不一致。根本の不一致発生理由は未確定。復旧後のUSBストレージ読出しは後述のH19実機で成功。利用者がリスクを理解して明示承認した後、両範囲の非重複・設定照合・復旧直前再確認を条件とするH19を実装した。platform全27テスト、UEFI通常版ビルド、単一・複数xHCIでのQEMU入力・GPT/FAT32読出し回帰試験に成功。H19実機のストレージ読出しと外付けLenovo入力・Esc終了が成功。[詳細](pci-h19-peer-resource-plan.md)。

## H19専用USB更新完了（2026-10-10）

専用SanDisk（30,784,094,208 byte）の旧H18をバックアップし、H19通常版EFIのみ置換。302,080 byte、SHA-256 `2ceaea47977e544bf7469cb562cc8f041df32de3cdcd00408eedc674fb5f08d9`。書込み直後と読み取り専用再マウント後の全byte一致、MUSHA.TXT保持、GPT検査、FAT32検査終了0を確認し、安全に取り外した。完了記録: `out/physical-usb-h19-minimal-20261010/verification.json`。USBの内容検証に加えてH19実機起動・ストレージ読出しも確認済み。外付けLenovo入力も追加実行で確認済み。内蔵入力は未確認。

## MacBook Pro 2018 H19初回実機結果（外付け未接続）

IMG_7527〜IMG_7536の全10ページを確認。PCI RECOVERED 7D00、SanDisk 0781:55A9のSuperSpeed列挙、USB READ OK、FAT32 READ OK、APP FILE READ OKを確認。MUSHA.TXTは16 byte、ハッシュ9A42A948C590F507。C2はK0 S1、C1は起動担当外のため復旧せずMEMORY DISABLED、C0はK0 S0。USB読出しを阻んでいた条件への対処は実機で有効と確認できたが、キーボード入力と反復安定性は未確認。[H19実機詳細](pci-h19-hardware-results.md)。

利用者追加回答: H19実行時は本体キーボードのみで、Lenovo等の外付けキーボードは未接続。今回のK0は外付けUSBキーボードの動作不良の証拠ではない。内蔵キーボード対応は別の確認事項。

## MacBook Pro 2018 H19外付け入力確認（最新）

IMG_7540〜IMG_7552の13枚で全12診断ページを確認。利用者がLenovoキーボードのA、Shift+A、Esc動作を報告。C0/00A0/root02に17EF:6009を認識しINPUT DONE K1 S0、KEY CODE 29、APP COMPLETEを表示。C2/7D00/root03のSanDiskでは、PCI復旧、USB/FAT32/MUSHA.TXT 16 byteのアプリ読出しが今回も成功。異なるコントローラーのストレージと外付け入力を同一セッションで使用し終了まで確認した。

この構成でのUSB読出し・外付け入力の復旧目標は達成。C1/0700のMEMORY DISABLEDは復旧対象外の制御器の状態で、上記成功を否定しない。全ポート対応や内蔵キーボード対応、反復・長時間安定性まで確認済みとは記載しない。[証拠・接続経路・残件](pci-h19-hardware-results.md)。
