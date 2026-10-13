# v0.3.0 hardware observations — NUC5i5RYH

## English

Received 2026-10-13 JST from the owner. **One successful external Hello execution is observed; hardware acceptance is incomplete.** Two attempts were reported, not three successful cold boots. The exact power-off/reset procedure is not yet recorded.

| Field | Observation |
|---|---|
| Machine | NUC5i5RYH (owner report); exact board/CPU, RAM, firmware version UNKNOWN |
| Firmware / display | Video begins with UEFI USB boot selection; Dell display visible. Secure Boot state, cable and GOP resolution UNKNOWN |
| USB / sector size | Prepared SanDisk 3.2Gen1, 30,784,094,208 bytes, 512-byte sectors; actual port/hub topology UNKNOWN |
| Keyboard | Model and connection during each attempt UNKNOWN; input in Musha-OS not verified |
| Source / build | Prepared source `35ea93371d8a3fc4af5f2ea82d7f9c1be6c97ae7`, `release/0.3.0`, clean build, release profile, features=[] |
| EFI | 458,752 bytes; SHA-256 `fb54af2af871bd79e4dd3803c37308c686fa2e2fc3fac11299c0150a09bc9f13` |
| MUSHA.ELF | SDK C Hello, 13,312 bytes; SHA-256 `d60006f88e1e36409867546df31c778cf3d5fb7c077fba376ebb5bd3fbb2810f` |
| Deployment | Changed-file update, old EFI backed up, file readback verified and safely ejected. `out/nuc5-0.3.0-35ea933/provenance.json`, `verified-files.json`, `usb-backup/update-result.json` |
| Whole USB hash / image | Not generated or hashed; existing GPT/FAT32 retained |

The hashes identify the prepared USB files, not values measured from the photographs.

| Attempt / check | Result | Evidence |
|---|---|---|
| First reported attempt | FAIL to reach external Hello | `IMG_7596.HEIC`: viewer shows `No medium | 0 files` and `USB keyboard not detected. Connect one, then restart.` |
| Second reported attempt | PASS external file → Hello output → exit 0 | `IMG_7600.jpg`: `MUSHA.ELF`, `Hello Musha-OS!`, `MUSHA.ELF exited: status 0` |
| Video observation | Supports second result | `IMG_7598.MOV`, duration 11.67 s; sampled start shows UEFI USB selection, middle and end show Hello/exit 0 |
| Cold boot ×3 / repeatability | NOT COMPLETE | First and second outcomes differ; no three-success record |
| Keyboard input / exFAT / network / fault injection | NOT RUN for this hardware session | Do not infer these from Hello |

Original evidence is under the owner's `/Users/hirofumiiwasaki/Downloads/`; local viewing conversions are under `out/nuc5-font-review/` (not committed). The video was inspected at sampled frames, not used as a boot-time benchmark.

First-attempt diagnosis: the OS viewer had no usable medium and did not report a keyboard. This is not evidence of an ELF validation failure or of a successful filesystem read with an absent ELF. USB enumeration/controller ownership/media setup and physical topology remain possible investigation areas; this screen alone does not identify the failing layer. Do not claim a fix or stable NUC support. Next record: unchanged USB/keyboard/port configuration, whether cold or warm boot, outcome for each attempt, RAM and firmware details, and a full photograph on failure.

### Font adjustment after the observed build

The owner requested smaller log text. The local follow-up changes only external-console rendering from scale 2 to native scale 1 (6×8 cells); the 80×24 model, wrapping, replacement bytes and ABI remain unchanged. Existing viewer rendering is unchanged. This smaller-font build is **not the hardware-tested build above**, is not written to USB, and is not yet committed/pushed.

Local validation: `out/nuc5-font-review/host-tests.log` (10 framebuffer tests), `report.json` (seven normal-EFI QEMU cases: Hello, long log, UTF-8 replacement, exit 42, PF, UD, absent viewer). The absent case retains the existing pixel reference and zero debug bytes. These are local checks, not additional physical boots.

## 日本語

2026-10-13受領。所有者の機種報告は **NUC5i5RYH** です。**外部Helloの実行成功を1回確認しましたが、実機受入は未完了です。** 2回の試行が報告されており、冷間起動3回の連続成功ではありません。電源断／リセット方法、RAM、UEFI版、接続ポート、キーボード型番は未記録です。

- 1回目（`IMG_7596.HEIC`）：ビューアーに`No medium | 0 files`、`USB keyboard not detected. Connect one, then restart.`。外部Helloへ到達していません。
- 2回目（`IMG_7600.jpg`）：`MUSHA.ELF`、`Hello Musha-OS!`、`MUSHA.ELF exited: status 0`を確認しました。
- 動画（`IMG_7598.MOV`、11.67秒）：抽出した開始フレームでUEFI USB選択、中盤・終盤でHelloと終了0を確認しました。起動時間の計測には使用しません。

使用予定として配備した通常版EFI・Helloのハッシュ、ソースcommit、USB容量は英語表に記載しています。写真からハッシュを実測したものではありません。配備証拠は`out/nuc5-0.3.0-35ea933/`、原本は所有者のDownloads、確認用変換は`out/nuc5-font-review/`です。全容量イメージ作成・全容量ハッシュは行っていません。

1回目はOS側に利用可能な媒体がなく、キーボードも未検出という表示です。ELF不正や「ファイルがないことを正常読出しで確認した」とは解釈しません。USB列挙・所有権移行・媒体初期化や接続構成のどこで止まったかは、この写真だけでは不明です。同じ接続・同じUSBのままで電源断からの起動と再起動を区別して記録し、失敗時の全画面を保存することが次の確認です。実機キーボード操作、exFAT、ネットワーク、例外試験は今回未確認です。

文字サイズの指摘を受け、外部コンソールだけ2倍から等倍（6×8）へ変更しました。80桁×24行、折返し・代替文字・ABI、従来ビューアーは維持しています。変更版は**今回の実機成功版とは別**で、未commit・未push・USB未更新です。ホスト10件と通常版QEMU7件の結果を上記のパスに保存しました。QEMU結果を実機の追加成功回数には数えません。
