# Musha-OS 配布・リリース自動化方針

作成日：2026-10-12  
状態：採用する設計方針。ここに記載した自動化・配備機能の実装完了を意味しない。  
想定配置先：`docs/release-distribution-policy.md`

## 1. 目的と原則

Musha-OS本体をGitHub Releasesで配布し、README.mdからダウンロード、初回USB作成、通常更新、起動確認へ進めるようにする。リリース用ソースからのビルド、試験、配布用イメージ・更新用ファイルの生成はGitHub Actionsで自動化する。

**初回導入はディスクイメージを利用できるようにし、通常更新は変更ファイルだけの差し替えを標準とする。通常更新でUSB全容量のイメージ生成・書込み・ハッシュ計算を行わない。**

OS本体の配布先はReleasesとし、GitHub Packagesを今回の必須要件にしない。生成バイナリーやディスクイメージをGitのソース履歴に追加しない。

この方針は配布と更新に関するもの。Ring 3の単一アプリ実行などのOS設計、進行中のファイルシステム・入力機能、各版の公開条件を勝手に変更しない。

## 2. 調査した既存実装と、適用時の扱い

2026-10-12に、GitHubの`release/0.2.0`、コミット`d661a4e650a3ec5b12657c65b036701cd6e2a2b7`で以下を確認した。ローカル作業が先行する可能性があるため、適用時は現在のブランチ・差分・実装を再確認する。このコミットへの巻き戻しや切り替えを指示するものではない。

| 対象 | 調査時点の状態 | 変更の方向 |
|---|---|---|
| `.github/workflows/release-candidate.yml` | 手動実行・PRによるビルド、試験、候補保存。タグ・Release作成はしない [R1] | 既存回帰試験を維持し、正式な配布候補生成と下書き添付を別の権限境界で追加 |
| `tools/prepare-release.py` | 通常EFIを分離ビルドし候補を作る。名称・manifest等に`0.1.0`の固定値が残る [R2] | バージョンを引数・タグと整合させ、検証済みEFIを再ビルドせずパッケージ化できるようにする |
| `docs/usb-image.md` | 既定64MiBのGPT/FAT32イメージ。大きい実USBへの単純コピーは配置要件を満たさない [R3] | 小さな配布イメージと、実媒体容量に応じた管理情報の調整を一体で設計 |
| 通常のファイル更新 | 変更ファイルのみを更新し、既存データ・媒体構成を保持する合意済み方針 | 既存ツールがあれば再利用し、全量書込み方式へ戻さない |

調査時点の古い準備記録から、現在の実機確認状況を逆算しない。互換性の実績は、最新の`docs/hardware-verification-status.md`と、対象バイナリーに対応する試験記録から記載する。

## 3. 配布物

ファイル名は以下を基本案とし、既存の命名規則に合わせて調整できる。`vX.Y.Z`は例示であり、特定版の公開を指示しない。

| 配布物 | 用途 |
|---|---|
| `musha-os-vX.Y.Z-x64-usb.img.gz` | 初回作成・復旧用の小さな圧縮ディスクイメージ。使用条件と容量調整方法を添付 |
| `musha-os-vX.Y.Z-x64-files.zip` | 既存USB向けのファイル更新用パッケージ |
| `SHA256SUMS` | 最終配布ファイルのSHA-256 |
| `build-manifest.json` | バージョン、タグ、コミット、通常ビルド条件、ツール版、EFI識別情報、媒体レイアウト情報 |
| 配備ツール・詳細手順 | 実装したホストOSでの初回作成・更新・検証を行うためのもの |
| QEMU専用イメージ・資料 | 必要な場合だけ追加し、一般利用者向けの初回導入物と区別 |

初回イメージと更新用ZIPには、**同じ通常版EFIの同じバイト列**を入れる。イメージ内から取り出したEFIとZIP内のEFIのハッシュ一致を試験する。

ライセンス・NOTICE・第三者の必要な帰属表示を、各配布形態で不足なく提供する。ZIPには、更新対象と配置先を定義したmanifestを含める。現在のEFI・診断データだけでなく、将来のアプリやデータ追加にも対応できる契約にする。

`build-manifest.json`にはpayloadのハッシュを記録し、`SHA256SUMS`には生成完了後の配布物とmanifestのハッシュを記録するなど、自己参照・循環ハッシュを避ける。圧縮前イメージのサイズ・SHA-256、最低媒体容量、対応論理セクターサイズ、配備ツールに要求するレイアウト版も記録する。

## 4. 初回USB作成：容量差を正しく扱う

### 4.1 必須の区別

UEFI起動用のFAT32に`EFI/BOOT/BOOTX64.EFI`を配置する方式と、媒体全体をイメージとして転送する方式は別である。正常な既存媒体を利用できる場合、初回でもファイル配置方式を利用できる。

調査した現行GPT読取りでは、主・副GPTの整合性や実媒体末尾のバックアップ情報を検査する。64MiB用のGPTを32GB媒体の先頭へコピーするだけでは不十分である [R3]。一方、必要な管理情報を正しい位置へ配置することと、空き領域も含めた32GB全量を転送することは同義ではない。

**容量差への対処が未完成のイメージを「任意容量のUSBへ書くだけで起動可能」として配布しない。** また、配布の都合でOS側のGPT検査を安易に緩めない。

### 4.2 基本案

GPTと起動用FAT32を維持し、配布イメージは媒体の公称容量ではなく、起動に必要な小さな固定レイアウトにする。具体的な容量は現在の生成器・FAT32・ランタイムの条件から決め、文書と試験に記録する。

ホスト側の配備処理は、対象の正確な容量と論理セクターサイズを調べ、イメージの必要な範囲を転送した後、バックアップGPTの媒体末尾への配置、主・副ヘッダー、配列、CRC、使用可能範囲、protective MBRなどを整合させる。単に末尾へヘッダーをコピーするだけの処理にはしない。

初版ではFAT32領域を固定サイズのままとし、残りを未割当てにしてもよい。空き全容量への拡張、exFATデータ領域の新規作成、ユーザーデータの移行は独立した機能として扱う。進行中のexFAT対応のレイアウトと矛盾する場合は、その設計と統合して決定する。

論理セクターサイズの不一致、小さすぎる媒体、破損イメージ、対象の曖昧さは書込み前に拒否する。512byte前提のイメージを4Kn媒体へそのまま適用しない。4Kn未対応なら明記して拒否する。

一般的なイメージ書込みソフトをREADMEに記載するのは、そのソフトと後処理を含む手順を検証してからとする。容量調整が必要なら、取得から後処理までを利用者に明示する。

### 4.3 安全性と検証

初回作成は既存データを消去する操作である。dry-runを既定とし、型番・容量・接続方式・識別子を表示し、対象媒体への明示承認後だけ実書込みする。内蔵・システムディスクを拒否し、直前に対象を再確認する。`/dev/diskN`などを固定値にしない。

通常ファイルに対するレイアウト調整ロジックと、実デバイスへアクセスする部分を分離する。最初は疎ファイルの仮想媒体だけで検証する。今回の開発指示は、実USBを消去する承認を含まない。

転送前イメージと容量調整後の媒体はGPT部分が意図的に異なる。検証は、その違いを考慮して、payloadの読み戻し、調整後メタデータの独立検査、変更禁止領域の不変性で行う。初回導入のたびに全空き領域を読み書きすることは既定にしない。F3などの全容量検査は別の媒体健全性試験である。

### 4.4 パーティション種別：Basic Dataを採用（v0.3.0、2026-10-13決定）

これまでの起動用FAT32はGPTの種別をEFI System Partition（ESP）としてきた。macOSのFinderとWindowsのエクスプローラーはESPを自動マウントしないため、利用者が`AUTORUN.ELF`や自分のファイルを置けない。v0.3.0以降、**起動用FAT32の種別はMicrosoft Basic Data（`EBD0A0A2-B9E5-4433-87C0-68B6B72699C7`）**とし、単一パーティション構成を維持する。UEFIのリムーバブル媒体からの起動は`\EFI\BOOT\BOOTX64.EFI`がFATで見つかれば足り、ESPの種別GUIDを要求しない。

- OS側のFAT32探索はGPTのBasic DataとESPの両方を受け入れ、FAT32候補がちょうど一つであることを要求する。既存のESP構成の媒体（Mac・Ryzen・NUC5で使用中の専用SanDisk）は引き続き起動できる。候補が複数ある場合は従来どおり拒否し、「安易に緩めない」原則を保つ。exFATデータ媒体の探索（Basic Data）と同じ媒体にFAT32とexFATが同居する構成は対象外のままとする。
- イメージ生成（`make-usb-image.py`）はBasic Dataを既定にし、互換試験のためESPも選べるようにする。配備（`deploy-image.py`）と通常更新（`update-usb.py`）は両種別を受け入れ、種別の書換えは行わない。
- 本決定は実機3機（NUC5・MacBook Pro 2018・Ryzen機）でBasic Data媒体からの起動を確認してから公開資料に反映する。既存ESP媒体については引き続き手動マウントが必要と案内する。

実装上の候補はESPとBasic Dataの合計で数え、BPB検証前に一つを要求する。既存のGPT検証は緩めない。
各媒体ではexFAT探索を先に行い、Unsupported時のみFAT32へ進む。最初に利用可能な媒体を保持し、
AUTORUN.ELFの有無で媒体を乗り換えない。有効なexFATとFAT32が同居する対象外構成では、
GPTのエントリー順にかかわらずexFATが選ばれ、FAT32単独探索は複数候補を拒否する。
2026-10-14：候補9f8d999のBasic Data媒体からNUC5・Ryzen・MacBook Pro 2018の起動を確認
（IMG_7647／7648／7650、[実機受入記録](hardware-acceptance-0.3.0.md)）。当該3構成の起動確認であり、
公開受入全体の完了や全機種互換性を意味しない。ローカル証拠は[Basic Data検証記録](basic-data-validation.md)。

## 5. 通常更新：ファイルだけを変更する

既存のGPT/FAT32、データ領域、ユーザーファイルを保持し、配布manifestで指定した変更ファイルだけを更新する。内容が同じファイルは省略する。通常更新ではディスクイメージを作らず、媒体全容量も走査しない。

対象ボリュームの識別、配置先の検査、版・レイアウト互換性の確認、旧ファイルのホスト側退避、一時ファイルへの書込みと検証、差し替え、同期と安全な取り外しを行う。絶対パス、`..`、symlink経由の脱出など、アーカイブ内の危険な配置先は拒否する。

書込み後は対象ファイルのハッシュを照合する。キャッシュの影響を避けた読戻しを行う場合は、安全な再マウント手順をホストOSごとに定義する。FAT32上の複数ファイル更新が停電に対して完全に原子的になるとは保証しない。途中失敗時の状態表示と復旧手順を持つ。

更新失敗、空き容量不足、レイアウト非互換があっても、自動的なフォーマットや全量書込みへ切り替えない。レイアウト変更が必要なら、バックアップと明示的な初回再作成・移行作業へ分離する。

「瞬時」は保証値にしない。更新時間と読書き量は変更ファイル規模に依存する設計とし、コンパイル・同期・取り外しの時間は別に計測する。

## 6. ビルドとパッケージ化

既存スクリプトを拡張し、通常ビルド、試験、パッケージ化の実装を重複させない。OSの配布バージョンを引数とタグから検証し、出力名・リリースノート・manifestを一致させる。独立した内部クレートまで機械的に同じ版へ変更することは求めない。

実機配布用のfeaturesを許可リストで管理し、QEMU専用debug、故障注入、実験用probeを混入させない。既存の必要な通常機能を失うように、根拠なく`--no-default-features`へ変更しない。配布用は独立した出力ディレクトリを使い、ビルド条件を記録する。

Rust・UEFI target・依存はリポジトリの固定設定とlockfileを尊重する。ランナーやClang、QEMU、firmwareの版・識別情報も記録する。既存試験を落としてビルドを通す変更は行わない。

正式候補は固定コミットとcleanなソースから生成する。未コミット変更を含む成果物はプレビューと明記し、正式候補へ昇格させない。検証済みEFIを受け取ってパッケージ化する経路を設け、方式Aでは実機確認後の文書コミットを同条件で再ビルドし、以下の同等性検査を必須とする。

### 6.1 Reproducible build conditions and tag provenance / 再現ビルド条件とタグの来歴

Procedure A is adopted after the two byte-identical local builds described in
[development instructions](development.md). Use the committed UEFI `/Brepro` +
`/debug:none` configuration and identical source/build inputs and toolchain.
A documentation-only successor may become the tag target **only after a clean
rebuild from that final commit is byte-identical to the hardware-accepted EFI**.
Record both source commits, both hashes, the environment and comparison evidence.
Generate the final manifest and packages from the final commit/rebuild; the tag
must still equal the build-manifest source commit. Do not relabel an old manifest
or bypass existing source checks. Any mismatch stops publication for investigation
and owner decision. Current local reproducibility does not establish future final
commit equivalence or hardware acceptance. Published tags/binaries remain fixed.

[開発手順](development.md)に記録したローカル2回のバイト一致により方式Aを採用します。
コミット済みUEFI設定 `/Brepro` + `/debug:none` と同一のビルド入力・ツール版を用います。
**文書のみの後続コミットは、その最終コミットのクリーン再ビルドが実機受入済みEFIと
バイト一致する場合だけタグ対象にできます。** 両ソースコミット・両ハッシュ・環境・比較証拠を
残し、最終manifestと配布物は最終コミット／再ビルドから生成します。
タグ＝build-manifestのソースコミットの規則は維持し、旧manifestの来歴の付替えや
既存検査の迂回は行いません。不一致なら公開を停止して原因調査・所有者判断へ戻します。
今回のローカル一致は将来の最終文書コミットの一致や実機受入の証明ではありません。
公開済みタグ・バイナリーは固定します。

### 6.2 Owner clarification: comparison boundaries (2026-10-14)

For two clean builds of the **same commit**, require identical EFI and all five
distribution archives, with matching manifests/checksums under the same recorded
toolchain, environment and packaging inputs. Record differences; do not silently
normalize existing output or declare byte equality using a payload-only comparison.

For an approved **documentation-only successor**, require identical EFI and
runtime payload (all managed USB content except generated provenance manifests,
plus the SDK Hello ELF). Regenerate packages with truthful final-commit provenance.
Compare extracted members: only approved documentation, source-commit provenance
and their derived manifests/checksums may differ. Record both commits, archive and
member hashes, approved paths and reasons; any executable/data difference or
unexplained metadata difference stops publication. The final tag equals the final
build-manifest source commit. No package format change is authorized by this rule.

Publication consists of five archives (initial image, file update, C SDK,
deployment tools, complete tar.gz), build-manifest.json and SHA256SUMS: seven
uploaded files. EFI is included inside the archives, not a standalone asset.
Deployment ZIP generation uses sorted member names, the fixed ZIP timestamp
1980-01-01 00:00:00, Unix regular-file mode 0644 and DEFLATE, independent of
source/copy mtimes and permissions. File paths and contents are unchanged.
The whole-package reproduction gate remains unverified until stage 3.

### 6.2 所有者による照合範囲の明確化（2026-10-14）

**同一コミット**のクリーンビルド2回は、同じ記録済みツール・環境・梱包入力のもと、
EFI・5配布アーカイブ・manifest・チェックサムの一致を要求します。不一致は記録し、
生成後の無断正規化やpayloadだけの比較で全バイト一致と扱ってはいけません。

承認済みの**文書のみの後続コミット**ではEFIと実行用payload（生成来歴manifestを除く
USB管理ファイル一式、およびSDK Hello ELF）の一致を要求します。最終コミットの正しい
来歴で再梱包し、展開メンバー単位で比較します。承認した文書・ソースコミット来歴・
それらから派生するmanifestとチェックサムだけ変更を認めます。両コミット、アーカイブ・
各メンバーのハッシュ、許可パスと理由を記録し、実行コード・データの差分や説明できない
メタデータ差分があれば公開を停止します。タグは最終build-manifestのソースコミットと
一致させます。この規則は配布形式の変更を許可するものではありません。

公開は5アーカイブ（初回イメージ・更新ZIP・C SDK・配備ツール・一式tar.gz）と
build-manifest.json・SHA256SUMSの計7ファイル。EFIはアーカイブ内に含めます。
deployment ZIPはメンバー名順、固定ZIP日時1980-01-01 00:00:00、Unix通常ファイル
0644、DEFLATEで生成し、入力やコピー先の更新時刻・権限へ依存させません。
配置パスと内容は変更しません。全配布物の再現性ゲートは段階3の再検証まで未確認です。

## 7. GitHub ActionsからReleaseへ

### 7.1 公開までの順序

```text
承認済みコミットを固定
  → 手動候補ビルド、または承認済みバージョンタグのpush
  → 通常ビルドと自動試験
  → 同じEFIから初回イメージ・更新ZIPを生成
  → manifest・チェックサム・配布物検査
  → 下書きReleaseへ添付
  → 必要な実機確認と所有者の公開承認
  → 同じ添付物のまま公開
```

`release: published`を唯一のビルド起点にせず、公開時点で配布物がそろっている状態にする。手動実行やタグを起点にできることはGitHubのイベント仕様を参照する [G1]。手動実行設定の既定ブランチへの配置要件も運用手順に含める。

### 7.2 候補生成と下書き添付の分離

PR・通常の候補作成は`contents: read`を基本とし、成果物をActionsへ保存するだけにする。失敗ログの保存先と、合格した配布候補を区別し、不合格のバイナリーを配布しない。

下書きへの添付は、承認された既存タグを対象とする専用ジョブ／ワークフローに限定する。`workflow_dispatch`の既定は候補のみとし、明示的な下書き作成指定か、運用開始を承認したタグ起点で実行する。必要な場合だけ`contents: write`を付与する。

タグはコミットへ解決し、ビルド元SHAとの一致を検査する。GitHub CLIを使う場合は`--draft`と`--verify-tag`相当の安全条件を設け、存在しないタグを自動作成させない [G2]。任意の手動入力やPR由来文字列をそのままシェルへ展開しない。

PRやforkの成果物を権限付きジョブで無条件に公開しない。下書き添付ジョブはアーカイブをデータとして扱い、未検証の配布スクリプトを実行しない。Actionsの参照は検証済みの固定コミットを用いる方針とする。

### 7.3 再実行と公開後の固定

同じタグの公開済みReleaseが存在すれば停止する。既存の下書きには、コミット・manifest・同名assetの内容が一致する場合だけ再利用を許可する。異なるファイルを`--clobber`などで無断上書きしない。失敗した下書きは未公開のままにし、失敗状況を報告する。

同一版の並行実行を制御し、公開直前にもタグの指すコミットと添付物のチェックサムを確認する。最終公開は当面、人が判断する。将来公開処理を自動化する場合も、明示的な承認段階を残す。

Immutable releasesを有効にする場合は、下書きへ全assetを添付してから公開する [G3]。この方針書だけでリポジトリ設定の変更や既存リリースの再作成を許可しない。

## 8. README.mdと説明書

READMEの冒頭側に以下を整理し、詳細は`docs/`へリンクする。現在の英語・日本語構成を維持する。

1. **ダウンロード**：GitHub Releasesと、初回用イメージ／通常更新用ZIPの選び方。自動生成の`Source code (zip)`と混同させない。
2. **想定互換性**：「互換性はx64機、NUC5以上を想定。」を記載し、UEFI・GOP・xHCI等の条件と、機種別の実績・制限を分ける。
3. **初めて使う**：取得、ハッシュ照合、対象USB確認、書込み、必要な容量調整、検証、安全な取り外し、USB起動。
4. **既存USBを更新する**：必要なファイルだけ差し替える。通常はフォーマット・全量書込み不要。
5. **安全上の注意**：初回のデータ消去と通常更新のデータ保持を明確に区別する。必要なSecure Boot設定も現行手順に合わせる。
6. **困ったとき**：起動しない場合の診断、旧EFIへ戻す手順、既知の制限。

macOS・Windows・Linuxの各詳細手順は、実際に提供・検証できたツールで記述する。未対応ホストを対応済みにせず、設計中のコマンドを現に使える手順として掲載しない。

GitHubへの添付が承認・完了するまでは、新しいダウンロードファイルが公開済みとは記載しない。既存の配布確定PDFと展示紙は今回変更しない。

## 9. 合格条件

| 分野 | 最低限の確認 |
|---|---|
| 版の整合 | タグ・OS配布バージョン・出力名・manifestが一致。欠落や不一致は公開側で拒否 |
| 成果物の整合 | 通常EFIと、イメージ・ZIP内EFIのSHA-256が一致。ライセンスと必要データが存在 |
| 通常版の試験 | debug版の試験に加え、実際に配る通常版そのものの起動・表示・入力・ファイル処理を確認 |
| 初回イメージ | 同容量と複数の大きい仮想媒体でGPTの調整とFAT32読出しに成功。小さい媒体・セクター不一致を拒否 |
| 配備範囲 | 書込み対象領域を記録し、全空き領域の転送・ハッシュ走査を既定で行わないことを確認 |
| 通常更新 | 変更ファイルだけ更新、同一ファイル省略、既存データ保持、失敗時の退避・復旧を試験 |
| CI | PRでは公開権限なし。失敗時は下書き添付なし。危険な入力と同一版の競合・再実行を試験 |
| 実機 | 対象のEFI・媒体・機種と結果を記録。未実施は未実施として残す |
| 公開 | 下書きの内容と実機結果を所有者が確認。同じバイナリーを再ビルドせず公開 |

ホスト単体試験、QEMU、実USB作成・更新、実PC起動、GitHub上のworkflow実行を別々に記録する。実機が未接続でも、方針・コード・通常ファイルによる試験は進められる。ただし実施していない確認を完了扱いにはしない。

## 10. 今回の作業権限

今回依頼する範囲は、既存ローカル開発へ本方針を統合し、README・設計書・ビルド／配備ツール・workflow・試験を実装すること。

**今回の指示だけではcommit、push、タグ作成・変更、Releaseの下書き作成・公開・変更、リポジトリ設定変更、物理USBへの書込みを行わない。** 自動化コードを作ることと、それを実際のリポジトリ・デバイスに対して実行することを分離する。これらの操作は対象と内容を示して別途承認を受ける。

## 参照資料

[R1] [既存release-candidate.yml（調査コミット固定）](https://github.com/hirofumi-iwasaki/musha-os/blob/d661a4e650a3ec5b12657c65b036701cd6e2a2b7/.github/workflows/release-candidate.yml)  
[R2] [既存prepare-release.py（調査コミット固定）](https://github.com/hirofumi-iwasaki/musha-os/blob/d661a4e650a3ec5b12657c65b036701cd6e2a2b7/tools/prepare-release.py)  
[R3] [既存USBイメージ仕様（調査コミット固定）](https://github.com/hirofumi-iwasaki/musha-os/blob/d661a4e650a3ec5b12657c65b036701cd6e2a2b7/docs/usb-image.md)  
[G1] [GitHub Actions：起動イベント](https://docs.github.com/en/actions/reference/workflows-and-actions/events-that-trigger-workflows)  
[G2] [GitHub CLI：gh release create](https://cli.github.com/manual/gh_release_create)  
[G3] [GitHub：Immutable releases](https://docs.github.com/en/code-security/concepts/supply-chain-security/immutable-releases)

## Managed marker migration (2026-10-14) / 管理マーカーの移行

English: the GPT/FAT32 disk layout remains **layout_version 1**. An update
package containing the new `DIAGNOSE.TXT` uses **manifest schema 2** and
`retired_files: ["MUSHA.TXT"]`. This is a file contract change, not a partition
migration. The new updater accepts schema 1 without removals and schema 2
with this exact retirement list and the replacement marker. Old schema-1-only
tools reject schema 2; use the matching deployment archive. Unknown layouts,
arbitrary retirement names, path aliases, symlinks and payload/removal overlap
are rejected before writes. Existing layout-1 media (including installations
without state metadata) need no format change.

Dry-run lists `changed` and `removed`. Apply backs up all replaced and retired
files on the host, checks each backup, atomically replaces changed payloads,
then removes the retired marker. An edited old marker is preserved in the
backup too. Unmanaged files, including `AUTORUN.ELF`, are untouched. Interrupted
updates are not automatically rolled back: retain the backup and update report.
The verified `update-manifest.json` is installed; `verified-files` records the
hash and size of every payload and that manifest, and `verified-absent` records
the retired name. Unchanged files are read for verification but never replaced.
No whole-medium image generation, write or hash is part of an update.

日本語：GPT/FAT32のディスク構成は **layout_version 1** のままです。
`DIAGNOSE.TXT` を含む更新ZIPは **manifest schema 2** と
`retired_files: ["MUSHA.TXT"]` を使用します。パーティション移行ではなく
管理ファイル契約の変更です。新ツールは削除なしのschema 1と、正確な廃止名一覧・
置換先を持つschema 2を受け付けます。旧ツールはschema 2を拒否するため、同梱の
配備ツールを使用してください。未知のレイアウト、任意の削除名、大小文字別名、
symlink、更新対象と削除対象の重複は書込み前に拒否します。
管理状態ファイルのない既存媒体も、既存EFIがあるlayout 1として扱います。

dry-runは変更と削除予定を表示します。applyは全置換・廃止ファイルをホストへ
退避・照合してから、変更ファイルを一時ファイル経由で置換し、旧マーカーを削除します。
編集済み旧マーカーも退避します。`AUTORUN.ELF` 等の管理対象外ファイルは保持します。
中断時は自動復旧しないため、退避と更新記録を保持してください。
`update-manifest.json` 自体も配置し、`verified-files` に全管理ファイルとmanifestの
長さ・SHA-256、`verified-absent` に旧名不在を記録します。同一内容は照合のみで
差し替えません。通常更新でUSB全容量の像生成・書込み・ハッシュ計算は行いません。
