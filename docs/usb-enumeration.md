# USB列挙診断

## 範囲

UEFI終了後、自前xHCIドライバで起動時の直結機器を順に調べる。
Supported Protocol capabilityのUSB 2 / USB 3ポート範囲、重複、BAR境界を検査し、
定義のないポートは操作しない。速度は既定IDまたは対称PSIから判定する。
非対称PSI、未知速度、上限8機器を超える接続は診断失敗とする。
32 / 64byte contextをHCのCSZに合わせる。

ポート電源を確保し、接続を100ms debounceする。USB 2はPR、USB 3はwarm resetを
最大200ms待ち、接続・PED・U0・過電流なしを確認する。10msのreset recoveryを置く。
PORTSCへの書込みは制御ビットを選別し、PEDやW1Cを意図せず書き戻さない。

## アドレスとEP0

Enable Slot、DCBAA登録、Slot / EP0 input context、Address Deviceの順に処理する。
初期EP0 packetはLow / Full-speedで8、High-speedで64、SuperSpeedで512byte。
Address Deviceの完了後、output contextのAddressed状態と1〜127のaddressを確認する。

GET_DESCRIPTORで先頭8byteを読み、長さ18・Device型・packet規則を検査する。
packetサイズが変わる場合はEvaluate ContextでEP0を更新する。
次に18byte全体を読み、VID / PIDとconfiguration数を確認する。
Setup / Data IN / Status OUTの3 TRBを一要求とし、後段から公開して最後にSetupを渡す。
一度に一転送だけ発行し、StatusのTransfer Eventのpointer、slot、EP ID、
Success、residual 0を検査する。Short Packetや未知イベントは失敗とする。
完了後Disable Slotを行い、DCBAA entryを消す。

## 終了と制約

すべての待機に時間とpoll回数の上限を置く。転送待ちは通常1000ms。
成功・失敗ともcontrollerを停止してPCI bus masteringを解除する。
DMA領域は途中失敗でも解放・再利用しない。診断後にアプリのRAM試験へ進む。

Configuration Descriptor、Set Configuration、HID入力、Mass Storage BOT、
ハブ、動的な接続・切断への対応は未実装。機器情報の確認をもって
キーボード入力やストレージ読出しが動くとは判定しない。

## 検証

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
