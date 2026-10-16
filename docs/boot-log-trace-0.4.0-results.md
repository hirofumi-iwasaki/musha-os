# Boot-log trace follow-up — v0.4.0

## English

Status: local implementation and focused regression, 2026-10-16. Contract: [boot log](boot-log.md); source scope: [v0.4.0](release-scope-0.4.0.md); earlier broad regression remains a separate [296-case record](boot-log-0.4.0-results.md).

`log=trace` was added without changing application ABI or SYS_LOG limits. F8/8 selects trace. Devices shows concise factual summaries; duplicate settings/GOP/media/LAN phase lines are retained in the ring and trace but replaced on the devices screen by their complete summary. Cache selection is a separate decision. PCI inventory remains one row per function; USB identities are deduplicated by controller/path/VID/PID across repeated scans. The bounded identity cache has 128 entries (the current per-controller slot limit is eight); saturation retains subsequent records at trace rather than allocating memory. Trace keeps control requests, stages, raw diagnostics and timing pairs; all levels keep the same full ring.

The first phases/devices/trace line and final startup line are exact `Musha-OS 0.4.0 starting` and `Musha-OS 0.4.0 ready`. Tags are explicit lowercase values, while proper names keep their original case. UEFI/runtime details fold into `core: runtime ready`; selected display dimensions are shown; `usb` replaces `USB host`; LAN shows actual link/DHCP or a concrete failure reason without development-certification prose. The complete tag inventory and the reason for consolidating former tag aliases are in the contract.

Applications now display their actual name: `app: starting AUTORUN.ELF`, real Hello/log bytes, then `app: AUTORUN.ELF exited, status 0` on the same console. The terminal startup-ready line follows the initial application outcome before viewer ownership. Selected names are copied from validated metadata while its media-bound service is alive; `SECOND.ELF` was exercised through three viewer launches. Non-ASCII UTF-16 code units become dots only for display; IDs, validation and ownership rules remain unchanged. A filtered loading trace is now emitted unconditionally to the ring, including at ready.

### Verified results

- **431 host tests passed**, zero failed/ignored (42 result groups); native workspace excludes the firmware boot target and enables Bluetooth security. The five focused configuration/producer tests include the five-level visibility matrix across five geometries, real record metadata, case preservation, identity-cache saturation and summary visibility. These five are included in 431, not added again.
- **31 selected QEMU case configurations passed**: startup/configuration 18, impact matrix 11, real shared application console 2. A three-launch configuration counts once. The final devices-only retest replaces its previous checkpoint; the earlier devices frame and two initial application observations are supplementary executions, not extra distinct cases.
- **11 distinct independent full-pixel comparisons passed**: nine startup/configuration types plus normal/debug-ready real-app screens. The final devices comparison replaces the old devices image. Primary and retained ring order/content are checked; only variable timestamps and structural case-insensitive diagnostic tokens are normalized by relevant legacy observers. Raw primary/ring bytes and proper-name assertions remain intact.
- The normal app screen uses the test-only `app-log-screen-probe` feature to hold **after the real exit line**, not before execution or before ready. It requires an empty debug port. The debug-ready version verifies that hidden loading trace remains in both primary and retained ring. Both screen images match an independent glyph renderer over the real 2048×2048 preconsole baseline, including untouched background pixels.
- The impact matrix covers r1+LAN three returns, r2+LAN three returns, fault and nonzero exit, `SECOND.ELF` three selected returns, C/Rust mouse values, kernel #GP/error framing, normal USB, BT passkey UI fixture and SETTINGS bounds/write/fsck/unchanged-region checks. Eight observer unit tests pass. No DMA or device lifetime rules were relaxed.

Evidence: `out/trace-finish/verification.json`, `impact.json`, `host-workspace.log`, `app-normal-pixels/result.json`, `app-ready-pixels/result.json`; startup checkpoints in `out/bootlog-trace-devices-final/report.json`. Review images: `out/trace-finish/{app-normal-held,devices-final,phases}.png`.

Initial observer failures came from assuming all phases were visible at devices and from counting repeated trace USB identities as device summaries; producer-role classification and exact independent rendering corrected those checks. The initial normal held-app build omitted the fixture feature; its real default app/exit/ready observation is kept as supplementary evidence, and the corrected explicit-feature run is the held-screen reference. No firmware failure remains in this focused matrix.

The previous 296-case run was not repeated wholesale: input decoding, parsers, crypto, DMA transitions, framebuffer/PAT implementation and rendering geometry were unchanged. Directly affected level filtering, producer roles, app naming/order and the app/viewer path were rechecked. The last unconditional loading-record fix affects only hidden trace retention at ready; the final debug-ready application run checks that change. Earlier immutable startup binaries and final summary-only devices binary keep separate source hashes.

Physical acceptance remains pending. Old development EFIs and the Parallels image predate this trace follow-up and were not regenerated; do not claim they parse `log=trace` or implement the new override. No push, USB write, remote CI, tag or release was performed. The owner-authored POSIX profile and its three link additions are committed separately without editing their bodies.

## 日本語

状態：2026-10-16のローカル実装と変更影響回帰です。[boot log仕様](boot-log.md)、[v0.4.0 scope](release-scope-0.4.0.md)、別source checkpointの[先行296ケース記録](boot-log-0.4.0-results.md)を参照してください。

log=traceを追加し、F8／8はtraceへ強制します。アプリABI／SYS_LOG上限は不変です。devicesでは機器と判断の事実を要約し、settings／GOP／media／LANのphase重複は対応する詳細1行へ統合します。元のphaseはring／traceへ保持し、cache判断は別1行です。PCIはfunction毎1行、USBはcontroller／path／VID／PIDで反復探索を重複抑止します。固定identity cacheは128件（現在のcontroller当たりslot上限は8）で、満杯後もtraceへ保持して動的割当てしません。制御転送・stage・raw診断・計測はtrace、ringは全レベルで全記録を保持します。

phases／devices／traceの先頭と起動完了末尾はexact Musha-OS 0.4.0 starting／Musha-OS 0.4.0 readyです。tagは明示した小文字で、固有名のcaseを保持します。UEFI／runtime経過はcore: runtime readyへ畳み、displayは実解像度、USB tagはusb、LANは実link／DHCP又は具体的拒否理由を示し開発認証の注記を出しません。全tagと統合理由は仕様へ記録しました。

実アプリ名でapp: starting AUTORUN.ELF→実Hello／生LOG→app: AUTORUN.ELF exited, status 0を同じconsoleへ描きます。最初のapp結果後、viewer所有移行前に起動readyを出します。選択名は有効なmedia service内で検証済catalogからコピーし、SECOND.ELFをviewerから3回起動しました。非ASCII UTF-16 code unitのドット化は表示だけで、ID／検証／所有規則は変えません。loading traceはreadyでも無条件でringへ保持します。

### 検証結果

- **host431合格、失敗／ignore 0**（42結果群）。firmware bootをnative対象外としBluetooth securityを有効化しました。focused5件は5レベル×5geometry、実record metadata、固有名、identity cache満杯、summary表示を検査し、431件に内包します。
- **選択QEMU31構成が合格**：startup／設定18、影響matrix11、実app同一console2。1構成内の3launchを水増ししません。最終devices再試験は旧checkpointの置換であり、旧devicesと初回app2観測は補助実行として分離します。
- **独立全pixel比較11種が合格**：startup／設定9種＋normal／debug-ready実app2種。最終devicesは旧画像を置換します。primary／retained ringの本文・順序を検査し、各旧観測器では可変timestampと構造診断tokenのcaseだけ正規化します。生証拠と固有名assertは保持します。
- normal app画面は専用app-log-screen-probeにより**実exit行の後**を保持します。起動／ready前の停止画像で代替せず、debug出力空を要求します。debug-ready版は表示を抑えたloading traceがprimary／ring両方に残ることを検査します。実2048×2048のpreconsole背景上へ独立glyph描画し、未描画背景を含め全pixel一致しました。
- 影響11構成はr1／LAN3復帰、r2／LAN3復帰、fault／非zero終了、SECOND.ELF3選択復帰、C／Rust mouse厳密値、kernel #GP／error frame、通常USB、BT passkey UI fixture、SETTINGS範囲／write／fsck／変更外領域を含みます。観測器unit8件も合格し、DMA／device寿命の条件を緩めません。

証拠はout/trace-finish/verification.json、impact.json、host-workspace.log、app-normal-pixels／app-ready-pixelsのresult.json、startup checkpointはout/bootlog-trace-devices-final/report.jsonです。目視用はout/trace-finish/{app-normal-held,devices-final,phases}.pngです。

初期観測器はdevicesで全phaseを要求し、反復trace USBを機器要約へ数えて不一致でした。producer分類と独立rendererで訂正しました。normal専用holdの初回buildはfixture feature指定が漏れていたため、実default app／終了／ready観測を補助証拠へ分離し、明示feature版を正式な保持画像としました。影響matrixに未解決firmware失敗はありません。

先行296件を全反復していません。入力decode／parser／crypto／DMA移行／FB・PAT／geometryは不変で、直接影響するlevel filter／producer分類／app名・順序／viewer経路を再検査しました。最後のloading無条件化はreadyの非表示trace保持だけを変え、最終debug-ready appで確認しました。先行startupと最終devicesは各source hashを保持します。

実機受入は未実施です。旧開発3EFI／Parallels imageはTrace追加前で再生成せず、新log=trace／override対応とは扱いません。push／USB書込み／remote CI／tag／Releaseはしていません。所有者のPOSIX profileと参照3文書は本文を変えず別commitにします。
