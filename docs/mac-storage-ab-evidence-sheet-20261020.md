# Mac storage A/B evidence sheet — 2026-10-20

## English

Goal 14 still needs a fixed-topology physical A/B comparison. The [capture procedure](mac-storage-t2-stages-0.4.0.md) is authoritative. Historical v0.2.0 A and later H19/IMG7650 read success are separate observations; the latter does not explain the former `No medium`/7D00 failure. This sheet inventories gaps without changing either original binary or log.

| Evidence | Visible or preserved | Missing for a causal A/B result |
|---|---|---|
| A binary | Owner-preserved v0.2.0 EFI described in the procedure | Exact file path, SHA-256 and same-run photo/boot identity |
| B binary | Development T2 diagnostic EFI path/hash/features in [local preparation](mac-local-preparation-0.4.0.md) | Same-run image identity and CONFIG hash; no assumption that the later BCE facade is in it |
| Topology | Prior records identify some controllers/ports | One photographed and unchanged cable/adapter/hub/physical port, same media/data for paired A/B runs |
| 7D00 | Earlier failure and later successful reads are retained separately | EARLY/PRE/POST EBS ID/BDF, raw BAR0/1, Command/Status, PMCSR, boot path/owner, bridge/resource and first divergent reason in the **same topology** |
| USB/storage outcome | H19/IMG7650 success establishes those individual read observations | Per-run FAT32 and separate exFAT filename/length/hash/read result, actual controller/root/slot/speed, explicit failure or recovery and repetition |
| T2/BCE | Host models and diagnostic records exist | Phase-complete physical resource evidence; [validator schema](t2-capture-completeness-20261020.md) marks omissions, not a substitute for capture |

For each paired run, record date/operator, machine model/firmware, A or B EFI path/hash/features, CONFIG bytes/hash, USB media/partition/filesystem/data hash, AC state, off interval ≥10 seconds, topology photos, 7D00 and T2 phase lines including every `OMIT`/`ERROR`, file read outcome and first difference. Preserve original images/video and note unreadable or absent fields **NOT CAPTURED**. If A succeeds and B fails in the same topology, isolate the first divergent phase; if both fail, keep media/topology unresolved. Do not infer a cause from a root-port number changed between historical photos. No USB write, format, raw media operation or hardware acceptance is performed by this sheet.

## 日本語

目標14の同一接続A/B実機比較は未完です。[採取手順](mac-storage-t2-stages-0.4.0.md)を基準にし、保存済みv0.2.0 A、後続H19/IMG7650の読取り成功、以前の`No medium`/7D00失敗を別証拠として保持します。後続成功から旧失敗原因の解決を推定しません。

各runで実施者/日時、Mac型番/firmware、A/B EFI原本path/hash/features、CONFIG、媒体とFAT32・別exFATのdata hash、AC/10秒以上off、同じcable/adapter/hub/物理portの写真、7D00の3 phase・BAR/Command/PMCSR/bridge/owner、OMIT/ERROR、実file読取り、最初の差を記録します。読めない値は**未採取**であり0ではありません。同一接続でA成功/B失敗なら最初の相違phase、両失敗なら媒体/接続の未確定を記録します。T2の欠落は[検証schema](t2-capture-completeness-20261020.md)で明示します。本書はUSB更新・format・実機受入を行いません。
