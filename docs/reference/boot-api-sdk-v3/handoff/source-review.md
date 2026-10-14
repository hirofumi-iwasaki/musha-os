# 参照・更新・文書検査記録

- 日付：2026-10-13
- 対象：文書版3.0
- 目的：前回2.0の2文書へ、直前のABI設計の説明を反映する。

## 1. 実際の編集元

この会話で作成・添付された次の文書を読み、本文を維持した上で更新した。

| 旧版 | 行数 | 編集前SHA-256 |
|---|---:|---|
| docs/boot-readiness-policy.md | 464 | `97d508dbf309f7d70812be8d87b2d043a926eed317797ad8724afd86be306455` |
| handoff/implementation-prompt.md | 214 | `cdcf18e7e6c564f8e6b869f6b47c7c6740e0b2339da087eaed1860329547f266` |

新しい文書は別ディレクトリーへ作成し、元の2.0を上書きしていない。
旧2.0の本文にはhandoff/source-review.mdへのリンクがあったが、今回利用できた2.0の実ファイル一式には同ファイルがなかったため、本3.0で参照記録を用意した。

## 2. 今回のコード確認範囲

**今回、Musha-OSの最新リポジトリや別スレッドのローカル変更を再監査していない。**
編集は旧2.0と今回のABIの説明を基準とする。旧2.0にある公開mainの参照点
`322ee6754b2092f73c663ac39b483a5a3c8b9093`、内部API版5等は歴史的な参照点として残し、
現在の完成度や互換性を証明するものとして使わない。開発スレッドで作業開始時のコードを確認する。

この依頼では、開発リポジトリへの文書統合・コード変更・実装テスト・commit／push・
タグ／Release操作・物理USB操作・PDF更新・スレッドへの送信を行っていない。

## 3. 技術事項の再確認に使った一次資料

方針書末尾の[S9]～[S16]に、今回追加した資料を記載した。
SysV psABI本文、ELF gABI、歴史的OpenBSD iBCS2マニュアル、Linux syscall入口実装、
Rustの公式ターゲット文書とターゲット定義、custom target／build-std、Clang／LLDの公式資料を参照した。
Intel SDMは公式の配布入口を確認し、CPU詳細の実装段階で仕様本文と版を確認する参照先として記載した。

これらは標準機構とツール条件の根拠であり、Musha-OSの固有ABIを指定するものではない。
特にRAX=状態／RDX=結果、64byte起動情報、NOTE、最初のET_EXEC／no-red-zoneはMusha-OS側の候補である。
Linux互換やiBCS互換を採用したと解釈しない。参照実装のコードを取り込んでもいない。

参照日は2026-10-13。master／main／latestの資料は将来変わるため、実装・公開時には
採用版と必要なソースrevisionを別途固定する。RustのSoftfloat指定とcustom targetの項目名等も、
使用するrustcの固定版で確認する。

## 4. 今回実行した文書・レイアウト例の検査

### Cのレイアウト確認

方針書第12.5節のCコードを抽出し、全11フィールドのoffsetと、int／long／pointerの型幅の
static assertionを加えた。再現用ソースは`review/check-start-info.c`。

環境：Linuxの作業コンテナー、Clang 17.0.0（Swift LLVM配布に含まれるClang）。
以下で**x64向けオブジェクトのコンパイル**を行い、成功した。

```sh
clang --target=x86_64-unknown-none-elf -std=c11 -ffreestanding \
  -mno-red-zone -Wall -Wextra -Werror \
  -c review/check-start-info.c -o /tmp/musha-start-info-check.o

readelf -h /tmp/musha-start-info-check.o
```

確認したこと：サイズ64byte、整列8byte、全フィールドoffset、int=4byte／long=8byte／pointer=8byte。
生成物はELF64・リトルエンディアン・x86-64の**ET_RELオブジェクト**であり、実行可能なアプリではない。
ELFヘッダーのSystem V表示を、Musha-OSで実行できる証拠として扱っていない。

このコマンドは文書の例を検査するためのもので、最終SDKのコンパイル・リンク手順ではない。
使用したClang 17をプロジェクトの採用版として指定するものでもない。

### Rustと文書の確認

CとRustのレコードについて、フィールドの並びと固定幅型の対応をテキストで照合した。
**Rustコンパイラーは本作業環境で利用しておらず、Rustのコンパイル検査・FFI実行は行っていない。**

コードフェンス、主要見出し、ローカルリンク、2文書の版・章参照・レジスター候補の整合、
旧方針の保持、差分の生成、ZIP内ファイルと元ファイルの一致を検査した。

### 行っていないABI試験

C↔Rustの関数呼出し実行、ET_EXECの生成・ロード、Ring 3への遷移、syscall往復、
保存レジスター、FPU／SIMD保存、例外・NMI・割り込み、QEMU上のアプリ実行、実機確認、
旧アプリのバイナリー互換性試験は行っていない。これらは更新した指示書のC／D工程へ残した。

## 5. 版固定について

採用方針、推奨値、未確定値を分離した。syscall番号・エラー値・NOTE符号化・仮想配置・
公開ABI値などを、この文書作成作業で勝手に正式採番してはいない。
Bで候補をexperimentalな実装仕様へ落とし、C／Dの合格と互換性判断を経て公開ABIを固定する。
