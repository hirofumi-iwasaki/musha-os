# Realtek identity and admission — 2026-10-20

## English

The only physically observed LAN identity in [IMG7653's checked frame](../out/ryzen-7653/observation.json) is `10EC:8168`, revision `0x26`, BDF `07:00.0`. The visible line also says `NET STEP DRIVER UNSUPPORTED / ABSENT`. The recorded EFI association comes from deployment context; the video alone does not prove its binary hash. This is one development boot, not link, DHCP, echo or repeatability evidence.

| Field | State | Admission consequence |
|---|---|---|
| PCI vendor/device `10EC:8168`, revision `0x26`, BDF `07:00.0` | Visually checked at video offset 9 s | Identifies a PCI function only; no MAC/PHY profile selected |
| Subsystem vendor/device, BAR type/base/length, PCI Command | **NOT CAPTURED** | No mapped transport or resource ownership claim |
| TxConfig `0x40` | **NOT CAPTURED** | No revision-dependent silicon profile claim |
| PHY/OCP/firmware identity and stop/drain behavior | **NOT CAPTURED / UNVERIFIED** | No reset, configuration, DMA or reuse permission |

`net/src/realtek.rs` is a committed, independent host model. This checkpoint adds only a pure parser for captured PCI-config bytes and separate TxConfig bytes, plus refusal tests. It requires at least the standard PCI bytes 0–8, exactly four TxConfig bytes, supported PCI ID, a present non-`0xFF` revision and a TxConfig value that is neither zero nor all ones. Absent, truncated, all-ones and unsupported input is rejected as `Identity`; a complete-looking value still returns `UnverifiedSilicon`. `Permit` has no public constructor and no runtime caller. This does not infer a safe `RTL8111H` profile from revision `0x26` or a synthetic TxConfig value. The test bytes are fixtures, not measured hardware values.

The next physical read must preserve raw `devices`/PCI observations and the exact diagnostic EFI, CONFIG, tool, timestamp, video frame, topology and read method. The already described read-only diagnostic requires a verified UC resource and memory decode before reading TxConfig. A failed or invisible line stays **NOT CAPTURED**. BAR sizing writes, arbitrary MMIO scans, reset, PHY/OCP writes, BME or DMA are outside this checkpoint.

Source provenance: the model declares Apache-2.0 and says no upstream code was translated. [The LAN plan](lan-plan-0.4.0.md) lists Linux GPL-2.0-only r8169 register/variant references as factual cross-checks; no Linux source is copied or compiled. Any later adopted profile must record its exact source revision, license obligations and independent implementation review before device access.

## 日本語

[IMG7653の確認済みframe](../out/ryzen-7653/observation.json)で実測したのはPCI `10EC:8168`、revision `0x26`、BDF `07:00.0`のみです。同画面はdriver未対応も示します。EFI hashの対応は配備記録と文脈によるもので、動画単独の証明ではありません。TxConfig `0x40`、subsystem、BAR、PCI Command、PHY/OCP/firmware、停止時drainは**未採取・未検証**です。

今回の純粋なhost parser/試験はPCI設定先頭9byteと別のTxConfig4byteの欠損・短縮、不正値、未対応IDを拒否します。見かけ上揃ったfixtureでも`UnverifiedSilicon`で止まり、実機write許可にはなりません。revision `0x26`だけでRTL8111Hの詳細profileは決めません。正常buildのRealtek有効化も行いません。

次の実機読取りでは生の値、EFI/CONFIG、tool/版、日時、frame、接続と読取り方式を記録します。TxConfigが画面に見えなければ未採取のままです。資源・UC mapping・memory decodeの検証前に任意MMIOを読まず、BAR sizing書込み、reset、PHY/OCP、BME/DMAへ進みません。出典とlicenseの境界は英語節の通りです。
