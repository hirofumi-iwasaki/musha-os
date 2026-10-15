# Stage4 local regression — 2026-10-15

Status: current

## English

This checkpoint follows the [r2 runtime integration](stage4-r2-0.4.0-progress.md). Results are local host/QEMU evidence, not NUC5/Ryzen acceptance. The full results and commands are in `out/stage4-regression/verification.json` and adjacent suite logs. All requested local suites pass: 313 workspace-library tests, 2 Rust SDK tests, 52 Python tests across 7 suites, and 26 malformed ELF cases for each ABI revision (overlapping mutations). QEMU has 275 successful case executions: CI54, C1/C2/C3 8/14/20, BasicData/ESP38, external debug27/normal26, hotplug/timing/hub7, address recovery2, resolution/hold20, USB/network variants13, startup16, diagnostic hold3, SETTINGS15, r2 runtime11, mixed FP1. These include overlapping paths and are not 275 independent features. CI54 cases use52 QEMU tool invocations within107 total build/image/test commands. Failed initial attempts and limited reruns are documented separately. The three timing sizes ran alone after other QEMU suites ended. Four SETTINGS sector/cluster geometries also pass isolated dosfstools `fsck.fat`; existing macOS checks and untouched-region assertions pass. Normal EFI SHA-256: `5eb59f8d79583a28d4e91645f86244e2b2556b5fbfb1e513550a44597d188f07`.

The mixed C/Rust guest checks double arguments/returns in both directions, mixed float/integer structs, seven integer arguments, nine floating arguments, and x87/XMM15 preservation across real TIME calls with WAIT between calls. It passed. The initial probe fault was traced by disassembly to its inline assembly allowing a later-used pointer to share RCX, which SYSCALL overwrites. Early output constraints fixed the probe; the kernel did not change. Formatting rebuilt the identical ELF. Independent C/Rust SDK archives extracted into paths containing spaces build without checkout dependencies; four Rust r2 ELFs including the FP guest and one C r2 IO ELF match repository builds byte for byte using identical build procedures.

The initial unqualified workspace host command tried to compile the UEFI binary as a macOS test and failed on existing architecture/panic constraints. All workspace libraries excluding that binary pass; the UEFI binary is covered by QEMU. Initial external absent-viewer comparisons ran before their reference images existed; only those cases are repeated after reference generation. TIME-only/WAIT-only initially used the input marker expectation, and mouse-only initially omitted the QEMU mouse device. Guest logs identify these runner errors; corrected fixtures are tested separately. These failed attempts and reruns are retained, not counted as additional unique acceptance cases.

Existing normal pixel comparisons retain their independent viewer/content checks; references were not replaced from a failing implementation image. No public ABI compatibility promise is made: r1 remains accepted, r2 is an experimental candidate. Live keyboard and mouse can share one input controller; mouse-only is supported. Simultaneous separate keyboard/mouse input controllers are not integrated. BT/gamepad production and viewer return remain later stages. WAIT is bounded polling, not hard realtime, and long unsampled PM clock wraps cannot be reconstructed. Stop failure is quarantined with no memory reuse; physical failure injection remains unverified.

The SDK archive excludes nested Cargo `target` output; the existing four package tests pass again after this filter/status-text change (overlapping the52 Python tests, not four new cases). Only this task’s generated SDK host-test target was removed.

The owner additions in `release-scope-0.4.0.md` and new `release-scope-0.5.0.md` / `scenario-games-godot.md` are preserved verbatim and excluded from the implementation commit. No push, physical USB write, remote CI, tag/Release or formal candidate generation was performed. Hardware acceptance and formal qualification remain open.

## 日本語

[r2実接続](stage4-r2-0.4.0-progress.md)後の追加検証です。ホスト/QEMUの証拠で、NUC5/Ryzen実機受入ではありません。結果・実行手順は `out/stage4-regression/verification.json` と隣接suite logへ集約します。要求された全ローカルsuiteは合格。workspace library313件、Rust SDK2件、Python7suite計52件、r1/r2の異常ELF各26件（変異内容重複）を確認。QEMU成功実行は275case：CI54、C1/C2/C3各8/14/20、BasicData/ESP38、external debug27/normal26、hotplug/timing/hub7、address recovery2、解像度/hold20、USB/network variants13、startup16、診断hold3、SETTINGS15、r2実接続11、混在FP1。経路が重複する実行件数で275独立機能ではありません。CI54caseはQEMU tool呼出52件、build/image/test全107command。初回失敗と限定再試験は別記録です。timing3サイズは他QEMU終了後に単独実行。SETTINGS4形状は隔離dosfstools `fsck.fat` でも正常で、既存macOS検査・変更外領域assertも合格。通常EFI SHA-256は `5eb59f8d79583a28d4e91645f86244e2b2556b5fbfb1e513550a44597d188f07`。

混在C/Rust guestは双方向double引数/戻り値、float/integer混合struct、整数7引数、浮動小数9引数、実TIME callを跨ぐx87/XMM15保存と間のWAITを確認して合格。初回probe faultは逆アセンブルで、SYSCALLが上書きするRCXと後続利用pointerをinline asmが共有できる指定に原因を特定しました。早期出力制約でprobeを修正し、kernel変更なしで合格。整形後もELFは同一。独立C/Rust SDK archiveを空白付きpathへ展開しcheckout依存なしで再build、FPを含むRust r2 4 ELFとC r2 IO 1 ELFは、同じbuild手順のrepo版とbyte一致です。

最初の無条件workspace host実行はUEFI binaryをmacOS testとしてcompileし既存architecture/panic制約で失敗。本体を除く全workspace libraryは合格し、UEFI本体はQEMUで確認します。外部ELF absent-viewerは参照画像生成前に実行したため、そのcaseだけ生成後に再試験。TIME-only/WAIT-onlyは誤ってinput markerを期待し、mouse-onlyはQEMU mouse指定が欠落していたrunnerを修正しました。guest logと再試験を保持し、失敗試行・再試験を追加の固有受入caseとして数えません。

通常pixel比較は独立したviewer/content判定を保持し、不合格の実装画像を無条件に参照へコピーしていません。r1受入を維持し、r2は実験候補で公開互換保証ではありません。同一入力controller上のkeyboard＋mouseとmouse単独に対応し、別々の入力controllerでの同時収集は未統合。BT/gamepad実producerとviewer復帰は後工程。WAITは上限付きpollでhard realtimeではなく、長時間未sampleのPM wrapは復元できません。停止未確認時は隔離しメモリ再利用せず、実機failure注入は未確認です。

SDK archiveは入れ子のCargo `target` 出力を除外し、filter/資格文言変更後も既存package4件が再合格（Python52件との重複で新規4件ではありません）。削除したのは今回host試験で自ら生成したSDK targetだけです。

所有者追記の `release-scope-0.4.0.md` と新規 `release-scope-0.5.0.md` / `scenario-games-godot.md` はbyteを保持して実装commitから除外。push・実USB書込み・remote CI・tag/Release・正式候補生成は行っていません。実機受入と正式公開資格は残件です。
