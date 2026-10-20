# Mac BCE hardware-adapter prerequisites — 2026-10-20

## English

This is the contract for a future thin adapter around the committed `musha-bce` host models. The model is absent from boot dependencies and has no real MMIO, DMA, interrupt or internal-keyboard path. The current baseline is 47 passing host tests (unit 1, adapter 20, registration 9, virtual-T2 13, wire 4) rerun on 2026-10-20. Existing negative tests are cited below; no new queue or mailbox implementation is introduced.

| State | Preconditions before transition | Ownership and failure rule |
|---|---|---|
| `Unobserved` → `ResourcesValidated` | Same boot's T2-D1 model, BCE `106b:1801` BDF and EARLY/PRE-EBS/POST-EBS identity; BAR2/4 **size, type, extent, bridge window and ownership**, PM/MSI state and unchanged admission across EBS. Raw BAR base alone fails. | No BAR access, BME or DMA before validation. Failure stays diagnostic only. |
| `ResourcesValidated` → `MailboxReady` | Verified BAR4 mapping, bounded read/write offsets, coherent timestamp clock, command-to-reply association, and a demonstrated way to service every required notification while USB/input continue. | `adapter::Mailbox` owns one handshake. Unexpected, duplicate or late reply, timeout, clock error or I/O failure is terminal. Its stop notification is not hardware quiescence. |
| `MailboxReady` → `QueuesRegistered` | Dedicated reserved, physically contiguous, aligned, coherent DMA with stable addresses; bounded CQ/SQ/payload extents without overlap; command transport maps actual acknowledged completion to the local `registration::Token`. Register CQ then SQ, wait for each ack. | `registered::RegisteredTransfer` owns queue metadata, one pending transfer and backing. Partial command publication, failed ack or malformed extent quarantines both queues and retains all backing. |
| `QueuesRegistered` → `TransferPending` → `QueuesRegistered` | Notification/poll visibility and copy→clear→sync→doorbell ordering proven for this controller. Validate completion QID/index/status/length and actual memory synchronization before retiring CPU ownership. | Delayed or out-of-order completion cannot free a slot early. Poll/deadline failure or stop retains backing and denies further transfer I/O. |
| Any active state → `Quarantined` → possible new boot/session | Observe device stop, all notifications/replies drained, DMA no longer possible, and bridge/IOMMU/PCI ownership released by a reviewed method. Preserve allocations until those facts are observed. | `flush`, `unregister`, model `drop`, local token increment, mailbox stop or BME-off alone does not authorize reuse. If stop is uncertain, leave memory pinned and the device disabled for the session. |

**Reset/recreation collision:** `Registration::new` with the same QID restarts local token values. A delayed old device reply can equal a new token; `bce/tests/registration.rs::recreated_metadata_is_not_a_reset_or_stale_reply_barrier` and `bce/tests/adapter.rs::integrated_reconstruction_still_does_not_prove_hardware_reset` intentionally demonstrate this limit. The adapter must forbid reusing QIDs, queue indices or DMA backing after fault/reconstruction until physical stop and reply drain are independently established. No host-only generation number can prove that barrier. The existing tests also cover late completion after timeout, failed CQ ack, ring wrap and retained backing.

The next T0 capture must show the three phase identities, BCE count and omissions/errors, raw and validated BAR2/4 resources with bridge windows, PM/MSI controls and the earliest change across EBS. Later T1–T3 captures must establish actual command/reply mapping, reserved coherent memory, notification visibility and observed stop. T4–T5 still require real VHCI/HID descriptor and internal key press/release evidence, with external input and storage maintained. QEMU with BCE count zero and scripted host tests cannot satisfy these physical gates.

## 日本語

これは既存`musha-bce` host modelを将来の薄い実機adapterへ接続する前提契約です。bootへは未依存で、実MMIO/DMA/割込み/内蔵キーボード経路はありません。現行47 host試験を2026-10-20に再実行し全件成功しました。新しいqueue/mailbox stackは追加していません。

上表の順に、T0のBAR2/4・bridge・EBS前後の資源所有を確認し、実command/reply対応と通知方式、専用連続coherent DMA、CQ→SQ ack、完了退役、実停止観測を満たす必要があります。raw BAR表示だけでMMIOを許可しません。登録失敗、遅延・重複reply、timeout、poll/時計異常、停止不明では両queueとbufferをquarantineして再使用しません。

| 状態 | 次へ進む実機前提 | 所有権・停止条件 |
|---|---|---|
| 資源未確認→確認済み | 3 phaseのBCE ID/BDF、BAR2/4の長さ/種別/bridge/所有、PM/MSI | 不足時は診断のみ。BARアクセス/BME/DMAなし |
| 資源確認→mailbox | 検証済BAR4、期限付きread/write、実commandとreplyの対応、通知回収 | 遅延・重複・異常reply/時計/I/O失敗はterminal。stop通知はDMA停止証明でない |
| mailbox→CQ/SQ登録 | 専用予約済み連続/coherent memory、範囲・非重複、実ack | CQ ack後にSQ。部分公開/ack失敗で両queueとbufferを保持 |
| 登録→転送→退役 | 完了の可視性とQID/index/status/長さ、copy→clear→sync→doorbell | 遅延完了は早期解放不可。期限超過/停止ならbuffer保持 |
| 異常→隔離→将来の再開始 | 実device停止・残通知排出・DMA終了・所有権解除の観測 | 観測不能なら同一sessionでQID/index/memoryを再利用しない |

同QIDでmodelを作り直すとlocal tokenは衝突し得ます。古い実device replyを新規replyから識別できないため、物理停止・通知排出・DMA終了を独立に観測するまではQID/index/背後memoryを再使用しません。`flush`、`unregister`、`drop`、BME-offだけでは停止証明になりません。必要なT0と後続の実機採取項目・host試験の対応は英語節の通りです。
