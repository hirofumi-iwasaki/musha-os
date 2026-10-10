# HID parsing and simulated BCE I/O

Development status: 2026-10-10, release/0.2.0. Both libraries are hardware-independent.
The HID decoder is not selected by the boot runtime, which retains the tested boot
keyboard decoder. The BCE crate remains absent from boot's production dependencies.

## HID report descriptor subset

`musha-input::hid::Layout::parse` compiles a bounded layout. `Decoder` validates
whole input reports before emitting normalized Keyboard/Keypad usage transitions,
compatible with the existing `Sources` selector. This is original Rust code based
on [USB HID 1.11](https://www.usb.org/sites/default/files/hid1_11.pdf), sections
6.2.2 and 8. The specification PDF and its sample source are not redistributed.

Supported scope:

- Generic Desktop / Keyboard Application collections, nested collections, short
  items, local usage lists/ranges (including extended page-qualified usages),
  global Push/Pop, Report IDs and unaligned input fields.
- Keyboard page 7, usage values 0–255; unsigned absolute arrays with 1–8-bit
  values and explicit logical-to-usage mapping; 1-bit variable key bitmaps.
- Constants consume input bits. Output/Feature fields do not consume input bits.
  Other usage pages/collections are ignored for key events while input offsets
  and report lengths remain tracked.
- State is retained separately per input report ID and combined across IDs.
  One report cannot release a key still held by another ID. Duplicate array
  entries and repeated reports produce no duplicate key transitions.
- Report lengths must match the declared layout exactly. Unknown IDs, invalid
  lengths/values and usages 1–3 (error/rollover) emit nothing and preserve state.
  Explicit disconnect remains the adapter's responsibility; `release_all`
  clears all report states when appropriate.

Bounds: 4096 descriptor bytes, 8 input report IDs, 32 keyboard fields, 256 local
usages, 4 pushed global states, 8 collection levels, and 4096 input bits per ID
(512 data bytes, plus one ID byte when present). Allocation-free, no unsafe code.
The compiled layout stores fixed-capacity usage tables; a future boot adapter
must budget its memory rather than parsing on an interrupt stack.

This is a deliberately limited parser: signed/relative keyboard fields, null-state
arrays, non-keyboard events, long items, delimiters/designator/string locals,
usage-repeat rules for short variable lists, mixed-page keyboard fields, and
arbitrary HID semantics are unsupported. Arrays require a complete usage map
matching their logical range; variables require one usage per bit. Unsupported
encodings fail explicitly. Physical/unit globals are ignored because this subset
only produces key usages. Tests use synthetic descriptors, not Apple captures.
T2 framing, device enumeration, descriptor acquisition, report-protocol selection,
Touch Bar and vendor Fn semantics are not implemented.

## Simulated BCE adapter boundary

`musha-bce::adapter` orchestrates operations through `Registers` and `Memory`
traits. The only implementations added are test peers backed by ordinary memory
and operation traces; there is no PCI, MMIO or DMA adapter.

`Mailbox` checks for stale replies before starting, synchronizes timestamp state,
sends the initial-start opcode, and publishes protocol words in order. Polls are
nonblocking: at most one reply and one due timestamp update. Timestamp updates
continue while waiting for the protocol reply and after readiness, with a 150 ms
schedule. A late caller sends one current value rather than a catch-up loop;
this does not establish that firmware tolerates missed deadlines. Nanosecond
conversion and next-deadline arithmetic are checked. Caller scheduling and real
interrupt/coherency behavior remain unverified.

Wrong/multiple/late replies, backward time, poll-budget exhaustion and I/O errors
are terminal. Cleanup makes one best-effort timestamp-stop notification and
records a failed stop write. Repeated stop/poll does no further I/O. A stop
notification is not proof that the coprocessor or DMA has stopped.

`Transfer<N>` models an **already-registered** SQ/CQ with one request in flight.
It does not register queues or infer that protocol readiness authorizes DMA.
The ordering is reserve, write descriptor, sync, SQ doorbell; then acquire/copy
completion, validate, copy payload, clear CQ, sync, CQ doorbell, metadata ack,
and SQ retirement. Publication or completion failures quarantine the metadata
and call the peer's retain operation, including after partially successful writes.
No cleanup path releases device buffers. Poll observations are conservative
budgets; a completion call checks the SQ both before I/O and during validation.

The trait contract alone cannot guarantee address ownership, barriers, cache
coherency, DMA quiescence or allocation lifetime. Dropping a model/peer is not a
hardware reset. Production integration must establish those separately, plus
firmware queue registration, interrupts/polling, VHCI, cancellation and shutdown.
BCE's index-without-generation replay limitation still applies.

Mailbox/timestamp sequencing references pinned FreeBSD `apple_bce.c` and
`apple_bce_mailbox.c`; SQ/CQ ordering references `apple_bce_queue.c`. All four
reference files including the header are retained unmodified with hashes and
BSD-2-Clause notices under `third_party/freebsd-apple-bce`. Rust adaptation keeps
that license; the test-only dependency on `musha-input` does not link BCE to boot.

## Validation

- HID: boot-like arrays with/without IDs, modifier ordering, output padding,
  unaligned bitmaps, independent/overlapping IDs, usage lists, Push/Pop,
  composite consumer reports, malformed/truncated/oversized descriptors,
  deterministic descriptor mutations, invalid report transactionality and
  normalized-event integration.
- BCE: exact register and memory operation traces, timestamp service during
  delayed replies/transfers, stale/wrong/multiple/late/duplicate replies,
  clock errors, each initialization/poll/publication/completion I/O failure,
  stop-write failure, short output buffers, invalid completions, shutdown with
  outstanding work and 100 ring-wrap transfers.
- End-to-end scripted test: simulated descriptor transfer → HID layout →
  simulated Shift+A report → shared input selector → disconnect releases.
  This is not Apple firmware or VHCI emulation.

## 日本語

実機前に進められる項目1（HID解析）と項目2（BCE模擬接続）を追加した。
解析したキー情報は既存の共通入力層へ渡せる。BCEは模擬レジスター／メモリーで
順序と異常時の停止・保持方針を確認した。実機へのアクセスは有効にしておらず、
内蔵キーボード動作を確認したという意味ではない。次の実機作業はT2-D1診断。実際のdescriptor採取は実通信・列挙を整えた後の
別段階で行い、取得した形式に合わせて対応範囲を調整する。

Validation result: 147 Rust workspace tests passed (18 newly added HID/adapter
integration cases), both libraries passed the UEFI target check, and normal plus
T2-D1 release builds passed. All four FreeBSD reference hashes verified; release
packaging tests passed (5 cases). Formatting and diff checks passed. The new
parser/adapter is not activated in boot; QEMU was not rerun for these unconnected
additions. The preceding runtime-input integration has its own QEMU evidence.

Follow-up: [入力溢れ復元・HIDファジング・実機採取手順](input-prehardware-validation.md).
