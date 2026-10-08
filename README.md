# Musha-OS

Musha-OSは、x86-64 / UEFIベースの薄いベアメタル実行環境です。
古いPCを活用し、アプリケーションに必要な画面、入力、USBストレージ、
有線ネットワークと自由に使えるRAM領域を提供することを目指します。

最古リファレンスはIntel NUC5i5RYH / RYK、比較機はNUC8（NUC8i5BEH）です。
QEMUと両実機で同一のUSBイメージを起動する構成を目標とします。

実装はRust（`no_std`）を主体とし、lwIPはC、CPU切替などはアセンブリを使用します。

## 現在の状態

実装を開始しました。GOPで挨拶を直接表示し、UEFI終了・専用スタックへの切替後に
`RUNTIME READY` を表示するところまでQEMUで確認済みです。
自前GDT / IDT / TSSとCPU例外診断を実装し、QEMUで#UD / #GP / #DFを確認済みです。
自前ページテーブル、RAM arenaと独自ドライバは未実装です。最初の診断アプリは `Hello Musha-OS!` と表示し、
画面・入力・RAM・ファイル・UDPの状態を確認する構成です。
[開発・ビルド手順](docs/development.md)を参照してください。

## 方針書

- [Musha-OS 基本方針 0.1.0](docs/policy-v0.1.md)

- [0.1.0 アーキテクチャ設計](docs/design-0.1.0.md)
- [0.1.0 実装計画と設計完了条件](docs/implementation-plan-0.1.0.md)

最初のリリース番号は `0.1.0` です。現在は設計作業中で、リリース済みではありません。

## リポジトリ構成

```text
README.md             プロジェクト概要
.gitignore            ローカル生成物の除外
 docs/policy-v0.1.md   基本方針・対象範囲・検証条件
```

## 開発と公開

既定ブランチは `main`、設計ブランチは `design/0.1.0` です。
GitHub: https://github.com/hirofumi-iwasaki/musha-os

## ライセンス

Musha-OSの独自コード、文書、設定ファイルは、個別に別の条件を明記したものを除き、
[Apache License 2.0](LICENSE)（SPDX: `Apache-2.0`）で提供します。

Copyright 2026 Hirofumi Iwasaki

第三者コードは各コードの元のライセンスと著作権表示を保持します。
取り込み前に配布条件と互換性を確認し、採用元・版・変更内容を記録します。
