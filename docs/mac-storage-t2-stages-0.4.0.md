# Mac storage and T2 evidence stages / Macストレージ・T2段階

Status: current (implementation prerequisites, 2026-10-15). No new hardware result.

## English

Goal14 storage and goal15 internal input remain separate physical gates. Existing H19/IMG7650 success does not explain older No medium failures. This document prepares the next implementation request; it activates no device and does not update USB.

### Storage: same-connection A/B

1. Owner identifies MacBook Pro2018 exact model/firmware, monitor, USB media/partition/filesystem and keyboard. Photograph the full cable/adapter/hub/physical-port chain and label each end. Keep media, data files and connection fixed throughout A/B; changing root-port numbers between earlier photos was not same-topology evidence.
2. A is the retained v0.2.0 EFI; B is the precisely identified development EFI. Record source, commit/features, SHA-256 and CONFIG for each. Owner separately preserves/copies only the intended EFI; no formatting/raw writing is proposed. Record boot selection, cold-off interval ≥10 s and AC state. Capture FAT32 read with filename/length/hash; run a separately identified exFAT data-media check with its own unchanged topology and record. A firmware boot or GOP image alone does not prove OS storage read.
3. Photograph EARLY/PRE EBS/POST EBS PCI pages for boot owner **7D00**, ID/BDF, raw BAR0/1, Command/Status, PMCSR; boot-device path/BOOT XHCI, resource/bridge/PEER BOUND/recovery reason, PCI RECOVERED or rejection, controller/root/slot/speed, USB READ/FAT32 or exFAT result and application/viewer. Include omission/error pages, not only final success. Record exact page/photo timestamps and EFI used.
4. Compare A/B observations before changing code. If B fails while A reads with identical topology, classify first differing phase; if both fail, report topology/media uncertainty. Keep non-boot-owner0700 skip and resource checks intact. Completion is repeatable FAT32/exFAT read evidence **or a bounded cause/limitation report**, not a claim that every Mac USB port works.

### T2: registration → queue → HID, each with a stop gate

| Stage | Local preparation possible now | Physical evidence required before next stage |
|---|---|---|
| T0 inventory | Review [research](history/macbook-t2-keyboard-0.2.0-research.md), [host contract](history/t2-host-model.md), [registration model](history/bce-registration-memory.md), resource-check tests | [T2-D1](history/t2-d1-diagnostics.md): model/PCI, BCE106b:1801, BAR2/4, size/resource/bridge ownership, PM/MSI and EBS changes. An address alone is not safe MMIO permission. |
| T1 mailbox/time | Plan bounded adapter, monotonic clock/timestamp scheduling, wrong/late reply stop and logging | Validated BAR2/4 access, protocol0x20001 exchange and periodic timestamp service without stopping USB/input. Decide MSI vs demonstrably complete polling; mailbox polling alone proves no DMA polling scheme. |
| T2 registration | Test encoders, contiguous memory/alignment/bounds, ack/quarantine, stale completion/reset boundary | Dedicated reserved coherent DMA and actual command transport register/ack of each queue, observed QID/index/length/status; no queue submission until registration is confirmed. |
| T3 queues/stop | Host queue wrap, delayed/out-of-order completion, ack failure, timeout and shutdown tests | Completion visibility, coherent copy/clear/consumer doorbell, ownership retirement and actual quiescence; retain DMA on uncertain stop. Model drop or local generation does not stop hardware or distinguish every reused-index completion. |
| T4 VHCI/HID | Descriptor parser bounds/report-ID/source fallback tests; design EP0 and keyboard interrupt-IN adapter | Actual ports/device/config/HID report descriptors via BCE VHCI; validate report shape before decoding. No assumption of 8-byte boot reports. |
| T5 input acceptance | Route normalized events through existing shared source selector; plan disconnect cleanup | Internal A/Shift+A/press/release and viewer navigation with external keyboard removed; external-keyboard regression/fallback, no duplicate/stuck modifiers, USB file read retained. Touch Bar Esc is separate; retain external Esc/timeout during development. |

The host model is not connected to BCE MMIO/DMA/interrupts or live internal HID. Ordinary QEMU xHCI results cannot replace T0–T5. Next code request should name its stage, exact observed resources, transport/notification decision, DMA ownership/stop proof, negative tests and preserved external path. Until T0 evidence is received, resource/adapter designs and host tests can progress; enabling mailbox/DMA or claiming internal input cannot.

Record per run: date/operator, exact EFI/hash/features/CONFIG, model/firmware, connection photo, filesystem/data hash, cold seconds/AC, all pages, first divergence/error, PASS/FAIL/BLOCKED and next-stage decision. Preserve original evidence; add new observations separately.

## 日本語

目標14のstorageと15の内蔵入力は別の実機gateです。H19/IMG7650成功から過去No mediumの原因解決を推定しません。本書は実装依頼の前提整理で、device有効化・USB更新はしません。

### storage：同一接続A/B

1. MacBook Pro2018の正確なmodel/firmware、monitor、USB媒体/partition/filesystem、keyboardを識別。cable/adapter/hub/物理port全経路を撮影し両端を識別。A/B間で媒体・data・接続を固定。過去写真のroot port番号変更は同一接続反復の証拠ではありません。
2. Aは保存したv0.2.0 EFI、Bは識別した開発EFI。出所/commit/features/SHA-256/CONFIGを記録。所有者が元EFIを保存し対象EFIのみ別途copyし、format/raw writeしません。boot選択、off ≥10秒、AC状態を記録。FAT32 file名/長さ/hash/読出しを採取。exFAT data媒体は別の固定接続runとして同様に記録。firmware起動/GOPだけではOS読出しの証拠になりません。
3. 起動担当**7D00**のEARLY/PRE EBS/POST EBSのID/BDF、raw BAR0/1、Command/Status、PMCSR、boot device path/BOOT XHCI、resource/bridge/PEER BOUND/復旧理由、PCI RECOVERED又は拒否を撮影。controller/root/slot/speed、USB READ/FAT32又はexFAT、app/viewerも採取。省略/errorページも保存し、写真時刻とEFIを対応させます。
4. コード変更前に比較。同一接続でA成功/B失敗なら最初の相違phaseを絞り、両方失敗なら接続/媒体未確定を記録。非boot-owner0700 skipとresource検査は維持。完了はFAT32/exFAT読出しの反復証拠又は**限定した原因/制約の記録**で、Mac全port対応ではありません。

### T2：登録→queue→HIDの実機確認段階

| 段階 | ローカル準備可能 | 次段階前に必要な実機証拠 |
|---|---|---|
| T0 inventory | [調査](history/macbook-t2-keyboard-0.2.0-research.md)、[host契約](history/t2-host-model.md)、[登録model](history/bce-registration-memory.md)、resource試験の確認 | [T2-D1](history/t2-d1-diagnostics.md) model/PCI/BCE106b:1801、BAR2/4のsize/resource/bridge所有権、PM/MSI、EBS変化。アドレス表示だけではMMIO許可にならない。 |
| T1 mailbox/時刻 | 有界adapter/単調時計/定期timestamp、異常・遅延reply停止設計 | 検証済BAR、protocol0x20001交換、USB/inputを止めないtimestamp。MSI又は全通知を回収できるpollingの根拠。mailbox pollだけでDMA pollを証明しない。 |
| T2 登録 | encoder/連続memory/alignment/範囲/ack/quarantine/stale/reset試験 | 専用予約coherent DMA、実command transportでqueue登録とack、QID/index/長さ/status。登録確認前にsubmitしない。 |
| T3 queue/停止 | wrap/遅延/順不同完了/ack失敗/timeout/終了試験 | 完了可視性、copy/clear/consumer doorbell、所有権退役、実quiescence。不確実停止はDMA保持。model drop/世代番号でdevice停止や全index再利用問題を解決した扱いにしない。 |
| T4 VHCI/HID | descriptor/report-IDの有界parser/source fallback、EP0/interrupt-IN設計 | BCE VHCI経由の実port/device/config/HID report descriptor。8-byte boot reportを仮定しない。 |
| T5 入力受入 | 正規化eventを既存source selectorへ接続する設計、切断cleanup | 外付け無しで内蔵A/Shift+A/押下/解放/viewer操作。外付け回帰/fallback、重複/stuck無し、USB読出し維持。Touch Bar Escは別、開発中は外付けEsc/timeoutを確保。 |

host modelは実BCE MMIO/DMA/割込み・内蔵HIDへ未接続。通常QEMU xHCIでT0–T5を代用しません。次の実装依頼には段階・実resource・通知方式・DMA所有権/停止根拠・異常試験・外付け経路維持を明記。T0証拠待ちでも設計/host試験は可能ですが、mailbox/DMA有効化や内蔵入力成功の主張はできません。

run毎に日時/実施者、EFI/hash/features/CONFIG、model/firmware、接続写真、filesystem/data hash、冷間秒数/AC、全page、最初の相違/error、PASS/FAIL/BLOCKED/次段階判断を記録。原証拠を保持し新結果は追記で分離します。
