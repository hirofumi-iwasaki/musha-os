# 次版の配布・配備手順 / Next-version distribution

この手順の成果物はまだGitHub Releasesに公開していません。
v0.1.0の確認実績とv0.2.0候補の確認は別です。
[配布方針](release-distribution-policy.md)・[実機確認状況](hardware-verification-status.md)・
[v0.2.0実機受入](hardware-acceptance-0.2.0.md)を参照してください。

## 配布物

- `musha-os-v0.2.0-x64-usb.img.gz`: 初回・復旧用64 MiB GPT/FAT32像、論理512 byteのみ。
- `musha-os-v0.2.0-x64-files.zip`: 通常更新用。update-manifest.jsonが配置先・長さ・SHA-256を定義。
- `musha-os-v0.2.0-x64-deployment.zip`: ホストツール、方針、手順、ライセンス・帰属表示。
- `build-manifest.json`と`SHA256SUMS`: ビルド元、dirty状態、タグ、条件、各成果物識別。
- `musha-os-0.2.0-candidate.tar.gz`: 開発・QEMU検証資料。Source code ZIPも起動媒体ではありません。

イメージとファイルZIPは同じEFIから生成します。イメージはEFIとMUSHA.TXTを保持し、
ライセンスはfiles.zipとdeployment.zipに同梱します。イメージだけを再配布せず、
これらを一緒に提供してください。

## 候補作成

固定Rust toolchain・Cargo.lockとClangを使用します。VERSIONはOS配布版を表し、
内部クレートの版は変更しません。通常featuresは空で、既存Cargo定義にdefault featureはありません。
独立target-dirでビルドし、任意Rust flags、debug/fault/probeを拒否します。

```sh
python3 tools/prepare-release.py --version 0.2.0 --output dist/candidate
# 未コミット変更を試すときだけ。正式候補には昇格できません。
python3 tools/prepare-release.py --version 0.2.0 --allow-dirty --output dist/preview
# 承認済み既存タグを指すcleanなHEAD上で。タグは作成しません。
python3 tools/prepare-release.py --version 0.2.0 --tag v0.2.0 --output dist/tagged-candidate
# 同一commit・version・tagの検証済み通常EFIと元manifestを再使用
python3 tools/prepare-release.py --version 0.2.0 --tag v0.2.0 --efi verified/BOOTX64.EFI --provenance verified/build-manifest.json --output dist/repack
```

公開時は再生成も不要です。試験した候補ファイルをそのまま添付してください。
再パッケージは元EFIの来歴・ハッシュ照合用で、既存assetの置換を許可しません。
来歴JSONは信頼できる保管元から取得する必要があります。EFIのバイトだけからビルド条件を証明する機能ではありません。

## 初回イメージ：通常ファイルでの容量調整

1. 配布物とSHA256SUMSを取得し、信頼できる配布元とハッシュを確認。
2. 対象USBの型番、正確な容量、論理セクターサイズを識別。初回作成は既存データを消去します。
3. 以下では実USBではなく、新規の疎ファイルだけを作成します。
4. 実デバイス用初回書込みアダプターは未提供。容量調整済み疎ファイル全体のdd転送を標準手順にしません。
5. 現在の実機用経路は、検証済みの既存FAT32配置を保持するファイル更新です。

```sh
(cd dist/candidate && shasum -a 256 -c SHA256SUMS)
cd dist/candidate
gzip -dk musha-os-v0.2.0-x64-usb.img.gz
cd ../..
python3 tools/deploy-image.py dist/candidate/musha-os-v0.2.0-x64-usb.img out/virtual-usb.img --capacity-bytes 34359738368
python3 tools/deploy-image.py dist/candidate/musha-os-v0.2.0-x64-usb.img out/virtual-usb.img --capacity-bytes 34359738368 --apply
```

最初のハッシュ照合は`(cd dist/candidate && shasum -a 256 -c SHA256SUMS)`で実行します。
capacityは仮想容量の例。実媒体について公称容量から推測してはいけません。
applyなしはdry-run。既存出力・device path・4Kn・64 MiB未満・128 GiB超・破損GPT/FATを拒否。
固定ESPは拡張せず、残りを未割当てにします。主副GPT、配列CRC、usable range、protective MBRを整合。
書込み範囲は先頭64 MiBから旧副GPTを除いた範囲と媒体末尾33セクターのみ。
読戻しも固定ESPと管理情報のみで、全空き容量を走査しません。exFATの作成・拡張は含みません。
一般的なUSB書込みソフトとの組合せは未検証です。

## 通常更新（macOS、次版ツール）

既存のMusha-OS FAT32ボリュームをマウントし、必要な個人データは事前に別媒体へ保存してください。
別プロセスによる同時更新やマウント変更を避けてください。ZIPは信頼できる配布元のSHA256SUMSと照合します。

```sh
python3 tools/update-usb.py dist/candidate/musha-os-v0.2.0-x64-files.zip --volume /Volumes/MUSHA-OS --backup out/usb-backup
# 上で表示されたUUIDを確認してから。ここでは実行しません。
python3 tools/update-usb.py dist/candidate/musha-os-v0.2.0-x64-files.zip --volume /Volumes/MUSHA-OS --backup out/usb-backup --apply --confirm-volume-uuid VERIFIED_UUID --eject
```

dry-runで対象・変更・省略を表示。外付け物理USB・FAT32・512byteを確認し、
内蔵／システム／不明媒体を拒否します。apply直前とeject直前に再識別。
旧ファイルはホスト側に退避し、一時ファイルへの書込み・同期・照合後に置換します。
レイアウト版1のみ対応。旧H19は既存EFI配置を移行入口として受け入れます。
無関係なデータの削除やGPT変更、媒体全容量のイメージ生成・転送・ハッシュ走査は行いません。
失敗しても全量書込みへフォールバックしません。
更新後はfile fsyncと同期・ファイル照合を行います。キャッシュを除いた検証が必要なら、
一度安全に取り外し、再接続・再識別後、同じコマンドのdry-runで全payloadがskippedになることを確認します。
この再接続・新ツールによる実USB更新は未検証です。

通常ディレクトリでの試験（macOSで実施、Windows/Linuxは未実行）:
```sh
python3 tools/update-usb.py dist/candidate/musha-os-v0.2.0-x64-files.zip --volume out/test-volume --backup out/test-backup --test-directory
```
test-volumeには既存EFI配置が必要です。`--apply`で通常ディレクトリへ反映できます。
物理媒体はテストモードに渡さないでください。

## 復旧・制限

FAT32の複数ファイル更新は停電に対して完全に原子的ではありません。
失敗時はbackup/update-result.jsonのcompletedとerrorを確認し、ホスト側backupの旧EFIと
変更済み旧ファイルを、対象を再識別したうえで元の相対位置へ戻して照合します。
新規追加ファイルの削除は記録を確認して個別に判断し、ユーザーファイルを一括削除しません。
USBがマウント不能なら更新を再試行せず、別媒体への救出と明示的な復旧作業へ分離します。

UEFI x64 / GOP / 対応xHCI / 外付けUSB Boot Keyboardが必要。未署名EFIのためSecure Boot設定も確認。
Mac内蔵キーボードはオプショナルで、今回の公開必須条件に追加しません。
Q終了後は再起動。FAT32起動USBとexFATデータUSBの自動選択制約は実機受入書を参照。

## Actionsと承認

PR・手動既定・v*タグpushは候補と試験記録のみ保存します。失敗ログはalwaysで保存し、
合格配布物は全試験成功後だけ保存します。タグは既存refをcommitへ解決してHEADと照合。
手動入力create_draft=trueと既存タグ指定の場合だけ、別ジョブへcontents:writeを付与します。
そのジョブではdefault branchのレビュー済みスクリプトだけを実行し、取得したassetはデータとして扱います。
同じタグの実行を直列化し、公開済みRelease・異なる下書き・不完全な下書きは拒否。
完全一致の下書きだけ再利用。タグ作成・clobber・公開コマンドはありません。
workflow_dispatchと信頼済み添付スクリプトをdefault branchへ導入してから運用してください。
Actions実行、下書き作成、実機確認、最終公開は今回未実施。公開時の再ビルドはしません。
