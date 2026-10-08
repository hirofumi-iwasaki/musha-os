# xHCI初期化の第1段階

対象は最初に見つかったsegment 0のxHCIコントローラー一台。
所有権取得・停止・リセットに続き、[DMA / ring診断](xhci-rings.md)も実装済み。
port reset、USB列挙、キーボードとストレージ処理は未実装。

## BARとマッピング

UEFI稼働中にPCI I/O Protocolを列挙し、class 0C0330とBDFを調べる。
GetBarAttributesでBAR0のQWORD memory resourceを取得し、開始と長さをコピーする。
返却されたresourceとhandle一覧は最終メモリマップ取得前にFreePoolする。
実機の稼働中BARへサイズ計測の書込みは行わない。

segment 0、translationなし、1MiB以上のbase、4KiB境界、4KiB〜1MiBの長さを要求する。
予約済み領域やUEFI map上のRAMと重なる場合は起動時に拒否する。
BAR全体をUC / RW / NXで恒等マッピングする。arenaへは含めない。
すべてのMMIOアクセスでアラインメントとBAR内の範囲を検査する。
メモリデコードが有効でなければ初期化しない。

## 状態遷移と待機

1. capability length、xHCI 1.0〜1.2、slot / port数、port register範囲を確認する。
2. PM timerが進むことを確認する。extended capability chainを最大256項走査する。
3. Legacy SupportがあればOS-owned byteを立て、BIOS-owned解除を最大1000ms待つ。
   BIOS-ownedを強制解除せず、失敗時は運用レジスタを書き換えない。
   取得後は定義されたSMI enableを無効化し、定義されたW1C statusだけをackする。
4. CNR解除を最大1000ms待ち、R/S・INTE・HSEEを無効化する。
   HCHaltedを最大100ms待つ。
5. PCI Commandを16bitアクセスしBus Master Enableを解除・読戻し確認する。
   隣接するPCI StatusのW1C bitを変更しない。
6. HCRSTを立て、解除とCNR解除をそれぞれ最大1000ms待つ。
   HCHaltedと4KiB page size対応を確認する。

各待機は時間期限のほか500万回の上限を持ち、停止した時計への無限待ちを防ぐ。
タイマーは各周回でpollする。コントローラーはhalted、bus mastering無効のまま残す。
失敗したcontrollerのDMA bufferを回収せず、起動時のBoot Services RAMもarenaへ回収しない。
機器失敗時はXHCI FAILEDを表示して診断アプリへ進み、全機能合格とは扱わない。

## 検証と残る作業

QEMU qemu-xhciで8portを検出し、reset完了とDMA無効状態を確認した。
`xhci-timeout` featureではhaltedのまま解除待ちを行い、20msの期限切れと
診断アプリへの継続を検査する。これはQEMU専用で実機版に含めない。
ホストでBAR resourceのI/O種別、translation、overflowを拒否することを検査した。
BIOS-ownedからOS-ownedへ実際に譲渡される待機経路は、実機での確認が必要。
NUC5のIntel USB routing / EHCI handoff、NUC8固有差、実機errataも未検証。

DMA / ring診断はNo-Op 600回と停止まで確認済み。Device Descriptorまでの[USB列挙](usb-enumeration.md)も確認済み。

一次資料: [Intel xHCI 1.2b](https://cdrdv2-public.intel.com/625472/625472_xHCI_Rev1_2b.pdf)
§4.2、4.22.1、5.4.1、5.4.2、7.1、
[UEFI PCI I/O GetBarAttributes](https://uefi.org/specs/UEFI/2.11/14_Protocols_PCI_Bus_Support.html)。
