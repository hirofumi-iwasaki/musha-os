# PCI機器の診断

Musha-OS 0.1.0は、x86のPCI Configuration Mechanism 1（CF8 / CFC）で
segment 0のbus 0〜255、device 0〜31を走査する。
function 0のvendor IDがFFFFならスキップし、headerのmultifunction bitが
立つ機器ではfunction 1〜7も調べる。vendor / device ID、class / subclass /
programming interfaceとBDFをQEMU診断ログに出す。

設定アドレスの選択以外には書き込まない。BARのサイズ計測、バスマスター許可、
ブリッジの再設定、リセットはこの段階では行わない。
起動CPUだけがアクセスし、割込みを禁止した状態でCF8/CFCの組を扱う。
デバイス情報は逐次出力し、可変長の保存領域は使わない。

画面には全機器数、class 0C0330のxHCI数、vendor 8086かつclass/subclass
0200のIntel Ethernet数を表示する。検出だけでドライバ対応とは判定しない。
未検出でも診断を継続する。実機の個別NIC対応はPCI ID allowlistとPHY初期化の
実装時に確定する。

## QEMUでの検証

q35、qemu-xhci、e1000eの構成で7機器を検出した。
xHCIは1B36:000D、Intel 82574は8086:10D3だった。
smoke試験は両者のclass / IDと列挙完了マーカーを検査する。
NICにはQEMUのuser networkを接続するが、Musha-OSはまだ通信を開始しない。

## 制約と次の作業

ファームウェアが割り当てたbus番号を前提とする。
複数segmentやECAM / MCFGは未対応。NUC5 / NUC8の実機PCI IDは未確認。
ACPIの検証・保存、PM timer / HPET、BAR範囲の予約とマッピング、
xHCI / NICの所有権移行・初期化・復旧処理を続ける。

仕様参照: [Intel E8501 chipset datasheet](https://www.intel.com/content/dam/doc/datasheet/e8501-chipset-north-bridge-datasheet.pdf)
のPCI configuration access（CF8 / CFC）。
