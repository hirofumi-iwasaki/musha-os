# 配布・更新ポリシー統合のローカル検証

日付: 2026-10-12（日本時間）
ブランチ: release/0.2.0
開始時HEAD: 6b2ee2d615d1e72124b1b087670de057b2f6bd43
開始時の未コミット変更: なし。ブランチ・HEADを保持し、変更は未コミットのまま残す。

## 変更ファイル

- README.md、VERSION、viewer/src/lib.rs: ダウンロード導線・要件・制限。表示版と配布版をVERSIONへ統一。内部クレート版は変更しない。
- docs/release-distribution-policy.md: 受領方針を配置。既存機能・公開必須条件は維持。
- docs/distribution-usage.md: 候補作成、初回像、通常更新、復旧、承認段階。
- docs/usb-install-macos.md、docs/usb-install-windows.md、docs/usb-install-linux.md: OS別の実装・未検証範囲。
- docs/test-usb-build.md、docs/usb-image.md、docs/release-scope-0.2.0.md: 配布方針への参照と従来全容量手順の扱い。
- tools/prepare-release.py: 通常EFIを独立ビルド、版・タグ照合、同一EFIでimage.gz/files.zip、deployment.zip、manifest・チェックサム生成。検証済みEFI再利用。
- tools/build-usb.py: 既定をファイルのみへ変更。従来画像生成は--with-imageで明示する開発用経路として保持。ビルド出力を分離。
- tools/distribution.py: 配布manifest、ZIP生成・読出し、パス・payload検証。
- tools/deploy-image.py: 小型GPT/FAT32像から容量別の新規疎ファイルを作成。物理デバイスには書かない。
- tools/update-usb.py: 変更ファイルの検出、同一内容省略、ホスト退避、一時配置・照合・置換、同期。macOS識別と明示UUID、任意のeject。通常ディレクトリ模擬モード。
- tools/candidate-identity.py、tools/draft-release.py: 既存タグ・SHA検証、下書き専用処理。公開・タグ作成・上書き処理なし。
- .github/workflows/release-candidate.yml: 回帰維持、タグ／手動／PR候補、成功時だけ配布artifact、失敗ログ分離、手動明示時だけ下書き専用write権限。公式参照を固定SHA化。
- tools/test-distribution.py、tools/test-release-package.py: 新規16試験と既存候補試験の更新。
- 本ファイル: 実施内容と未確認事項。

## 採用方法

初回は64 MiBのGPT＋FAT32 ESP。512byte論理セクター、容量64 MiB～128 GiB。
FAT32領域のサイズを維持して主副GPTの位置・使用可能範囲・配列CRC・protective MBRを整える。
読み書きは固定領域と媒体末尾の管理情報に限定し、空き全容量を処理しない。
OS側のGPT検査は変更していない。実デバイス用初回アダプターは未提供。
既存FAT32配置を使う経路を維持する。

通常更新はmanifest指定のファイルだけを比較・退避・置換し、GPT・FAT32と無関係なデータを保持。
イメージ生成／raw書込み／全媒体ハッシュ／失敗時の全量fallbackはない。
FAT32更新を停電時に完全に原子的とは保証しない。復旧はホスト退避を利用する。

## 使用コマンド

リポジトリルートで固定toolchainの環境変数を設定して実行:

```sh
export RUSTUP_HOME="$PWD/.local-tools/rustup"
export CARGO_HOME="$PWD/.local-tools/cargo"
export PATH="$CARGO_HOME/bin:/opt/homebrew/bin:$PATH"
export MUSHA_FIRMWARE_DIR=/opt/homebrew/share/qemu
cargo fmt --all
cargo test --locked --workspace --exclude musha-boot
python3 tools/test-distribution.py
python3 tools/test-release-package.py
python3 tools/test-usb-build.py
python3 tools/test-usb-image.py
sh out/distribution-regression/run.sh
python3 tools/prepare-release.py --allow-dirty --output dist/distribution-local-validation
/private/tmp/musha-actionlint-1.7.7 -shellcheck= .github/workflows/release-candidate.yml
git diff --check
```

回帰run.shは前工程の14グループの実行スクリプトを新しいログ先へ複製したもの。
リポジトリの恒久的な実行定義はrelease-candidate.ymlを参照。
正式候補はcleanな固定commitで--allow-dirtyを外す。
初回仮想像・通常更新の具体コマンドは[配備手順](distribution-usage.md)を参照。
この作業中に実デバイス用コマンドやdraft-release.pyの実APIを呼んでいない。

## 試験結果

| 検証 | 結果 |
|---|---|
| Rustホスト | 224件成功 |
| 新規配布・更新・GPT・下書きモック | 16件成功 |
| 既存候補パッケージ | 6件成功 |
| 既存USB bundle／GPT image | 1件／4件成功 |
| 既存回帰14グループ | 全成功。ホスト・HID mutation、USB・入力・通信、hub、複数xHCI、CPU例外、timeout、FAT32/exFAT、ファイル取消・切断、viewerを維持 |
| 通常版64 MiB FAT32／exFAT | QEMUで一覧・Enter・戻る画面一致、debug出力0、像の不変を確認 |
| GPT調整後128 MiB通常版 | QEMU起動・一覧・Enter・戻る画面一致 |
| 64 MiB・128 MiB・32 GiB＋512byte仮想像 | GPT/CRC/FAT32/EFI読出し成功 |
| 実I/O範囲 | 32 GiB疎ファイルでread/writeを追跡し、固定領域と末尾以外へのアクセスがないことを確認 |
| 拒否条件 | 小容量・4Kn・破損GPT・出力衝突、ZIP危険パス・symlink・大小文字衝突・hash不一致、容量不足・layout不一致を確認 |
| 差分更新 | 変更1ファイルだけ置換、同一ファイルのmtime維持、ユーザーデータ維持、旧EFI退避、途中失敗・一時ファイルcleanupを確認 |
| 再利用 | 版・タグ・commit・dirty・features・hash不一致を拒否 |
| 下書き操作モック | dry-run、--draft/--verify-tag、公開済み拒否、完全一致再利用・内容違い拒否 |
| Actions | actionlint 1.7.7成功（shellcheck無効）、YAML構文・権限境界・成功artifact分離の試験成功 |
| 文書 | ローカルリンクとdiff --check成功 |

通常EFI SHA-256:
`c54be4cf76143829325551a7dd3110f68da7d77c1d0f23214f8385b92a7b919e`（432640byte）。
ログはout/distribution-validation/、out/distribution-regression/、
out/distribution-normal-fat32/、out/distribution-normal-exfat/、out/distribution-normal-larger/。
ローカル成果物はdirtyを記録したpreviewであり、公開候補へそのまま昇格しない。
以前のv0.2.0-rc1成果物・実USB上のEFIは変更していない。

## 未確認・次の承認

- 実USBへの新ツールによる更新、再接続後の照合、新通常版の実PC起動は未実施。
- Windows/Linuxの物理アダプターと検証は未実装・未実施。
- 初回raw配備の実デバイス用識別／限定範囲転送アダプターは未提供。汎用書込みソフトは未検証。
- GitHub Actions上の実行・実APIでの下書き添付は未実施。静的検査とモックでの確認に限る。
- 正式候補はレビュー後のcleanなcommitと既存タグで作成し、自動試験・実機確認を改めて記録する。
- commit、push、タグ作成・変更、Releaseの作成・公開・変更、GitHub設定変更、実USB書込みは今回行っていない。
- 次にこれらを行うときは、対象commit・タグ・成果物・USBを明示して別途承認を受ける。
- MI68の配布確定PDF・展示紙は変更していない。

最終プレビュー `dist/distribution-local-validation` は以下のEFIで再検証した。
SHA-256: `fc1c370667736b50eda7980b8e8d7642c7cb6dc5ec81c82aeb0095b73bdeee18`、432640byte。
上記c54b…は先に検証したプレビューの識別値であり、最終成果物と同一とは扱わない。
最終版もFAT32、exFAT、容量調整後128 MiBの3経路で通常版QEMU試験に成功した。
最終配布ZIP・圧縮イメージ・元EFIのバイト一致と全assetチェックサムも成功。
実際の最終ZIPで更新CLIのdry-run→apply→再apply（変更0）を通常ディレクトリで確認し、
ユーザーデータ保持と不要な再バックアップがないことを確認した。
最終ログ: out/distribution-validation/final-artifacts.json、
final-normal-*.log、update-dry-run.json、update-applied.json、update-unchanged.json。
この最終追記は試験後に記録したため、パッケージ同梱の本書は先行プレビュー時点の記録である。
最新の結果は本ファイルと上記ログを参照する。検証後のEFI再ビルド・asset変更は行っていない。
