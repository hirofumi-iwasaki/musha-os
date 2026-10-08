# USB storage read diagnostics

## English

### Supported scope

Target SCSI transparent Bulk-Only Transport with configuration index 0,
alt setting 0, class 8 / subclass 6 / protocol 50h.
Validate configuration boundaries up to 1024 bytes, exactly one Bulk IN and one Bulk OUT endpoint,
endpoint counts, and packet sizes.
Packets are 8 / 16 / 32 / 64 bytes at Full-speed, 512 at High-speed, and 1024 at SuperSpeed.
For SuperSpeed, require an immediately following six-byte companion descriptor
and pass bMaxBurst (0–15) into the context.
Streams, UAS, and Low-speed are unsupported.

Allocate IN / OUT rings, a 31-byte CBW, a 13-byte CSW,
and a data buffer up to 4096 bytes from the dedicated DMA pool.
Configure Endpoint, then Set Configuration.
Handle only LUN 0; do not issue Get Max LUN.
BOT transfers CBW, optional Data IN, and CSW sequentially.
Validate completion-event pointer, slot, endpoint, Success, and residual 0,
plus CSW signature, tag, residue 0, and status.
Do not issue commands concurrently. Reuse rings and buffers only after completion.

### Diagnostic commands

- Try TEST UNIT READY at most three times. On failure status, issue REQUEST SENSE (18 bytes).
  Wait 100ms and retry only for fixed-sense Not Ready / Unit Attention.
- Obtain block count and sector size with READ CAPACITY(10).
  Support 512 / 4096-byte sectors; reject last LBA FFFFFFFFh because it requires READ CAPACITY(16).
- Read one sector each at LBA 0 and the last LBA with READ(10), recording FNV-1a 64-bit hashes.
  Check LBA ranges and sector size before submission.

The CBW encoder generates only these four command types. WRITE / FORMAT and similar commands are not provided.
Bulk OUT is used only to send CBWs; no Data OUT writes to media are issued.
On successful reads, screens at least 444 pixels high display USB READ OK and USB BYTES.

Each transfer has a 1000ms deadline, bounded by both clock and poll count.
STALL, short transfers, malformed CSWs, phase errors, and similar conditions fail diagnostics:
stop the controller and disable bus mastering.
BOT Reset Recovery, Clear Feature, endpoint reset, and continued recovery from media errors are not implemented.
Do not free or reuse DMA memory after shutdown.

Diagnostics run in port order. If continuous keyboard input begins first,
inspect subsequent storage after Esc ends input.
Simultaneous input / storage polling is unsupported.
This stage provides boot-time block-read diagnostics; application block handles are not yet available.
[MBR / FAT32 root-file reads](fat32.md) have been added.
Application block / file APIs are still not provided.

### Validation

Verified SuperSpeed and High-speed with QEMU q35 / qemu-xhci / usb-storage.
In addition to the UEFI virtual USB drive, attach a raw USB disk containing 4MiB of known data.
Compare capacity for 512-byte / 4096-byte sectors and first / last hashes with host expectations.
Attach test raw disks read-only; their SHA-256 values after exit matched those at creation.
Tests combined with continuous keyboard input, Esc termination, DMA shutdown,
and RAM application completion also passed.

The storage-timeout feature omits the READ(10) Data IN doorbell.
Smoke verified a 20ms timeout, DMA shutdown, and RAM application completion.
Host tests check CBW byte order, out-of-range LBAs, capacity sentinels / sector sizes,
CSW tags / residues / status, malformed companions, and truncated descriptors.

Full-speed storage, the physical 32GB USB drive, NUC5 / NUC8,
hardware retries for Not Ready / Unit Attention, and BOT error recovery remain untested.
Do not declare hardware compatibility or completion of 0.1.0.

Primary sources:
[USB Mass Storage Bulk-Only Transport 1.0](https://www.usb.org/sites/default/files/usbmassbulk_10.pdf)
§3–6,
[T10 SBC-3 early draft](https://t10.org/ftp/t10/document.05/05-344r0.pdf)
READ(10) / READ CAPACITY(10),
[Intel xHCI 1.2b](https://cdrdv2-public.intel.com/625472/625472_xHCI_Rev1_2b.pdf)
§4.11, 6.2.3.

---

## 日本語

**USBストレージ読出し診断**

### 対応範囲

Configuration index 0、alt setting 0、class 8 / subclass 6 / protocol 50hの
SCSI transparent Bulk-Only Transportを対象とする。最大1024byteのconfigurationを
境界検査し、Bulk IN / OUT各一端点、endpoint数、packetを確認する。
Full-speedは8 / 16 / 32 / 64byte、High-speedは512byte、
SuperSpeedは1024byte packet。SuperSpeedでは直後の6byte companionを要求し、
bMaxBurst(0〜15)をcontextへ渡す。streams、UAS、Low-speedは非対応。

専用DMA poolにIN / OUTのring、31byte CBW、13byte CSW、最大4096byteの
データbufferを確保する。Configure Endpoint、Set Configurationの順に設定する。
LUN 0のみを扱い、Get Max LUNは発行しない。
BOTはCBW、必要ならData IN、CSWを逐次転送する。完了eventのpointer、slot、
endpoint、Success、residual 0に加え、CSW signature、tag、residue 0、statusを検査する。
コマンドを並列発行しない。ringとbufferは完了後にだけ再利用する。

### 診断コマンド

- TEST UNIT READYを最大3回。失敗statusにはREQUEST SENSE(18byte)を行い、
  fixed senseのNot Ready / Unit Attentionの場合だけ100ms待って再試行する。
- READ CAPACITY(10)でblock数とsector長を取得する。
  512 / 4096byte sectorを扱い、FFFFFFFFhのlast LBAはREAD CAPACITY(16)が必要として拒否する。
- READ(10)でLBA 0と最終LBAを各一sector読み、FNV-1a 64bit hashを記録する。
  提出前にLBA範囲とsector長を検査する。

CBW encoderは上記4種類のコマンドだけを生成する。WRITE / FORMAT等は提供しない。
Bulk OUTはCBW送信専用で、媒体へ書くData OUTは発行しない。
読出し成功時、高さ444pixel以上の画面にUSB READ OKとUSB BYTESを表示する。

各転送の期限は1000ms、時計とpoll回数で制限する。
STALL、short transfer、壊れたCSW、phase error等は診断失敗としてcontrollerを
停止し、bus masteringを解除する。BOT Reset Recovery、Clear Feature、
endpoint reset、媒体エラーからの継続復旧はまだ実装しない。
停止後もDMAメモリは解放・再利用しない。

ポート順に診断するため、先にキーボードの継続入力へ入った場合はEsc終了後に
後続ストレージを調べる。入力とストレージの同時pollは未対応。
この段階は起動時のblock読出し診断で、アプリ向けblock handle、
[MBR / FAT32のルートファイル読出し](fat32.md)を追加した。
アプリ向けblock / file APIはまだ提供しない。

### 検証

QEMU q35 / qemu-xhci / usb-storageでSuperSpeedとHigh-speedを確認した。
UEFI用の仮想USBに加え、4MiBの既知データを持つraw USBを接続し、
512byte / 4096byte sectorの容量と、先頭・末尾hashをホスト側の期待値と照合した。
試験raw diskはreadonly接続し、終了後のSHA-256も生成時と一致した。
継続キーボード入力、Esc終了、DMA停止、RAMアプリ完了との組合せも成功した。

storage-timeout featureはREAD(10)のData IN doorbellを省略する。
20msの期限切れ、DMA停止、RAMアプリ完了をsmokeで確認した。
ホスト試験はCBWのbyte order、範囲外LBA、capacity sentinel / sector長、
CSWのtag / residue / status、不正companionと切れたdescriptorを検査する。

Full-speedストレージ、実物の32GB USB、NUC5 / NUC8、
Not Ready / Unit Attentionの実機再試行、BOTのエラー復旧は未検証。
実機互換性と0.1.0の完成は宣言しない。

一次資料:
[USB Mass Storage Bulk-Only Transport 1.0](https://www.usb.org/sites/default/files/usbmassbulk_10.pdf)
§3〜6、
[T10 SBC-3 初期draft](https://t10.org/ftp/t10/document.05/05-344r0.pdf)
READ(10) / READ CAPACITY(10)、
[Intel xHCI 1.2b](https://cdrdv2-public.intel.com/625472/625472_xHCI_Rev1_2b.pdf)
§4.11、6.2.3。
