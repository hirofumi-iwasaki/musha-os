# BCE host model and shared input / 実機待ちの間の実装

Status: hardware-independent preparation for v0.2.0, 2026-10-10.
No BCE driver is activated by this change. T2-D1 remains the next hardware test.

## Implemented

- `musha-bce` is a `no_std`, allocation-free library with unsafe code forbidden.
  It has no PCI, MMIO, interrupt, device-memory or boot dependency.
- Explicit little-endian mailbox message, contiguous submission descriptor,
  queue-memory configuration and completion decoding, based on the pinned
  FreeBSD layouts. Scatter/gather and named queue registration are not implemented.
- A one-shot firmware protocol handshake. A wrong reply, deadline, backwards time,
  stopped clock (observation budget), or shutdown makes the instance terminal.
  No automatic retry/restart can accept a late reply from the previous attempt.
- One bounded SQ with 2–256 slots and one unused ring entry. Completion can be
  out of order, but retirement remains in submission order. QID, slot, flags,
  result status and returned byte count are checked. Local tickets include a
  sequence to reject stale CPU-side retirement requests.
- One-entry-at-a-time CQ cursor. It provides the clear/consumer-index ack plan and
  does not advance until the adapter reports successful acknowledgement.
- Stop/error/timeout quarantines metadata and denies retirement or new submissions.
  No error path grants permission to reuse an outstanding device buffer.
- `musha-input::BootKeyboardState` is the existing xHCI boot-report decoder moved
  into a transport-independent crate. `musha_xhci::keyboard::State` re-exports it,
  so the current runtime uses the shared implementation without changing callers.
  The boot application now routes external transitions through `Sources<2>` and
  detaches the source on every xHCI diagnostic return.

## Adapter contract and limits

These are host-side models, not a T2 firmware emulator or a working driver.
The scripted test peer supplies bytes and logical time; it does not model cache
coherency, interrupts, real DMA, USB enumeration or Apple firmware behavior.

A future hardware adapter must:

1. Validate actual BAR/resource ownership and allocate separately reserved DMA
   memory before using the structural wire encoders. An encodable address is not
   proof that it is mapped, safe, or accessible by T2.
2. Reserve an SQ slot before descriptor publication. Sync descriptor/data writes,
   then ring the doorbell. A publication failure must quarantine, not roll back
   potentially device-owned memory.
3. Take one coherent CQ entry at the cursor, validate it against the SQ, copy any
   payload into CPU-owned storage, clear its pending flag, sync that write, and
   ring the CQ consumer doorbell. Only after successful hardware ack may it
   advance the CQ model and retire the SQ ticket. Ack failure stops both models.
4. Keep DMA allocations reserved when a model is quarantined or dropped. The
   model owns only metadata: dropping it does not stop DMA. Reconstructing a new
   model is not permission to reuse a still-active hardware queue or mailbox.
5. Supply monotonic milliseconds and an appropriate observation budget. Every
   SQ poll/submit/complete observation counts against in-flight budgets; budget
   exhaustion is a conservative stop, not a calibrated physical timeout.
6. Preserve unrelated xHCI/NIC progress and provide the T2 periodic timestamp
   service. MSI routing versus polling, memory barriers and timestamp writes
   remain unimplemented until the hardware interface is established.

BCE completions carry an SQ index but no host generation. A duplicate arriving
before reuse is detected; an old completion with identical fields after full
index reuse is **not distinguishable** by this model. A local sequence cannot
solve that wire-protocol limitation. Correct CQ consumption/acknowledgement and
an established reset boundary are required; failure never triggers a blind retry.
Reserved flags and non-success statuses currently cause conservative quarantine.
This policy may need refinement after comparison with actual firmware behavior.

The shared input layer still supports the existing 8-byte HID boot report only.
It does not assume that the internal keyboard supports that report format.
`musha-input::sources::Sources` now provides external-first selection/fallback
over normalized usage transitions. Each connected source retains its own held
keys; disconnect releases the selected source and falls back to the next source.
Unchanged held keys do not emit duplicates. Generation handles reject events
from an older connection. Releases precede new presses during selection changes.
Adapters must keep inactive sources updated and explicitly disconnect on failure.
The selector is connected to the external-keyboard boot session (slot 0).
Slot 1 is reserved; actual T2 integration still requires the hardware adapter
and descriptor evidence.
Simultaneous aggregation, report-ID parsing and Touch Bar handling remain absent.
Runtime cleanup uses selector disconnect to release held keys; decoder
`release_all` remains an alternative for transport adapters.

## Test evidence

- 123 Rust host tests passed, including 16 BCE codec/scripted-peer cases and
  3 new shared-input cases. Existing xHCI tests continue to exercise the re-export.
- Golden wire bytes check field order and byte order, not just encode/decode round trips.
- Cases include delayed/absent/wrong/repeated mailbox replies, backwards/stalled
  clocks, bounded queues, out-of-order completion, 1,000 SQ submissions with wrap,
  CQ acknowledgement/wrap, invalid QID/index/length/status/flags, duplicate
  completion, late completion after timeout, shutdown with pending work, and
  stale CPU tickets. Quarantined entries remain retained.
- QEMU: repeated keyboard input with ring wrap, Shift+A and Esc, GPT/FAT32 read,
  and devices on a second xHCI passed.
- QEMU cooperative traffic and keyboard-disconnect cases passed, including
  ARP/ICMP, 257 UDP echoes, file rechecks, checksum checks and DMA shutdown.
- Both new libraries compile for `x86_64-unknown-uefi`; normal and T2-D1 boot
  builds are checked separately. No BCE library dependency was added to boot.
- USB image/bundle/release packaging tests passed (10 cases). The release
  license inventory now includes the BCE reference notices.

Local logs: `out/t2-model/validation/` (generated, not versioned).
CI runs workspace host tests, UEFI checks for both new libraries, and verifies
hashes of the retained FreeBSD sources. The QEMU checks above are local results;
no Apple T2 execution or internal-keyboard success is claimed.

## Next hardware-dependent step

Use the already prepared T2-D1 image and capture its model/PCI/BAR/PM/MSI pages.
Then bind the model to a resource-validated mailbox adapter, initially with a
single protocol exchange and explicit failure logging. Retain the existing
external keyboard and USB-file-read behavior throughout. Do not enable BCE DMA
until ownership, completion visibility, quiescence and notification requirements
have been established.

## 日本語要約

実機を使わずに検証できるBCE通信形式、送受信状態、キューの所有権管理と異常系を実装した。
既存キーボードの押下／解放処理もUSB hostから独立させ、外付け入力は同じ処理を利用する。
FreeBSD原本とライセンスは`third_party/freebsd-apple-bce`へ固定保存した。

異常時はキューを再利用しない。これはhost側の管理規則であり、実際のDMA停止を実現する
コードではない。古い完了通知をindex再利用後に識別できないwire上の制限も残る。
実機のBAR検証、割込み／polling選択、T2への通信、内蔵HID descriptor対応は後続段階。
現在のT2-D1 USBを更新する必要はなく、移動中の実機操作も不要。

## Step 3 follow-up: input source selection

共通入力層に入力元の優先選択と切断時の切替を追加した。外付けをslot 0、内蔵をslot 1
とすることで外付け優先にできる。入力形式は正規化したキー押下／解放イベントであり、
内蔵側に8-byte boot reportを強制しない。切替時のShift解放、同じキーの重複防止、
切断後に届く古いイベントの拒否、rollover中の切断をテストした。
実行時への組込みはT2アダプターと合わせて行う。今回も実機操作やUSB更新は不要。

追加後の検証: Rust workspace 127 tests passed、共通入力層のUEFI target check成功。
入力元選択はまだ起動処理へ接続していないため、今回の追加に対するQEMU再検証は行っていない。

## Step 4: regression verification (2026-10-10)

Added public-API integration tests exercising the decoder, source selector and a
simulated application's held-key set through 1,000 connect/disconnect cycles.
They check duplicate presses/unmatched releases, changes to inactive sources,
rollover followed by disconnect, stale session events, inactive-source removal,
and fallback with identical held modifiers/keys. These host tests run in the
existing workspace CI suite; they do not simulate a working T2 device.

Results on the step 4 tree:

- 129 Rust tests passed; formatting and diff checks passed.
- Both shared libraries passed the UEFI target check; normal and T2-D1 builds passed.
- Rebuilt QEMU debug image: `out/t2-model/step4-debug.img`, SHA-256
  `ace148a1bb8741a7f01efba67fd74f63e484c64ed64c8f02d328fd4c89d555ab`.
- Multi-controller GPT/FAT32 plus keyboard exit/ring-wrap regression passed.
  Added `--keyboard-wrap` to the corresponding existing CI invocation.
- Cooperative scenarios `traffic`, `input-idle`, `no-keyboard`,
  `keyboard-disconnect`, `link-down` all passed. The first four validate
  ARP/ICMP, 257 UDP echoes, file rechecks, ring wraps, checksums and DMA stop;
  link-down validates continued input/file progress and shutdown of both DMA users.
- USB image/bundle/release packaging: 10 tests passed.

QEMU checks the existing xHCI execution path using the extracted decoder.
Source selection/fallback is still host-test-only, not connected to boot, and
neither QEMU nor these tests establish internal-keyboard or real BCE behavior.
The qemu-debug build reports the existing unused `diagnostics::page` warning;
normal and T2-D1 builds succeeded without that warning. No physical USB changed.

項目4の実機不要の回帰検証を完了。新しい入力元切替はhost integration testで検証し、
既存の外付け入力・USB読出し・通信は再ビルドしたQEMU環境で確認した。
次の実機依存作業はT2-D1診断結果の採取であり、内蔵キーボード対応の完了は未確認。

## Runtime input integration (2026-10-10)

`boot/src/app.rs` now owns `Sources<2>` for its input session. Ready connects the
external source at priority slot 0. All decoded xHCI transitions pass through the
selector before the application's existing FIFO. After `xhci::diagnose` returns,
the runtime disconnects the source and enqueues releases for its held keys. This
covers Esc, diagnostic deadline, disconnect and hardware/application errors.
Input availability derives from the selector's active source. No T2 source is
connected yet; live internal/external fallback remains unverified.

The selector cleanup happens after xHCI's existing quiescence and changes only
CPU state. Successful/continuing application execution consumes the releases in
the existing tail loop. On application error the source is still cleared and
releases enqueued, but the failed application is not called again to consume them;
normal shutdown/error propagation continues. No new hardware access is added.

QEMU regression now holds Shift+A without automatic key-up before deleting the
keyboard and requires each release exactly once at both detach and application
boundaries. The Esc test requires its application release exactly once. The app
error scenario holds A and checks detach before final failure, alongside existing
DMA-stop checks. These assertions run in the existing CI scenarios.

Validation for this integration: 129 workspace host tests passed; QEMU
multi-controller/ring-wrap, Esc cleanup, cooperative traffic, input-idle,
no-keyboard, held-Shift+A disconnect and held-A application failure passed.
Normal and T2-D1 builds passed. The pre-existing unused `diagnostics::page`
warning remains in debug builds. Physical USB and T2 hardware were not modified.
