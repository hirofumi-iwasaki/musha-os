# v0.3.0 open issues

## English

Owner decision (2026-10-14): the automatic boot filename is `AUTORUN.ELF`; the previous `MUSHA.ELF` is not accepted. This implementation choice does not change the ABI, syscalls, NOTE or SDK-generated ELF. When AUTORUN.ELF is present, the viewer does not start.

Current acceptance basis (2026-10-14): IMG_7633 shows one cold-start NO MEDIUM with PRE OWN D8=0; IMG_7634 shows one Hello/exit 0 with D0=0x7FF/D8=0xF already routed. Operations after run 1 and power-off durations for runs 2/3 are UNKNOWN; the owner approved this correction. Three consecutive cold-start successes are not proven. Earlier success counts below remain historical owner reports, not acceptance of the next candidate. See [hardware acceptance](hardware-acceptance-0.3.0.md).

Historical 2026-10-14 report: normal EFI 5e4af31e passed owner-reported NUC5 cold 3/3 and Ryzen cold 3/3 / hot 1/1. Ryzen startup is slow: IMG_7628 2.mov still shows discovery at 24 seconds and Hello/exit 0 by 26.307 seconds. Sequential controller attachment waits, including empty ports, are a likely contributor; stage timings are unmeasured. Track latency separately from functional acceptance. No EFI change made. These counts do not qualify the next candidate.

Owner decision, 2026-10-13: these are limitations, not work to investigate in this release-preparation task.

| Issue | Current evidence / next work |
|---|---|
| Mac storage | 0.3.0 development EFI: no supported FAT32/exFAT medium. Historical 10/12 rc1 FAT32 success differs; regression vs topology unknown. v0.4.0+: compare v0.2.0 EFI under identical connections and capture diagnostic builds. No investigation performed here. |
| NUC5 No medium | Cold NO MEDIUM observed in IMG_7633. Intel routing plus missing attachment grace identified by review; fixes implemented in 958c4ab, hardware recovery and the provisional 500 ms grace require candidate retesting. |
| H15 multiple keyboards | Selected Razer receiver prevents Lenovo input; multi-device polling remains future work. |
| Unoptimized debug vector 8 | Unresolved debug-profile fault; release-profile QEMU success does not close it. |
| I218/I219 hardware networking | Physical communication unverified; controller detection is not communication. |
| Rust guest | Compile-only layout/wrapper checks; no runnable Rust SDK, custom core/alloc or guest claim. Excluded from C SDK distribution. |
| Mac internal keyboard | T2 unimplemented; unsupported. |

Measured historical latency: IMG_7630 showed 17,996 ms of accumulated attachment/SS waits across six passes; ownership/reset/quiesce did not reach their deadlines. The current grace/rescan implementation requires new normal-candidate timing, not inference from diagnostic-page duration. See [timing evidence](usb-startup-timing.md).

Release gates still pending: same candidate EFI, NUC5/Ryzen three cold boots each and Ryzen one hot reboot with READ OK-to-Hello timing, Mac Basic Data boot, owner-authorized final-source remote CI and final artifacts. Documentation drafts have been received; older results do not satisfy the new candidate gates. [Hardware records](hardware-acceptance-0.3.0.md).

## 日本語

所有者決定（2026-10-14）：自動実行名は `AUTORUN.ELF` とし、旧 `MUSHA.ELF` は受理しません。実装上の選択でありABI・syscall・NOTE・SDK生成ELFは変更しません。AUTORUN.ELFがあるとビューアーは起動しません。

現在の受入判定（2026-10-14）：IMG_7633で冷間NO MEDIUMを1回（PRE OWN D8=0）、IMG_7634で切替済みD0=0x7FF/D8=0xFのHello・終了0を1回確認。run 1後の操作とrun 2/3の電源断時間は所有者も正確には覚えておらずUNKNOWNです。所有者はこの訂正を承認済みです。冷間3回連続成功は未証明。以下の成功回数は過去の所有者報告として保持し、次候補の受入には転用しません。[実機受入記録](hardware-acceptance-0.3.0.md)を参照。

過去の2026-10-14報告：通常版5e4af31eでNUC5冷間3/3、Ryzen冷間3/3・ホット1/1成功（所有者申告）。Ryzenは起動が遅く、IMG_7628 2.movの24秒時点でも探索画面、26.307秒でHello・終了0。未接続ポートを含む制御器ごとの接続待ちの累積が有力要因だが、処理別の実測は未実施。機能受入とは別に起動時間を追跡する。EFI変更なし。これらの回数を次候補の受入へ転用しません。

2026-10-13所有者判断。以下は本作業で調査を始めず、制約・残件として保持します。

- Macストレージ：0.3.0開発版でFAT32/exFAT未認識。10/12 rc1成功との違いは退行か接続構成差か未特定。0.4.0以降に同一接続でv0.2.0 EFIとのA/Bと診断ビルド採取を行う。
- NUC5 No medium：IMG_7633で冷間NO MEDIUMを観測。レビューでIntelルーティング未切替＋接続猶予なしを原因として特定し958c4abで修正済み。実機での解消と暫定500ms猶予の妥当性は候補で再確認する。
- H15複数キーボード：Razer選択時のLenovo入力不可は未修正。
- 未最適化debugのvector 8：未解決。release構成の成功と区別する。
- I218/I219：実機通信未確認。制御器検出は通信成功ではない。
- Rust guest：型・ラッパーのcompile-only検査のみ。Rust実行SDK、core/alloc、guest実行は未実装でC SDK配布へ含めない。
- Mac内蔵キーボード：T2未実装のため未対応。

過去の実測：IMG_7630で6走査の接続・SS待ち合計17,996ms。所有権移行・リセット・停止は期限へ達していません。現行の猶予・再探索実装では新通常候補で再計測し、診断ページ表示時間から推測しません。[計測証拠](usb-startup-timing.md)を参照。

公開候補EFIによるNUC5・Ryzen各3回冷間起動とRyzenホット1回・READ OK→Hello秒数、MacのBasic Data起動、所有者承認後の最終ソースのリモートCI、最終配布物の確認は残件です。README／リリースノート草案は受領済み。旧版の結果を次候補の合格には転用しません。
