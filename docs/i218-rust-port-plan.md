# I218-V Rust port plan / I218-V Rust移植方針

## English

Target: Musha-OS 0.1.0, NUC5i5RYH/RYK I218-V. Established: 2026-10-08.

Use the FreeBSD Intel e1000 implementation as the porting source. Keep Musha-OS original code under Apache-2.0 and preserve BSD-3-Clause copyright, conditions and disclaimer for derived Rust code. Record the exact source revision and hashes, retain the original selected reference files, and include all required notices in binary/image distributions. Do not incorporate Linux GPL driver code. Rewriting or translating C into Rust does not remove upstream license obligations.

Source repository: https://github.com/freebsd/freebsd-src . Pinned `releng/14.3` revision: `833d39bd2e38421a14ea489a956261a0fa993fbc`, path `sys/dev/e1000`. The selected unmodified C/header files, notices and hashes are under `third_party/freebsd-e1000`; they are references and are not compiled.

### Architecture and scope

Keep QEMU 82574 and I218 initialization separate. Reuse the existing bounded polling session, DMA allocation validation and lwIP ARP/IPv4/ICMP/UDP path where the I218 register/descriptor specification permits it. Hardware-dependent register accesses, PHY access, ownership, reset, power-state recovery and link setup belong to the I218 backend. Do not route I218 through the 82574 initializer by adding its PCI ID alone.

Start with static IPv4, ARP, ICMP echo and UDP echo, software checksums and interrupts disabled. Physical test addressing must be configured separately from QEMU's `10.0.2.15/24`. DHCP, TCP, application UDP handles, Wake-on-LAN and suspend/resume are separate work. NUC8 I219 is not automatically supported by an I218 port.

### Work order and completion gates

1. **Provenance and bounded PHY access.** Preserve source/terms, classify exact I218 IDs, translate software/firmware semaphore and paged MDIO read transactions, validate register addresses, and test success/error/timeout/lock-release paths with a simulated register backend. Keep unverified paths experimental and DMA disabled.
2. **NUC5 probe integration.** Discover the actual PCI/revision/BAR and PHY ID, report firmware/PHY access state on screen, and use finite waits. An opt-in probe is a diagnostic, not a network-ready result. PHY access can still fail until power recovery is implemented.
3. **Power and reset.** Port the applicable I218/PCH-LP ownership, ULP/SMBus recovery, MAC/PHY reset, configuration-completion and device-specific workarounds. Preserve reserved bits, coordinate with firmware, and avoid NVM writes. Confirm exact device/revision before applying workarounds.
4. **Link and common data path.** Configure auto-negotiation, detect speed/duplex, validate MAC address, buffers, filters and descriptors, then enable RX/TX/DMA in the required order. Reuse lwIP only after the hardware path is ready. Keep the QEMU driver passing regression tests.
5. **Physical acceptance.** Verify cold boot and restart, ARP/ping/UDP with a named peer, continued input/file progress during traffic, link loss, Esc and DMA shutdown. Record observations in the hardware test template. Linux live-USB read-only identification can provide baseline PCI/PHY/firmware information if needed.

Before hardware arrival, register-backend tests and QEMU regression can validate software ordering and error handling, but cannot establish I218 electrical link, PHY recovery or actual DMA behavior. Never report NUC5 network support complete based only on these tests.

### Progress

Implementation started on branch `feature/i218-rust-port`. Source capture, the bounded PHY-access foundation and opt-in NUC5 probe integration are implemented; see [implemented scope](i218-phy-probe.md). MAC/PHY power recovery, reset, link setup and physical packet tests remain pending. The [probe verification record](i218-phy-probe.md) records 65 passing host tests, normal/experimental QEMU regressions, license staging and image hashes. Physical I218 behavior remains unverified.

## 日本語

対象: Musha-OS 0.1.0、NUC5i5RYH/RYKのI218-V。策定日: 2026-10-08。

FreeBSDのIntel e1000実装を移植元にする。Musha-OS独自部分はApache-2.0を維持し、派生RustコードにはBSD-3-Clauseの著作権表示・条件・免責を保持する。移植元の正確な版とhash、選択した原本を保存し、binary/image配布にも必要な表示を添付する。LinuxのGPLドライバコードを取り込まない。CからRustへの翻訳で元の条件が消えるとは扱わない。

移植元: https://github.com/freebsd/freebsd-src 。`releng/14.3`の固定commitは`833d39bd2e38421a14ea489a956261a0fa993fbc`、対象は`sys/dev/e1000`。変更しないC/header原本、notice、hashを`third_party/freebsd-e1000`へ保存する。原本は参照用で、compileしない。

### 構成と範囲

QEMU 82574とI218の初期化を分ける。I218のregister/descriptor仕様で許される範囲で、既存の上限付きpollセッション、DMA領域検証、lwIPのARP/IPv4/ICMP/UDPを共用する。registerアクセス、PHYアクセス、制御権、reset、電源状態からの復帰、link設定はI218側に置く。PCI ID追加だけで82574の初期化へ通さない。

最初は固定IPv4、ARP、ICMP echo、UDP echo、software checksum、割込み無効に絞る。実機試験のIP設定はQEMU用`10.0.2.15/24`と別に定める。DHCP、TCP、アプリUDP handle、Wake-on-LAN、suspend/resumeは別作業。I218移植でNUC8 I219も自動対応するとは扱わない。

### 作業順と完了条件

1. **出典と上限付きPHYアクセス。** 原本・条件を保持し、I218の正確なIDを識別する。software/firmware semaphoreとpage付きMDIO読出しを移植する。registerアドレスを検証し、模擬backendで成功・error・timeout・lock解放を試験する。未検証経路は実験用としDMAを無効に保つ。
2. **NUC5 probeの統合。** 実際のPCI/revision/BAR、PHY IDを取得し、firmware・PHYアクセス状態を画面に表示する。待ちは有限にする。明示的に有効にするprobeは診断であり、通信準備完了ではない。電源復帰未実装の間はPHYアクセスが失敗し得る。
3. **電源とreset。** 当該I218/PCH-LPに必要なownership、ULP/SMBus復帰、MAC/PHY reset、設定完了待ち、機種別回避処理を移植する。reserved bitを保ち、firmwareと調整し、NVMを書き換えない。回避処理はdevice/revisionを確認して選ぶ。
4. **linkと共通送受信。** 自動交渉、速度／duplex検出、MAC・buffer・filter・descriptor検証後に、所定の順でRX/TX/DMAを有効化する。hardware経路が準備できてからlwIPへ接続する。QEMUの回帰試験も維持する。
5. **実機合格確認。** cold bootと再起動、相手を特定したARP/ping/UDP、通信中のinput/file進行、link切断、Esc、DMA停止を確認し、実機試験様式へ記録する。必要に応じLinux live USBの読出し専用診断でPCI/PHY/firmware情報を取得する。

実機到着前の模擬register試験とQEMU回帰試験でsoftwareの順序・error処理は確認できるが、I218の電気的link、PHY復帰、実際のDMA動作は保証できない。これらだけでNUC5 LAN対応完了と報告しない。

### 進捗

`feature/i218-rust-port`で着手。移植元保存、上限付きPHYアクセス、実験用NUC5 probe統合を実装。[実装範囲](i218-phy-probe.md)を参照。MAC/PHY電源復帰、reset、link設定、実機packet試験は未完了。[probe検証記録](i218-phy-probe.md)にホスト65試験成功、通常／実験構成のQEMU回帰、ライセンス添付、image hashを記録。実I218動作は未検証。
