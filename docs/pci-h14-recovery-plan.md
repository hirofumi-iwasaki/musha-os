# H14: UEFI終了時のPCI設定消失への限定復旧

## 根拠と範囲
H13の実機写真 IMG_7470/7471/7472: 0700/7D00 は PRE EBS で CMD=7、BAR=81700000/8F900000、POST EBS で両方ゼロ。起動元は7D00。00A0は不変。
機種名やIntel IDで分岐しない。起動元に一意に対応し、初期/終了直前のID・BAR・CMDが一致し、D0が維持された32-bit non-prefetchable memory BARのみ復旧。通常の有効機器は設定を書かず従来経路へ。

## 書込み前条件
- 終了前にUEFI GetBarAttributesの範囲とBARが一致。他の全PCI IO機器の全memory BAR resourceとの重複がない（取得できないresourceは検査不成立）。
- 終了直後と現在ともCMD=0、BAR0/1=0、ID不変、xHCI class/type0 header。unknown/multiple boot ownerは拒否。
- 保存メモリーマップ内の重複領域はMMIO(type11)のみ、runtime領域は拒否。
- 現在の上位PCI-to-PCI bridgeをsecondary busから一意に追跡。最大8段、loop拒否。MSE有効、primary/secondary/subordinate整合、全段のnon-prefetch memory windowが対象全範囲を転送可能。bridgeの書換え/再割当ては行わない。

## 復旧順序と失敗
BAR0のみ保存値を復元・読み戻し。CMDを16-bitでMSEのみ有効（BME/IO disabled、Status W1Cに触れない）。失敗時はCMD=0へ戻し、BME=0を確認して診断。アドレス再割当て、PM状態変更、bridge復旧、全機器へのCMD=7書込みは禁止。
その後既存xHCI ownership/halt/resetへ。専用DMAメモリーのリング構造を初期化してからBMEを有効にし、既存の順序でハードウェアにリングアドレスを登録する。通常経路の順序は維持。BME有効化をハードウェア登録後へ移した試行はQEMU HOST ERRORを起こしたため採用しない。DMA領域は全失敗経路で予約維持。
起動元を最初に試すが元indexを維持してDMA sliceの対応を変えない。起動元不明なら従来順序。初回以降の復旧済み機器は通常経路となる。

## 検証
純粋policyテストで正常機無変更、Mac実測復旧、ID/BAR/PM変化、非起動元、範囲不一致、64bit/prefetch BAR拒否を確認。QEMUで通常、複数controller、GPT起動USBを確認。QEMU標準BARは64bitであり今回の32bit復旧対象外。実際の復旧書込み経路のハードウェア動作はMac実機テストで確認。実機成功は次回のH14写真でのみ確定。
UEFI仕様のExitBootServices所有権移行を参照: https://uefi.org/specs/UEFI/2.10/07_Services_Boot_Services.html

## 完了検証記録（2026-10-10）
- musha-platform: 19 tests PASS（復旧判定とbridge window境界を含む）。
- 通常xHCI + GPT fixture: A/Shiftのdown/up、Esc、FAT32/API読込み、DMA停止 PASS。
- 複数xHCI + second側起動/keyboard + first側GPT fixture: 起動元0020優先、元indexでslice分離、両側enumerationと入力/ファイル/DMA停止 PASS。fixtureをfirst側に置いた既存試験の期待countをcontroller別に修正。
- 実GPT起動image: A/Shift/Esc、起動USB MUSHA.TXT/API読込み、DMA停止 PASS。
- USB配布用はfeatures=[]通常版。QEMU診断用imageは別ファイル。
- 64bit BAR、bridge復旧、PCI resource allocator、T2/BCE keyboard driverは今回の対象外。Mac実機でRECOVERED/REJECTおよびUSB列挙結果を確認する必要がある。
