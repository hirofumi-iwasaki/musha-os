# Stage2 local development results / 工程2 ローカル開発結果

Status: current

## English

This records the stage2 development implementation on `release/0.4.0`, after stage1 commit `35545a6fa7e515293e94e13b46ab2dd1733caea8`. No push, tag, Release, Actions dispatch, formal release candidate or physical USB operation was performed. See the [subsequent full regression](stage2-full-regression-0.4.0.md) for the owner-requested completion and corrective reruns. The owner's concurrent scope edits (goal9 USB mouse, goal12 BLE mouse trial, goal13 Ryzen LAN, goal17 device matrix and acceptance wording) are preserved and are not implementation changes in this checkpoint.

Implemented goals5/6/7 and the stage2 portion of16: FAT32 boot-device CONFIG bounded firmware read and reverse-order file cleanup before GOP, F8/digit8 override, max/explicit GOP selection and framebuffer recapture, startup-only quiet/ready/phases/devices policy, visible required failures, application/viewer output preservation and normal-build PM-epoch records/summary. Pairing configuration is retained; Bluetooth/pairing writes remain later work. LAN status describes the existing driver, not new physical support. See [configuration](startup-configuration.md) and the [new hardware template](hardware-test-record-template.md). The README v0.3.0 banner and historical hardware records are unchanged.

Evidence is under `out/stage2-040/`, summarized by `verification.json`:

- `host-tests.log`: 285 Rust host tests passed, including parser/GOP cases, five resolutions (320×356 through2048²), all log levels, footer bounds, quiet failure and normal timing saturation.
- `verified-config-matrix/`: 16 QEMU cases passed: four levels; missing directory/file; malformed, oversized and non-ASCII; available low/explicit and unavailable GOP; F8/digit8; quiet forced failure; viewer page repaint with ready footer preserved. quiet's entire final framebuffer equals its post-GOP/pre-OS baseline.
- `concise-log-screens/`: three additional QEMU checks passed after removing Rust Option formatting and moving PM values to the devices summary. `ready.png` in each ready/phases/devices folder is the final visual reference. The earlier16 checks precede this text-only refinement.
- `diagnostic-hold-verified/report.json`: all three existing diagnostic-hold cases passed after explicitly configuring devices on both bootable FAT32 choices.
- `hello-normal/report.json` and `hello-grid-check/result.json`: feature-free C Hello passed the existing screen observation and independent full-pixel rendering. The original console contents and grid are unchanged; ready is drawn below them. `check-grid-console.py --ready-footer` explicitly selects this new expected footer, preserving the old checker mode.
- Normal EFI SHA-256: `63f5906da3a641c1ac27e57d0e595ac77386a1b27cf398de4d632c2425336220`. It was rebuilt after the log text refinement. The Hello observation preceded that refinement, which does not affect ready-mode console pixels.

The development-only `startup-config-probe` adds a bounded firmware key window and framebuffer capture window; `MUSHA_BOOT_PROBE_HOLD=ready` holds before application drawing. `startup-failure-probe` injects a required clock failure. Neither is a distribution feature. The startup smoke fixture creates regular files only and inserts CONFIG into a newly generated image; production image generation still leaves SETTINGS empty. Existing detailed-diagnostic consumers explicitly set devices on every bootable FAT32 fixture, since missing CONFIG now correctly defaults to ready.

Known failures kept as evidence: an unoptimized debug-profile EFI double-faulted during storage; optimized release-profile development builds pass. The initial quiet-failure harness incorrectly asserted no repaint for required failures and was corrected. The first diagnostic-hold run found that firmware could choose its bootable USB data fixture, which lacked CONFIG; both diagnostic boot choices now explicitly carry devices. A subsequent invocation mistakenly supplied a debug EFI to the tool's production-only `--efi` path; it is not a production acceptance result. See the diagnostic reports for the final rerun.

At this earlier checkpoint, the full matrix had not been performed; it is now covered by the subsequent report. Not performed: physical NUC5/Ryzen F8/8/display/timing acceptance, physical USB writes, formal clean two-build reproducibility or CI dispatch. Firmware read/close error return branches are conservative defaults with reverse-order close attempts, but injected firmware-error mocks were not run. QEMU and host results do not establish physical compatibility or the required machine-specific timing targets.

## 日本語

工程1 commit `35545a6fa7e515293e94e13b46ab2dd1733caea8` 後、`release/0.4.0` 上の工程2開発実装を記録する。push／tag／Release／Actions／正式候補生成／実USB操作は行っていない。所有者依頼後の完了・修正再試験は[全回帰結果](stage2-full-regression-0.4.0.md)を参照。所有者による並行 scope 差分（目標9 USB mouse、12 BLE mouse試行、13 Ryzen LAN、17 device matrixと受入文言）は保持し、今回の実装差分と区別する。

目標5/6/7と工程2分の16を実装した。FAT32起動デバイスのCONFIGをfirmware経由で上限付き読出しし、GOP前にfileを逆順close。F8／数字8、max／明示GOPとframebuffer再取得、起動出力限定quiet／ready／phases／devices、必須失敗表示、アプリ／viewer出力保持、通常版PM-epoch記録／要約を接続した。pair設定は後工程用に保持し、Bluetooth／pairing書込みは未実装。LAN状態は既存driverを表し、新規実機対応を意味しない。[起動設定](startup-configuration.md)・[新実機様式](hardware-test-record-template.md)を参照。README冒頭v0.3.0と過去実機記録は変更していない。

証拠は `out/stage2-040/`、集約は `verification.json`。

- `host-tests.log`: Rust host285件合格。parser／GOP、320×356〜2048²の5解像度、全log、footer境界、quiet失敗、通常timing件数飽和を含む。
- `verified-config-matrix/`: QEMU16件合格。4level、directory／file欠落、不正文法／上限超過／非ASCII、有効低解像度／明示mode／未対応GOP、F8／8、quiet強制失敗、viewer page再描画後ready保持。quietはGOP後・OS描画前と最終framebufferが全byte一致。
- `concise-log-screens/`: Rust Option表記を除き、PM値をdevices要約へ移した後の追加3件合格。ready／phases／devices各folderの `ready.png` が最終表示証拠。先の16件はこの文言調整前。
- `diagnostic-hold-verified/report.json`: 起動可能な両FAT32経路へdevicesを明示後、既存diagnostic-hold全3件合格。
- `hello-normal/report.json` と `hello-grid-check/result.json`: feature無しC Helloが既存表示試験と独立全pixel照合に合格。console内容とgridは不変でreadyは下部余白に描画。`check-grid-console.py --ready-footer` で新期待値を明示し、旧checker動作を保持。
- 通常EFI SHA-256: `63f5906da3a641c1ac27e57d0e595ac77386a1b27cf398de4d632c2425336220`。最終log文言調整後に再buildした。Hello観測は文言調整前だが、ready-mode console pixelには影響しない。

開発専用 `startup-config-probe` は上限付きfirmware key待機とframebuffer撮影待機を追加し、`MUSHA_BOOT_PROBE_HOLD=ready` でアプリ描画前に停止する。`startup-failure-probe` は必須clock失敗を注入する。いずれも配布featureではない。smoke fixtureは新規regular image内にCONFIGを挿入し、製品image生成の空SETTINGSは変えない。詳細診断consumerはCONFIG欠落がreadyになる新仕様に合わせ、起動可能な全FAT32 fixtureにdevicesを明示する。

失敗証拠も保持する。未最適化debug-profile EFIはstorage中double fault、最適化release-profile開発buildは合格。初回quiet失敗harnessは必須失敗にも無描画を要求しており修正。初回diagnostic-holdはCONFIG無しの起動可能USB data fixtureをfirmwareが選ぶ場合を検出し、両起動経路にdevicesを設定した。その後debug EFIをproduction用 `--efi` に渡した呼出しは実行指定ミスであり、製品受入結果ではない。最終再実行はdiagnostic reportを参照。

この先行時点で未実施だった全行列は後続文書に記録済み。未実施: NUC5／Ryzen実機のF8／8／display／timing受入、実USB書込み、正式clean二回build再現性、CI dispatch。firmware read／close errorは保守的に既定値へ戻し逆順closeを試みるが、firmware error mock注入は未実施。host／QEMUを実機互換性や機種別timing目標の達成に読み替えない。
