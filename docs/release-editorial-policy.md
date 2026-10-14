# Release documentation and publication rules

## English

These rules apply to README.md, versioned release notes, and GitHub Release descriptions, including corrections after publication.

### Language and structure

- Use a neutral product/version title, followed by `## English` and then `## 日本語`.
- Finish the entire English section before starting Japanese. Do not interleave translations or use bilingual slash headings.
- Write headings, prose, table labels, and link labels in the section's language. Commands, paths, identifiers, and literal UI messages retain their exact spelling.
- Each language must stand alone and contain equivalent scope, instructions, limitations, and validation evidence. English must not be an abbreviated summary.

### Required content

README must explain the OS, automatic startup checks, bundled applications, prerequisites, controls, downloads, and current limitations. Distinguish current verification from historical results and future plans.

Release notes must state the version/date, changes, purpose of each download, controls, limitations, known issues, completed tests, and pending hardware checks. Distinguish source archives from binaries, initial/recovery images from normal-update ZIPs, and simulated tests from hardware evidence.

### Publication checklist

1. Prepare bilingual README and versioned release notes before the release commit and tag.
2. Check language order, untranslated prose, equivalent facts and numbers, links, version, asset names, and source/hash references.
3. Build and test the approved artifacts. Do not claim hardware coverage from host or QEMU tests.
   Before publication, review the release notes' PENDING fields and update them with actual results or explicitly retained limitations. After publication and download verification, record the publication date and change the README English banner from `v0.3.0 release candidate (not published)` to `v0.3.0 release`, and remove the corresponding Japanese `公開候補（未公開）` wording. Keep candidate wording until publication is approved; do not mark unfinished checks as complete.
   Under procedure A, complete hardware acceptance before removing the three-machine pending statements in distribution policy §4.4 and the deployment guide. Record the results, rebuild the final documentation commit, compare its entire EFI with the accepted EFI, and generate its matching manifest/packages before tagging. A mismatch blocks publication. Keep separate owner approvals for push, candidate generation, USB update, remote CI, tag and Release.
4. Derive the GitHub Release description from the versioned notes. Convert relative links into absolute GitHub URLs pinned to the appropriate documentation commit. Keep binary provenance tied to its original source commit.
5. Read back the published README and Release description. Verify links and artifact hashes, and record the PR, tag, Release URL, and remaining validation items.

### Corrections after publication

Authorized text corrections may update README and notes on main and the GitHub Release title/body. Record the correction date and material scope in both languages.

Do not move the tag or rebuild/replace binaries, archives, manifests, or checksums just to correct wording. Published archives and tagged documentation remain historical snapshots; explain differences from corrected online documentation. Behavior or package changes require a new version, validation, and separate publication approval.

See the [distribution policy](release-distribution-policy.md) and [deployment procedure](distribution-usage.md).

---

## 日本語

この規約はREADME.md、版ごとのリリースノート、GitHub Release本文、および公開後の訂正に適用します。

### 言語と構成

- 共通の製品名・版のタイトルに続けて、`## English`、`## 日本語`の順に配置します。
- 英語全体を書き終えてから日本語を開始します。翻訳を交互に並べたり、スラッシュで二言語の見出しを併記したりしません。
- 見出し、本文、表のラベル、リンクの表示名は各セクションの言語で書きます。コマンド、パス、識別子、画面表示の引用は正確な綴りを保ちます。
- 両言語だけでそれぞれ理解でき、対象範囲、手順、制限、検証根拠が対応する内容にします。英語を短い要約だけにしません。

### 必須の内容

READMEにはOS、自動起動するチェック処理、組込みアプリ、前提条件、操作、配布物、現在の制限を説明します。現在の確認結果、過去の実績、将来の計画を区別します。

リリースノートには版・日付、変更、各配布物の用途、操作、制限、既知の問題、完了した試験、未完了の実機確認を記載します。ソースとバイナリー、初回・復旧イメージと通常更新ZIP、模擬試験と実機確認を区別します。

### 公開前後の確認手順

1. リリース用コミットとタグの前に、二言語のREADMEと版ごとのリリースノートを準備します。
2. 言語の順序、未翻訳の本文、事実・数値の対応、リンク、版、配布物名、ソース・ハッシュの参照を確認します。
3. 承認された配布物をビルド・試験します。ホストやQEMUの試験を実機確認として扱いません。
   公開前にリリースノートのPENDING欄を確認し、実際の結果または明示して残す制限へ更新します。公開とダウンロード照合後、公開日を記入してREADME英語バナーの `v0.3.0 release candidate (not published)` を `v0.3.0 release` に変更し、日本語の `公開候補（未公開）` も外します。公開承認までは候補表記を維持し、未完了の検証を完了扱いにしません。
   方式Aでは実機受入後に配布方針§4.4・配備手順の3機確認前表記を外します。結果を記録した最終文書コミットを再ビルドし、受入済みEFIとの全バイト一致を確認して、そのコミットのmanifest・配布物を生成してからタグを作成します。不一致なら公開を停止します。push・候補生成・USB更新・リモートCI・タグ・Releaseはそれぞれ所有者承認を得ます。
4. 版ごとのノートからGitHub Release本文を作成します。相対リンクは適切な文書コミットに固定したGitHubの絶対URLへ変換します。バイナリーの来歴は元のソースコミットを指すままにします。
5. 公開後のREADMEとRelease本文を読み戻します。リンクと配布物のハッシュを確認し、PR、タグ、Release URL、残る検証項目を記録します。

### 公開後の訂正

承認された文章の訂正では、mainのREADME・ノートとGitHub Releaseのタイトル・本文を更新できます。訂正日と主な訂正範囲を両言語に記録します。

文章の訂正だけを理由に、タグの移動やバイナリー、アーカイブ、manifest、チェックサムの再生成・置換を行いません。公開済みアーカイブとタグ内の文書は当時の記録として保持し、訂正済みオンライン文書との違いを説明します。動作やパッケージの変更には、新しい版、検証、別途の公開承認が必要です。

[配布方針](release-distribution-policy.md)と[配備手順](distribution-usage.md)も参照してください。
