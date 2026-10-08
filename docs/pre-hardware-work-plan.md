# Work plan before hardware arrival / 実機到着前の作業方針

## English

Established: 2026-10-08. Target release: 0.1.0.
The user has secured a NUC5 and a 32GB USB drive; hardware is expected in several days.
Continue implementation and QEMU validation in the following order. Do not declare hardware compatibility or release completion before physical tests.

1. **GPT support and a shared bootable USB image.** Validate GPT headers, partition-array CRCs, primary / backup consistency, and partition bounds. Generate a reproducible GPT image with a FAT32 ESP, `EFI/BOOT/BOOTX64.EFI`, and `MUSHA.TXT`. Boot the actual image in QEMU, read the file with the runtime USB driver, and verify that the image is unchanged. The generator creates regular files only; writing to a physical USB drive requires identifying that drive separately.
2. **Application file API.** Provide read-only `open / read / close`, bounded buffers and handles, explicit errors, EOF, and invalidated-handle checks. Validate known contents and failures in host tests / QEMU. Define asynchronous progress before integrating simultaneous I/O.
3. **QEMU networking.** Implement the 82574 initialization path and connect lwIP with static IPv4; validate ARP, ICMP, and UDP. Keep I218-V / I219-V initialization separate and reserve physical validation for NUC5 / NUC8. Retain upstream licenses and attribution.
4. **Concurrent progress of input, files, and communication.** Refactor the runtime into a bounded cooperative polling loop so keyboard input does not block file or network progress. Validate idle input, sustained traffic, timeouts, and termination.
5. **Hardware diagnostics and test records.** Show PCI IDs, USB information, initialization stages, and failure details. Prepare a result template recording model, firmware, RAM, connections, image hash, and results. Follow the existing NUC5 preparation instructions.

After each stage, record completed scope, tests, known limitations, and the next stage in this file and the relevant English / Japanese documentation. Keep changes in dedicated Git branches.

### Progress

- Stage 1: Implemented and verified in QEMU. GPT validation, reproducible bootable images, and exact medium-capacity sizing are available. See [shared USB image](usb-image.md). Physical boot tests remain pending.
- Stage 2: Initial bounded file API implemented and verified in QEMU; see [file API](file-api.md). Generic device-backed asynchronous reads remain for the simultaneous-I/O design.
- Stage 3: Initial QEMU 82574/lwIP diagnostic implemented and verified; see [network diagnostics](network.md). ARP, ICMP, UDP, ring wraps, checksum rejection and link-loss cleanup pass. NUC PHY initialization remains pending.
- Stage 4: Initial [cooperative I/O runtime](cooperative-io.md) implemented and verified. Input, snapshot-file reads and QEMU networking progress together; idle/no-keyboard/disconnect, link loss, TX timeout, application error and Esc cleanup were tested. General asynchronous disk requests and the application UDP API remain pending.
- Stage 5: Pending; proceed with hardware diagnostics and test records next.
- Hardware validation: Pending arrival of the NUC5. NUC8 availability remains unconfirmed.

## 日本語

策定日: 2026-10-08。対象リリース: 0.1.0。
利用者はNUC5と32GB USBを確保済みで、実機到着は数日後の予定。
以下の順で実装とQEMU検証を進める。実機試験前に実機互換性やリリース完成を宣言しない。

1. **GPT対応と共通の起動用USBイメージ。** GPTヘッダー、partition arrayのCRC、主・副の整合性、partition境界を検証する。FAT32 ESP、`EFI/BOOT/BOOTX64.EFI`、`MUSHA.TXT`を含む再現可能なGPTイメージを生成する。実際のイメージをQEMUで起動し、ランタイムのUSBドライバでファイルを読み、イメージが変化しないことを検査する。生成ツールは通常ファイルだけを作る。実物USBへの書込みには別途対象の特定が必要。
2. **アプリ向けファイルAPI。** 読出し専用の`open / read / close`、上限付きbufferとhandle、明示エラー、EOF、無効handle検出を提供する。既知内容と失敗をホスト試験・QEMUで確認する。同時I/Oへ統合する前に非同期の進め方を定める。
3. **QEMU上のネットワーク。** 82574の初期化経路を実装し、固定IPv4のlwIPを接続してARP・ICMP・UDPを検証する。I218-V / I219-Vの初期化は分離し、NUC5 / NUC8で実機検証する。依存元のライセンス・帰属表示を保持する。
4. **入力・ファイル・通信の同時進行。** キーボード入力がファイル・通信を止めないよう、上限付きの協調pollループへ整理する。無入力、継続通信、timeout、終了を検証する。
5. **実機の診断表示と試験記録。** PCI ID、USB情報、初期化段階、失敗詳細を表示する。機種、ファームウェア、RAM、接続、イメージhash、結果の記録様式を用意する。既存のNUC5準備手順に従う。

各段階で完了範囲、試験、制約、次の段階を本書と関連する英語・日本語文書へ記録する。変更は専用Gitブランチで管理する。

### 進捗

- 第1段階: 実装とQEMU検証完了。GPT検証、再現可能な起動イメージ、媒体の正確な容量指定に対応。[共通USBイメージ](usb-image.md)を参照。実機起動試験は未実施。
- 第2段階: 初期の上限付きfile APIを実装しQEMUで確認。[ファイルAPI](file-api.md)を参照。一般の非同期device-backed読出しは同時I/O設計に残る。
- 第3段階: 初期のQEMU 82574/lwIP診断を実装・検証。[ネットワーク診断](network.md)を参照。ARP・ICMP・UDP、ring周回、checksum拒否、link切断時の停止を確認。NUC PHY初期化は未実装。
- 第4段階: 初期の[協調I/Oランタイム](cooperative-io.md)を実装・検証。入力・snapshot file read・QEMU通信が同時に進む。無入力・キーボードなし・切断、link切断、TX timeout、アプリerror、Esc cleanupを確認。一般の非同期disk要求とアプリUDP APIは未実装。
- 第5段階: 未着手。次は実機診断・試験記録へ進む。
- 実機検証: NUC5の到着待ち。NUC8の確保状況は未確認。
