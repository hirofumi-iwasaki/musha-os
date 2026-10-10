# H19 PCI資源不一致への条件付き復旧

記録日: 2026-10-10。H18実機写真IMG_7516〜7526を根拠とする。利用者の明示承認後にH19を実装。MacBook Pro 2018で起動担当7D00の復旧とUSBファイル読出しを確認。外付けLenovoのA・Shift+A・Escも追加実行で成功。

## 現在判明している停止原因

起動媒体を担当する7D00のUSB制御器は、Boot Services終了前にはBAR0=0x8F900000、CMD=7で、終了後にはBAR0/CMDが0になる。現在の復旧処理は、別の機器00:1f.5の資源情報不一致により設定復元前に拒否している。

00:1f.5のID 8086:A324はIntel SPIフラッシュ制御器。実設定BAR0=0xFE010000に対し、UEFI記述はbase=0x81617000、length=0x1000。PCIIO取得前後・再読取り・CF8経路は一致。これが今回の直接の拒否原因である。ファームウェア記録が古い可能性はあるが、記録が食い違った根本原因は未確定であり、USB復旧後のファイル読出しはH19実機で確認済み。

EDK IIのGetBarAttributes実装は保存済みPciBar.BaseAddress/Length/Alignmentから記述を作る。記述のMAX=0xFFFを実アドレス終端として扱わない。この参考実装だけで対象Macの内部実装を断定しない。

## 修正案

現在はpeerの基底不一致を一律拒否する。提案は、32bit memory BARのpeerに限り、実設定側の最大占有可能範囲とUEFI記述範囲の両方を衝突検査するもの。対象USB自身の基底不一致は引き続き拒否する。

PCIの通常メモリBARはサイズが2の累乗かつ自然境界に整列する。非ゼロの基底Bについて、占有サイズはBの最下位の1ビットの値以下となる。0xFE010000では上限0x10000であり、実設定側の保守的検査範囲は[0xFE010000,0xFE020000)。これは実サイズの確定値でも、MMIOを読み書きする許可範囲でもない。UEFI側の[0x81617000,0x81618000)とともに、復旧先[0x8F900000,0x8F910000)とは重ならない。

実装時の必須条件:

- 対象を非ゼロの通常32bit memory BARに限定。64bit、予約種別、曖昧な記述、範囲演算異常は拒否する。
- 同一機器のID、class、header、command、全BARをPCIIOとCF8で照合。取得前後の安定性を要求する。
- 記述範囲と実設定側の保守的範囲のいずれかが復旧先と重なる場合は拒否する。
- 例外対象peerは固定容量で保持し、容量超過は拒否。復旧直前にも保存した設定との一致を再確認する。
- 全peer、起動経路、親ブリッジ転送範囲、メモリマップ、USB制御器の状態の既存検査を維持する。
- 許可後の書込みは既存の起動担当USB制御器のBAR0復元とMemory Space Enableに限定し、Bus Masterはこの復元段階では無効。SPI制御器への書込み、BARサイズ探索、試験的MMIO読取りは追加しない。

## 実装・検証

利用者はリスクを理解したうえで、条件付き復旧の実装・回帰検証・専用USB更新を明示承認した。初回の自動承認レビュー拒否後、この承認に基づき処理を進めた。

- `platform/src/pci_resources.rs`: 実設定側の保守的上限計算、範囲重複判定。ゼロ・非対応BAR・桁あふれ・不正な記述範囲は拒否。
- `boot/src/pci.rs`: 基底不一致のpeerについて両範囲を検査。PCIIOとCF8の全10設定値を照合し、取得前後の安定性とMemory Space Enableを要求。候補USB自身の不一致は拒否。
- 最大16件のscalar証拠をBootInfo外に保存。容量超過は拒否。復旧直前にもCF8で再照合し、対象範囲の非重複を再確認。設定変化時はPEER STATE CHANGEDで停止。
- SPIへの書込み、サイズprobe、親ブリッジ設定変更は追加していない。既存の全peer検査・メモリマップ検査・ブリッジ経路検査・起動担当判定を維持。
- 通常の有効なUSB制御器は既存のUnchanged経路を使用する。

platform全27テスト成功。H18の実値、旧記述サイズの外側でも保守的範囲内なら衝突扱いとなる例、2の累乗配置、未対応種別、境界・桁あふれを検証。UEFIコンパイル・通常版ビルド成功。単一xHCIと複数xHCI（機器を2番目に接続）のQEMU GPT/FAT32・キーボード終了試験が成功。記録は`out/qemu-h19-single`と`out/qemu-h19-multiple`。qemu-debug版には既存の未使用page関数警告が1件ある。

QEMU回帰試験はMacのファームウェア不一致・終了後のBAR消失を再現していない。新しい条件分岐の実機成功を証明するものではない。

通常版成果物: `out/usb-h19-peer-bound-20261010/files/EFI/BOOT/BOOTX64.EFI`、features=[]、SHA-256 `2ceaea47977e544bf7469cb562cc8f041df32de3cdcd00408eedc674fb5f08d9`。

## 実機確認手順（実施済み）

H19で起動し、PEER BOUNDの範囲表示、PCI RECOVERED、KEYBOARD READY、USB READ OK、FAT32 READ OK、APP FILE READ OKを確認する。Lenovo有線キーボードだけを接続し、Razer無線ドングルは外す。Shift+AとEscの入力を確認する。

未成功なら全診断ページを記録する。PEER STATE CHANGED、RESOURCE REJECT、PCI RECOVERY REJECT、または復旧後の新しいUSB段階で次の停止点を区別する。追加実行で外付け入力・ファイル読出し成功を確認し、今回の構成での対処目標は達成。RyzenのH19実機回帰も未確認。

## 一次資料

- [Linux Intel SPI PCI driver](https://github.com/torvalds/linux/blob/master/drivers/spi/spi-intel-pci.c): A324識別。
- [EDK II PciIo.c](https://github.com/tianocore/edk2/blob/master/MdeModulePkg/Bus/Pci/PciBusDxe/PciIo.c): GetBarAttributesの記述生成。
- [PCI Local Bus Specification 3.0（PCI-SIG文書、TI掲載）](https://e2e.ti.com/cfs-file.ashx/__key/communityserver-discussions-components-files/639/1016.PCI.Local.Bus.Specification.Revision.3.0.pdf): §6.2.5.1、BARサイズと配置。

## H19専用USB更新完了（2026-10-10）

専用SanDisk（30,784,094,208 byte）の旧H18をバックアップし、H19通常版EFIのみ置換。302,080 byte、SHA-256 `2ceaea47977e544bf7469cb562cc8f041df32de3cdcd00408eedc674fb5f08d9`。書込み直後と読み取り専用再マウント後の全byte一致、MUSHA.TXT保持、GPT検査、FAT32検査終了0を確認し、安全に取り外した。完了記録: `out/physical-usb-h19-minimal-20261010/verification.json`。USBの内容検証は完了したが、H19の実機起動・入力・ストレージ動作は未確認。

## 実機結果追記

2026-10-10、MacBook Pro 2018のH19全10ページを受領。7D00復旧、SanDisk列挙、FAT32/MUSHA.TXT読出しに成功。キーボード検出は0。[実機結果と残件](pci-h19-hardware-results.md)を最新の確認状況とする。上記の実機未確認の記述は媒体作成時点のもの。

## 外付け入力の実機確認追記

IMG_7540〜IMG_7552と利用者報告により、MacBook Pro 2018 / H19でLenovo 17EF:6009のA・Shift+A・Esc終了とSanDiskのファイル読出しを同一セッションで確認。C0が入力、復旧したC2がストレージを担当。内蔵キーボード・全ポート・長時間安定性は未確認。[最新の実機記録](pci-h19-hardware-results.md)。
