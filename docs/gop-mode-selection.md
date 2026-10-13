# GOP mode selection / GOP モード選択

## English

Before framebuffer capture, memory-map acquisition, and `ExitBootServices`, the
bootloader queries every GOP mode (`0..MaxMode`). It selects the largest
`width * height` among RGB8/BGR8 modes with nonzero dimensions and a stride at
least as large as the width. Area calculation uses 64 bits. Bit-mask and BLT-only
modes, failed queries, and invalid layouts are excluded. Query buffers are freed.
The current mode wins equal-area ties; otherwise the first largest mode wins.
If the current mode wins, no `SetMode` is issued. A failed `SetMode` does not
trigger a lower-mode retry: boot continues using the firmware's current mode.
Mode information and framebuffer address are captured again after selection,
so the runtime does not retain pre-switch pointers or dimensions.

The diagnostic summary and observation record contain one line:
`GOP WIDTH/HEIGHT/STRIDE <width>/<height>/<pixels-per-scan-line>`.
Candidate traces remain behind `qemu-debug`; the normal build emits no debug-port
output. The viewer renderer and its layout rules are unchanged by mode selection.
At the GOP validation checkpoint the external console used native 6×8 text.
The release follow-up uses height-based console scale: <2160 → 1, <2880 → 2,
otherwise 3, preserving the default QEMU 2048-high scale-1 pixels.

### Verification and pixel-reference change (2026-10-13)

- Host framebuffer tests: **16 passed**, including six mode-policy tests covering
  largest area, current-mode ties, failed SetMode, unavailable/invalid modes,
  complete enumeration, and 64-bit area arithmetic.
- QEMU/OVMF exposes 30 modes. Initial mode 0 is **1280×800**; selected mode 27 is
  **2048×2048**, BGR8, stride 2048. The 2560×1600 mode has fewer total pixels.
  The diagnostic record contains `GOP WIDTH/HEIGHT/STRIDE 2048/2048/2048`.
- Normal EFI absent-ELF viewer regression: **PASS**, list → text → list exact
  pixel equality, debug output **0 bytes**. Normal Hello ELF: **PASS**, debug
  output **0 bytes**. Debug viewer smoke also passed.
- The first comparison against the old 1280×800 reference failed as expected
  because dimensions changed. The old `out/v3-normal-final-pixels/` is preserved.
  `tools/resize-viewer-reference.py` derives the new expected images from that
  verified reference, **not from the new guest screenshots**. It checks uniform
  surrounding background and applies the unchanged 80×28 viewer grid and integer
  scale rule: scale 2 → 4, origin (16,16). Source/output hashes are in
  `out/v3-gop-max-pixels/reference-provenance.json`.
- If the current mode already has maximum area, selection performs no SetMode
  (host-tested). This QEMU configuration actually changes mode; unchanged-mode
  QEMU pixel equivalence is not claimed.
- Evidence: `out/gop-mode-selection/report.json`, `host-tests.log`,
  `viewer-debug/debug.log`, `new-reference/absent/pixels/result.json`, and
  `hello-final/report.json` under that directory.
- Normal EFI SHA-256:
  `cd707f0c85efab2f31d603539d7289d2ec43d48cc7581ecaf47d6788ba6541ff`.

Reproduction (repository root; configured Rust/QEMU toolchain on PATH):

```sh
cargo test -p musha-framebuffer
cargo build -p musha-boot --release --target x86_64-unknown-uefi --target-dir out/gop-mode-selection/target-final
python3 tools/resize-viewer-reference.py --source out/v3-normal-final-pixels --output out/v3-gop-max-pixels --width 2048 --height 2048
python3 tools/smoke-external-elf.py --firmware-dir /opt/homebrew/share/qemu --normal --efi out/gop-mode-selection/target-final/x86_64-unknown-uefi/release/musha-boot.efi --reference out/v3-gop-max-pixels --case absent --output out/gop-mode-selection/new-reference
python3 tools/smoke-external-elf.py --firmware-dir /opt/homebrew/share/qemu --normal --efi out/gop-mode-selection/target-final/x86_64-unknown-uefi/release/musha-boot.efi --case hello-fat32 --output out/gop-mode-selection/hello-final
```

The reference generator requires a fresh output directory. Use `--reference` to
select a different resolution's reference. This section is the original local checkpoint. Later development-build observations are in hardware-acceptance-0.3.0.md; candidate cold boots remain NOT PERFORMED. Real-firmware SetMode failure remains unverified.

API reference: [UEFI 2.10, Graphics Output Protocol](https://uefi.org/specs/UEFI/2.10/12_Protocols_Console_Support.html).

## 日本語

フレームバッファ情報の保存・メモリマップ取得・ExitBootServices より前に、
GOP の全モードを QueryMode で列挙します。RGB/BGR 各8bit の有効なモードから
幅×高さが最大のものを選びます。同面積なら現在のモードを優先し、変更不要なら
SetMode を呼びません。取得失敗・無効な寸法や stride は候補から除外し、
QueryMode のバッファは解放します。SetMode 失敗時は別モードを再試行せず、
ファームウェアの現在モードを使います。選択後に情報と描画先を取り直します。

診断サマリーと観測記録へ `GOP WIDTH/HEIGHT/STRIDE` の1行を追加しました。
通常版の debug ポート出力はありません。ビューアーの描画規則は変更していません。
GOP検証時点では外部ログは6×8でした。公開準備では高さ2160未満を1倍、2880未満を2倍、それ以上を3倍とし、既定QEMUの1倍ピクセルを保持します。

ホスト16件と、通常版のビューアーピクセル回帰・Hello ELF 実行が成功しました。
QEMU では30モード中、1280×800 から2048×2048へ変わったため、旧参照との初回比較は
不一致でした。旧参照を残し、既存の倍率規則（2倍→4倍）で旧画像から期待画像を生成し、
一覧→本文→一覧が完全一致することと debug 0 byte を確認しました。
参照更新の手順・ハッシュ・証拠パスは英語節に記載しています。

同面積時に現在モードを維持する分岐はホスト試験済みですが、今回の QEMU は
実際にモードが変わる構成です。この節は最初のローカル検証時点の履歴です。後続の開発版観測はhardware-acceptance-0.3.0.mdを参照してください。公開候補の冷間起動は未実施、実機のSetMode失敗も未検証です。
