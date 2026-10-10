# ACPI timer discovery diagnostics / ACPIタイマー検出診断

## English

H7 IMG_7448 confirms memory initialization passed; APP FAILED follows missing PM timer before USB work. Discovery currently returns None for every error, including table mapping/type/RP policy, checksums, root layout, absent/duplicate FADT and unsupported timer fields. Do not infer PM timer absence from None.

H8 first captures bounded scalar discovery evidence while firmware mappings exist: checkpoint, candidate address/size, overlapping map record type/attributes, FADT length/revision/flags, legacy PM_TMR address/length, extended GAS space/width/offset/access/address. Retain no firmware pointers or slices in runtime. Display copied evidence when timer is unavailable; preserve existing checks and stop before USB without calibrated time. In particular ACPI bytes() has an RP-bit check while using firmware mappings; do not remove it based solely on the post-ExitBootServices RAM fix. An unrelated root child header can also terminate discovery: expose stage and candidate before deciding fallback/skip policy.

Sequence: instrument discovery and initial-map acquisition; validate normal QEMU timer discovery and build normal H8; capture Mac failed stage. Then fix measured parser/readability condition, or separately design HPET/invariant-TSC fallback if PM timer really absent. No guessed I/O ports, uncalibrated delays, checksum bypass, or premature alternative timer. USB update is separate.

## 日本語

H7 IMG_7448でメモリー初期化通過、USB処理前のPMタイマー取得失敗に伴うAPP FAILEDを確認。現状はマップtype／RP条件、checksum、root構造、FADT不在／重複、非対応タイマーをすべてNoneへまとめており、タイマー不在とは断定できない。

H8でfirmware有効中に固定長の値だけを保存：段階、対象アドレス／長さ、重なるmap記述子type／属性、FADT長さ／revision／flags、従来PM_TMRアドレス／長さ、拡張GAS space／width／offset／access／address。firmwareのポインターやsliceはruntimeへ保持しない。タイマー取得不可の画面に値を表示し、検証条件を維持、校正された時間がない状態でUSB処理へ進まない。bytes()のRP検証はfirmwareページテーブル使用中なので、ExitBootServices後のRAM修正を理由に無条件除去しない。無関係なroot子テーブルのheader失敗でも終了するため、段階と対象を表示してからfallback／skip方針を決める。

順序：検出と初期map取得を診断、通常QEMUと通常H8ビルドを確認、Mac停止段階を採取。その後に実測された解析／読取条件を修正。PMタイマーが本当に無ければHPET／invariant TSC代替を別途設計する。I/Oポート推測、未校正delay、checksum無効化は行わない。USB更新は別作業。

## Validation / 検証

Platform tests: 13 passed. QEMU normal PM timer, USB/FAT32/input smoke passed. Normal H8 features=[] build passed. Mac failure path needs the next physical test.

platformテスト13件、QEMU通常PMタイマー・USB／FAT32／入力の検証は成功。通常H8はfeatures=[]でビルド成功。Mac失敗経路は次の実機試験で確認する。
