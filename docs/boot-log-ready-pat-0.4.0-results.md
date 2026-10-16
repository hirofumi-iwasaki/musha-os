# Ready-before-application and verified PAT WC — local results

Status: current. Local verification on 2026-10-16; physical acceptance remains pending.

## English

The ready milestone now follows required USB host, boot-media and keyboard discovery and confirmed device stop, before executing AUTORUN. A bounded file read may finish before this milestone. Viewer-only startup reports ready before entering the viewer. Ready remains a single startup milestone; application start, raw output, exit, fault and rejection use the same console at ready level. Reader failures identify the actual application and never enter Ring3. Quiet suppresses startup but preserves application output and context.

After ExitBootServices and before constructing OS page tables, PAT entry 1 becomes WC with one verified IA32_PAT write; the other seven entries remain unchanged. The BSP requires PAT0=WB, supported cache/MTRR state and a bounded audit proving no active firmware mapping/page-walk uses entry 1. Uninspectable tables, cycles, unsupported paging and exhausted bounds are rejected. Cache-disable/WBINVD/global TLB invalidation and temporary MTRR disable protect the change. Readback mismatch halts cacheoff without rollback or normal logging. MTRR restoration is separately verified. Installed GOP leaves must be index 1/RW/NX; both DMA regions remain UC/RW/NX before publishing the cache snapshot. AP startup is outside this BSP-only protocol.

Final QEMU values: PAT `0007040600070406` → requested/readback `0007040600070106`, PAT writes 1, MTRR restored, firmware tables 1028, GOP pages 4096, installed leaves verified. Physical effective WC, firmware compatibility and rendering speed remain untested. See [boot-log specification](boot-log.md).

Validation: 438 host tests passed, including 26 memory/PAT tests. Final QEMU coverage is **21 configurations**: startup five levels plus F8/8 (7), held real application five levels plus exit42/fault/loader rejection/reader rejection/normal-debug0 (10), and runtime r1/r2/fault/exFAT return (4). The return cases execute 10 applications in total, including three-launch repetition and r1/r2 DHCP UDP echo; those executions are not added to configuration count. Independent whole-frame glyph comparison passed 8 captures (7 startup plus normal real-app ready). The normal application screen contains, in order: ready, AUTORUN start, Hello, exit status 0. Fault retains `error=n/a`; rejected applications never enter Ring3. Runtime source hashes match the final host/startup snapshot.

Evidence: `out/ready-pat/verification.json`, `out/pat-entry1-final-v2/verification.json`, and child captures/reports. Feature-free normal EFI SHA256: `983fea8b50fbe5ea5bd7f5d3bf08d6e27b67cfc32024fba2a9cfc4917cac63f0`. Screenshot: `out/ready-pat/hello-normal/hello-fat32/screen.png`.

Excluded attempts are retained: an initial ready checkpoint before the final PAT0 guard; a return test mistakenly built with qemu-debug alone (diagnostic app instead of viewer); and an exFAT selected-name flag rejected before QEMU because that flag supports FAT32 only. Correct external-ELF and exFAT repeat fixtures passed without production changes. The former broad 296-case matrix was not repeated; this run covers affected paging, startup, application display and reuse paths. Existing development EFIs and Parallels images were not regenerated or relabeled. No push, USB access, remote CI, tag or Release occurred. Public ABI, SYS_LOG bounds, image packing and pixel values are unchanged.

## 日本語

ready は必須 USB host・起動媒体・keyboard の探索と device 停止確認後、AUTORUN 実行前に出す。bounded file read の完了が先になる場合がある。viewer のみの起動も viewer 開始前に ready を出す。ready は一度だけの起動 milestone とし、アプリ開始・生出力・終了・fault・拒否は ready level でも同じ console に表示する。読取り拒否は実アプリ名を示し Ring3 へ入らない。quiet は起動を抑制するがアプリ出力と文脈は保持する。

ExitBootServices 後、OS page table 構築前に BSP が PAT entry 1 を WC へ変更し、IA32_PAT は他の 7 entry を保持して 1 回だけ書く。PAT0=WB、対応 cache/MTRR 状態、active firmware mapping/page-walk が entry 1 を使わないことを bounded audit で確認する。監査不能 table・cycle・未対応 paging・上限超過は拒否する。cache-disable/WBINVD/global TLB invalidation と MTRR 一時無効化で保護し、PAT 読戻し不一致は cacheoff のまま rollback や通常ログへ戻らず停止する。MTRR 復元も別途検証する。導入済 GOP 全 leaf の index 1/RW/NX と両 DMA 領域の UC/RW/NX を確認してから snapshot を公開する。AP 起動は BSP 専用の今回 protocol の対象外である。

最終 QEMU 実値は PAT `0007040600070406` → 要求／読戻し `0007040600070106`、PAT 書込み 1 回、MTRR 復元、firmware table 1028、GOP page 4096、導入済 leaf 検証済み。実機の実効 WC・firmware 適合・描画速度は未確認である。[boot-log 仕様](boot-log.md)を参照。

host は memory/PAT 26 件を含む 438 件合格。最終 QEMU は **21 構成**：起動 5 level と F8/8 の 7、実アプリ保持画面 5 level と exit42/fault/loader 拒否/reader 拒否/normal debug0 の 10、r1/r2/fault/exFAT 復帰の 4。復帰は計 10 回のアプリ実行（三連続と r1/r2 DHCP UDP echo を含む）であり、構成数に重複加算しない。独立 glyph 描画による全画素比較は起動 7 と通常実アプリ ready の計 8 画面が合格。通常画面で ready→AUTORUN 開始→Hello→status 0 終了を確認した。fault は error=n/a、拒否は Ring3 非入場を確認。最終 host/startup snapshot と runtime source hash は一致する。

証拠は `out/ready-pat/verification.json` と `out/pat-entry1-final-v2/verification.json`、各 child report/capture。通常 feature-free EFI SHA256 は `983fea8b50fbe5ea5bd7f5d3bf08d6e27b67cfc32024fba2a9cfc4917cac63f0`。画面は `out/ready-pat/hello-normal/hello-fat32/screen.png`。

集計外の試行も保存した：最終 PAT0 guard 前の ready 初回、qemu-debug 単独で viewer の代わりに診断 app を起動した回復試験、FAT32 専用 selected-name flag を exFAT に指定して QEMU 前に拒否された試行。正しい external-ELF/exFAT 反復 fixture で production 変更なく合格した。従来 296 構成全回帰は繰り返さず、今回影響する paging・起動・アプリ表示・再利用を検証した。既存開発 EFI／Parallels image は再生成・来歴変更していない。push・USB・remote CI・tag・Release は行っていない。公開 ABI、SYS_LOG 上限、image packing、pixel 値は不変である。
