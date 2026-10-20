# Macストレージ・T2の証拠採取段階

Status: current（2026-10-15、実機結果の追加なし）。

## Current trace capture — English (2026-10-16)

Use the exact stage-0 development EFI/CONFIG identified in the [local preparation record](mac-local-preparation-0.4.0.md); preserve the older A/B binary provenance. A later EFI with phase-status lines needs its own recorded source/features/SHA-256 and owner-approved deployment; do not attribute those lines to the older EFI. Select `log=trace` before boot, or record whether F8/8 actually forced Trace. Devices alone hides low-level T2 observations. This is a single scrolling console: the old instruction to wait for eight-second rotating diagnostic pages after exit does **not** apply. No panel/page revival or new capture UI is included.

Start continuous video before boot and identify EFI/hash, CONFIG/hash, model/firmware, AC/off interval, media/filesystem and the fixed cable/adapter/hub/port topology. Capture visible exact-case lines; the optional timestamp is not part of each label:

| Record group | Labels/fields to preserve |
|---|---|
| Identity and scope | `device: T2-D1 READ ONLY / NO MMIO / NO DMA`, `T2 MODEL`, `T2 SMBIOS`, `T2 PCI SEG 0 COUNT ... OMIT ...` |
| Inventory and bridges | `T2 PCI` BDF/ID/class; `T2 BRIDGE`, `T2 BR MEM`, `T2 BR PREF UPPER` raw windows |
| BCE snapshots | `T2 BCE COUNT ... OMIT ...`; on phase-status builds, all three `T2 PHASE EARLY/PRE EBS/POST EBS CAPTURED` or `NOT CAPTURED` lines, even at count zero; each available `T2 EARLY`, `T2 PRE EBS`, `T2 POST EBS` BDF/ID, followed by Command/Status/header, BAR0–5/layout and PMCSR/MSI/MSIX control or ABSENT/error |
| Refusal and completion | `T2 ID CHANGED`, `T2 CAP ERROR`, `T2 BAR RANGES UNVALIDATED / DRIVER NOT STARTED`, `T2_DIAGNOSTICS_COMPLETE`; every OMIT/ERROR/refusal |
| Independent storage/input | actual USB owner/resource/recovery result, FAT32/exFAT file name/length/hash/read result, external key press/release and viewer/app; exact ready alone is not file or T2 acceptance |

T2 records are emitted early, before OS page-table construction and later USB work; `0.000` is not measured elapsed time. On a phase-status build, `CAPTURED` means that phase's existing config-read loop finished, not that a camera captured the line or that the values are valid. A `NOT CAPTURED` phase must not be transcribed as a zero-valued ID/BAR snapshot. The viewer can overwrite the console, and long inventories can scroll before a camera captures each row. There is currently no hardware serial/file export or ring-history viewer. Video/stills establish only legible observed lines; absent screenshots must be marked **NOT CAPTURED / evidence insufficient**, including when a later complete marker is visible. Do not claim this procedure guarantees a full three-phase snapshot. A reliable bounded read-only capture aid is a separate remaining task. Raw BAR values still provide neither size/resource ownership nor MMIO permission; obtain missing resource evidence separately without BAR-sizing writes. A BCE-count-zero QEMU observation is not a physical T2 result.

## 現行Traceの採取 — 日本語（2026-10-16）

[ローカル準備記録](mac-local-preparation-0.4.0.md)のstage0開発EFIとCONFIGを正確に識別し、旧A/B原本の来歴を保持します。後続のphase状態表示入りEFIを使う場合はsource/features/SHA-256と所有者承認による配備を別途記録し、旧EFIに新表示を帰属させません。`log=trace` を起動前に指定するか、F8/8で実際にTraceへ上書きできたかを記録します。Devicesだけでは低levelのT2情報は画面に出ません。現在は単一scrollコンソールであり、旧手順の「終了後8秒ごとに診断ページを待つ」は成立しません。旧panel復活や新capture UIは追加しません。

起動前から連続録画し、EFI/CONFIGのhash、model/firmware、AC/冷間秒数、媒体/filesystem、cable/adapter/hub/port全経路を対応付けます。上表の大文字小文字を保持したT2識別・PCI/bridge・BCE数・各phaseの`CAPTURED`/`NOT CAPTURED`（新EFIの場合）・EARLY/PRE EBS/POST EBS・BAR/capability・省略/拒否/完了、および独立したUSB読出し・外付け入力・viewer/app結果を保存します。prefixの `device:` と時刻は新formatterによるものです。

T2行はOS page構築や後続USB処理より前に出るため `0.000` を実測経過時間と解釈しません。新EFIの`CAPTURED`は既存PCI設定読取りloopの終了であり、撮影済みや生値の有効性を意味しません。`NOT CAPTURED`相をゼロ値のID/BAR測定として転記しません。inventoryがscrollしたりviewerが上書きしたりし、現状は実機serial/file出力・ring履歴画面もありません。録画で判読できた行だけを証拠とし、欠落は **未採取／証拠不足**。完了markerが後で見えても全3phaseの採取済みとはしません。確実な読取り専用採取補助は別の残件です。raw BARからsize/所有権/MMIO許可は証明できず、足りないresource情報はBAR sizing書込み無しで別採取します。BCE無しQEMUを実T2証拠にしません。

目標14のstorageと15の内蔵入力は別の実機gateです。H19/IMG7650成功から過去No mediumの原因解決を推定しません。本書は実装依頼の前提整理で、device有効化・USB更新はしません。

### storage：同一接続A/B

1. MacBook Pro2018の正確なmodel/firmware、monitor、USB媒体/partition/filesystem、keyboardを識別。cable/adapter/hub/物理port全経路を撮影し両端を識別。A/B間で媒体・data・接続を固定。過去写真のroot port番号変更は同一接続反復の証拠ではありません。
2. Aは保存したv0.2.0 EFI、Bは識別した開発EFI。出所/commit/features/SHA-256/CONFIGを記録。所有者が元EFIを保存し対象EFIのみ別途copyし、format/raw writeしません。boot選択、off ≥10秒、AC状態を記録。FAT32 file名/長さ/hash/読出しを採取。exFAT data媒体は別の固定接続runとして同様に記録。firmware起動/GOPだけではOS読出しの証拠になりません。
3. 起動担当**7D00**のEARLY/PRE EBS/POST EBSのID/BDF、raw BAR0/1、Command/Status、PMCSR、boot device path/BOOT XHCI、resource/bridge/PEER BOUND/復旧理由、PCI RECOVERED又は拒否を撮影。controller/root/slot/speed、USB読出し/FAT32又はexFAT、app/viewerも採取。省略/error行も保存し、写真時刻とEFIを対応させます。
4. コード変更前に比較。同一接続でA成功/B失敗なら最初の相違phaseを絞り、両方失敗なら接続/媒体未確定を記録。非boot-owner0700 skipとresource検査は維持。完了はFAT32/exFAT読出しの反復証拠又は**限定した原因/制約の記録**で、Mac全port対応ではありません。

### T2：登録→queue→HIDの実機確認段階

| 段階 | ローカル準備可能 | 次段階前に必要な実機証拠 |
|---|---|---|
| T0 inventory | [調査](history/macbook-t2-keyboard-0.2.0-research.md)、[host契約](history/t2-host-model.md)、[登録model](history/bce-registration-memory.md)、resource試験の確認 | [T2-D1](history/t2-d1-diagnostics.md) model/PCI/BCE106b:1801、BAR2/4のsize/resource/bridge所有権、PM/MSI、EBS変化。アドレス表示だけではMMIO許可にならない。 |
| T1 mailbox/時刻 | 有界adapter/単調時計/定期timestamp、異常・遅延reply停止設計 | 検証済BAR、protocol0x20001交換、USB/inputを止めないtimestamp。MSI又は全通知を回収できるpollingの根拠。mailbox pollだけでDMA pollを証明しない。 |
| T2 登録 | encoder/連続memory/alignment/範囲/ack/quarantine/stale/reset試験 | 専用予約coherent DMA、実command transportでqueue登録とack、QID/index/長さ/status。登録確認前にsubmitしない。 |
| T3 queue/停止 | wrap/遅延/順不同完了/ack失敗/timeout/終了試験 | 完了可視性、copy/clear/consumer doorbell、所有権退役、実quiescence。不確実停止はDMA保持。model drop/世代番号でdevice停止や全index再利用問題を解決した扱いにしない。 |
| T4 VHCI/HID | descriptor/report-IDの有界parser/source fallback、EP0/interrupt-IN設計 | BCE VHCI経由の実port/device/config/HID report descriptor。8-byte boot reportを仮定しない。 |
| T5 入力受入 | 正規化eventを既存source selectorへ接続する設計、切断cleanup | 外付け無しで内蔵A/Shift+A/押下/解放/viewer操作。外付け回帰/fallback、重複/stuck無し、USB読出し維持。Touch Bar Escは別、開発中は外付けEsc/timeoutを確保。 |

host modelは実BCE MMIO/DMA/割込み・内蔵HIDへ未接続。通常QEMU xHCIでT0–T5を代用しません。次の実装依頼には段階・実resource・通知方式・DMA所有権/停止根拠・異常試験・外付け経路維持を明記。T0証拠待ちでも設計/host試験は可能ですが、mailbox/DMA有効化や内蔵入力成功の主張はできません。

run毎に日時/実施者、EFI/hash/features/CONFIG、model/firmware、接続写真、filesystem/data hash、冷間秒数/AC、判読できた全Trace行と未採取項目、最初の相違/error、PASS/FAIL/BLOCKED/次段階判断を記録。原証拠を保持し新結果は追記で分離します。

ローカル実装・試験の対応表と開発EFIの識別は[ローカル準備記録](mac-local-preparation-0.4.0.md)を参照してください。

Development EFI identity / 開発EFIの識別: [three diagnostic artifacts and original provenance](hardware-acceptance-0.4.0.md#development-efi). Select the purpose-specific artifact and record CONFIG; these are not normal-candidate acceptance. / 用途別artifactとCONFIGを識別し、通常候補受入とは区別してください。
