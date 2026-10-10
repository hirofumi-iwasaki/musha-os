# H17: BARとUEFI資源記述の整合性検査設計

状態: 実装・検証・USB更新済み。2026-10-10。H16の実機結果を受けた次の診断であり、USB復旧成功を目的として安全条件を緩める変更ではない。

## 判明したことと未確定事項

IMG_7502では、復旧対象7D00のpeer検査がsegment 0、01:00.0、BAR2で `DESCRIPTOR TYPE` を理由に停止した。先頭raw word `0000000001002B8A` をlittle-endianで展開するとtag 8A、payload 43、ResourceType 1（I/O）となる。一方、現在の走査はconfig BARが非0かつbit 0が0のときだけmemory記述を要求する。したがって、実際のmemory重複を検出したとはまだいえず、config側の分類と記述側の分類が一致していない。

写真にはそのpeerのconfig BAR値がない。64bit BAR上位DWORDの誤分類、読み取り位置・幅・結果の不一致、取得中の値の変化、firmwareの資源記述の不一致を区別できない。これらは仮説であり、原因と断定しない。H16の拒否を維持して証拠を追加する。

## 次の検査の範囲

機種名・vendor IDによる例外を設けず、共通PCI資源走査に診断を追加する。最初の拒否peerと復旧候補を固定長の値として記録する。全peerの詳細を無制限に保存・表示しない。USB入力の複数機器選択は別課題として扱う。

診断の取得はBoot Servicesが利用できる段階で完結する。GetMemoryMap取得からExitBootServices成功までの既存の重要区間には、新しいfirmware呼び出しや出力を挿入しない。終了後は保存した値だけを表示し、protocolやresourceのポインターを保持しない。

## 保存する証拠

1. segment/bus/device/function、PCIIO get_locationのstatus、ID、class、header type。各config読取りについてoffset、width、count、statusと取得値を保存する。失敗を値0で代用しない。
2. header type 0はBAR0〜5、type 1はBAR0〜1を一括snapshotし、各DWORDを表示する。その他のheaderは未対応と明示する。Commandはstatusと分けて表示する。
3. 各BARについて、raw DWORD、low BAR番号、memory32/memory64/I/O/reserved/zero、64bitの上位DWORDであるかを明示する。64bit lowが最後の有効BARにある場合は不正として拒否する。上位DWORDを独立したBARとしてGetBarAttributesへ渡さない。分類規則の診断結果と現在の走査結果も比較する。
4. GetBarAttributesの呼出BAR番号、戻りstatus、nullか否か、現在対応している固定48byte形式のraw 6 wordsを保存する。取得できた構造について種別、flags、granularity、minimum、maximum、translation、length、終端、checksumを表示し、構造の検査結果と復旧policyの検査結果を区別する。
5. 資源取得の直前と直後に同じpeerのconfig snapshotを取得し、変化の有無を記録する。安定性確認の再取得は1回までとし、一致する値が出るまで反復しない。変化や再取得失敗があれば未確定として拒否する。
6. segment 0かつBDF範囲が有効なpeerに限り、同じconfig DWORDを既存CF8/CFC読取りで照合する。PCIIOとCF8/CFCの取得順と段階を記録する。segment 0以外では照合を実行せず、未対応と表示する。CF8のアドレス選択以外のconfig書込みは行わない。

CF8/CFCは既存の排他・割込み制御を維持する。順次取得のため値の差だけでアクセス方式の誤りとは断定しない。直前・直後のsnapshotと合わせて評価する。

## 記述解析と安全条件

構造解析ではMemory/I/O/Bus/Unknownを区別できるようにするが、今回の診断では現在のmemory復旧判定を変更しない。type 1を見ただけで安全なI/Oとして走査から除外しない。configがmemoryなら種別不一致として拒否する。

GetBarAttributesにはallocation sizeの返却がない。現在の固定48byte形式を対象とする前提を維持し、宣言長に従った無制限な読取りをしない。複数記述・可変長・未知種別・translationは未対応として拒否する。読み取り済みの範囲内でのみフィールドを解析し、形式が不正ならdecoded値を確定資源として使用しない。resourceは取得した検査内で解放する。

BARサイズを測るためのall-ones書込み、BAR復元の追加試行、Command変更、bridge window変更、機器resetを診断に追加しない。既存の所有者一致、early/pre snapshot一致、PM、bridge、memory map、重複、復旧手順と失敗時停止条件を維持する。通常MSE有効機器の初期化順序とDMA制御も変更しない。

## 証拠からの判定

| 観測 | 判定と次の作業 |
| --- | --- |
| BAR2が64bit BAR1の上位部分 | BAR walkとsnapshotの分類を確認。上位部分の独立走査を防ぐ共通修正を別途設計する |
| PCIIOとCF8/CFCが安定して異なる | offset/width/count、BDF、アクセス経路を調査。復旧は拒否する |
| 資源取得の前後でconfigが変わる | 取得中の状態変化を調査。安定した資源と扱わない |
| configはmemory、記述はI/Oで安定 | firmware/config資源の不一致。例外で無視せず、追加の根拠が必要 |
| 両者I/Oで一致 | 現在の走査がmemory扱いに至った経路を調査。I/O除外のpolicy変更は別途検証する |
| config/記述がmemoryで一致 | rangeとtranslation、重複検査を再評価。拒否理由が次のpeerへ移る可能性も記録する |

今回の完了条件は、最初の不一致を再現可能な値で説明できること。USB検出成功は診断の合格条件にしない。原因が残る場合も、その未確定箇所を明示する。

## 検証計画と実機での確認

純粋関数の検査ではmemory32、memory64と上位部分（非0・0・bitパターン）、I/O、prefetchable、reserved、zero、header type 0/1、末尾の不正64bit BARを網羅する。記述はMemory/I/O/Bus/Unknown、切り詰め、tag/length/end/checksum不正、translation、空・overflow rangeを検査し、構造解析の成功が復旧許可にならないことを確認する。

取得処理は失敗status、null resource、前後の値の変化、再取得失敗、segment 0以外、BDF範囲外、固定保存容量の上限について確認する。最初の拒否根拠を後の検査で上書きしないこと、resourceの解放と終了後のpointer非参照を確認する。

既存platform testsと通常features=[] buildを実行し、単一・複数controller QEMUでUSB/FAT/file読取り、キーボード入力と終了、DMA停止を確認する。正常経路の診断だけで復旧許可が変化していないことも確認する。QEMUの合格だけでは実機firmware不一致の解消を保証しない。

MacBookでは全診断ページ、特に01:00.0の全BAR、読み取りstatus、直前/直後/CF8値、資源rawと分類を撮影する。Ryzenでは既知のLenovo有線キーボード単独で基準試験し、USB読取りと入力を確認する。Razerドングル同時接続の既知の入力選択問題は、この資源診断の結果と区別する。他の利用可能な実機も通常MSE有効経路の確認対象とする。

実装着手時には本設計と共通PCI設計書を更新し、検査結果と残る制約を記録する。USB更新は実装・検証後に、専用個体の照合、起動ファイルのバックアップ、置換、読み戻し、ディスク検査を行う。

仕様参照: [UEFI 2.10 PCI Bus Support / GetBarAttributes](https://uefi.org/specs/UEFI/2.10/14_Protocols_PCI_Bus_Support.html#efi-pci-io-protocol-getbarattributes)。前段: [H16共通解析設計](pci-h16-resource-parser-plan.md)、[H14復旧設計](pci-h14-recovery-plan.md)。

## H17実装と検証記録

2026-10-10: BAR分類と固定記述の構造解析をplatformの純粋関数として追加した。既存memory解析の許可条件は変更しない。peer資源取得の前後のconfig不一致・読取り失敗・不正なBAR layoutは復旧拒否を維持する。最初の拒否についてbefore/after、1回のrepeat、segment 0限定CF8/CFC照合を保存する。取得順、offset/width/count/status、全BAR分類とprefetch bit、記述の構造フィールドを終了後の診断ページへ表示する。属性statusとnull結果を分けて記録する。

保存はBootInfoの1ページ上限を維持するため、Boot CPU専用の固定配列へ分離した。firmwareのpointerは保存しない。診断journalは固定512行に拡大し、上限到達を明示する。複数controllerが同じ拒否peerを持つ場合も各controllerの記録を保持する。Boot Services終了の重要区間と通常xHCI初期化順序は変更しない。

platform 22 tests PASS、UEFI checkと通常features=[] release build PASS。単一controllerと複数controller（2番目にUSB機器を接続）のH17 qemu-debug版のQEMU GPT fixtureでファイル読取り・入力・終了のsmoke PASS。diff check PASS。成果物はout/usb-h17-bar-consistency-20261010。実機のUEFI読取りstatusや不一致はQEMUで再現できていないため、MacBookの全H17ページによる確認が次の必須工程となる。Ryzenの実機確認も未実施。

QEMUログ内のHARDWARE H17識別子とEscape入力・DMA停止マーカーを照合済み。通常features=[]版は検証用ログを出さないため、ログマーカー方式のsmokeにはqemu-debug版を用いた。USBへコピーする通常版は別の成果物としてmanifestのfeatures=[]とSHA256を照合する。

専用SanDiskへのH17通常版更新完了。旧H16のバックアップ、読み取り専用再マウント後のbyte一致、manifestのfeatures=[]とSHA256一致、GPT検査、FAT32 fsck exit code 0、MUSHA.TXT保持、安全な取り外しを確認した。記録: out/physical-usb-h17-minimal-20261010/verification.json。実機起動での原因確定は未完了。

## 2026-10-10 実機結果追記

MacBook ProのH17全11ページを受領。[詳細結果](pci-h17-hardware-results.md)。読取りは安定し、PCIIOとCF8/CFCは一致。設定領域BAR2とUEFI論理資源番号2を混同したため、物理BAR4のI/O記述をmemory検査へ渡していた。共通処理の番号対応修正が次の工程。USBは未成功、Ryzen H17は結果未取得。上記の「次の必須工程」「原因確定は未完了」は写真受領前の記録であり、この追記を最新状況とする。
