# LAN feature boundary, 2026-10-20

## English

The boot crate now enables `network` by default. Its optional `musha-net` dependency, boot runtime module, NIC admission, and lwIP C compilation are excluded by `--no-default-features`. Normal distribution builds use Cargo defaults, so the existing 82574 LAN path remains included. The I218 opt-in features imply `network`. The LAN-off facade reports `lan: disabled by build feature` and lets USB storage, keyboard input, and the application run normally. Read-only PCI inventory remains available. This does not enable any new physical NIC.

Evidence from isolated release builds (`--locked --release --target x86_64-unknown-uefi -p musha-boot`):

| Build | SHA-256 | EFI bytes | `.text` raw | `.rdata` raw | `.data` raw | `.data` virtual |
|---|---|---:|---:|---:|---:|---:|
| Default | `30331939baee02fd14407e1941cfa4d0a87ac9f785c58955826019c3f9ea920b` | 850,432 | 646,656 | 41,472 | 160,256 | 18,545,088 |
| LAN off | `3157d74e142f0864d2dc06bcbc19a6725785a2a647cafb5ef85eec3208e2ee97` | 803,328 | 601,088 | 39,936 | 160,256 | 18,473,392 |

The LAN-off image is 47,104 bytes smaller. A fresh isolated LAN-off build produced the same hash and no `musha-net` or lwIP objects. The default dependency tree includes `musha-net`; the no-default tree excludes it while retaining `musha-xhci` and `musha-platform`. Fresh diagnostic builds passed `tools/smoke-qemu.py`: `/private/tmp/musha-lan-off-debug-smoke3` with `--expect-network-disabled`, and `/private/tmp/musha-lan-default-debug-smoke3` with default LAN. Both reached `APP_FILE_OK`, keyboard events, `STORAGE_RELEASED`, `APP_LIFECYCLE_OK`, and session completion. LAN off reported the disabled phase and no `MUSHA: NET_` markers. The default image reached `NET_READY` and `NET_QUIESCED DMA_DISABLED`. A separate `tools/smoke-cooperative.py --case traffic` run at `/private/tmp/musha-lan-default-traffic-20261020` passed ARP/ICMP, 257 verified UDP echoes, file rechecks, ring wraps, checksums, and both DMA stop checks using a disposable echo-on fixture. The initial attempts used the non-diagnostic release EFI with a debug-marker smoke script and therefore yielded no diagnostic output; they were superseded by the passing diagnostic-image runs.

The normal release builder has been corrected to retain Cargo defaults. `efi_size_minimal` stays `null`: Bluetooth is still compiled into the no-default image, and there has been no all-optional-drivers-off boot or size measurement. Physical LAN acceptance remains open.

## 日本語

boot crate の `network` feature を既定有効にし、`--no-default-features` で `musha-net` 依存、LAN 実行 module、NIC 採用、lwIP C の compile を除外します。通常配布 build は Cargo の既定 feature を使い、従来の 82574 LAN 経路を維持します。I218 の任意 feature は `network` を要求します。LAN 無効版は `lan: disabled by build feature` を表示し、USB 記憶・キーボード・アプリを継続します。読取り専用 PCI 一覧は維持します。新しい実 NIC を有効化する変更ではありません。

上表は分離した release build の SHA-256 と PE section 実測値です。LAN 無効版は 47,104 bytes 小さく、空の target から再 build して同一 hash、`musha-net` と lwIP の object がないことを確認しました。依存 tree でも LAN の有無を確認し、`musha-xhci` と `musha-platform` は両方に残ります。新たに診断 feature 付き両版の QEMU smoke が合格し、file 読取り、キー入力、記憶解放、アプリ終了、session 完了に到達しました。LAN 無効版には `MUSHA: NET_` marker がなく、通常版には `NET_READY` と DMA 停止があります。さらに使い捨て echo-on fixture による通常版の通信回帰で ARP/ICMP、257 件の検証済 UDP echo、file 再検査、ring wrap、checksum、両 DMA 停止が合格しました。最初の再実行では診断機能のない release EFI を診断 marker 用 smoke script に渡したため出力がなく、合格した診断版の実行で置き換えました。

通常配布 builder は Cargo 既定 feature を保持するよう修正しました。Bluetooth が no-default 版にも残るため `efi_size_minimal` は `null` を維持し、全任意 driver 除外版の boot や size として扱いません。実機 LAN の受入は未完です。
