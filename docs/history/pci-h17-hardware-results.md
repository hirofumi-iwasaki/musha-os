# H17 MacBook Pro実機結果

記録日: 2026-10-10。利用者提供IMG_7504〜IMG_7515。全11診断ページを確認し、9/11のみ重複。正確なMac型番は未記録。

## 結果

起動・表示・runtime/arena/timer到達、STATE SESSION COMPLETE、APP COMPLETEを確認。USB NO KBD/STORAGEは継続。診断完了をUSB成功と扱わない。

復旧候補7D00はPCI RECOVERYでRESOURCE UNKNOWN。最初の拒否はpeer segment 0、01:00.0、指定番号2のDESCRIPTOR TYPE。0700はNOT BOOT OWNERで復旧を行わずMEMORY DISABLED。00A0は初期化・26ポート走査に到達したが、全ポートCCS=0で対象機器なし。

## 読取りの整合性

GetBarAttributesの前・後・追加1回のPCIIO読取りはいずれもstatus 0で一致。CF8/CFCも利用可能で、10項目すべてMATCH 1。属性取得status 0、NULL 0、location status 0。取得中の値の変化や読取り経路差は当該実行では観測していない。

peerのID DWORDは67EF1002、class DWORDは030000C2、command/statusは00100007、headerは00800040。

| 設定領域スロット | raw | 意味 | UEFI論理資源番号 |
| --- | --- | --- | --- |
| BAR0 | B000000C | 64bit memory、prefetch | 0 |
| BAR1 | 00000000 | BAR0の上位32bit | — |
| BAR2 | C000000C | 64bit memory、prefetch | 1 |
| BAR3 | 00000000 | BAR2の上位32bit | — |
| BAR4 | 00003001 | I/O base 0x3000 | 2 |
| BAR5 | 81500000 | 32bit memory | 3 |

指定番号2から返された記述は構造正常、TYPE 1（I/O）、GENERAL 0、SPECIFIC 0、GRANULARITY 0、MIN 0x3000、MAX 0xFF、TRANSLATION 0、LENGTH 0x100。MAXは撮影された値をそのまま記録する。

## 原因と次の方向

boot/src/pci.rsのresource_safetyは設定領域スロットを64bit BARで2増やし、そのスロット番号をGetBarAttributesへ渡している。UEFIの論理資源番号は64bit BARでも1増えるため、物理BAR2の検査で論理番号2（物理BAR4のI/O）を取得する。写真とコード、仕様・EDK IIの列挙処理がこの番号混同を裏付ける。Mac固有の拒否回避ではなく共通処理の修正が必要。

次の設計では設定領域スロットと論理資源番号を分離し、64bit上位枠を資源として数えない。未実装・ゼロBARを含む番号の保持、type 0/type 1ヘッダー、I/Oとmemoryの混在、複数64bit BARを検証する。記述の種別・基底アドレスとconfigの対応も確認し、不明・不一致は拒否を維持する。全peerの重複検査を完了しない限り復旧を許可しない。単なる拒否スキップやI/O記述をmemoryとして解釈する変更は行わない。

今回の修正候補は資源検査の拒否理由を説明するが、修正後のブリッジ・メモリーマップ等の安全検査通過や、Mac USB入力・読出し成功を保証しない。Ryzen H17実機回帰試験は未取得。H18実装・USB更新は今回行っていない。

## 一次資料

- [UEFI 2.10 PCI Bus Support](https://uefi.org/specs/UEFI/2.10/14_Protocols_PCI_Bus_Support.html): BAR indexと設定ヘッダーのオフセットは、32/64bit BARの組合せにより異なる。
- [EDK II PCI列挙処理](https://github.com/tianocore/edk2/blob/master/MdeModulePkg/Bus/Pci/PciBusDxe/PciEnumeratorSupport.c): GatherDeviceInfoではBarIndexを1ずつ増やし、設定オフセットはPciParseBarの戻り値で進める。
