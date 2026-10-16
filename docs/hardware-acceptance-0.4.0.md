# v0.4.0 hardware observations / v0.4.0実機観測

Status: current (blank acceptance worksheet, 2026-10-15). All gates PENDING; no run performed by this document task.

## Development EFI

### Latest UDP-default-off refresh — 2026-10-16

#### English

Source is fixed commit `2605135dc5efb983caabeb438f5fe39e6c8c937e`, built from a clean working tree. The three development-only locked/release builds below preserve equal before/after compile-source hashes. They include the current ready-before-app console, PAT entry 1 WC transition and FS/GS correction. Each directory includes offline DOCS/licenses and the generated README marker. CONFIG uses `log=trace` and `display=max`; **only I218 adds `lan.echo=on`**. Echo otherwise defaults off while DHCP remains active. No AUTORUN is bundled. These are not deployed USB media or physical results. Older artifacts below keep their original provenance.

| Purpose | Explicit features | EFI SHA256 | Exact local EFI |
|---|---|---|---|
| t2-realtek | `t2-diagnostics,diagnostics-detail` | `02f471ec848dc297ac5c75d55e6059957e2b2a5f21e9f0f8b00f34aa061b9d98` | `/Users/hirofumiiwasaki/Github/musha-os/out/refresh-2605135/development/t2-realtek/files/EFI/BOOT/BOOTX64.EFI` |
| i218 | `i218-network,diagnostics-detail` | `905896848bcb0e50de5296b6b60bbeff47562e4d9b5a00880bfe396f3d0ebd83` | `/Users/hirofumiiwasaki/Github/musha-os/out/refresh-2605135/development/i218/files/EFI/BOOT/BOOTX64.EFI` |
| hci-probe | `bluetooth-hci-probe,diagnostics-detail` | `13507c591387f736c169965ce8831ced054a46c99c6276d4d09bebe924bd6381` | `/Users/hirofumiiwasaki/Github/musha-os/out/refresh-2605135/development/hci-probe/files/EFI/BOOT/BOOTX64.EFI` |

Local exact-EFI QEMU checks passed 7 configurations: T2 diagnostic stop 3, I218 Hello/return 1, HCI diagnostic stop 3. Initial runner-only failures (SDK PATH and HCI debug/production observer selection) are retained separately; corrected invocations passed without source changes. Evidence: `out/refresh-2605135/verification.json`.

Registry/commands/source and CONFIG hashes: `out/refresh-2605135/development/registry.json` and each `manifest.json`. Independent normal candidate generation from two clean checkouts of this same commit passed condition 1: all six files (four distributions, build-manifest and SHA256SUMS) are byte-identical. Evidence: `out/refresh-2605135/condition1/report.json` and `comparison.json`. Both manifests measure EFI size from the packaged PE bytes and retain minimal as null/unavailable because LAN/Bluetooth cannot yet both be excluded. This compares the same local environment twice, not local output against CI; the successful CI run 38882434511 has separately verified artifacts.

Next owner observations: Ryzen trace captures PCI/revision/BAR/TxConfig40 or a safe skip reason, before any MAC-profile/register-write enablement. NUC5 checks no #GP, USB read → ready → AUTORUN and viewer return; I218 must either obtain a real lease or continue with its exact NVM/budget refusal after confirmed stop. Only a real lease with the explicit echo setting permits UDP echo acceptance. The published v0.3.0 formatted-media six-route/machine cold boots remain a separate gate. Mac T2 MMIO/DMA integration, physical Bluetooth and all final hardware acceptance remain pending.

#### 日本語

source は固定commit `2605135dc5efb983caabeb438f5fe39e6c8c937e`、生成開始時の作業ツリーはcleanです。上の開発専用3本は独立locked/release buildの前後compile-source hash一致を確認し、ready→app順序・PAT entry 1 WC切替・FS/GS修正を含みます。各dirにoffline DOCS/licensesと生成READMEマーカーを同梱。CONFIGは全て`log=trace`／`display=max`、**I218だけ`lan.echo=on`を追加**しています。未指定echoはoff、DHCPは継続します。AUTORUNは同梱せず、USB配備・実機結果ではありません。下の旧artifact来歴は保持しています。

exact EFIの限定QEMUは7構成合格（T2診断停止3／I218 Hello復帰1／HCI診断停止3）。初回のSDK PATH不足とHCI debug/production observer選択不一致は実行器の失敗として保持し、source無変更の修正invocationで合格。証拠は`out/refresh-2605135/verification.json`です。

registry・command・source/CONFIG hashは`out/refresh-2605135/development/registry.json`と各`manifest.json`。同commitの独立clean checkout2本から通常候補を生成し、4配布物＋build-manifest＋SHA256SUMSの6ファイル完全一致で条件1合格。`out/refresh-2605135/condition1/report.json`／`comparison.json`が証拠です。manifestのEFIサイズは配布PEから計測し、LAN/Bluetoothを両方除外できないためminimalは理由付きnull/unavailableを維持。同じローカル環境2回の比較であり、別途成果物照合済みのCI run38882434511との環境横断一致を主張しません。

次は所有者のRyzen trace採取（PCI/revision/BAR/TxConfig40又は安全skip理由）で、MAC版選択・register書込み有効化より先です。NUC5は#GP無し、USB読出し→ready→AUTORUN→viewer復帰と、I218実lease又は停止確認後のNVM/budget理由付き継続を確認。UDPecho受入は実lease＋明示on時のみです。公開v0.3.0によるフォーマット3経路×2機種の冷間6回は独立gate。Mac T2実MMIO/DMA、実Bluetooth、最終実機受入は未完です。

### Stage 0 refreshed trace/PAT development EFI — 2026-10-16

#### English

These three **new development-only** binaries come from compile source commit `77184002b07acbcc200520ef8487d2c387c724da` (including the FS/GS correction, single retained console, ready-before-app and verified PAT entry 1 WC transition). Each isolated locked/release build records equal before/after compile-source hashes. Documentation-only working-tree edits are listed separately in the manifests. CONFIG is `log=trace` / `display=max`; CONFIG is a runtime setting, not a feature. The former development EFIs and Parallels artifacts below retain their original hashes and source provenance and are not overwritten. No USB deployment or physical run occurred.

| Purpose | Explicit features | EFI SHA256 | Exact local EFI |
|---|---|---|---|
| t2-realtek | `t2-diagnostics,diagnostics-detail` | `8b3fe1ec3bfc2aaf93c644da7f61e1da956d0b646637fb49c816ff7b1fd419fb` | `/Users/hirofumiiwasaki/Github/musha-os/out/stage0-ready-pat/development/t2-realtek/files/EFI/BOOT/BOOTX64.EFI` |
| i218 | `i218-network,diagnostics-detail` | `3b6e0d1ddb70950805bccaa3c719ba70c5ff43729ca69e3f828841e6e218e157` | `/Users/hirofumiiwasaki/Github/musha-os/out/stage0-ready-pat/development/i218/files/EFI/BOOT/BOOTX64.EFI` |
| hci-probe | `bluetooth-hci-probe,diagnostics-detail` | `2d82f69fbcaa4f02fd201bd0f8d10426b6fa3d1563793eb53358d49106af1cb5` | `/Users/hirofumiiwasaki/Github/musha-os/out/stage0-ready-pat/development/hci-probe/files/EFI/BOOT/BOOTX64.EFI` |

Registry and full commands: `out/stage0-ready-pat/development/registry.json`, each directory's `manifest.json` and `source-before.json`. Local EFI checks passed 7 QEMU configurations: T2 diagnostic stop 3, I218 normal Hello/return 1, and HCI diagnostic stop 3. Evidence: `out/stage0-ready-pat/development-checks.json` and `check-hci-diagnostic/report.json`. An initial HCI run used the ordinary runtime-terminal observer; the healthy diagnostic-session terminal required the existing dedicated diagnostic observer. This was an observer mismatch, not an HCI hardware result. No AUTORUN is bundled; copy the independently built SDK Hello only when following the owner-run application check. The runtime README marker and offline DOCS/licenses accompany each EFI. Source commit identity remains the build-time identity even if these records are committed later.

NUC5 retest: no #GP; required USB/media/keyboard discovery → exact ready → AUTORUN start/output/exit (when installed); I218 either obtains a real DHCP lease or continues nonfatally with a concrete NVM/budget reason after confirmed stop. UDP echo is checked only after an actual lease. Ryzen captures Realtek identity/BAR/TxConfig40 or explicit safe skip at trace; no revision or TxConfig value is assumed. HCI is an explicit Reset/transport probe, not proof of pairing/HOGP interoperability. Realtek runtime writes remain gated. Physical firmware/PAT compatibility, effective WC and speed, I218/RTL hardware and actual Bluetooth remain PENDING. This refresh does not close any acceptance gate.

#### 日本語

この3本は **新しい開発専用 EFI** で、compile source commit は `77184002b07acbcc200520ef8487d2c387c724da`（FS/GS 修正、単一 retained console、app 前の ready、検証付き PAT entry 1 WC 切替えを含む）。個別 target の locked/release build ごとに compile-source の前後 hash 一致を記録した。文書のみの作業 tree 差分は manifest に分けて記録する。CONFIG は `log=trace`／`display=max` で、runtime 設定であり feature ではない。下記の旧開発 EFI と Parallels artifact は元 hash／来歴を保持し上書きしていない。USB 配備・実機実行はしていない。

上表が実ファイル、feature、SHA256 である。 ローカル EFI 確認は T2 診断停止 3、I218 通常 Hello／復帰 1、HCI 診断停止 3 の計 7 QEMU 構成が合格。証拠は `out/stage0-ready-pat/development-checks.json` と `check-hci-diagnostic/report.json`。HCI 初回は通常 runtime の終端観測器を指定し、正常な診断 session 終了と不一致になったため既存専用 observer で確認した。実 HCI 装置の成功結果ではない。全 command／来歴は `out/stage0-ready-pat/development/registry.json` と各 `manifest.json`／`source-before.json`。AUTORUN は同梱せず、所有者の app 確認時に別 build の SDK Hello をコピーする。runtime README marker、offline DOCS／license を同梱する。後続の文書 commit で build 時の source identity を付け替えない。

NUC5 は #GP 無し、必須 USB／媒体／keyboard 探索→exact ready→AUTORUN 開始／出力／終了（配置した場合）を確認する。I218 は実 DHCP lease 取得、又は停止確認後の具体的 NVM／budget 理由付き非致命継続を確認し、実 lease 後だけ UDP echo を行う。Ryzen は trace で Realtek identity／BAR／TxConfig40 又は安全な skip 理由を採取し、revision／TxConfig 値を仮定しない。HCI は明示 Reset／transport probe であり pairing／HOGP 互換の証明ではない。Realtek runtime 書込み gate は閉じたまま。実機 firmware／PAT 適合、実効 WC／速度、I218／RTL 実機、Bluetooth 実接続は PENDING であり、今回再生成で受入 gate を完了しない。

### English

Additional corrected detailed T2/Realtek development EFI (2026-10-16): `out/t2-detail-segment-fix-20261016/files/EFI/BOOT/BOOTX64.EFI`, SHA256 `710cf02f0a669631f81a05d0d1c21690c98f51bd8c1c4055c36b6e2dc39c52fb`, source commit `12a4aa0c1395856d8e5a56959fb6b1803107f86c` plus the recorded uncommitted diagnostic-only Ready-stop correction, features `t2-diagnostics,diagnostics-detail`, CONFIG `log=devices`. Includes the committed FS/GS correction. Unlike the older t2-only59a701b1 artifact, it renders/rotates journal detail pages; the old artifact and its origin remain preserved below. RTL-specific values are still physically unmeasured. See [capture instructions](ryzen-realtek-identification.md).

### Appended NUC5 observation and corrected development EFI — 2026-10-16

`IMG_7654.MOV` with the prior `268b9a49…` I218 EFI is **FAIL**, not acceptance: software-NVM configuration refused; later separate #GP (VECTOR D, ERROR18h, RIP140056D89h) during FS restore. No Hello exit/viewer return or DHCP/echo is established; cold-off/AC conditions are unverified. Original worksheet rows and evidence remain unchanged. See [exact diagnosis](nuc5-segment-recovery-0.4.0.md).

Corrected development-only EFI: `out/i218-segment-fix-20261016/files/EFI/BOOT/BOOTX64.EFI`, SHA256 `2275d088fe7bebedab61556cced9ff705cff309c4c6dae1cd800ad553f7fcf53`, `i218-network` only, `log=devices`, HEADd44cdab plus recorded uncommitted source. This corrects CPU recovery/timer handling; software-NVM refusal remains and physical retest is PENDING. It does not replace or relabel the three historical EFI identities above. No USB action in this correction task.

These are **three development diagnostics, not the normal release candidate**. All physical/hardware gates below remain PENDING. No USB was written, formatted, mounted or deployed by this preparation. The published v0.3.0 files ZIP used for the separate formatted-media six-run gate is unchanged.

| Purpose / exact development EFI | Requested features | EFI SHA256 |
|---|---|---|
| Ryzen Realtek identification and Mac same-topology storage A/B / `out/mac-local-preparation-040-round3/development-t2-diagnostics/files/EFI/BOOT/BOOTX64.EFI` | `t2-diagnostics` | `59a701b1cbfe4797114a1d8eb02d8e59c12ecdc1ac53549812277de240cd5322` |
| NUC5 I218 link/DHCP/UDP echo / `out/i218-development-040/files/EFI/BOOT/BOOTX64.EFI` | `i218-network` | `268b9a49705dde18fb249378c0ae9ccf0f2de6d6a925c786404b083da4953220` |
| ASUS USB-BT50LE HCI Reset experiment only / `out/asus-hci-development-040/files/EFI/BOOT/BOOTX64.EFI` | `bluetooth-hci-probe` | `5494be940e0d2b8e03c5bc9b4515822cffca985b3379f5776a9f87d43b294304` |

The existing `59a701b1…` EFI was **retained and rehashed**, never regenerated or substituted. Its original manifest records HEAD `fdcd0369fef577e772c5d52b780625673dcf9faa` with dirty source; original source-set SHA256 is `18e147cc41c782e55ce6b382371039928ae18ae23a582074a27acc769b23aebe`. Its original before/after source evidence and `development-t2-diagnostics/build-manifest.json` remain under `out/mac-local-preparation-040-round3/`. Later formatting/commits do not change that artifact's recorded origin. T2-D1 is inventory evidence, not active BCE/internal-input support; use this artifact for the stated Ryzen/Mac diagnostic observations only.

Both new EFIs were built with `cargo build --locked --release --target x86_64-unknown-uefi -p musha-boot --no-default-features --features <feature> --target-dir <separate-output>/target`. Each records HEAD `969ece347a22496be51954ce63d2d2e5308d5f0a`, dirty-source provenance and source-set SHA256 `485d38f756931f7d18d6c0584c8266dc6bb46aacc1bc0777ea6599a059ea7336`, with byte-for-byte source hashes unchanged during each build. Complete per-file hashes, dirty lists, command, build log, manifest and SHA256SUMS are respectively in `out/i218-development-040/` and `out/asus-hci-development-040/`. Source-set hashes describe the file list recorded by each manifest; do not infer cross-build source identity solely by comparing differently scoped historical inventories.

`i218-network` also enables `i218-phy-probe`. **`log=devices` is a SETTINGS/CONFIG.TXT runtime setting, not a build feature**; record the actual media CONFIG bytes/hash before a test. The HCI probe transitively enables `qemu-debug` and is an explicit development transport reset, not the normal Bluetooth pairing/runtime build. It sends HCI Reset without loading firmware; an ASUS reset response does not prove CSR reference-dongle, pairing, Xbox or HOGP compatibility. Record exact USB IDs/revision/class and response/error as the [Bluetooth quickcheck](bluetooth-hardware-quickcheck.md) specifies. Instructions describe owner execution only; the normal candidate identity above remains PENDING.

### Trace follow-up source boundary — 2026-10-16

The following three EFI hashes predate the trace follow-up and retain their original `log=devices` contract. They are not trace-enabled binaries; do not set `log=trace` on them or claim the new F8 behavior. No development EFI or Parallels image was regenerated for this follow-up. A newly built follow-up EFI is required for trace diagnostics.

### Single-console development EFIs — 2026-10-16

These three local development files replace the display implementation for the **next** hardware retest; they do not supersede historical observations or constitute release candidates. Each uses `SETTINGS/CONFIG.TXT` containing `log=devices` and `display=max`. They were built from HEAD `19888ff` plus the recorded boot-log implementation; the source before/after each build matched. Those unchanged compile sources were subsequently committed as `c141f04`; documentation changes do not relabel the original dirty-build provenance. Full commands, compile-source digest and CONFIG/README hashes are in `out/bootlog-development-20261016/registry.json`. The HCI probe transitively enables the development debug port; the other two requested feature sets do not. No USB write or physical execution was performed.

| Purpose / 用途 | Features | EFI SHA256 | Local development directory / ローカル開発dir |
|---|---|---|---|
| t2-realtek | `t2-diagnostics,diagnostics-detail` | `017a896ee047d7c8515985c154a27dc7b180595049962e9f650c7b5b99f1380c` | `out/bootlog-development-20261016/t2-realtek/files` |
| i218 | `i218-network,diagnostics-detail` | `65a0a0b00e2391915aa1e66e9d4d13e8584a897aa485b7a47d13d1e20d0887da` | `out/bootlog-development-20261016/i218/files` |
| hci-probe | `bluetooth-hci-probe,diagnostics-detail` | `cf9ca285a8705e09172d82ce202fc89b477753ed3d4f027c54734747ab8ef803` | `out/bootlog-development-20261016/hci-probe/files` |

NUC5: first verify no #GP, USB read → AUTORUN → ready. I218 remains opt-in: a link/lease may succeed, or the preserved NVM configuration refusal must continue nonfatally; perform UDP echo only after a real DHCP lease. Ryzen: photograph the sequential log containing Realtek identity/BAR/TxConfig40 **or its safe skip reason**; no actual Realtek value has yet been collected. HCI Reset/transport and Xbox pairing still require physical evidence. PAT WC is used only if firmware already configured a WC slot; otherwise the recorded UC fallback remains, with no PAT write. Keep old EFI hashes when comparing observations.

### 日本語

修正詳細T2/Realtek開発EFI追記（2026-10-16）：`out/t2-detail-segment-fix-20261016/files/EFI/BOOT/BOOTX64.EFI`、SHA256 `710cf02f0a669631f81a05d0d1c21690c98f51bd8c1c4055c36b6e2dc39c52fb`、source commit `12a4aa0c1395856d8e5a56959fb6b1803107f86c`＋記録した診断限定Ready停止の未commit修正、feature `t2-diagnostics,diagnostics-detail`、CONFIG `log=devices`。commit済FS/GS修正を含み、旧t2単独59a701b1版と違いjournal詳細を描画・巡回します。旧artifact・来歴は下記に保持。RTL固有値は実機未採取。[採取手順](ryzen-realtek-identification.md)参照。

### NUC5観測追記と修正開発EFI — 2026-10-16

旧`268b9a49…` I218 EFIによる`IMG_7654.MOV`は**FAIL**です。software-NVM構成拒否と、その後の別#GP（VECTOR D、ERROR18h、RIP140056D89h、FS復元時）を記録。Hello終了/viewer復帰・DHCP/echoは未確認、電源断/AC条件も不明。元表・旧証拠は保持。[正確な診断](nuc5-segment-recovery-0.4.0.md)参照。

修正開発EFIは`out/i218-segment-fix-20261016/files/EFI/BOOT/BOOTX64.EFI`、SHA256 `2275d088fe7bebedab61556cced9ff705cff309c4c6dae1cd800ad553f7fcf53`、`i218-network`のみ・`log=devices`、HEADd44cdab＋記録した未commit差分。CPU復帰/timerを修正し、software-NVM拒否は保持。実機再確認はPENDINGで、上の旧3EFI識別を置換しません。本修正作業でUSB操作なし。

以下は**3本の開発診断EFIで通常release候補ではありません**。下記実機gateは全てPENDING。本準備で実USBの書込み/format/mount/配備は行っていません。別のformatter6runに用いる公開v0.3.0 files ZIPも不変です。

| 用途・開発EFI | 指定feature | EFI SHA256 |
|---|---|---|
| Ryzen Realtek特定・Mac同一接続storage A/B / `out/mac-local-preparation-040-round3/development-t2-diagnostics/files/EFI/BOOT/BOOTX64.EFI` | `t2-diagnostics` | `59a701b1cbfe4797114a1d8eb02d8e59c12ecdc1ac53549812277de240cd5322` |
| NUC5 I218 link/DHCP/UDP echo / `out/i218-development-040/files/EFI/BOOT/BOOTX64.EFI` | `i218-network` | `268b9a49705dde18fb249378c0ae9ccf0f2de6d6a925c786404b083da4953220` |
| ASUS USB-BT50LE HCI Reset別実験限定 / `out/asus-hci-development-040/files/EFI/BOOT/BOOTX64.EFI` | `bluetooth-hci-probe` | `5494be940e0d2b8e03c5bc9b4515822cffca985b3379f5776a9f87d43b294304` |

既存`59a701b1…`は**元fileを保持して再hashしただけ**で再生成/すり替え無し。元HEADは`fdcd0369fef577e772c5d52b780625673dcf9faa`＋dirty source、元source-set SHA256は`18e147cc41c782e55ce6b382371039928ae18ae23a582074a27acc769b23aebe`。元before/after証拠とdevelopment-t2-diagnostics/build-manifest.jsonは`out/mac-local-preparation-040-round3/`へ保持。後続format/commitで出自を付け替えません。T2-D1はinventoryで実BCE/内蔵入力有効化ではなく、記載のRyzen/Mac診断観測に使用します。

新2本はlocked/release/x86_64-unknown-uefi/no-default-featuresで個別feature・target-dirを指定。HEAD`969ece347a22496be51954ce63d2d2e5308d5f0a`＋dirty、source-set SHA256`485d38f756931f7d18d6c0584c8266dc6bb46aacc1bc0777ea6599a059ea7336`を記録し各build前後のsource全file hash一致。`out/i218-development-040/`・`out/asus-hci-development-040/`にfile別hash/dirty一覧/command/build log/manifest/SHA256SUMSを保存。source-set hashは各manifestに列挙した範囲のもので、歴史inventoryと範囲が違えば単純比較で同一sourceと判断しません。

`i218-network`は`i218-phy-probe`も有効化。**`log=devices`はCONFIGの実行時設定でbuild featureではありません**。実媒体CONFIG内容/hashを別途記録。HCI probeは`qemu-debug`を伴う開発transport resetで通常BT pair/runtime buildではなく、firmware投入無しReset応答がCSR基準dongle/pair/Xbox/HOGP対応を証明するものではありません。[BT手順](bluetooth-hardware-quickcheck.md)の実USB ID/revision/class・応答/errorを採取。実行は所有者手順で通常候補identityはPENDINGのままです。

### Trace仕上げのsource境界 — 2026-10-16

次の3EFI hashはTrace追加前であり、元のlog=devices契約を保持します。Trace対応binaryではないためlog=traceを設定したり新F8動作を主張しません。今回仕上げでは開発EFI／Parallels imageを再生成していません。Trace採取には仕上げsource以降の新buildが必要です。

### 単一コンソール開発EFI — 2026-10-16

次の実機再試験用に表示を更新したローカル開発版3本であり、過去の観測を置換せず正式候補でもありません。CONFIGは `log=devices`／`display=max`。HEAD `19888ff`＋記録したbootlog差分から生成し、各build前後source一致を確認しました。同じcompile sourceは後に `c141f04` としてcommitしましたが、文書変更により元dirty buildの来歴は付け替えません。全command・compile-source digest・CONFIG／README hashは `out/bootlog-development-20261016/registry.json`。HCI probeだけは既存推移featureにより開発debug-portが有効です。USB書込み・実機実行はしていません。

| Purpose / 用途 | Features | EFI SHA256 | Local development directory / ローカル開発dir |
|---|---|---|---|
| t2-realtek | `t2-diagnostics,diagnostics-detail` | `017a896ee047d7c8515985c154a27dc7b180595049962e9f650c7b5b99f1380c` | `out/bootlog-development-20261016/t2-realtek/files` |
| i218 | `i218-network,diagnostics-detail` | `65a0a0b00e2391915aa1e66e9d4d13e8584a897aa485b7a47d13d1e20d0887da` | `out/bootlog-development-20261016/i218/files` |
| hci-probe | `bluetooth-hci-probe,diagnostics-detail` | `cf9ca285a8705e09172d82ce202fc89b477753ed3d4f027c54734747ab8ef803` | `out/bootlog-development-20261016/hci-probe/files` |

NUC5は#GP無し、USB読取り→AUTORUN→readyを最初に確認します。I218はopt-inを維持し、link／lease成功又は既存NVM設定要求理由付きの非致命継続を確認し、実DHCP lease成功時だけUDP echoします。RyzenはRealtek identity／BAR／TxConfig40又は安全なskip理由を逐次ログから撮影します。実Realtek値は未採取です。HCI Reset／transportとXbox pairingも実機証拠待ちです。PATは既存WC slotだけ利用し、無ければUC fallback理由を記録します。PAT書換えは行わず、比較時は旧EFI hashも保持します。

## English

### Candidate identity and evidence rules

Normal source commit: PENDING. EFI SHA256: PENDING. Build features: PENDING. Asset/member manifest: PENDING. CONFIG exact bytes/hash: PENDING. Boot/data USB model/serial/bytes/sector/filesystem/partition: PENDING. Firmware/machine/ports/adapters/hubs/monitor: PENDING. No previous-version hash or historical D8 result is silently reused as v0.4 evidence. Cold means ≥10 seconds powered off; record AC connected/disconnected, run ordinal/time, exact media and topology. Distinguish owner report from independently visible evidence; unknown is PENDING, not PASS.

### Gate worksheet

| Gate / 確認項目 | Status / 状態 | Evidence / 証拠 |
|---|---|---|
| QEMU / exact candidate regression | PENDING | — |
| Condition1: two clean same-source full-asset builds reproduce | PENDING | — |
| Condition2: accepted-runtime equality across documentation successor; otherwise renewed acceptance | PENDING | — |
| NUC5 cold1 / cold2 / cold3 (≥10s off, record AC) | PENDING | — |
| Ryzen cold1 / cold2 / cold3 and hot1 with BT dongle | PENDING | — |
| OS-owned SETTINGS write → reboot → read-back; CONFIG untouched | PENDING | — |
| macOS and Windows mount/filesystem verification after OS writes | PENDING | — |
| Rust Hello on NUC5 and Ryzen | PENDING | — |
| USB mouse sample on NUC5 and Ryzen | PENDING | — |
| Viewer launch/return ×3 without reboot | PENDING | — |
| Bluetooth Xbox1914 pairing, durable save, reconnect and SYS_INPUT | PENDING | — |
| NUC5 I218 link/DHCP/UDP12345 echo | PENDING | — |
| Ryzen Realtek exact-profile link/DHCP/UDP echo | PENDING | — |
| Mac same-topology storage A/B and FAT32/exFAT read or bounded cause/limit | PENDING | — |
| Mac T2 internal keyboard/viewer and external regression | PENDING | — |
| Documentation stage2 / formatter gate / index and links / docs.zip decision | PENDING | — |
| Final candidate source/hash/provenance and final-source CI (not run here) | PENDING | — |
| Published download byte/hash verification after authorized publication | PENDING | — |

### Normal-candidate run worksheet

| Machine/run | EFI/source | Date/JST | Off seconds / AC / topology | Viewer/Hello and read-to-ready timing | Result | Original evidence |
|---|---|---|---|---|---|---|
| NUC5 C1 | PENDING | PENDING | PENDING | PENDING | PENDING | — |
| NUC5 C2 | PENDING | PENDING | PENDING | PENDING | PENDING | — |
| NUC5 C3 | PENDING | PENDING | PENDING | PENDING | PENDING | — |
| Ryzen C1 + dongle | PENDING | PENDING | PENDING | PENDING | PENDING | — |
| Ryzen C2 + dongle | PENDING | PENDING | PENDING | PENDING | PENDING | — |
| Ryzen C3 + dongle | PENDING | PENDING | PENDING | PENDING | PENDING | — |
| Ryzen H1 + dongle | PENDING | PENDING | PENDING | PENDING | PENDING | — |
| Mac storage A/B | PENDING | PENDING | PENDING | PENDING | PENDING | — |
| Mac internal input | PENDING | PENDING | PENDING | PENDING | PENDING | — |

### Feature evidence form (copy per machine/feature)

Record feature and exact candidate/sample/CONFIG hashes; machine/firmware/device actual IDs/revisions; cable/port topology photo; initial/changed/final values; power interval/AC; expected vs observed result; failure/timeout and safe-stop behavior; original photo/video/log paths and timestamps; owner review. For SETTINGS, record allowed OS-owned filenames, write/reboot/read-back and host mount/check evidence without rewriting user CONFIG. For input, record press/release/neutral/disconnect values. For LAN, capture actual profile/link/DHCP address plus matching sent/returned UDP12345 payload and OS counter. For viewer, record three distinct launch/return cycles. Mac storage and BCE stage evidence follow [Mac stages](mac-storage-t2-stages-0.4.0.md); [BT](bluetooth-hardware-quickcheck.md), [I218](i218-hardware-quickcheck.md) and [formatted-media](formatted-media-boot-quickcheck.md) have separate procedures. Execution of this worksheet does not authorize USB writes or publication.

## 日本語

### 候補識別と証拠規則

通常source commit・EFI SHA256・features・asset/member manifest・CONFIG内容/hash・起動/data USB型番/serial/容量/sector/filesystem/partition・実機/firmware/port/adapter/hub/monitorはPENDING。過去版hashやD8証拠を暗黙に今回へ再利用しません。冷間はoff≥10秒、AC接続/切断・run番号/時刻・媒体/接続を記録。所有者報告と写真で直接確認できた事実を区別し、不明をPASSとしません。

### gate記入表

| Gate / 確認項目 | Status / 状態 | Evidence / 証拠 |
|---|---|---|
| QEMU/正確な候補回帰 | PENDING | — |
| 条件1：同一sourceからclean2buildで全配布物再現 | PENDING | — |
| 条件2：文書後継の受入runtime一致、不一致なら再受入 | PENDING | — |
| NUC5冷間1/2/3（off≥10秒、AC記録） | PENDING | — |
| Ryzen冷間1/2/3・dongle付き温間1 | PENDING | — |
| OS所有SETTINGS書込み→再起動→読戻し、CONFIG不変 | PENDING | — |
| OS書込み後macOS/Windows mount・filesystem検査 | PENDING | — |
| NUC5/RyzenでRust Hello | PENDING | — |
| NUC5/RyzenでUSB mouse sample | PENDING | — |
| 再起動無しviewer launch/return3回 | PENDING | — |
| BT Xbox1914 pair/永続保存/再接続/SYS_INPUT | PENDING | — |
| NUC5 I218 link/DHCP/UDP12345 echo | PENDING | — |
| Ryzen Realtek実profile link/DHCP/UDP echo | PENDING | — |
| Mac同一接続storage A/B・FAT32/exFAT又は原因/制約 | PENDING | — |
| Mac T2内蔵keyboard/viewer・外付け回帰 | PENDING | — |
| 文書第2段/formatter gate/索引リンク/docs.zip判断 | PENDING | — |
| 最終候補source/hash/provenance・final-source CI（今回未実行） | PENDING | — |
| 承認公開後downloadのbyte/hash照合 | PENDING | — |

### 通常候補run記入表

| 機種/run | EFI/source | 日時JST | off秒/AC/接続 | viewer/Hello・read→ready時間 | 結果 | 原証拠 |
|---|---|---|---|---|---|---|
| NUC5冷間1 | PENDING | PENDING | PENDING | PENDING | PENDING | — |
| NUC5冷間2 | PENDING | PENDING | PENDING | PENDING | PENDING | — |
| NUC5冷間3 | PENDING | PENDING | PENDING | PENDING | PENDING | — |
| Ryzen冷間1＋dongle | PENDING | PENDING | PENDING | PENDING | PENDING | — |
| Ryzen冷間2＋dongle | PENDING | PENDING | PENDING | PENDING | PENDING | — |
| Ryzen冷間3＋dongle | PENDING | PENDING | PENDING | PENDING | PENDING | — |
| Ryzen温間1＋dongle | PENDING | PENDING | PENDING | PENDING | PENDING | — |
| Mac storage A/B | PENDING | PENDING | PENDING | PENDING | PENDING | — |
| Mac内蔵入力 | PENDING | PENDING | PENDING | PENDING | PENDING | — |

### feature証拠様式（機種/機能毎にcopy）

feature/候補/sample/CONFIG hash、実機/firmware/device実ID/revision、接続写真、初期/変更/最終値、電源断/AC、期待/観測、失敗/timeout/停止、原写真/video/log path/時刻、所有者reviewを記録。SETTINGSは許可OS所有名・書込み→再起動→読戻し・host mount/checkを記録し利用者CONFIGを書きません。入力は押下/解放/neutral/切断値。LANは実profile/link/DHCPとUDP12345送受信同一payload/OS counter。viewerは異なる3launch/return。Macは[段階表](mac-storage-t2-stages-0.4.0.md)、[BT](bluetooth-hardware-quickcheck.md)・[I218](i218-hardware-quickcheck.md)・[formatter](formatted-media-boot-quickcheck.md)は別手順。本様式はUSB書込みや公開を承認しません。
