# v0.3.0 open issues

## English

Owner decision, 2026-10-13: these are limitations, not work to investigate in this release-preparation task.

| Issue | Current evidence / next work |
|---|---|
| Mac storage | 0.3.0 development EFI: no supported FAT32/exFAT medium. Historical 10/12 rc1 FAT32 success differs; regression vs topology unknown. v0.4.0+: compare v0.2.0 EFI under identical connections and capture diagnostic builds. No investigation performed here. |
| NUC5 No medium | Two observations on old development EFI; not reproduced on current GOP/Basic Data build. Preserve failure evidence and test candidate cold boots with fixed topology. |
| H15 multiple keyboards | Selected Razer receiver prevents Lenovo input; multi-device polling remains future work. |
| Unoptimized debug vector 8 | Unresolved debug-profile fault; release-profile QEMU success does not close it. |
| I218/I219 hardware networking | Physical communication unverified; controller detection is not communication. |
| Rust guest | Compile-only layout/wrapper checks; no runnable Rust SDK, custom core/alloc or guest claim. Excluded from C SDK distribution. |
| Mac internal keyboard | T2 unimplemented; unsupported. |

Release gates still pending: same candidate EFI, NUC5/Ryzen three cold boots each, Mac Basic Data boot, owner-authorized remote CI, supplied release-note/README drafts. Ryzen final-frame still-image supplement is pending. [Hardware records](hardware-acceptance-0.3.0.md).

## 日本語

2026-10-13所有者判断。以下は本作業で調査を始めず、制約・残件として保持します。

- Macストレージ：0.3.0開発版でFAT32/exFAT未認識。10/12 rc1成功との違いは退行か接続構成差か未特定。0.4.0以降に同一接続でv0.2.0 EFIとのA/Bと診断ビルド採取を行う。
- NUC5 No medium：旧dev EFIで2回観測、現行GOP／Basic Data版では未再現。公開候補の冷間起動で接続構成を固定して記録する。
- H15複数キーボード：Razer選択時のLenovo入力不可は未修正。
- 未最適化debugのvector 8：未解決。release構成の成功と区別する。
- I218/I219：実機通信未確認。制御器検出は通信成功ではない。
- Rust guest：型・ラッパーのcompile-only検査のみ。Rust実行SDK、core/alloc、guest実行は未実装でC SDK配布へ含めない。
- Mac内蔵キーボード：T2未実装のため未対応。

公開候補EFIによるNUC5・Ryzen各3回冷間起動、MacのBasic Data起動、所有者承認後のリモートCI、README／リリースノート草案の受領は残件です。Ryzen最終画面の静止画補足も未受領です。
