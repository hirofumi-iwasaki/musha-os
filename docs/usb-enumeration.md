# USB enumeration diagnostics

## English

### Scope

After UEFI exit, our own xHCI driver inspects devices connected at boot, including supported USB2 hub children, one by one.
Validate USB 2 / USB 3 port ranges, overlap, and BAR boundaries in Supported Protocol capabilities;
do not operate undefined ports. Determine speed from default IDs or symmetric PSI.
Treat asymmetric PSI, unknown speeds, and connections exceeding bounded traversal or simultaneous-slot limits as diagnostic failures.
Use 32 / 64-byte contexts according to the HC's CSZ.

Ensure port power and debounce connection for 100ms.
Wait at most 200ms for USB 2 PR or USB 3 warm reset,
then verify connection, PED, U0, and no overcurrent. Allow 10ms for reset recovery.
Select control bits carefully when writing PORTSC; do not unintentionally write back PED or W1C bits.

### Addressing and EP0

Process Enable Slot, DCBAA registration, Slot / EP0 input contexts, and Address Device in that order.
The initial EP0 packet size is 8 bytes for Low / Full-speed, 64 for High-speed, and 512 for SuperSpeed.
After Address Device completion, verify Addressed state in the output context and an address of 1–127.

Read the first eight bytes with GET_DESCRIPTOR and validate length 18, Device type, and packet rules.
If the packet size changes, update EP0 with Evaluate Context.
Then read all 18 bytes and check VID / PID and the configuration count.
One request consists of three TRBs: Setup / Data IN / Status OUT.
Publish later stages first, handing over Setup last.
Issue only one transfer at a time. Validate the Status Transfer Event's pointer,
slot, EP ID, Success, and residual 0.
EP0 uses per-slot producer/cycle state. Validate short Data Stage residuals and still wait for Status; exact-length callers reject short replies. Unknown events remain failures.
Retain keyboard and parent-hub slots as needed. Release children before parents, then clear DCBAA entries.

### Shutdown and limitations

Bound every wait by time and poll count. Transfer waits normally allow 1000ms.
On both success and failure, stop the controller and disable PCI bus mastering.
Do not free or reuse DMA regions, even after partial failure.
Proceed to the application's RAM test after diagnostics.

[Boot Keyboard diagnostics](usb-keyboard.md) add Configuration Descriptor,
Set Configuration, and Interrupt IN.
[Mass Storage BOT read diagnostics](usb-storage.md) have also been added.
USB2 hub boot traversal and upstream keyboard-disconnect monitoring are implemented; see [scope and validation](usb-multi-controller-hub-plan.md). USB3.0/5Gbps hub initialization is implemented but its wire transfers are unverified. USB3.1/3.2 hubs and dynamic attach/re-enumeration remain unsupported.
Device information alone does not establish working keyboard input or storage reads.

### Validation

Verified the following with QEMU q35 / qemu-xhci:

- USB storage: SuperSpeed, VID:PID 46F4:0001.
- Keyboard: High-speed and Full-speed, VID:PID 0627:0001.
- 600 No-Op commands, enumeration of two devices, controller shutdown, DMA disablement, and RAM application completion.
- Injected nonresponse for the 18-byte transfer, followed by a 20ms timeout, shutdown, DMA disablement, and application completion.
- Host tests for packet / context boundaries, malformed Descriptors, completion events, and PORTSC writes.

Low-speed, custom PSI, CSZ=64, Evaluate Context after an EP0 packet change,
and NUC5 / NUC8 hardware remain untested.
Do not declare hardware support or completion of 0.1.0.

Primary source: [Intel xHCI 1.2b](https://cdrdv2-public.intel.com/625472/625472_xHCI_Rev1_2b.pdf)
§4.3, 4.6, 4.8, 6.2, 6.4, 7.2.

---

## 日本語

**USB列挙診断**

### 範囲

UEFI終了後、自前xHCIドライバで起動時の直結機器と対応USB2ハブ配下の機器を順に調べる。
Supported Protocol capabilityのUSB 2 / USB 3ポート範囲、重複、BAR境界を検査し、
定義のないポートは操作しない。速度は既定IDまたは対称PSIから判定する。
非対称PSI、未知速度、探索件数・同時slotの上限を超える接続は診断失敗とする。
32 / 64byte contextをHCのCSZに合わせる。

ポート電源を確保し、接続を100ms debounceする。USB 2はPR、USB 3はwarm resetを
最大200ms待ち、接続・PED・U0・過電流なしを確認する。10msのreset recoveryを置く。
PORTSCへの書込みは制御ビットを選別し、PEDやW1Cを意図せず書き戻さない。

### アドレスとEP0

Enable Slot、DCBAA登録、Slot / EP0 input context、Address Deviceの順に処理する。
初期EP0 packetはLow / Full-speedで8、High-speedで64、SuperSpeedで512byte。
Address Deviceの完了後、output contextのAddressed状態と1〜127のaddressを確認する。

GET_DESCRIPTORで先頭8byteを読み、長さ18・Device型・packet規則を検査する。
packetサイズが変わる場合はEvaluate ContextでEP0を更新する。
次に18byte全体を読み、VID / PIDとconfiguration数を確認する。
Setup / Data IN / Status OUTの3 TRBを一要求とし、後段から公開して最後にSetupを渡す。
一度に一転送だけ発行し、StatusのTransfer Eventのpointer、slot、EP ID、
Success、residual 0を検査する。EP0はslot別producer／cycleを使う。短いData応答のresidualと最後のStatusを検査し、所定長の要求では不足を拒否する。未知イベントは失敗とする。
必要なkeyboardと親hub slotを保持し、子から親の順にDisable Slotを行いDCBAA entryを消す。

### 終了と制約

すべての待機に時間とpoll回数の上限を置く。転送待ちは通常1000ms。
成功・失敗ともcontrollerを停止してPCI bus masteringを解除する。
DMA領域は途中失敗でも解放・再利用しない。診断後にアプリのRAM試験へ進む。

[Boot Keyboard診断](usb-keyboard.md)でConfiguration DescriptorとSet Configuration、
Interrupt INを追加した。[Mass Storage BOT読出し診断](usb-storage.md)も追加した。
USB2ハブの起動時列挙と上流portの切断監視を追加した。[設計と検証](usb-multi-controller-hub-plan.md)を参照。USB3.0／5Gbps hub初期化は実装済み・実転送未検証。USB3.1／3.2 hubと動的な接続・再列挙は未対応。機器情報の確認をもって
キーボード入力やストレージ読出しが動くとは判定しない。

### 検証

QEMU q35 / qemu-xhciで以下を確認した。

- USBストレージ: SuperSpeed、VID:PID 46F4:0001。
- キーボード: High-speedとFull-speed、VID:PID 0627:0001。
- No-Op 600回、2機器の列挙、controller停止、DMA無効化、RAMアプリ完了。
- 18byte転送の応答停止を注入し、20ms timeout後の停止・DMA無効化とアプリ完了。
- ホスト試験でpacket / context境界、不正Descriptor、完了event、PORTSC書込みを検査。

Low-speed、独自PSI、CSZ=64、EP0 packet変更時のEvaluate Context、
NUC5 / NUC8実機は未検証。実機対応や0.1.0完成は宣言しない。

一次資料: [Intel xHCI 1.2b](https://cdrdv2-public.intel.com/625472/625472_xHCI_Rev1_2b.pdf)
§4.3、4.6、4.8、6.2、6.4、7.2。
