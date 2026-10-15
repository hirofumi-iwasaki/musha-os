# H15: PCI復旧拒否の根拠を保存する診断改修

H14実機 IMG_7477/7479: 起動元7D00はRESOURCE CONFLICTでBAR書込み前に拒否。bool判定は実重複と情報不足を区別できない。まず診断だけを改修し、安全条件は緩和しない。

## 設計
機種名・vendor IDで分岐しない。既存全PCI memory BAR検査の許可条件と走査順序を維持。各controllerに最初の拒否理由、相手segment/bus/device/function、エラーまたはBAR番号、base/lengthを固定長で保存する。終了後の復旧判定時に保存値を表示する。ポインターやfirmware handleを保存しない。通常のMSE有効機器は従来どおり無変更。BARの復元、bridge書換え、情報取得失敗の無視を追加しない。

## 表示と限界
BAR OVERLAPのみRESOURCE CONFLICT、それ以外はRESOURCE UNKNOWN。列挙・protocol・location・header・BAR読込み・属性取得・descriptor形式・rangeを区別する。DETAILはBAR OVERLAPならBAR番号、BAR ATTRIBUTESならUEFI status。他段階は0。未確定peerは全ビット1。最初の拒否のみを示し、後続の問題が無いことは意味しない。

## 検証
通常・複数controllerのQEMU入力/ファイル/DMA停止、既存policy test、UEFI通常build、USB読み戻しを確認する。実機診断結果で次の修正を決定する。H15では復旧成功を主張しない。

## 実施結果
2026-10-10: musha-platform 19 tests PASS。QEMU単一controller/GPT fixture、および複数controller/second側起動・keyboard/GPT fixtureはsmokeスクリプト終了0（入力・ファイル・cleanup検査を含む）。UEFI通常build features=[] PASS、diff check PASS。USB更新の最終検証は認証後に別記録へ保存。
