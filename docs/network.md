# QEMU network diagnostics / QEMUネットワーク診断

## English

Implemented on 2026-10-08 for the QEMU `e1000e` 82574 device (PCI `8086:10D3`). It now runs within the [cooperative input/file/network session](cooperative-io.md). An application UDP API remains unimplemented. I218-V (NUC5) and I219-V (NUC8) initialization and physical verification remain pending. Other NICs report unsupported.

The original Rust driver uses legacy 16-byte descriptors, 16 RX entries and 8 TX entries, 2KiB buffers, and a dedicated 64KiB LoaderData allocation below 4GiB. This allocation is excluded from the application arena and mapped UC/RW/NX. Both controller BARs are checked for overlap and mapped UC. The supported NIC's bus mastering is disabled immediately after ExitBootServices, before constructing page tables and the arena. Initialization validates the device, memory decode, MAC, link, reset and EEPROM reload. DMA starts only after ring addresses are installed. RX uses DD/EOP/error/length validation and copies into CPU-owned storage. TX waits for completion before reusing buffers. Volatile accesses, compiler fences and x86 fences preserve DMA ordering. All device interrupts remain masked.

lwIP 2.2.1 runs with `NO_SYS=1` on the boot CPU. IPv4, ARP, ICMP and UDP are enabled, with software checksums, a fixed heap/pools and a bounded eight-frame TX queue. TCP, sockets, DHCP, IPv6, fragmentation/reassembly and other disabled facilities are not provided. The diagnostic uses `10.0.2.15/24`, gateway `10.0.2.2`, and UDP echo port `12345`. Only a test peer on this subnet is validated.

The cooperative debug/no-keyboard session is bounded to ten seconds; normal keyboard sessions end with Esc. Per pass, RX work is bounded to eight frames; TX is one completion observation and at most one new frame, with no completion spin loop. Pending TX has a 100ms deadline and finite observation limit. Reset/reload/link waits are also bounded. On success, initialization failure, timeout or link loss, RX/TX and PCI bus mastering are disabled; DMA storage remains reserved. A lwIP assertion attempts and verifies bus-master disablement for both NIC and USB before halting. A successful run displays `NET TEST OK` when the framebuffer has enough space, even if no packets arrived. Communication success must be established with the packet test below, not this display alone.

### Build and tests

Clang is required alongside the pinned Rust toolchain. `CC` can select a Clang executable. The build compiles the unmodified selected C sources into freestanding x86-64 COFF objects with warnings treated as errors, then links them into the UEFI image without a host C runtime. The Rust/C boundary uses synchronous fixed-width values and borrowed buffers; C retains no Rust buffer pointers.

```sh
cargo test --workspace --exclude musha-boot
cargo build --locked --release --target x86_64-unknown-uefi -p musha-boot --features qemu-debug
cp target/x86_64-unknown-uefi/release/musha-boot.efi out/esp/EFI/BOOT/BOOTX64.EFI
python3 tools/smoke-network.py --qemu /opt/homebrew/bin/qemu-system-x86_64 --firmware-dir /opt/homebrew/share/qemu
python3 tools/smoke-network.py --qemu /opt/homebrew/bin/qemu-system-x86_64 --firmware-dir /opt/homebrew/share/qemu --case link-down
```

The test uses QEMU's loopback TCP socket Ethernet backend and crafts real Ethernet frames; it validates MAC/IP addresses, ARP replies, ICMP payloads/checksums, 97 UDP echoes/checksums, six RX ring wraps and twelve TX ring wraps. Corrupt IPv4 and UDP checksums must produce no echo, after which valid traffic must still succeed. The link-down case uses QMP to disconnect the NIC and requires bounded termination and verified DMA disablement. `--usb-image PATH` boots a real GPT/FAT32 image read-only, additionally checks the application file marker and unchanged SHA-256. No physical NIC, USB drive or internal disk is touched. Results and screenshots are under `out/qemu-network-normal` and `out/qemu-network-link-down`.

Initial positive and link-down tests passed on QEMU 11.1.2 / Rust 1.99.0. Host tests now total 48; the [cooperative I/O tests](cooperative-io.md) verify concurrent progress and failure isolation. Hardware, application UDP handles and NUC PHY initialization remain unverified.

### Stage 3 image verification record

The actual GPT/FAT32 debug image passed the full packet test and application file read with unchanged SHA-256:
`be7fa42ee256cc41754d9eced498c0d425aa43ada09322e0e8a98940600ae0d5`.
The normal image `out/musha-usb-network.img` (without `qemu-debug`) was also booted read-only; Shift+A and Esc were sent and the final greeting, file-read result, and `NET TEST OK` screen were visually checked. Its unchanged SHA-256 is
`dfbc44064e98195424e1835e4f0cc1750a50bf03cb27a2ace3cc15873dcbb932`.
Clang 21.0.0 compiled the C port. `out/NOTICE` accompanies local binary/image artifacts.
These 64MiB images are test artifacts. Before writing a 32GB USB drive, regenerate the image with the exact physical medium capacity as specified in [shared USB images](usb-image.md); the backup GPT must be at the end of the actual medium. Physical USB writing and boot remain pending.

### Sources and licensing

The driver is original code based on Intel's [82574 datasheet](https://www.mouser.com/pdfdocs/82574datasheet.pdf). The test backend follows [QEMU network socket documentation](https://www.qemu.org/docs/master/system/invocation.html). The port follows lwIP's [NO_SYS model](https://www.nongnu.org/lwip/2_1_x/group__lwip__nosys.html). See [third-party dependencies](third-party.md) and the repository `NOTICE`; retain upstream attribution when distributing binaries.

## 日本語

2026-10-08にQEMUの`e1000e`、82574（PCI `8086:10D3`）向け診断を実装した。現在は[入力・file・通信の協調セッション](cooperative-io.md)内で動く。アプリ用UDP APIは未実装。NUC5のI218-V、NUC8のI219-Vの初期化と実機検証は未実施。他のNICは非対応と報告する。

自作Rustドライバは16byteのlegacy descriptor、RX 16件、TX 8件、2KiB bufferを使用する。専用64KiBのLoaderDataを4GiB未満へ確保し、アプリarenaから除外してUC/RW/NXへmapする。2つのcontroller BARは重複を検査しUCへmapする。対応NICのbus masteringはExitBootServices直後、ページテーブル・arena構築前に無効化する。機種、memory decode、MAC、link、reset、EEPROM reloadを検査し、ringの設定完了後にDMAを開始する。RXはDD/EOP/error/長さを検査してCPU所有bufferへコピーする。TXは完了前にbufferを再利用しない。volatileアクセス、compiler fence、x86 fenceでDMA順序を保つ。device interruptは全てmaskする。

lwIP 2.2.1を`NO_SYS=1`で起動CPU上から呼び出す。IPv4・ARP・ICMP・UDP、software checksum、固定heap/pool、8frameの上限付きTX queueを使用する。TCP、socket、DHCP、IPv6、fragmentation/reassemblyなど無効化した機能は提供しない。診断用の固定IPは`10.0.2.15/24`、gatewayは`10.0.2.2`、UDP echo portは`12345`。このsubnet上の試験相手だけを検証した。

debug版・キーボードなしの協調セッションは10秒に制限し、通常版のキーボードセッションはEscで終える。各passのRXは8frameまで、TXは完了観測1件と新規frame最大1件で、完了を待ち続けない。未完了TXは100msと観測回数で制限する。reset/reload/link待ちにも上限を設ける。成功、初期化失敗、timeout、link切断の各経路でRX/TXとPCI bus masteringを停止し、DMA領域は予約したままにする。lwIP assertionはNICとUSBのbus mastering無効化を試みて確認し、停止する。成功時、画面に余裕があれば`NET TEST OK`を表示する。受信なしでも表示するため、通信成功は下記packet試験で判断する。

### ビルドと試験

固定Rust toolchainに加えてClangが必要。`CC`でClang実行ファイルを指定できる。変更していない採用元Cソースをfreestanding x86-64 COFFへ警告をerrorとしてcompileし、host C runtimeを使わずUEFIへlinkする。Rust/C境界は同期呼出し、固定幅の値、借用bufferで構成し、CはRust buffer pointerを保持しない。

英語節のコマンドでhost試験・通信試験・link切断試験を実行する。QEMUのloopback TCP socket Ethernet backendを使い、実際のEthernet frameでMAC/IP、ARP応答、ICMP内容/checksum、UDP echo 97回/checksum、RX ring 6周とTX ring 12周を検査する。不正なIPv4/UDP checksumにはechoしないこと、その後の正常通信が成功することを要求する。link-down試験はQMPでNICを切断し、期限内の終了とDMA無効化を要求する。`--usb-image PATH`は実際のGPT/FAT32イメージを読出し専用で起動し、アプリfile markerとSHA-256不変も確認する。実物NIC・USB・内蔵diskは操作しない。結果・画面は`out/qemu-network-normal`、`out/qemu-network-link-down`へ保存する。

初期の正常系とlink切断試験はQEMU 11.1.2 / Rust 1.99.0で成功。host試験は現在計48件。[協調I/O試験](cooperative-io.md)で同時進行と障害の分離を検証した。実機、アプリ用UDP handle、NUC PHY初期化は未検証。

### 参照元とライセンス

ドライバはIntelの[82574 datasheet](https://www.mouser.com/pdfdocs/82574datasheet.pdf)を参照した自作コード。試験backendは[QEMU socket文書](https://www.qemu.org/docs/master/system/invocation.html)、portはlwIPの[NO_SYSモデル](https://www.nongnu.org/lwip/2_1_x/group__lwip__nosys.html)に基づく。[第三者依存](third-party.md)と本体`NOTICE`を参照し、binary配布時も採用元の帰属表示を保持する。

### 第3段階のイメージ検証記録

実際のGPT/FAT32 debugイメージでpacket全試験とアプリfile読出しに成功し、SHA-256不変を確認した:
`be7fa42ee256cc41754d9eced498c0d425aa43ada09322e0e8a98940600ae0d5`。
`qemu-debug`を含まない通常版`out/musha-usb-network.img`も読出し専用で起動し、Shift+AとEscを送信した。最後の挨拶、file読出し結果、`NET TEST OK`を画面で確認した。不変のSHA-256:
`dfbc44064e98195424e1835e4f0cc1750a50bf03cb27a2ace3cc15873dcbb932`。
C portにはClang 21.0.0を使用した。ローカルbinary/image成果物には`out/NOTICE`を添付した。
64MiBイメージは試験用。32GB USBへ書く前に[共通USBイメージ](usb-image.md)の手順で実物媒体の正確な容量を指定して再生成し、副GPTを媒体末尾へ置く。実物USBへの書込みと起動は未実施。
