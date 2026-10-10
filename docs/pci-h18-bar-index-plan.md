# H18 共通BAR番号対応の修正

2026-10-10。H17実機結果に基づく共通処理の修正。Mac固有の例外は設けない。

## 設計と実装

resource_safetyのPCI設定領域スロット番号とGetBarAttributesへ渡す論理資源番号を分離。64bit BARはスロットを2、論理番号を1増やす。32bit・I/O・未実装のゼロBARは両方1増やす。64bit上位枠は別資源として数えない。type 0の6枠、type 1の2枠という境界を維持する。

メモリー資源の基底を設定値と照合する。64bitでは上位32bitを結合し、低位属性ビットを除く。不一致はBAR BASE MISMATCHで拒否。構造、種別、translation、範囲、読取り安定性、全peer重複検査、ブリッジ、メモリーマップ、boot owner等の既存安全条件を維持する。PCI設定への新規書込みやサイズprobeは追加しない。

H17の01:00.0構成では設定BAR0/2/4/5と論理0/1/2/3が対応する。物理BAR2のメモリー検査には論理1を使用する。以前のI/O記述の拒否を回避するのでなく、正しい資源を取得して検査する。

## 検証

platform 23件PASS（複数64bit BAR、I/O、32bit、ゼロ枠の番号対応を含む）。通常features=[] UEFI release build PASS。H18 qemu-debug版のGPT fixtureで単一・複数コントローラー構成の起動、ファイル読出し、入力、Esc終了PASS。通常版成果物はout/usb-h18-bar-index-20261010。QEMUの成功をMacのUSB復旧成功と扱わない。H18実機結果は未取得。

通常EFI SHA-256: `34e3a982244e14cca2976b3353170d7501f539b113cd6b8b4bce9501fe3adac4`。

USB更新の最終状態は実機確認状況ファイルとout/physical-usb-h18-minimal-20261010の記録を参照する。

専用SanDisk更新完了。旧H17バックアップ、通常版全byte一致（読み取り専用）、GPT検査、FAT32 fsck終了コード0、MUSHA.TXT保持、安全な取り外しを確認。H18実機起動結果は未取得。

## 実機結果受領後の追記

H18全11ページを受領。旧番号混同による拒否を通過したが、別peer 00:1f.5のBAR基底不一致で復旧を拒否。USBは未成功。上記の実機未取得は受領前の記録。[実機結果と次の検査](pci-h18-hardware-results.md)を最新状態として参照する。
