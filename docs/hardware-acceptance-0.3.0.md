# v0.3.0 hardware observations

## English

### Candidate 4319b58 NUC5 cold boot failure (owner report, 2026-10-13)

EFI SHA-256 `41a61efcd96edf6142bf81a37ae2ca90ca4396ea71386da698f8cac8bccd2ae8`, associated with `out/release-display/physical-usb/provenance.json`. Owner reports a cold boot with both USB keyboard and SanDisk attached but no xHCI port detecting a device. This is a failed candidate acceptance attempt, not three successful cold boots. `IMG_7606.mov` is 1.975 seconds; sampled frames show the runtime/CORE READY stage, not the later PORTSC inventory. Thus the all-ports-disconnected detail is the owner's report, not independently visible in this short clip. Frames: `out/intel-routing/video/`. Intel shared-port routing is a hypothesis, not a confirmed cause. This result supersedes “no attempt yet” for this EFI; earlier entries below retain their historical state.

### New boot-display candidate — hardware NOT PERFORMED (2026-10-13)

Source `4319b58dda03400032c0307eaaa3b85fe8150ac0`; release profile, features=[]; EFI SHA-256 `41a61efcd96edf6142bf81a37ae2ca90ca4396ea71386da698f8cac8bccd2ae8` (455,680 bytes). Manifest: `out/release-display/candidate/build-manifest.json`. This replaces the candidate for the next acceptance session, not the historical observations below. QEMU full regression passed 186 cases (`out/release-display/report.json`); USB EFI-only deployment, readback and safe eject completed (`out/release-display/physical-usb/provenance.json`). NUC5 and Ryzen three cold boots each, and MacBook Pro 2018 Basic Data boot, must be performed after the new USB update. No successful physical boot of this EFI is recorded yet.

### Current development observations (owner report, 2026-10-13)

These results are the owner's supplied interpretation by Claude of the videos below. EFI SHA-256 for the prepared Basic Data USB: `b743ac0129a9692adf865c37c33adb2c23ca2c0238baf5b9437ebe20faee2b1a`. Each session has a retrospective provenance record under `docs/hardware-evidence/2026-10-13/{nuc5,ryzen7,macbook-pro-2018}/provenance.json`, linked to `out/physical-basic-data-a3153d2/result.json` and its verified file update. The videos do not independently attest the hash; this is the deployment/session association, not a fresh EFI measurement.

| Machine / evidence | Observations | Acceptance boundary |
|---|---|---|
| NUC5i5RYH / IMG_7602.mov | GOP 1600/1200/1600; PCI E, XHCI 1, Intel LAN 1; TIMER READY; IMAGE 0x14000000 SIZE 0x10C8000; TABLE 0x9D170000; MMIO 0 BASE 0xAA120000. MUSHA.ELF → Hello Musha-OS! → exited: status 0. Diagnostic overlap resolved. Boot menu now shows `UEFI : USB : USB : PART 0 : OS Bootloader`. | Ring 3 Hello success. Board/CPU details, RAM, firmware, Secure Boot and topology remain UNKNOWN pending owner response. Old development EFI had two No medium observations; not reproduced on this current build. |
| Ryzen 7 / IMG_7603.mov | GOP 3840/2160/3840; PCI 34, XHCI 5, Intel LAN 0; USB READ OK; USB BYTES 72AE00000; MMIO 4 BASE 0xFCC00000. Hello → exit 0 reported. | Ring 3 Hello success reported; final video is blurred, still-image supplement pending. Native-scale console is too small at 4K. Exact machine details remain unrecorded. |
| MacBook Pro 2018 / IMG_7604 2.mov, IMG_7605.MOV | GOP 3360/2100/3584; PCI 23, XHCI 3, Intel LAN 0; MMIO 2 BASE 0x8F900000. External keyboard attached: KEYBOARD READY. Without it: USB keyboard not detected. Both show No medium / No supported FAT32/exFAT medium. | Boot/display and external-keyboard detection only; no external Hello success. Internal keyboard unsupported because T2 is unimplemented. Storage cause unknown, deferred to v0.4.0 or later by owner. Historical 10/12 rc1 FAT32 success is not the current result. |

**Cold boot ×3: NOT PERFORMED; repeat on the public candidate EFI on NUC5 and Ryzen.** Mac candidate acceptance requires Basic Data boot only, not storage recovery. Current development observations do not satisfy that future candidate gate. Network detection is not physical network communication. See [open issues](open-issues-0.3.0.md).

### Historical earlier development EFI (before GOP/Basic Data)

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

### 候補4319b58のNUC5冷間起動失敗（2026-10-13、所有者報告）

EFI SHA-256 `41a61efcd96edf6142bf81a37ae2ca90ca4396ea71386da698f8cac8bccd2ae8`、配備記録は `out/release-display/physical-usb/provenance.json`。USBキーボード・SanDisk接続済みでもxHCI全ポートが未検出だったとの所有者報告です。候補受入の失敗1回として記録し、冷間起動3回成功には数えません。提供動画 `IMG_7606.mov` は1.975秒で、抽出フレームはruntime/CORE READY段階です。全ポート未検出という詳細はこの動画で独立確認できず、所有者報告として区別します。Intelルーティングは未確定の仮説です。このEFIの「未試行」という旧記述は本追記で更新し、以下の過去の記録は保持します。

### 新しい起動表示の候補 — 実機未実施（2026-10-13）

ソース `4319b58dda03400032c0307eaaa3b85fe8150ac0`、release構成・features=[]。EFI SHA-256 `41a61efcd96edf6142bf81a37ae2ca90ca4396ea71386da698f8cac8bccd2ae8`（455,680 byte）。manifestは `out/release-display/candidate/build-manifest.json`。次の受入対象をこの候補へ変更します。以下の過去の実機観測は変更しません。QEMU全回帰186件合格（`out/release-display/report.json`）。USBのEFIのみの更新・読み戻し・安全な取外しを完了しました（`out/release-display/physical-usb/provenance.json`）。更新完了後にNUC5・Ryzen各3回の冷間起動とMacBook Pro 2018のBasic Data起動を行う必要があり、このEFIの実機成功はまだ記録していません。

### 現行開発版（2026-10-13、所有者報告）

以下は所有者が提供した、Claudeによる動画読取り結果です。配備済みBasic Data USBのEFI SHA-256は `b743ac0129a9692adf865c37c33adb2c23ca2c0238baf5b9437ebe20faee2b1a`。各回の `docs/hardware-evidence/2026-10-13/{nuc5,ryzen7,macbook-pro-2018}/provenance.json` に配備記録との紐付けを保存しました。動画からハッシュを測定したものではなく、配備内容と所有者の実行報告を事後に対応付けた記録です。

- **NUC5：Ring 3 Hello成功**。IMG_7602.mov、GOP 1600/1200/1600。PCI E、XHCI 1、LAN 1、TIMER READY。IMAGE 0x14000000 / SIZE 0x10C8000、TABLE 0x9D170000、MMIO 0 / BASE 0xAA120000。Hello→終了0、診断パネルの重なり解消、UEFI USBブート項目の表示を確認。RAM・UEFI版等のUNKNOWNは所有者回答待ちのままです。
- **Ryzen：Ring 3 Hello成功の報告**。IMG_7603.mov、GOP 3840/2160/3840。PCI 34、XHCI 5、LAN 0、USB READ OK、USB BYTES 72AE00000、MMIO 4 / BASE 0xFCC00000。終了0の最終映像はぼけており静止画補足待ち。4Kの等倍文字は小さすぎるためA1で改善します。
- **Mac：起動・外付けキーボード検出まで**。IMG_7604 2.mov・IMG_7605.MOV、GOP 3360/2100/3584。PCI 23、XHCI 3、LAN 0、MMIO 2 / BASE 0x8F900000。外付け接続時KEYBOARD READY、未接続時USB keyboard not detected。双方No medium／No supported FAT32/exFAT medium。内蔵キーボードはT2未実装のため未対応。10/12 rc1のFAT32成功とは区別し、退行か接続構成差かは未特定。ストレージ調査は所有者判断で0.4.0以降へ先送りします。

**冷間起動3回：未実施（公開候補EFIでNUC5・Ryzen各3回を行う）**。Macの候補確認はBasic Data起動のみです。旧開発版のNUC5 No mediumは2回観測、現行版では未再現。LAN検出は実機通信成功ではありません。[残件一覧](open-issues-0.3.0.md)を参照してください。

### 以下はGOP／Basic Data対応前の履歴

2026-10-13受領。所有者の機種報告は **NUC5i5RYH** です。**外部Helloの実行成功を1回確認しましたが、実機受入は未完了です。** 2回の試行が報告されており、冷間起動3回の連続成功ではありません。電源断／リセット方法、RAM、UEFI版、接続ポート、キーボード型番は未記録です。

- 1回目（`IMG_7596.HEIC`）：ビューアーに`No medium | 0 files`、`USB keyboard not detected. Connect one, then restart.`。外部Helloへ到達していません。
- 2回目（`IMG_7600.jpg`）：`MUSHA.ELF`、`Hello Musha-OS!`、`MUSHA.ELF exited: status 0`を確認しました。
- 動画（`IMG_7598.MOV`、11.67秒）：抽出した開始フレームでUEFI USB選択、中盤・終盤でHelloと終了0を確認しました。起動時間の計測には使用しません。

使用予定として配備した通常版EFI・Helloのハッシュ、ソースcommit、USB容量は英語表に記載しています。写真からハッシュを実測したものではありません。配備証拠は`out/nuc5-0.3.0-35ea933/`、原本は所有者のDownloads、確認用変換は`out/nuc5-font-review/`です。全容量イメージ作成・全容量ハッシュは行っていません。

1回目はOS側に利用可能な媒体がなく、キーボードも未検出という表示です。ELF不正や「ファイルがないことを正常読出しで確認した」とは解釈しません。USB列挙・所有権移行・媒体初期化や接続構成のどこで止まったかは、この写真だけでは不明です。同じ接続・同じUSBのままで電源断からの起動と再起動を区別して記録し、失敗時の全画面を保存することが次の確認です。実機キーボード操作、exFAT、ネットワーク、例外試験は今回未確認です。

文字サイズの指摘を受け、外部コンソールだけ2倍から等倍（6×8）へ変更しました。80桁×24行、折返し・代替文字・ABI、従来ビューアーは維持しています。変更版は**今回の実機成功版とは別**で、未commit・未push・USB未更新です。ホスト10件と通常版QEMU7件の結果を上記のパスに保存しました。QEMU結果を実機の追加成功回数には数えません。
