# v0.1.0 validation record / 検証記録

2026-10-10, macOS / Apple Silicon, Rust 1.99.0, Apple Clang 21, QEMU 11.1.2.
Scope is defined in [release-scope-0.1.0.md](release-scope-0.1.0.md).

## Confirmed before freezing source

- Formatting check passed after formatting the H19 additions; no runtime behavior was changed for publication.
- Host tests: 100 passed (API 5, framebuffer 5, filesystem 9, memory 10, network 16, platform 27, xHCI 28).
- Python image/build/package checks: 10 passed (4 + 1 + 5).
- Third-party source checksum checks: lwIP, FreeBSD e1000 and r-efi AUTHORS matched.
- Independent normal build with features=[]: QEMU showed RUNTIME READY, ARENA READY,
  USB READ OK, FAT32 READ OK, APP FILE READ OK, KEY CODE 29 and SESSION COMPLETE after injected A, Shift+A and Esc.
- Normal-build executable code (.text), initialized data (.data), unwind and relocation
  sections matched the physical H19 EFI byte-for-byte. The complete binary differed only
  in the PE timestamp, debug-directory timestamp and CodeView build identifier.

Physical H19 EFI: 302080 bytes, SHA-256
`2ceaea47977e544bf7469cb562cc8f041df32de3cdcd00408eedc674fb5f08d9`.
The MacBook Pro 2018 user reported A, Shift+A and Esc working on Lenovo 17EF:6009.
Photographs IMG_7540–IMG_7552 show input on C0, SanDisk 0781:55A9 reads on C2,
MUSHA.TXT length 16, hash `9A42A948C590F507`, and session completion.
See [H19 hardware evidence](pci-h19-hardware-results.md).
This does not claim that a newly timestamped release file was independently booted on hardware.

## Publication gates and durable evidence

The release process requires the full QEMU suite: USB/input, packet-level ARP/ICMP/UDP,
cooperative traffic/idle/no-keyboard/disconnect/link-down, multi-controller storage/input,
five USB2 hub levels, seven CPU/page-fault cases, four USB/storage timeout cases,
TX timeout, application error and corrupt GPT/FAT32. GitHub's Mac workflow repeats these
checks on the PR head; its result is separate from local execution.

Build the final bundle from a fresh clean checkout of the merged commit with:

```sh
python3 tools/prepare-release.py --release --output dist/v0.1.0
```

The isolated target directory excludes stale debug/fault builds. `manifest.json` records
commit, tool versions, Cargo.lock hash, features, working-tree state and every payload hash.
The Release's validation record supplies final commit/CI links, final EFI checksum and
metadata-only comparison against the physical H19 file. `SHA256SUMS` verifies the archive;
the bundle includes its own checksums for all payloads. Downloaded assets must be verified
before publication is reported complete.

## 日本語

公開前にホスト100件、イメージ・配布物10件、第三者ソースのハッシュ、整形検査を確認。
通常版QEMUでは読出し・入力・Esc終了を確認した。実機H19と実行コード・データは一致し、
ファイル全体の差分はビルド時刻とデバッグ識別情報のみだった。
実機の成功記録と、新しい時刻を持つ配布バイナリの起動試験は区別する。

公開時は全QEMU回帰とPRのMac CIを確認し、マージ後のクリーンなソースから通常版を生成する。
最終コミット・チェックサム・CI結果・実機H19との比較はReleasesの検証記録に残す。
内蔵キーボード、NUC実機、Ryzen H19再試験、全ポート、反復起動・長時間安定性は未確認。
