# H16: 共通BAR資源解析と拒否根拠の明確化

H15実機IMG_7491で01:00.0の記述形式が拒否された。原因となる個別フィールドは未確定。復旧成功を前提に条件を緩めない。

## 仕様と共通設計
UEFI 2.10 §14.4.18のQWORD記述（tag 8A、payload 43、memory type 0、終端79）を純粋関数 memory_bar_descriptor に集約する。64bitのhost base/lengthを返し、通常controller選択と全PCI peer検査で共有する。タグ、長さ、種別、終端、checksum、translation、rangeを個別に拒否する。Checksum 0は仕様に従い検査省略、非0は48byteの和が0を要求する。アドレス変換は有効な仕様項目だが現行直接MMIO/復旧は同一host/PCI addressを前提とするため未対応として明示拒否する。機種名やvendorで分岐しない。

通常controllerのサイズ・アラインメント・アドレス上限は維持。peerは大きな64bit memory rangeも検査する。複数記述・可変長拡張は未対応として拒否し、firmwareポインターを宣言長で無制限走査しない。従来同様firmwareが返した固定48byte形式を対象とする（APIにallocation sizeはない）。BAR最大値は既存実装との互換性のため追加制約にしない。

## 診断と安全性
最初の失敗にpeer BDFとBAR番号、拒否理由、固定48byteの6個のlittle-endian raw wordを保存する。後の表示で復元できる。失敗時BASE/LENGTHは未確定0とし、rawをbaseと誤表示しない。所有者・snapshot一致・PM・bridge・memory map・重複検査、BAR復元手順は変更しない。未確定資源を無視しない。通常MSE有効機器は既存経路を維持。

## 検証
切り詰め、タグ、長さ、種別、終端、translation、checksum、空/overflow range、4GiB超の大きなpeer資源とcontrollerサイズ拒否を単体検証する。正常/複数controller QEMUで入力・ファイル読取り・DMA停止を検査し、USB通常buildと読み戻しを照合する。

仕様: https://uefi.org/specs/UEFI/2.10/14_Protocols_PCI_Bus_Support.html#efi-pci-io-protocol-getbarattributes

## 実施結果
2026-10-10: musha-platform 20 tests PASS。QEMU正常単一controllerおよび複数controller/second側deviceのGPT fixture・keyboard exit smoke完了。通常UEFI features=[] build PASS、diff check PASS。USB更新・照合結果はphysical-usb-h16記録に保存する。実機で拒否項目を確認するまではUSB復旧成功は未確認。
