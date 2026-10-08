# CPUテーブルと例外診断

2026-10-08実装。対象はx86-64のBSP、ring 0、割込み無効の初期ランタイム。

## テーブルと寿命

GDTはnull、64bit code（selector 0x08）、data（0x10）、64bit TSS（0x18、2項目）。
lgdt後にfar returnでCSを変更し、DS / ES / SSを設定、ltrでTSSを有効にする。
IDTは256個の16byte interrupt gate、ring 0、selector 0x08を使う。
sgdt / sidt / CS / TRの読戻しを検査してから起動成功を出力する。

GDT / IDT / TSSはEFIイメージ内の固定領域に置き、移動・回収しない。
BootInfoと通常64KiBスタックに加え、緊急スタック用32KiBをLoaderDataとして
ExitBootServices前に確保する。前半16KiBは#DFのIST1、後半16KiBはNMIのIST2。
TSSのI/O map offsetは104とし、I/O bitmapは提供しない。
ページテーブル管理を導入する際も、これらを恒等マップ・予約範囲として保持する。

## 例外入口

vector 0〜31はそれぞれ専用スタブを持つ。CPUがerror codeを積むvectorは
8、10、11、12、13、14、17、21、29、30。ほかはスタブでerror 0を補う。
共通入口へvector / error / RIP / CS / RFLAGS / RSP / SSを渡す。
Win64呼出規約でRCXにframeの基点を渡し、スタックを16byte境界に揃え、
32byte shadow spaceを確保してRustの診断関数を呼ぶ。方向フラグをクリアする。
ハンドラは復帰しないので汎用レジスタの保存・iretqは行わない。

CPU EXCEPTION、VECTOR、ERROR、RIP、CR2をGOPへ表示する。
CR2は#PFだけで読み取り、それ以外は0と表示する。
最初の診断中に例外が重なった場合は描画の再入を避けて停止する。
外部割込みはまだ有効にしない。vector 32〜255は未知vector 255として停止する
共通入口を使い、個別のIRQ処理やEOIは行わない。

## 試験と制約

QEMUで通常起動、#UD、#GP、#DFを確認する。
#UDはud2、#GPはGDT範囲外selector 0x28のDSへのロードで発生させる。
#DFは#GP gateを故障注入で無効化してから#GPを起こし、IST1を検査する。
故障注入featureは通常ビルドに含めない。

NMI、machine check、全予約vectorの挙動は未検証。#PFは自前ページテーブルで検証済み。
復旧、割込み駆動、panic情報の表示、緊急stackのguard page、FPU / SIMD状態の
完全な管理、実機試験は後続作業。通常スタックを完全に失った状態の#DF試験も未実施。
GDTからIDT切替までの短い移行中にNMIを扱う保証はまだない。

参照: [Intel SDM Volume 3の例外・64bit IDT / TSS仕様](https://www.intel.com/content/www/us/en/developer/articles/technical/intel-sdm.html)
