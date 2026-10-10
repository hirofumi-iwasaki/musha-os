# Mac early memory initialization diagnostics

## English

MacBook Pro 2018 booted the H2 USB image through the greeting, then stopped with MEMORY INVALID before USB discovery. This is a software validation result, not evidence of failed RAM. The precise failed condition is not yet known. Keep all memory protection and validation checks. Add stationary on-screen checkpoint, BootInfo map dimensions, image/table range, offending map descriptor where identifiable, reserved/DMA/MMIO range, PE and page-table stages. No early page rotation: the CPU must remain halted after failure. Capture the next screen before changing acceptance criteria.

No Apple-specific workaround or memory-map acceptance change is made without a measured failing descriptor. RAM initialization and USB enumeration are separate issues. Existing dirty work is retained; no USB rewriting or push is part of this change.

## 日本語

MacBook Pro 2018はH2で挨拶表示まで起動後、USB列挙より前にMEMORY INVALIDで停止した。ソフトウェアの検証結果であり、RAM故障を示すものではない。失敗条件はまだ不明。検証・メモリ保護条件を維持し、停止画面にチェック名、マップ寸法、image/table領域、判別できる不正マップ記述子、予約／DMA／MMIO領域、PE／ページテーブル段階を表示する。早期停止画面は自動切替せず、停止を維持する。次の画面で失敗条件を確認してから受入条件を修正する。

測定なしでApple専用回避処理やマップ検証緩和は行わない。USB列挙とは別件。既存変更を保持し、この改修ではUSB書換え・pushは行わない。

## Split DMA coverage correction

English: IMG_7428 identifies DMA LOADER COVERAGE as the failed checkpoint. Replace the single-descriptor requirement with bounded, order-independent coverage of the entire DMA range by adjacent LoaderData descriptors. Reject gaps, wrong types, read protection, read-only and runtime attributes; require nonzero aligned range below 4GiB. Preserve cache handling and DMA allocation. On failure show the first seven overlapping records and total count. Splitting is a hypothesis until hardware re-test; no Apple-specific type exceptions are introduced.

日本語：IMG_7428でDMA LOADER COVERAGEの失敗を確認した。単一記述子への包含条件を、隣接するLoaderData記述子による全範囲の被覆検証へ変更する。記述子順には依存せず、隙間・対象外type・read protection・read-only・runtimeを拒否し、0以外のページ整列した4GiB以下の領域を要求する。cache設定とDMA割当ては維持する。失敗時は重なる記述子の先頭7件と総件数を表示する。分割が実機原因かは再試験で確認し、Apple専用type例外は追加しない。

## H5 evidence and H6 next action

### English

IMG_7445 still stops at DMA LOADER COVERAGE. One overlapping LoaderData record is shown; splitting alone did not resolve this run. Do not transcribe the blurred hexadecimal attribute as confirmed evidence. UEFI GetMemoryMap describes RP/RO as protection capabilities, not necessarily active settings; the current rejection may be too strict. Runtime mapping semantics are separate and must not be discarded with RP/RO. Source: https://uefi.org/specs/UEFI/2.10/07_Services_Boot_Services.html#getmemorymap

H6 preserves acceptance, allocation and mappings. It shows USB/LAN owner, first rejected descriptor or gap address, exact rejected attribute mask, and decoded RT/RP/RO/XP/WB flags for the first five overlapping descriptors. Failure remains stationary before USB DMA starts. Next: if only RP/RO capability bits reject allocated LoaderData, review removal of those capability checks together with owned page-table permissions and cache policy; retain full LoaderData coverage, bounds, reservations, and runtime rejection. Check the same capability-versus-setting assumption in arena selection before implementing a correction. A runtime-marked allocation requires a separate investigation of allocation/map transitions rather than unconditional acceptance. No USB update is performed in this diagnostic change.

### 日本語

IMG_7445でもDMA LOADER COVERAGEで停止。重なるLoaderData記述子は1件で、分割対応だけでは改善しなかった。写真のぼやけた属性値は確定値として転記しない。UEFI GetMemoryMapのRP／ROは保護設定能力であり、現在の保護状態とは限らないため、現状の拒否条件が厳しすぎる可能性がある。runtimeの意味は別で、RP／ROとまとめて無視しない。仕様URLは英語節に記載。

H6は受入条件・割当て・マッピングを維持。USB／LANの所有者、最初の拒否記述子または隙間のアドレス、拒否属性マスク、先頭5記述子のRT／RP／RO／XP／WBを表示する。USB DMA開始前の静止した停止画面を維持。次はRP／RO能力ビットだけが原因なら、自前ページテーブルの権限・cache方針と併せて能力ビット拒否の除去を検討する。全範囲LoaderData、境界、予約領域、runtime拒否を維持し、arena選択にも同じ誤解がないか確認する。runtime付きなら割当て／マップ遷移を別途調査し、無条件受入はしない。この変更ではUSBは更新しない。

## H7 policy supersedes earlier RP/RO rejection

English: [Confirmed policy](memory-attribute-policy.md) supersedes the RP/RO rejection described above; Runtime and structural checks remain. H6 measured RT=0/RP=1/RO=1/XP=1/WB=1.

日本語：[確定方針](memory-attribute-policy.md)が上記のRP／RO拒否方針を更新する。Runtime・構造検証は維持。H6でRT=0／RP=1／RO=1／XP=1／WB=1を実測。
