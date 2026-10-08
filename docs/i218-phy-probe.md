# I218 PHY probe / I218 PHY診断

## English

This is the first implementation milestone of the [I218 Rust port plan](i218-rust-port-plan.md). It does **not** implement I218 networking. The default build still supports only the QEMU 82574 NIC and reports unsupported LAN on NUC5. NUC5/NUC8 hardware has not been tested.

### Implemented

`net/src/i218.rs` is a Rust adaptation of the pinned FreeBSD BSD-3-Clause PHY and ICH semaphore routines. It provides exact Intel I218-V / V2 / V3 classification, a serialized register backend, bounded software ownership acquisition/cancellation/release, MDIO completion/error/address checks, paged HV reads, and a PHY ID/revision read under a single ownership interval. Reserved EXTCNF_CTRL bits are retained. An already-held semaphore is never forcibly cleared. Ordinary Result-error paths attempt cleanup, and release failures supersede earlier transaction errors. Panic/abrupt power loss cleanup is not guaranteed.

The generic backend must provide finite register accesses and delays. Ownership first waits at most 100 iterations of 1ms for an existing owner, then at most 1000 iterations of 1ms for our request. Each MDIO transaction polls at most 1920 times, with at least 50µs between observations. The runtime uses the ACPI PM timer for these short delays, including 24/32-bit wrap handling; each delay additionally has a finite CPU-iteration limit for a stalled timer. These waits occur during initialization, before network polling begins.

The page API accepts common page 0 and direct HV pages 768–2047, excluding wakeup page 800. Registers must be 0–31. Pages 1–767 use a different debug protocol and are rejected; wakeup protocols and upper register encodings are not implemented. Page 768 uses the upstream page-zero alias. Page selection writes use PHY address 1; data reads use address 2 for page 0 and address 1 for HV pages. Common registers 0–15 do not require page selection. No arbitrary PHY data-write API is exposed.

### Experimental runtime integration

The opt-in `i218-phy-probe` feature allows UEFI discovery to select **only** PCI `8086:15A3`, the expected NUC5 I218-V3 candidate, in addition to the existing 82574. Actual hardware identification is still required. Other I218-V variants and I219 remain excluded from this runtime path even though the pure classifier recognizes the I218 variants.

The selected BAR is validated/mapped by the existing memory path. PCI bus mastering is disabled before page tables/arena construction and remains disabled. At network initialization, the probe checks memory decoding and BAR bounds, reads PCI revision, STATUS and FWSM, then acquires software ownership to read PHY ID/revision. The expected PHY family is `015400A0`, with the revision shown separately. A missing, malformed or unexpected PHY is an error. The snapshot records `I218 NETWORK NOT IMPLEMENTED`; even a successful PHY probe returns no network session, does not initialize lwIP, and does not enable RX/TX or DMA.

The probe issues MMIO writes for the semaphore and MDIO read commands. Paged library reads can also change PHY page selection. It is not a purely read-only PCI scan. It does not reset the MAC/PHY, recover ULP/SMBus state, write NVM, negotiate a link or transmit packets. Access may fail if firmware or PHY power state is not yet suitable; this is evidence for the next porting stage, not a reason to claim support. A semaphore release failure requires recording the exact state before a controlled restart/power cycle.

Build explicitly for an eventual NUC5 diagnostic:

```sh
cargo build --locked --release --target x86_64-unknown-uefi \
  -p musha-boot --features i218-phy-probe
cp target/x86_64-unknown-uefi/release/musha-boot.efi out/esp/EFI/BOOT/BOOTX64.EFI
cp LICENSE out/LICENSE
cp NOTICE out/NOTICE
```

Use the [USB image instructions](usb-image.md) with the exact physical USB capacity when preparing a disk image. Record the feature set and hash in the [hardware test template](hardware-test-record-template.md). No physical USB drive has been written as part of this milestone. `sh tools/build-esp.sh` restores the default build and stages `out/LICENSE` / `out/NOTICE`. Include both license files when distributing EFI executables or disk images; filenames alone or a source URL do not replace the required text.

### Next work

Port applicable power/ownership recovery, MAC/PHY reset and completion waits, then link negotiation and device-specific workarounds. Validate I218 descriptor/filter/DMA configuration before joining the common lwIP path. Static physical IPv4 settings and actual packet tests remain pending. QEMU cannot exercise this physical PHY path.

### Validation record — 2026-10-08

- Rust 1.99.0: 65 host tests passed, including 11 I218 tests and two short-delay tests. Cases cover exact classification, paired IDs, paged/alias addressing, invalid pages/registers, an existing owner, denied requests, MDIO timeout/error/wrong-address/backend failure, clock errors, invalid PHY IDs, lost ownership, release failure, and failed page selection without a subsequent data read.
- The `i218-phy-probe,qemu-debug` UEFI build passed QEMU shared-image boot/input/file checks and the cooperative ARP/ICMP/257-UDP-echo regression. An `e1000` NIC remained unsupported and did not enter the I218 path. These emulated NICs do not validate the physical I218 probe.
- The default `qemu-debug` build passed cooperative traffic and link-down cleanup with the shared image unchanged. The normal build was restored, booted separately without debug features, and its greeting, file/input results and diagnostic panel were checked in a screenshot. The complete normal image remained unchanged.
- All eight pinned upstream source hashes matched. The complete BSD notice is present in repository NOTICE; staged `out/NOTICE` / `out/LICENSE` matched their originals.

Local artifacts (64MiB QEMU images, not physical 32GB USB images):

| Artifact | SHA-256 |
|---|---|
| `out/musha-usb-i218-probe-qemu-debug.img` | `abac30dc8678bdbb6c1887295657a2719bb977ded997cd7592223f9fc37f2bcb` |
| `out/musha-usb-i218-foundation-default-debug.img` | `ce88f039dc818b5d813c57f0be31894066b1ab5d30ce84e425af23ac14b6d53f` |
| `out/esp/EFI/BOOT/BOOTX64.EFI` (normal) | `53f702b2890c221bb944e51465c8f2bc03f6c3837d22e5187c88bb63b17787e9` |
| `out/musha-usb-i218-foundation-release.img` | `f4fcfc0c6ac65be34eca6a347ec06672bc6a322708482deb80dcbeb6abdbc8e8` |

Host results: `out/i218-host-tests.log`. Normal screenshot: `out/qemu-i218-foundation-release/screen.ppm`. QEMU logs use the existing normal/cooperative case directories and can be replaced by later runs. Physical probe, power/reset, link and packet tests: **NOT RUN / NOT IMPLEMENTED**, as applicable.

## 日本語

[I218 Rust移植方針](i218-rust-port-plan.md)の最初の実装段階。**I218での通信はまだ実装していない。** 通常ビルドの対応NICはQEMU 82574だけで、NUC5 LANは未対応表示となる。NUC5／NUC8の実機試験は未実施。

### 実装した範囲

`net/src/i218.rs`は固定したFreeBSD BSD-3-ClauseのPHY・ICH semaphore処理をRustへ移植したもの。Intel I218-V／V2／V3の正確な識別、直列化register backend、上限付きsoftware ownership取得・要求取消・解放、MDIO完了／error／address検査、page付きHV読出し、単一のownership区間でのPHY ID／revision取得を提供する。EXTCNF_CTRLのreserved bitを保持し、既に取得されているsemaphoreを強制解除しない。通常のResultエラーではcleanupを試み、解放失敗は元のtransactionエラーより優先して報告する。panic・突然の電源断時のcleanupは保証しない。

backendは有限のregisterアクセス・delayを提供する。既存owner待ちは1ms×最大100回、要求後の取得待ちは1ms×最大1000回。各MDIO transactionは50µs以上間隔で最大1920回観測する。実機backendの短いdelayはACPI PM timerを使い、24／32bitの周回を扱う。timer停止時にも各delayのCPU観測回数で上限を設ける。これらは初期化中に行い、通信poll開始前の処理となる。

page APIは共通page 0とHV page 768–2047に対応し、wakeup page 800を除く。registerは0–31。page 1–767は別のdebug protocolを使うため拒否し、wakeup protocol・上位register encodingは未実装。page 768は元実装に従いpage 0へ変換する。page選択はPHY address 1、data読出しはpage 0でaddress 2、HV pageでaddress 1を使う。共通register 0–15はpage選択しない。任意のPHY data書込みAPIは提供しない。

### 実験用ランタイム統合

明示指定する`i218-phy-probe` featureでは、既存82574に加え、NUC5のI218-V3照合候補である**PCI `8086:15A3`だけ**をUEFIから選べる。実際の個体確認は必要。純粋なclassifierが識別する他のI218-V variantsやI219は、この実機経路では選択しない。

BARは既存のmemory処理で検査・mapする。PCI bus masteringはpage table／arena構築前に無効化し、無効のまま保持する。network初期化時にmemory decoding・BAR境界を確認し、PCI revision・STATUS・FWSMを読み、software ownershipを取得してPHY ID／revisionを読む。想定PHY familyは`015400A0`でrevisionは別表示。欠落・不正・想定外PHYはエラーとなる。`I218 NETWORK NOT IMPLEMENTED`を記録し、PHY probe成功でもnetwork sessionを返さない。lwIP初期化、RX/TX・DMA有効化を行わない。

probeはsemaphoreとMDIO読出し命令のためMMIOへ書き込む。ライブラリのpage付き読出しではPHY page選択も変更する。純粋なPCI読出し診断ではない。MAC/PHY reset、ULP/SMBus復帰、NVM書込み、link交渉、packet送信は行わない。firmware・PHY電源状態によってはアクセスに失敗する。この結果は次の移植段階に使い、対応完了とは扱わない。semaphore解放失敗では、状態を記録してから計画的に再起動／電源再投入する。

将来のNUC5診断版を作る場合は英語節のコマンドで明示buildする。[USBイメージ手順](usb-image.md)に従い、実物USBの正確な容量でimageを生成する。featureとhashを[試験記録様式](hardware-test-record-template.md)へ記録する。本段階では実物USBへ書き込んでいない。`sh tools/build-esp.sh`で通常ビルドへ戻し、`out/LICENSE`／`out/NOTICE`も配置する。EFI・disk image配布には両ライセンス文書を添付する。ファイル名だけや元ソースURLだけでは必要な本文を添付したことにはならない。

### 次の作業

必要な電源・ownership復帰、MAC/PHY reset・設定完了待ち、link交渉・機種別回避処理を移植する。I218のdescriptor/filter/DMA設定を検証してから共通lwIPへ接続する。実機用固定IPv4設定と実packet試験は未完了。QEMUではこの実PHY経路を動作確認できない。

### 検証記録 — 2026-10-08

- Rust 1.99.0でホスト試験65件成功。I218の11試験と短いdelayの2試験を含む。正確な機器識別、ID pair、page／alias address、不正page・register、既存owner、要求拒否、MDIO timeout／error／address不一致／backend失敗、clock error、不正PHY ID、ownership喪失、解放失敗、page選択失敗後にdata読出しを発行しないことを確認。
- `i218-phy-probe,qemu-debug` UEFI版で共通imageの起動・input・file確認と協調ARP／ICMP／UDP echo 257件の回帰試験成功。`e1000`は未対応のままでI218経路へ入らない。これらの仮想NICでは実I218 probeの動作は検証できない。
- 通常の`qemu-debug`版で協調通信とlink切断時cleanup、image不変を確認。通常ビルドへ戻してdebug featureなしで別途起動し、挨拶・file・input・診断パネルを画像で確認。通常imageのhashは不変。
- 固定した移植元8ファイルのhash一致。BSD原文noticeを本体NOTICEへ含め、配置した`out/NOTICE`／`out/LICENSE`が原本と一致することを確認。

生成物とSHA-256は英語節の表を参照。imageはQEMU用64MiBで、実物32GB USB用ではない。ホスト結果は`out/i218-host-tests.log`、通常版画像は`out/qemu-i218-foundation-release/screen.ppm`。QEMU logは既存のnormal／cooperative試験ディレクトリにあり、後日の試験で置換され得る。実機probe、電源／reset、link、packet試験は、該当する項目について**未実施／未実装**。
