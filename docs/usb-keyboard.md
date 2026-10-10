# USB Boot Keyboard diagnostics

## English

### Configuration and input

After obtaining the Device Descriptor, read the first nine bytes and then the full configuration at index 0.
Limit it to 1024 bytes. Validate descriptor lengths, total length, boundaries,
HID descriptors, endpoint count for the target interface, packet size, and interval.
Select a Boot Keyboard with alt setting 0, class 3, subclass 1, and protocol 1.
Allow one input candidate per configuration; multiple candidates are a failure.
Only enumerate nontarget devices; do not Set Configuration for them.

Target directly connected Low / Full / High-speed keyboards.
Register the Interrupt IN endpoint with Configure Endpoint.
Use DCI=endpoint number*2+1, CErr=3, MaxBurst=0, and DCS=1.
HS interval is bInterval-1; FS / LS interval is floor(log2(bInterval))+3.
Maximum packet sizes are 8 bytes for LS, 64 for FS, and 1024 for HS;
high-bandwidth additional transactions are unsupported.
Send Set Configuration and Set Protocol(Boot) through EP0.

Read eight-byte Boot Reports using Interrupt IN Normal TRBs.
Poll the event ring without CPU interrupts.
Check pointer, slot, DCI, Success, and residual 0.
Short reports, stalls, and similar conditions are diagnostic failures.
On rollover (usage 1–3), retain previous key state without generating incorrect releases.
Report presses / releases for modifiers and up to six ordinary usages.
Duplicate usages or reordering must not generate extra events. Ignore the reserved byte.

### Diagnostic lifetime

After KEYBOARD READY, the normal build continues input until the application receives Esc.
A build with only `qemu-debug` ends after ten seconds for smoke testing.
`input-persistent` retains debug output while checking continuous input as in the normal build.
Display HID usages on key presses in the application screen;
log presses / releases at both device and application levels in debug output.
The usage line is visible on screens at least 388 pixels high.
Input polling returns immediately after one event check; application steps run even without input.
Do not wait on time; a NAKed transfer does not prevent application execution by the CPU.
Submit one transfer at a time and do not reuse the ring / report until completion.
Update the Link cycle to wrap around the 255 usable TRBs.
IRQ enablement and CPU sleep are not implemented yet.

On exit, Disable Slot and discard pending IN transfers.
Do not reuse DMA regions before completion.
After controller shutdown and bus-mastering disablement, proceed to the RAM diagnostic application.
Pass input to the [API version 3 FIFO](app-api.md).
This is a single-keyboard input session; simultaneous polling of multiple keyboards is unsupported.
Character mapping, JIS / US layout conversion, repeat, LED control, USB3.1/3.2 hubs, hotplug re-enumeration,
SuperSpeed keyboards, other-configuration discovery, and generic Report Descriptor parsing are unsupported.

USB2 hub paths and upstream disconnect monitoring are described in the [hub implementation plan](usb-multi-controller-hub-plan.md).

### Validation

With QEMU usb-kbd at High-speed / Full-speed, send Shift+A press / release through QMP
and check both directions of usage E1 and 04 notifications.
A five-second idle test confirmed RAM diagnostic progress and application completion after DMA shutdown.
Continuous-input mode verified 160 additional presses / releases, 325 reports,
transfer-ring wraparound, application termination with Esc, and DMA shutdown.
Host tests check malformed configuration boundaries, packets, nontarget interfaces,
rollover, duplicate keys, endpoint IDs, and short-report rejection.

NUC5 hardware, Low-speed, CSZ=64, and physical composite keyboards remain untested.
See [development instructions](development.md) for test procedures
and [NUC5 tests](nuc5-bringup.md) for hardware preparation.

Primary sources: [USB HID 1.11](https://www.usb.org/sites/default/files/hid1_11.pdf)
§7.2.6, Appendix B / F,
[Intel xHCI 1.2b](https://cdrdv2-public.intel.com/625472/625472_xHCI_Rev1_2b.pdf)
§4.3, 4.6.5, 4.11, 6.2.3.

---

## 日本語

**USB Boot Keyboard診断**

### 設定と入力

Device Descriptor取得後、configuration index 0の先頭9byteと全体を読む。
最大1024byte。descriptor length、total length、境界、HID descriptor、
対象interfaceのendpoint数、packetとintervalを検査する。
alt setting 0、class 3、subclass 1、protocol 1のBoot Keyboardを選択する。
一configuration中の入力候補は一つに限定し、複数候補は失敗とする。
非対象機器は列挙のみ行い、Set Configurationしない。

直結Low / Full / High-speedを対象とし、Interrupt IN端点をConfigure Endpointで
登録する。DCI=endpoint number*2+1、CErr=3、MaxBurst=0、DCS=1。
HS intervalはbInterval-1、FS / LSはfloor(log2(bInterval))+3。
packetはLS最大8、FS最大64、HS最大1024byte、高帯域追加transactionは非対応。
Set ConfigurationとSet Protocol(Boot)をEP0へ送信する。

Interrupt INのNormal TRBで8byte Boot Reportを読む。CPUの割込みは使わず、
event ringをpollする。pointer、slot、DCI、Success、residual 0を確認する。
short reportやstall等は診断失敗とする。rollover(usage 1〜3)は以前のキー状態を
保持し、誤った解放を生成しない。modifierと最大6個の通常usageについて押下・解放を
通知し、重複usageと並び替えで余分な通知を生成しない。reserved byteは無視する。

### 診断の寿命

通常版はKEYBOARD READY表示後、アプリがEscを受信するまで入力を続ける。
`qemu-debug` 単独ビルドはsmoke用に10秒で終了する。
`input-persistent` featureはdebug出力を残して通常版と同じ継続入力を検査する。キー押下時のHID usageを
アプリの画面へ表示し、デバッグ出力にはデバイスとアプリ双方の押下・解放を記録する。
usage表示行は高さ388pixel以上の画面で確認できる。
入力pollは一回のevent検査で即座に復帰し、無入力中にもアプリstepを実行する。
時間待ちを使わず、転送がNAK中でもCPUからのアプリ実行を妨げない。
一転送ずつ提出し、完了までring / reportを再利用しない。255 usable TRBの
Link cycleを更新して周回する。IRQ有効化やCPUのsleepはまだ行わない。

終了時にDisable Slotし、未完了のIN転送も破棄する。完了前のDMA領域を再利用しない。
controller停止・bus mastering解除後、RAM診断アプリへ進む。
入力は[API版3のFIFO](app-api.md)へ渡す。単一キーボードの入力セッションであり、
複数キーボードを同時にpollする処理は未対応。
文字配列、JIS / US配列変換、repeat、LED制御、USB3.1／3.2ハブ、hotplug再列挙、
SuperSpeedキーボード、他configurationの探索、汎用Report Descriptor解析は未対応。

[協調I/O](cooperative-io.md)も参照。全device初期化後にkeyboard pollを開始し、その合間にNIC通信とsnapshot file readを進める。キーボード切断を検知したらUSB DMAを停止し、残る処理を継続する。

USB2ハブ経路と上流切断監視は[ハブ実装計画](usb-multi-controller-hub-plan.md)を参照。

### 検証

QEMU usb-kbdのHigh-speed / Full-speedで、Shift+Aの押下・解放を
QMPで送信し、usage E1と04の両方向通知を検査した。
5秒の無入力試験でもRAM診断が進み、DMA停止後にアプリが完了することを確認した。
継続入力モードで160回の追加押下・解放、325report、transfer ring周回、
アプリによるEsc終了とDMA停止を確認した。
ホスト試験は不正configuration境界、packet、非対象interface、
rollover、duplicate key、endpoint IDとshort report拒否を確認する。

NUC5実機、Low-speed、CSZ=64、実物の複合キーボードは未検証。
試験方法は[開発手順](development.md)、実機準備は[NUC5試験](nuc5-bringup.md)を参照。

一次資料: [USB HID 1.11](https://www.usb.org/sites/default/files/hid1_11.pdf)
§7.2.6、Appendix B / F、
[Intel xHCI 1.2b](https://cdrdv2-public.intel.com/625472/625472_xHCI_Rev1_2b.pdf)
§4.3、4.6.5、4.11、6.2.3。
