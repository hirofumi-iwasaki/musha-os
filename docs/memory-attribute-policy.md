# Memory attribute correction / メモリー属性判定修正

## English

Decision (2026-10-10, H7): IMG_7447 identifies USB DMA, LoaderData descriptor 9, RT=0/RP=1/RO=1/XP=1/WB=1, rejected solely by attribute policy. UEFI GetMemoryMap Attribute describes capabilities, not necessarily current settings. RP/RO/XP are protection capabilities; Runtime instead requires preservation/mapping semantics. Source: https://uefi.org/specs/UEFI/2.10/07_Services_Boot_Services.html#getmemorymap

Implementation audit: AllocatePages requests LoaderData below 4GiB for USB and LAN DMA. Initialization occurs after ExitBootServices. loader_coverage requires complete LoaderData coverage and bounded, aligned nonzero ranges. arena selects Conventional WB-capable RAM after subtracting reserved ranges. Both incorrectly rejected RP/RO capability bits. The new private page tables map DMA RW/NX/UC and arena RW/NX/WB, preserve PAT/MTRRs, enable EFER.NXE and CR0.WP, and flush caches/translations at the switch. Image section permissions and stack guard remain separate. No write is attempted merely because a firmware attribute suggests capability; access policy comes from the installed page tables.

Confirmed scope: remove RP/RO rejection from LoaderData coverage and Conventional arena selection; keep Runtime rejection in both, WB requirement for arena, full coverage, bounds, type restrictions, reserved subtraction and overlap checks. Retain allocation, cache mapping and page-table implementation. Use one exported Runtime mask in validation and failure diagnostics. Keep decoded capability flags, update the footer and H7 identifier; move the reject mask off row 17, which the final MEMORY RESULT overwrote in H6. No Apple-specific exception. This does not accept firmware/runtime/reserved memory or change PE write/execute permissions. DMA cache capability acceptance stays as before; cache-policy expansion is outside this correction.

Validation: regression with measured 0x2600F capabilities, split coverage, and arena reserved holes; wrong type/gap/Runtime still rejected. Run memory tests, normal split-controller USB/FAT32/input QEMU, and RO/NX/guard fault tests. Build normal EFI with no test features. Mac hardware still requires re-test; successful QEMU does not prove Mac USB or DMA operation. Preserve dirty work and branch; no push or USB update in this implementation request.

## 日本語

確定方針（2026-10-10、H7）：IMG_7447でUSB DMA、LoaderData記述子9、RT=0／RP=1／RO=1／XP=1／WB=1を確認。停止は属性判定による。UEFI GetMemoryMapのAttributeは能力を表し、現在の設定とは限らない。RP／RO／XPは保護設定能力、Runtimeは保存・マッピングに関する別の意味。仕様リンクは英語節に記載。

実装再点検：USB／LAN DMAはAllocatePagesで4GiB以下のLoaderDataとして確保。初期化はExitBootServices後。loader_coverageは全範囲LoaderData、非ゼロ・整列・境界を検証。arenaは予約領域を除いたWB対応Conventional RAMを選択。両者のRP／RO能力による拒否が誤り。自前ページテーブルはDMAをRW／NX／UC、arenaをRW／NX／WBに設定し、PAT／MTRRを保持、EFER.NXE・CR0.WPを有効化し、切替時にcacheと変換をflushする。PE節権限とstack guardは別途維持。実アクセス権限は導入するページテーブルで決める。

修正範囲：両検証からRP／RO拒否を除去。Runtime拒否、arenaのWB条件、全範囲被覆、境界、type制限、予約領域除外・重複検証は維持。割当て・cache・ページテーブル実装は変更しない。Runtime拒否マスクを検証・診断で共有。属性flagsを維持し、説明とH7表示を更新。H6でMEMORY RESULTに上書きされた17行の拒否マスクを別行へ移す。Apple専用例外は設けず、firmware／runtime／予約領域やPEの書込・実行権限を受入れ変更しない。DMA cache能力条件は従来どおりで、この修正で拡大しない。

検証：実測0x2600F能力、分割被覆、arena予約領域の回帰テストと、type違い・隙間・Runtime拒否を確認。memoryテスト、複数controllerのUSB／FAT32／入力QEMU、RO／NX／guard例外試験を実施。テスト機能なし通常EFIを作成。Mac実機は再検証が必要で、QEMU成功からMac USB／DMA動作を保証しない。未コミット変更とブランチを維持し、この実装依頼ではpush・USB更新はしない。

## Validation result / 検証結果

10 memory tests passed. QEMU split-controller USB/FAT32/input and RO/NX/guard exception cases passed. Normal H7 build succeeded without features. Mac hardware confirmation is pending.

memory 10件、QEMU複数controllerのUSB／FAT32／入力、RO／NX／guard例外試験はすべて成功。通常H7ビルドはfeaturesなし。Mac実機確認は未実施。
