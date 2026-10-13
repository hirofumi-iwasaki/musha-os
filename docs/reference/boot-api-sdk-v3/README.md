# Musha-OS 起動分離・API／ABI／SDK統合方針 3.0

前回の文書版2.0を更新し、ABIに関する今回の説明を統合した、**開発スレッドへの引き継ぎ資料**です。
文書版3.0はOSのバージョンや外部ABIのバージョンではありません。

## 主なファイル

| ファイル | 役割 |
|---|---|
| [docs/boot-readiness-policy.md](docs/boot-readiness-policy.md) | 起動完了・初期化・記録・描画の分離とAPI／ABI／SDKの統合方針。第12節にABI詳細案 |
| [handoff/implementation-prompt.md](handoff/implementation-prompt.md) | 開発スレッドで使う更新済みの実装指示 |
| [CHANGELOG.md](CHANGELOG.md) | 2.0からの変更と維持した方針 |
| [handoff/source-review.md](handoff/source-review.md) | 編集元、今回の確認範囲、一次資料、レイアウト例の検査結果 |
| [review/v2-to-v3.diff](review/v2-to-v3.diff) | 旧版2文書との差分。閲覧用で、自動適用用ではない |
| [review/check-start-info.c](review/check-start-info.c) | 文書に記載した64byte構造体のCレイアウト検査。SDK実装ではない |

## 反映方法

ZIPをmusha-os開発スレッドへ添付し、下記の文章を送ってください。詳細はimplementation-prompt.mdにあります。

```text
添付の統合方針3.0を、前回2.0の更新版として使用してください。
handoff/implementation-prompt.mdを実装依頼として読み、
docs/boot-readiness-policy.mdを現在のMusha-OSへ統合してください。

iBCSは採用せず、System V AMD64を基礎とする通常C ABIと、
Musha-OS固有のELF起動・システムコールABIを区別してください。
方針書第12節の64byte起動情報、スタック整列、SYSCALL／IRETQ、
レジスター割当、FP／SIMD、Clang／Rustターゲット条件を設計へ反映し、
候補値と未確定値を台帳で管理してください。

まずA（起動・記録・描画・I/O寿命の分離）と、
B（API・ABI仕様、SDKの土台、コンパイル／整合試験）を進めてください。
C／DのRing 3実行・実用I/Oは後続とし、ヘッダーやmockだけで完成扱いにしないでください。

現在のブランチ・未コミット変更・実装済み機能は保持してください。
commit、push、タグ／Release／GitHub設定変更、実USB操作、
配布確定済みPDFの変更は行わず、完了範囲と未実装・未検証を分けて報告してください。
```

このREADMEは**引き継ぎパッケージの説明**です。リポジトリ本体のREADME.mdへ丸ごと上書きするファイルではありません。
既存ファイルは差分とローカル進捗を確認して統合し、英語先行・日本語併記等の規約に従ってください。
数値候補を変更・確定する場合は、ABI仕様と生成元を正本にし、異なる定義を複数箇所へ放置しないでください。

## 今回の成果物の範囲

文書と引き継ぎ指示の更新です。アプリローダー、Ring 3、syscall、SDKを実装したものではありません。
Mac側のリポジトリ編集・別スレッドへの送信・GitHubへの書込みは行っていません。
