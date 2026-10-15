# 実機確認状況（配布用PDFの参照資料）

Historical filename note (2026-10-14): recorded screens used `MUSHA.TXT`; new candidates use `DIAGNOSE.TXT`. Historical observations below are unchanged.

当時のファイル名は `MUSHA.TXT`。2026-10-14以降の候補は `DIAGNOSE.TXT` を使用します。過去の実機・検証記録は変更していません。


Historical captures retain their original filename, MUSHA.ELF. From the renamed candidate onward, use AUTORUN.ELF.

過去の撮影記録は当時のファイル名 MUSHA.ELF のまま保持します。改名後の候補以降は AUTORUN.ELF を使用します。

過去の観測の訂正（2026-10-14）：IMG_7633で冷間NO MEDIUMを1回（PRE OWN D8=0）、IMG_7634で切替済みD0=0x7FF/D8=0xFのHello・終了0を1回確認。run 1後の操作とrun 2/3の電源断時間は所有者も正確には覚えておらずUNKNOWNです。所有者はこの訂正を承認済みです。冷間3回連続成功は未証明。以下の成功回数は過去の所有者報告として保持し、次候補の受入には転用しません。[実機受入記録](hardware-acceptance-0.3.0.md)を参照。

## Current pre-release status / 公開前の現状 (2026-10-14)

Current normal candidate: `9f8d9997e06d6edefdcb60593de07d4243b3649e`.
EFI: `ba23cabb988af8349924b78914000d030899ac3589568ce8b79fa7e3bcf21cad`.
Deployment: `out/release-sdk-guide-stage3/usb-normal-deploy/provenance.json`.

| Machine | Current evidence | Remaining acceptance |
|---|---|---|
| NUC5i5RYH | IMG_7647.MOV shows Hello/exit 0. Owner reports cold 3/3, each with AC unplugged ≥10 s. | PASS under owner decision: existing IMG_7645 D8=0 diagnostic + normal cold 3/3; no D8 measurement in normal. |
| Ryzen 7 | Refreshed candidate IMG_7648.MOV shows 4K Hello/exit 0; owner reports cold 3/3. Filmed READ OK-to-Hello approximately 3–5 s. | All three: power off ≥10 s, AC connected (owner). Hot 1 PASS: IMG_7652 Hello/exit 0, approx. 3–5 s; unfilmed individual timings unknown. |
| MacBook Pro 2018 | IMG_7650: Basic Data boot, USB READ OK, Hello/exit 0. Owner reports cold 3/3, power off ≥10 s, AC connected. IMG_7651 additionally shows Hello/exit 0 with external keyboard disconnected (owner-reported configuration). | Boot gate PASS. Prior storage failures remain unexplained; T2 internal keyboard unsupported. |

現行通常版候補9f8d999のNUC5冷間3回連続成功を所有者報告として記録しました。
各回ともACを抜いて10秒以上。IMG_7647.MOVでは1回のHello・終了0を直接確認し、
何回目かは未特定です。通常版のD8は未測定で、所有者決定で既存IMG_7645の診断D8=0証拠1回＋通常版冷間3回を受入条件とし、合格です。
Ryzen更新候補の冷間3/3は所有者報告済みで、IMG_7648でHello・終了0、READ OK→Hello概算3〜5秒を確認。全3回とも電源OFFで10秒以上、AC接続のままと確認済み。ホット1回もIMG_7652でHello・終了0を確認し合格（概算3〜5秒）です。MacはIMG_7650でBasic Data起動・USB読込み・Hello・終了0を確認し、冷間3/3成功（電源OFF10秒以上、AC接続）の所有者報告で起動条件合格です。
IMG_7651では外付けキーボードを外した追加試験でもHello・終了0を確認しました（接続条件は所有者報告）。内蔵キーボード入力は未対応のままです。
Macストレージの過去の失敗原因の調査は0.4.0以降。所有者変更後の条件で3機の実機受入ゲートは合格です。`d8d0eef`で再ビルド一致確認とCIは合格。後続の公開対象コミットは別途確認します（[リリースノート](release-notes-0.3.0.md)参照）。
[run別記録](hardware-acceptance-0.3.0.md)。

### 訂正前の経過（当時の報告・判断）

NUC5追記（2026-10-14）：所有者よりIMG_7633.MOVは1回目のループ、IMG_7634.MOVは3回目の成功と確認。後者36.967秒の動画でMUSHA.ELF／Hello Musha-OS!／exited: status 0を直接確認。18.483秒のPRE OWN P12 PORTSCは00001203（CCS1 PED1 SPEED4 PLS0）、D0=000007FF、D8=0000000F、所有権移行0ms。1回目ではROOT12未接続とDIAGNOSTIC HOLD NO MEDIUMの反復を確認。動画と試行番号の対応は解決済み。3回とも最終的にHello成功との所有者報告は保持するが、1回目の診断停止からHelloに至る遷移は現行コードで説明できず未収録のため、安定した冷間3回成功とは判定しない。表示だけの問題とせず、ルーティング後の接続遅延を仮説として残す。両動画は直前配備の診断版f10672d4へ関連付け。今回バイナリー・USB変更なし。
追記（所有者回答）：ReadyはHello Musha-OS!／exited: status 0を指すと確認。冷間3回すべてのHello成功は所有者確認済みとして記録する。一方、添付動画で確認したNO MEDIUMの回と、その3回との対応は未確定。映像を成功の直接証拠とはしない。現行コードの診断停止はUSB再試行を行わないため、反復後に同じ起動でHelloへ進んだという解釈は未解決。
NUC5 IMG_7633.MOV（2026-10-14）：所有者報告は冷間3回成功、1・2回目は2周、3回目は1周でReady。ただし成功画面の意味を確認中。44.253秒の動画ではDIAGNOSTIC HOLD NO MEDIUM、SCANNED K0 S0、STOPPED DMA DISABLEDと9ページの反復を確認し、Hello／終了画面は未確認。列挙時ROOT 12～15はCCS0/PED0/PLS5、所有権移行はBIOS1/OS0→BIOS0/OS1・0ms。媒体未検出の証拠があるためHello冷間3回合格にはしない。直前配備の診断EFI f10672d4…へ関連付け（動画からハッシュ測定したものではない）。外部アプリ未検出時のdiagnostic_capture→diagnostic_pages(false)は保存ページを反復表示するだけで、USB再試行は行わない。Intelルーティング直後の即時空き判定が遅れて現れる接続を逃した可能性はあるが、原因確定ではない。新通常版の受入移行は保留。証拠：out/nuc5-7633/observation.json、sample-3/6/7.png。
2026-10-14当時：待機省略後の診断版f10672d4でRyzen冷間3/3・ホット1/1成功（所有者申告、IMG_7632.MOV）。動画は診断1/23までで、Helloと末尾計測ページは未収録。短縮後の秒数、NUC5の同版冷間3回、新通常版受入は未確認。[実機記録](hardware-acceptance-0.3.0.md)。
更新日: 2026-10-14（日本時間）。写真・利用者の報告・ローカル検証記録に基づく。別スレッドでPDFを作成する場合は、本ファイルを最新状況の参照元とする。過去の日付の準備記録にある「実機未確認」「実物USBへの書込み未実施」を現在の全体状況として転記しない。

## 配布資料に使用できる確認状況

| 項目 | 確認状況 | 範囲・留意点 |
| --- | --- | --- |
| Ryzen 7機のRing 3 Hello | 更新候補9f8d999／ba23cabbで冷間3/3（所有者報告、電源OFF10秒以上・AC接続） | IMG_7648.MOVで4K・Hello・終了0。READ OK→Hello概算3〜5秒。ホット1回もIMG_7652で成功、概算3〜5秒。[記録](hardware-acceptance-0.3.0.md) |
| Ryzen 7機のUSB起動・画面表示 | 実機で確認 | 利用者所有機の特定構成。Ryzen全般の対応保証ではない。マザーボード・CPUの詳細型番は未記録 |
| Ryzen機のUSBメモリー読出し | 実機で確認 | SanDiskの容量取得、FAT32、MUSHA.TXT 16byte、アプリ側読出しを確認 |
| Ryzen機の有線キーボード入力 | 実機で確認 | H4でLenovo 17EF:6009のShift・A入力、Escによる終了を確認。任意のHID機器対応ではない |
| Ryzen機のUSB2ハブ配下列挙 | 実機で一部確認 | H4でハブ配下routeの機器列挙を確認。ハブ全種・高速TT・SuperSpeed配下転送の確認とは区別する |
| 複数入力機器の同時接続 | 既知の制約あり | H15でLenovo有線とRazer無線ドングルを同時接続すると、選択されたRazer側だけを監視しLenovo入力を受け付けない状況を確認。未修正 |
| MacBook Pro 2018のUSB起動・画面表示 | 実機で確認 | H19全10ページとruntime/arena/timer到達を確認。利用者が2018年モデルと明示 |
| MacBook Pro 2018のOS独自USBストレージ読出し | 現行候補IMG_7650でUSB読込み・外部Hello成功 | 過去IMG_7604 2.mov／IMG_7605.MOVのNo mediumの原因は未特定、切り分けは0.4.0以降。以下は過去H19の成功：7D00の条件付き復旧、SanDisk容量取得、FAT32、MUSHA.TXT 16 byteのアプリ読出しまで確認 |
| MacBook Pro 2018の外付けキーボード入力 | 0.3.0開発版で検出、入力成功は過去H19 | IMG_7604 2.mov／IMG_7605.MOVではKEYBOARD READYまで。以下は過去H19： Lenovo 17EF:6009、A・Shift+A・Escの動作を利用者が確認。C0/00A0でINPUT DONE K1、KEY CODE 29、終了を確認 |
| MacBook Pro 2018の内蔵キーボード入力 | T2未実装のため未対応 | 外付けLenovoの成功とは区別する |
| 専用SanDisk USBの更新 | Mac実機で実施・照合済み | 既存GPT/FAT32を保った起動EFIの更新、バックアップ、読み戻し、ディスク検査、安全な取り外しを実施。ホスト側の更新作業であり、Musha-OSの書込み機能ではない |
| H17の実機動作 | Macの診断完了・USB未成功 | 全11診断ページを受領。BAR読取りは安定し、資源番号の混同を特定。RyzenのH17結果は未取得 |
| NUC5i5RYH | 通常版候補9f8d999／ba23cabbで冷間3/3成功（所有者報告、全回AC断10秒以上） | IMG_7647.MOVで1回のHello・終了0を確認。既存IMG_7645診断1回＋通常版冷間3回を所有者が受理、受入合格。過去の試験と区別。[記録](hardware-acceptance-0.3.0.md) |
| NUC8 | 実機結果未取得 | NUCの実機LAN通信は未実装・未確認 |
| ネットワーク通信 | QEMUで確認 | Intel 82574経路のARP/ICMP/UDP echo。実機通信成功の記録なし |
| AHCI/NVMe、Wi-Fi、Bluetooth、音声、GPU直接制御等 | 実機確認なし | 実装範囲は各設計書を参照。対応済みと表示しない |

## 0.3.0開発版の過去の経過（現行候補の受入ではない）

2026-10-14当時：通常版候補5e4af31eでRyzen冷間3/3・ホット1/1成功（所有者申告、IMG_7628 2.mov）。当時はNUC5とRyzenの冷間起動受入完了と記載していたが、現在の候補の受入完了を意味しない。MacのBasic Data起動は未完了。Ryzenの起動待ち時間は別課題として記録。以下は各試験当時の経過。

2026-10-14当時：通常版候補5e4af31eでNUC5冷間3/3成功（所有者申告）。IMG_7627.MOVでHello・終了0を確認。Ryzen通常版候補冷間3回とMacのBasic Data起動は未完了。以下は各試験当時の経過記録。

2026-10-14当時：診断版d2f5d0c0でNUC5冷間3/3成功（所有者申告、IMG_7626.MOVはRUN SESSION RESULT OKまで確認）。Ryzen診断版冷間3/3・ホット1/1と合わせ診断版再試験を完了。通常版候補の実機受入は別途未実施。

2026-10-14追記：IMG_7622.movでRyzen 7のHello・終了0を確認。所有者報告は冷間起動3/3、ホットリブート1/1成功。所有者確認により最新診断版d2f5d0c0へ関連付け確定。通常版公開候補の受入とは区別する。

2026-10-14追記：IMG_7619.movは前版の冷間再起動失敗（直前のRyzen 7試験の文脈）。USB READ OK・容量表示後にNo mediumへ遷移。全ポート未検出とは区別し、後続の失敗段階は未特定。最新診断版の試験ではない。

2026-10-14追記：NUC5診断版の冷間起動3回は所有者報告で成功／媒体なし（キーボード検出）／成功。3回連続成功は未達。IMG_7612.jpgと関連EFIはhardware-acceptance-0.3.0.md参照。USB3の上限付き回復と失敗時診断保持を追加した次版の実機確認待ち。

2026-10-14追記：IMG_7611.MOVで診断版7eba0377のNUC5実行がSanDisk認識・FAT32読込み・Ring 3 Hello・終了0まで成功。root 1の拒否は再現せず、冷間起動条件と反復安定性は未確認。通常候補の冷間起動受入は引き続き未達。[詳細](hardware-acceptance-0.3.0.md)。

NUC5・Ryzenでは開発版でRing 3 Hello成功（IMG_7602.mov／IMG_7603.mov）。NUC5の候補冷間起動は失敗し、USB機器探索・Address Deviceを調査中です。MacBook Pro 2018は起動・外付けキーボード検出までで、ストレージ未認識、原因未特定です。過去のH19／10月12日rc1のFAT32成功を現在の成功として転記しません。内蔵キーボードはT2未実装のため未対応です。公開候補EFIによるNUC5の冷間起動3回成功は未達、Ryzenの冷間起動3回とMacの候補Basic Data起動受入は未完了です。[各回の証拠・ハッシュ紐付け](hardware-acceptance-0.3.0.md)。

## 配布用の短い記載例（過去H19の範囲）

実機では、Ryzen 7機の特定構成でUSB起動、USBメモリー内のFAT32ファイル読出し、有線キーボード入力とEscによる終了を確認しています。MacBook Pro 2018ではH19でUSB制御器の復旧とOS独自ドライバによるFAT32ファイル読出しまで確認しました。同じMacで外付けLenovoキーボードのA・Shift+A入力とEscによる終了も確認しました。内蔵キーボードはT2未実装のため未対応です。NUC5i5RYHでは外部C Helloの画面出力と終了コード0を確認しましたが、初回は媒体・キーボード未検出で、反復安定性は未確認です。NUC8と実機ネットワーク通信は未確認です。

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
- 9f8d999候補の実機受入は完了。NUC5は所有者決定で既存IMG_7645診断1回＋通常版冷間3回を採用。`d8d0eef`で一致確認・CI合格。次は後続文書コミットでの最終確認。NUC8と実機LAN通信は未確認。

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

## 2026-10-15 Ryzen Realtek identification prerequisite / Realtek特定の前提

### English

Goal13 now requires Ryzen Realtek LAN as well as NUC5 I218-V. Exact Ryzen Ethernet PCI vendor:device/subsystem/revision and motherboard manufacturer/product/revision remain **UNKNOWN**. The six retained JSON/text/log files under `out/ryzen-*`, the Ryzen hardware results and the 2026-10-13 provenance do not contain those explicit identifiers. Video appearance and CPU/xHCI inventory are not used to infer a chip. This is an identification finding, not a LAN test or a change to historical acceptance.

Before Realtek driver work, the owner captures the same machine's PCI Hardware Ids and motherboard identity using the [read-only Windows/Linux procedure](ryzen-realtek-identification.md), then appends dated evidence and identifiers here. Existing normal devices panels do not list every unsupported PCI vendor. Required future physical evidence is Ryzen link/DHCP/OS UDP echo, separately from NUC5 I218-V. No new physical test was performed for this entry.

### 日本語

目標13はNUC5 I218-Vに加えRyzen Realtek LANも必須。Ryzen Ethernetの正確なPCI vendor:device／subsystem／revisionとboard manufacturer／product／revisionは**未特定**。保持する `out/ryzen-*` JSON/text/log計6件、Ryzen実機結果、2026-10-13来歴には明示IDが無い。動画の見た目、CPU／xHCI一覧からchipを推定しない。特定状況の追記であり、LAN試験や過去受入の変更ではない。

Realtek着手前に所有者が同一機体のPCI Hardware Idsとboardを[Windows/Linux読取り専用手順](ryzen-realtek-identification.md)で採取し、日時・証拠・識別値をこの新節へ追記する。現行通常devices表示は未対応PCI vendor全件の一覧ではない。今後必要な実機証拠はRyzenのlink／DHCP／OS UDP echoで、NUC5 I218-Vと別に検証する。この追記で新しい実機試験は行っていない。
