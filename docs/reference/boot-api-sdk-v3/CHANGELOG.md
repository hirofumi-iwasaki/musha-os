# 文書変更記録

## 3.0 — 2026-10-13

対象：`docs/boot-readiness-policy.md`、`handoff/implementation-prompt.md`。
**文書版3.0であり、Musha-OS 3.0や外部ABI 3.0の意味ではありません。**

### 主な更新

1. **iBCS／iBCS2を採用しないことを明示**。SysV AMD64の通常C ABI、静的ELF64、Musha-OS固有の起動・syscall ABIを区別。
2. **採用方針・推奨案・未確定値を区別**。前回の説明中の具体例を、そのまま公開済み仕様にしない。
3. **通常C ABIを具体化**。LP64、整数・FP引数、保存GPR、通常callの16byte整列、no-red-zoneの初期SDK候補。
4. **ELF検査を具体化**。ET_EXEC、PT_LOAD、BSS、ページ単位W^X、NOTE、非対応の動的依存／TLS等の扱い。
5. **起動ABIを具体化**。_startと通常C関数のスタック整列を分離。RDI起動情報、既知の初期状態、crt0／main／終了syscall。
6. **64byte起動情報案をC／Rustで記載**。8byte整列、全field位置、予約欄、ユーザー仮想アドレス、読出し専用保持を明示。
7. **syscall候補を具体化**。SYSCALL／IRETQ、RAX番号、RDI／RSI／RDX／R10／R8／R9、RAX状態・RDX結果、保存・破壊状態。
8. **危険な曖昧さを解消**。RDXの入力兼出力、raw syscallとC wrapperの違い、64bit符号拡張、TSSだけではSYSCALLのstack切替にならない点。
9. **Clang／RustのFP ABIを独立して検証**。Softfloat、custom target、core／alloc再ビルド、nightly等の条件と、既存カーネルtoolchainの維持。
10. **ABI台帳と試験を追加**。番号・エラー値・NOTE・配置・CPU ID等は未確定として管理し、FFI・実CPL・異常復帰・FP保存・旧バイナリー試験へ接続。
11. **起動状態の表を整合**。APP_ENTERINGとAPP_RUNNINGを明示的に分け、移行直前の記録だけを実到達の証拠としない。
12. **不足していた参照記録を同梱**。旧版にリンクだけあったhandoff/source-review.mdを本版で用意。

### 維持したもの

起動分離、API／ABI／SDKの3層、OS_READYの実サービス条件、描画・診断とI/O寿命の分離、単一Ring 3アプリに主導権を渡す方針、定期step／pollを外部SDKへ強制しないこと、A＋B先行／C＋D後続、DMA安全性、現在のFAT32／exFAT／ビューアー等の進捗、通常USB更新は変更ファイルだけ、既存リリース条件を自動拡大しないこと、操作の承認範囲は維持しました。

### この更新で行っていないこと

開発リポジトリの最新監査・編集、SDKやOSの実装、Ring 3／syscall実行試験、実機試験、commit／push／Release操作、物理USB操作、確定済みPDF変更、別スレッドへの送信。

文書内のCレイアウト例の整合確認と、文書・ZIPの検査の結果はhandoff/source-review.mdに記録します。それらはMusha-OSのABI実装やC↔Rust実行試験ではありません。

---

## 2.0 — 2026-10-13

対象：`docs/boot-readiness-policy.md`、`handoff/implementation-prompt.md`。

### 追加・更新

1. **API・ABI・SDKの責任を分離**：操作の意味、機械語の境界、言語別の開発環境を別の成果物として定義。
2. **OS_READYと公開APIを接続**：起動プロファイル、利用可能なサービスと上限、権限、版付き起動レポートを区別。内部構造体のポインターを外部へ渡さない。
3. **起動状態を精密化**：アプリへ移る直前のAPP_ENTERINGと実行到達を区別。初回描画は表示APIのソフトウェア上の完了とし、画面を描かないアプリも認める。
4. **現行内部APIからの移行**：静的リンク／コールバックの実装は維持し、外部ABIへそのまま固定しない。内部API版と外部ABI版・SDK版を分離。
5. **外部アプリ方式**：将来の静的ELF64、Ring 3、crt0、ユーザー領域、版・CPU・必要機能検査、C／Rustの同一OS上のHelloを定義。
6. **公開ABIの注意点**：Cの通常呼出しとsyscallを区別。固定幅型、構造体配置、互換性、無効ハンドル／ポインター、FPU・SIMD条件を追加。
7. **Clang／LLD・Rust向けSDK**：sysroot、libmusha、musha-sys／musha、補助ライブラリ、CMake／Cargo、ELF検査とQEMU支援を整理。
8. **I/Oの動作契約**：同期・イベント型・任意async、要求／完了／取消し、OS所有バッファ、waitの通知競合を追加。定期pollを最終SDKへ強制しない。
9. **工程をA～Dへ整理**：直近は起動分離と仕様／SDK土台。Ring 3の最小経路と実用I/Oは後続工程として明示。
10. **試験を段階別に追加**：C／Rustのデータ配置、targetビルド、ホスト依存混入、ELF、実CPL、旧ABI、長い計算中のI/O進行等を区別。

### 保持した方針

- CORE_READY、OS_READY、アプリ実行・初回画面の区別。
- 記録と描画を分離し、表示やページ巡回を起動条件にしないこと。
- 必須と任意のサービス、診断ビューとI/O資源の寿命の分離。
- メモリ・機器の安全検査、DMA停止前に再利用しないこと、エラー記録。
- アプリの主導権と専有RAM内の自由、現行協調実装を移行基盤として使えること。
- 既存のFAT32/exFAT、ファイルビューアー、入力安定化などの進捗を維持すること。
- 通常USB更新は変更ファイルのみ。リリース条件を無条件に膨らませないこと。
- commit／push／タグ／Release／実USB／確定済みPDFへの操作は、この依頼では行わないこと。

### この更新で実行していないこと

コード実装、SDKの実ビルド、ABIのバイナリー互換性試験、QEMU／実機試験、Mac側ファイル更新、開発スレッドへの送信、GitHubへの書込み。

文書内に示す関数・配置先・ツール構成は、既存機能と明示したものを除き設計案です。実装段階に応じて確定・検証し、未完成な部分を使用可能として案内しないよう指示しています。
