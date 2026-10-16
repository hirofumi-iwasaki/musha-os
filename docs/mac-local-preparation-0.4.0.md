# Macストレージ・T2 ローカル準備記録 v0.4.0

Status: current（2026-10-15、ホストモデルのみ。実機採取待ち）。

## Stage 0 local completion — English (2026-10-16)

The current host composition is `registered::RegisteredTransfer<N>` in `bce/src/registered.rs`. It owns the existing CQ/SQ registrations, transfer model, declared contiguous extent and one pending transfer token. CQ acknowledgement precedes SQ registration; both acknowledgements precede submission. Queue count/direction/stride/CQ identity, complete queue extents and CQ/SQ/payload non-overlap are checked before publication or ownership reservation. One monotonic clock spans registration and transfer. Command publication failure, registration/CQ acknowledgement failure, timeout, exhausted polling or shutdown quarantines both queues and retains backing; later calls perform no further transfer I/O. Flush/unregister remain separate existing lifecycle models, not hardware drain or release permission.

This composes existing models rather than adding another mailbox/queue implementation. It is host-only, absent from boot dependencies and hardware dispatch. The `Transport` contract assumes an established command transport and validated/acknowledged command completion-to-token mapping; it does not emulate BCE firmware. The wrapper allows one pending transfer; true out-of-order ownership tests remain in the existing queue model. Reconstructing the same QIDs still restarts local tokens: the explicit limitation test confirms collision rather than claiming a reset barrier. No reset/reclaim API, allocation/coherency proof or real notification processing was added.

Current focused result: **47 host tests passed**, comprising unit1 + adapter20 + registration9 + virtual-T2 13 + wire4 (doc-tests0). This includes the previous39 and eight composition tests; do not sum the checkpoints. Tests cover CQ-before-SQ admission, pre-I/O extent/overlap rejection, publication/status errors, failed CQ acknowledgement retaining one owned slot, thirteen sequential transfers across ring wrap, delayed completion, timeout/shutdown, shared-clock/poll/deadline failure and reconstruction limits. Exact command, source hashes and log: `out/stage0-mac-preparation/verification.json`, `source-before.json`, `source-after.json`, `bce-host.log`. Host T1–T3 preparation is complete within these stated contracts; physical transport, DMA, interrupt/poll visibility and internal HID remain pending.

The new trace/PAT **development-only** T2/Realtek EFI is `/Users/hirofumiiwasaki/Github/musha-os/out/stage0-ready-pat/development/t2-realtek/files/EFI/BOOT/BOOTX64.EFI`, SHA-256 `8b3fe1ec3bfc2aaf93c644da7f61e1da956d0b646637fb49c816ff7b1fd419fb`. It was built at `77184002b07acbcc200520ef8487d2c387c724da`, features `t2-diagnostics,diagnostics-detail`, CONFIG bytes `log=trace\r\ndisplay=max\r\n`, CONFIG SHA-256 `a7f626b86244b06fba2e74c04f0bad3b60aa1d81d6ce5a06b966bc5eeb865b27`. Exact provenance: `out/stage0-ready-pat/development/registry.json`. The later host-only BCE facade is not linked into this EFI. This identity supersedes neither the preserved older development binary below nor the owner's saved v0.2.0 A binary. No physical USB/run occurred.

Capture uses the current [trace procedure](mac-storage-t2-stages-0.4.0.md), not old rotating pages. Full T2 capture is not guaranteed: scrolling and later viewer ownership can hide early records; no physical serial/file mirror or log-history UI is promised. Missing observations remain a T0 evidence gap requiring separate capture assistance, not presumed success.

## 段階0のローカル仕上げ — 日本語（2026-10-16）

`registered::RegisteredTransfer<N>` が既存CQ/SQ登録・転送model・宣言した連続領域・1件の保留tokenをまとめて所有します。CQ ack後にSQを登録し、両方のack前には提出しません。count/方向/stride/CQ ID、queue全範囲、CQ/SQ/payloadの重なりをI/O・所有権予約前に拒否し、登録から転送まで単一の単調時計を検査します。command公開失敗、登録/CQ ack失敗、timeout、poll上限、shutdownでは両queueをquarantineし、以後の転送I/Oと再利用を拒否します。flush/unregisterは既存の別modelに残り、実DMA停止・解放許可ではありません。

別mailbox/queueを作り直したのではなく、既存modelの統合境界です。boot依存・実hardware dispatchには接続していません。command transport成立と、検証・ack済みcompletionからlocal tokenへの対応は呼出し側の契約で、BCE firmwareの再現ではありません。同時保留は1件で、順不同完了は既存queue試験が担当します。同QIDの生成し直しではtokenが再一致し得る制約も保持し、reset/reclaim APIや実連続性/coherencyの証明は追加しません。

ホスト **47件成功**（unit1、adapter20、registration9、virtual-T2 13、wire4、doc-test0）。既存39件＋統合8件であり過去checkpointと合算しません。上記境界、13回提出のring wrap、遅延、ack失敗、期限・poll・時計・shutdown、再生成制約を検査しました。証跡は上記 `out/stage0-mac-preparation/`。この契約内のT1–T3ホスト準備は完了ですが、実command transport/DMA/通知可視性と内蔵HIDは実機待ちです。

最新開発用EFIのpath/hash/features/CONFIGは英語節とregistryの通りで、通常配布候補ではありません。後から追加したBCE host facadeはこのEFIに含まれません。下記の旧開発EFIと所有者保存のv0.2.0 A原本は来歴を保持します。USB書込み・実機runはしていません。現行Traceでは旧自動ページを待たず逐次ログを採取しますが、scroll/viewerによる欠落があり全情報の撮影を保証しません。欠落はT0証拠不足とし、別の採取補助が必要な残件として扱います。

## Earlier local checkpoint — 2026-10-15 / 以前の記録

## 今回の実装と既存機能

既存の `bce/` を確認し、mailbox、登録、queue、HIDの別モデルは作り直していません。今回追加した `wire::ContiguousRegion` は、宣言した連続領域の非ゼロ長・加算上限・2の累乗alignment・領域全体の包含を検査します。queue登録encoderでは `count × stride` の全範囲を検査し、`Transfer::submit_in_region` は範囲外の提出をI/O・所有権予約より前に拒否します。実際の物理連続性、予約済memory、DMA coherencyをこの型で証明することはできません。

| 段階 | 対応コードとホスト試験 | 実機前の残課題 |
|---|---|---|
| T0 | 既存resource/PCI観測の契約確認 | T2-D1の正確なmodel、BAR2/4、size/resource/bridge、PM/MSI、EBS変化 |
| T1 | `adapter::Mailbox`。1回のpollでreply最大1件・timestamp最大1回、150 ms周期、単調時計、期限/poll上限、異常/重複/遅延replyの停止 | 実BARアクセス許可、protocol交換、MSI又は全通知を処理するpoll方式 |
| T2 | `wire` golden encoder、今回の領域包含検査、`registration::Registration` の登録ack前submit禁止・sticky quarantine | 専用coherent DMAの予約、command transportと実ackの対応、CQ依存の登録順 |
| T3 | `queue` の順不同完了と提出順退役、ring wrap、`adapter` のcopy→clear→sync→doorbell→退役、各I/O失敗・遅延・timeout・shutdown | 実可視性/doorbell、停止の観測、確実なquiescenceまでDMA保持 |

既存の `Transfer::submit` と `wire::register_queue` は構造検査のみの低水準モデルAPIとして残ります。統合側では領域付きの検査を通し、登録状態を確認してから提出する必要があります。今回もbootへBCE modelを接続していません。

## stale completionとresetの境界

`recreated_metadata_is_not_a_reset_or_stale_reply_barrier` は、同じQIDで `Registration::new` を呼び直すとlocal tokenが再び一致し得ることを意図的に確認します。生成し直すだけで古いreplyを排除できるとは扱いません。既存queueのwire completionにも世代番号はありません。index再利用後の同一wire replyをlocal sequenceだけで識別できません。

同じオブジェクトでは異常・停止後の古いreplyを拒否し、quarantineが解除されません。実機adapterで再初期化を認めるには、device停止、残通知排出、DMA所有権の終了を別途観測する必要があります。今回reset/reclaim許可APIは追加していません。flush/unregisterの成功、modelのdrop、local tokenの増加はいずれも実DMA停止の証明ではありません。

## ホスト検証

2026-10-15、`aarch64-apple-darwin`、独立出力先 `/private/tmp/musha-mac-round3-host`。`musha-bce` は39試験成功（unit 1、adapter 12、registration 9、virtual T2 13、wire 4、doc-test 0）。既存36試験と新規3試験です。ack失敗・順不同完了・遅延・shutdownの検証は既存試験を含みます。ログとソースhashは `out/mac-local-preparation-040-round3/bce-host-tests.log` と `bce-host-evidence.json` に保存しました。これは実BCE firmware、MMIO、DMA、割込み、内蔵keyboardの成功結果ではありません。QEMU xHCIの結果もT2-D1の代用になりません。

## storage A/Bと開発EFI

Bの診断用開発EFIを作成しました（通常配布候補ではありません）。

- path: `/Users/hirofumiiwasaki/Github/musha-os/out/mac-local-preparation-040-round3/development-t2-diagnostics/files/EFI/BOOT/BOOTX64.EFI`
- SHA-256: `59a701b1cbfe4797114a1d8eb02d8e59c12ecdc1ac53549812277de240cd5322`
- features: `t2-diagnostics` のみ。target: `x86_64-unknown-uefi`、release/locked build。
- commit: `fdcd0369fef577e772c5d52b780625673dcf9faa`、branch: `release/0.4.0`。未commit変更込み。
- compile-source set SHA-256: `18e147cc41c782e55ce6b382371039928ae18ae23a582074a27acc769b23aebe`。build前後一致。
- 証跡: `out/mac-local-preparation-040-round3/efi-source-before.json`、`efi-source-after.json`、`source-dirty.patch`、`source-status.txt`、`efi-build.log`、`development-t2-diagnostics/build-manifest.json`。
- 生成bundleの `SETTINGS/` は空で、CONFIGは含みません。実機runで媒体上の既存CONFIG又は未配置を必ず別記録し、A/Bで同じ値にします。

この診断EFIはread-only T2-D1観測に加え、資源/UC/RO検証後のRealtek TxConfig診断読出しを含みます。BCE MMIO/DMA有効化はありません。compile-source hashはRust/C/header/assemblyとCargo設定の集合であり、配布文書のhashを同一扱いにしません。package成果物hashはbuild manifestへ記録しています。Aの保存済みv0.2.0 EFIは所有者が保持する原本を使い、そのpath/hashを採取時に記録してください。既存の通常配布候補からAの実機使用履歴を推定しません。

本作業で物理USBへの書込みはしません。所有者がA/B各EFIを識別し、[採取段階](mac-storage-t2-stages-0.4.0.md)に従い同じ接続でFAT32/exFATの読出し結果と全診断pageを保存します。実BCE MMIO/DMAの有効化はT2-D1待ちです。
