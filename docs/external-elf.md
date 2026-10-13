# External ELF loading — local integration probe

## English

Status: implemented and tested locally on 2026-10-13, on `release/0.3.0`.
This is development-order item 2 of [the v0.3.0 scope](release-scope-0.3.0.md),
behind `external-elf-probe`. It is not the default release boot path and does not
complete the publication gates. The ABI remains experimental candidate r1.

### File selection and lifetime

- Read `/MUSHA.ELF` in the root catalog of the data medium selected by the existing
  file service. FAT32 and exFAT use the same reader. ASCII case differences are
  accepted; ambiguous matching names are rejected. No directory search, launcher,
  or additional-media scan is added. Existing catalog limits still apply.
- Missing file falls through to the existing viewer. No readable file service
  uses the existing recovery/viewer behavior. A matching file must be 64 bytes to
  16 MiB. Each read is at most 4096 bytes, with one outstanding request and a
  five-second deadline starting when the file is found.
- The 16 MiB staging region is permanently kernel-owned zero-filled image memory,
  separate from the application arena and USB DMA buffers. Only this probe raises
  the loaded EFI memory-size bound from 16 to 32 MiB; the on-disk EFI remains small.
  No extra UEFI service is used after ExitBootServices.
- The viewer shell is initialized by the existing cooperative runtime, but its
  file callbacks and completion/input processing wait for the boot-file decision.
  In particular, missing keyboard input must not terminate a pending ELF read.
- Short reads, stream errors, disappearance and timeout cannot publish a partial
  image. Closing cancels pending work without releasing an in-flight DMA slot.
  A failed or interrupted device session invalidates even an already-read image.
- A successful read requests the existing session to stop. USB and network owners
  quiesce before the runtime returns and before the arena is reused. An unconfirmed
  USB stop returns an error; a failed network stop halts in its existing owner.
- The immutable staging bytes are checked with `musha_memory::elf::Plan` and then
  consumed by the same C2 mapper, permissions, start record and syscall path used
  by the embedded probe. No user-controlled physical pointer or framebuffer is
  exposed. C1/C2/C3 probes remain separately selectable; embedded and external ELF
  modes are mutually exclusive.

### Local validation

The reproducible harness builds **one EFI** and varies only the read-only virtual
USB data images. No host USB device or deployment command is used.

```sh
export CARGO_HOME="$PWD/.local-tools/cargo"
export RUSTUP_HOME="$PWD/.local-tools/rustup"
export PATH="$CARGO_HOME/bin:/opt/homebrew/bin:$PATH"
cargo test --locked --workspace --exclude musha-boot
python3 tools/smoke-external-elf.py --firmware-dir /opt/homebrew/share/qemu --output out/external-elf-accepted
python3 tools/smoke-ring3.py --firmware-dir /opt/homebrew/share/qemu
python3 tools/smoke-ring3-elf.py --firmware-dir /opt/homebrew/share/qemu
```

The external harness covers Hello, changed Hello text using the identical EFI,
missing file/viewer, malformed ELF, and oversized metadata on both filesystems;
it also covers no keyboard and separate storage/input controllers on FAT32.
It records EFI/image hashes, commands, debug logs and screenshots in its output
folder, and checks USB/network quiescence precedes user entry. Output directories
must be new; existing evidence is not overwritten.

The five new host tests exercise both formats, bounded/exact copying, name
selection, pre-I/O size rejection, short reads, backend errors, removal after a
partial read, canceled request quarantine, deadline and post-read invalidation.
Existing C2 host and QEMU checks continue to validate ELF/NOTE and memory policy.

Acceptance results: 12 external QEMU cases passed (`out/external-elf-accepted/report.json`),
238 workspace host tests passed (`out/external-elf-final/host-tests.log`), and all
8 C1 / 14 C2 regressions passed (`out/ring3/report.json`, `out/ring3-elf/report.json`).
The feature-free normal release build passed list/open/back pixel comparison with
`out/v3-normal-final-pixels`; evidence is `out/external-elf-final/normal-pixels/result.json`.
The malformed-file rejection screenshot was visually inspected. These are local
QEMU/host results, not physical hardware or remote CI results.

### Remaining work and limits

- Wire the path into the normal feature-free release build after kernel screen
  output and launch/exit/fault presentation are ready. Runtime readiness records
  currently still describe the static viewer shell before the external handoff.
- `log`/exit still use the C2 debug channel. Read/format rejection text is drawn on
  screen, but mapping/CPU rejection presentation and a missing-file status message
  are future work. No screen-Hello or hardware success is claimed.
- The filename is now an implementation choice for the preview, not a stable ABI.
  The maximum-size image, deadline on slow hardware, hot removal during a real USB
  transfer, and allocation exhaustion remain unverified on hardware.
- SDK packaging/relocation documentation, default-build external QEMU acceptance,
  fault/exit screen checks and three hardware cold boots remain release work.
  C3 recovery is still optional and is not used by this one-shot path.

Initial local failures were retained separately: the staging section exceeded the
old loaded-image bound; a changed-text fixture accidentally shortened the ELF;
and the no-keyboard viewer completion initially interrupted loading. These were
corrected before acceptance; those failed runs are not passing evidence.

No commit, push, remote Actions, tag, Release, settings, physical USB or PDF change
is included in this work.

## 日本語

2026-10-13、`release/0.3.0`で工程2の外部ELF読込みをローカル実装しました。
`external-elf-probe`を指定する試験経路です。通常版の既定起動にはまだ組み込まず、
ABIは実験候補r1のままとします。公開条件の完了を示すものではありません。

既存サービスが選択したデータ媒体のルートにある`MUSHA.ELF`を、FAT32／exFAT共通の
ストリーム経路で読みます。ASCIIの大文字小文字を区別せず、同名候補の重複は拒否します。
64バイト～16 MiB、1回最大4096バイト、読込み開始から5秒の制限を設けました。
ファイル不在時は従来ビューアーへ進みます。複数媒体の追加探索は行いません。

途中の短い読込み・エラー・切断・期限超過を検出し、不完全なELFは実行しません。
USBとネットワークの終了処理を通し、DMA停止後に既存C2のELF／NOTE検査・配置へ渡します。
キーボード未接続時のビューアー終了判定も、ELF読込み中は待つようにしました。
読込み用16 MiBはカーネル専用領域で、アプリarenaやDMA領域とは別です。

QEMU試験は同じEFIに対して仮想USB上のELFだけを変更します。両ファイル形式のHello、
別メッセージ、ファイル不在、不正ELF、容量超過に加え、FAT32ではキーボードなしと
別コントローラー構成も対象です。ログ・ハッシュ・画面は実行先ディレクトリへ保存します。
ホスト試験では境界・部分読込み・切断・取消しと期限を確認します。
今回、外部ELF 12件、ホスト238件、C1回帰8件、C2回帰14件が合格し、通常版の
一覧／開く／戻るの画面一致も確認しました。不正ELF拒否画面は目視確認済みです。

次は工程3の画面ログ・終了コード・障害表示と通常ビルドへの統合です。
現段階の`log`／終了結果はデバッグ出力で、不正形式などの拒否理由のみ画面に描画します。
起動記録も現状はビューアー初期化を示すため、外部アプリへの移行表示を整える必要があります。
SDK配布、通常版での外部実行検証、実機3回のコールドブートは残件です。
実USB操作、commit／push、公開、PDF変更は行っていません。
