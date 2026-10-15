# BCEキュー登録モデルとHIDメモリー測定

2026-10-10、release/0.2.0。実機I/Oへ接続しない準備実装。

## キュー登録

`musha-bce::wire` に、保存済みFreeBSD `apple_bce.h` / `apple_bce_queue.c` の
64-byte command slotを明示的little-endianで生成する処理を追加した。
登録0x20、解除0x30、flush 0x40。予約byteはゼロ。登録にはQID、要素数、
vector/CQ、最大32byteの名前、方向、アドレス、長さを含む。
FreeBSDの切捨てと異なり、空・NUL入り・32byte超の指定名は拒否する。
名前なしは別途許可。既存の構造検査に加え、user QIDを2〜255へ限定する。
アドレスの符号化成功は実メモリー使用許可を意味しない。

`registration::Registration` は1つのuser queueに対して、以下を管理する。

1. New → 登録待ち → Registered。成功まで転送開始を拒否。
2. 転送は一件だけ。転送完了・CQ ack後にローカルticketを返す。
3. 未完了転送がない状態でflush → Flushed。
4. Flushedからのみ解除 → Unregistered。新たな転送を開始すると再flushが必要。
5. 応答失敗、古いticket、期限切れ、時計巻戻り、観測回数超過、転送失敗、
   部分的なpublish失敗はQuarantined。状態は終端で再試行しない。

登録用command transportは既に確立済みという前提。要求の生成前に待機状態へ移り、
callerは要求を一度だけpublishする。publishに失敗したら必ず `quarantine` を呼ぶ。
`poll`へ渡す応答はcommand SQ/CQを検証・ackした後の結果で、ticketはtransportの
pending要求との対応表から取得する。ファームウェアにticketがあるとは仮定しない。
古い同一slot応答の問題は既存CQの所有権・ack契約で扱う必要がある。

**モデルの境界:** 既存 `Transfer::new` は引き続き登録済みを前提とする低レベルモデル。
今回のgateを自動的に強制するAPI変更は行っていない。組込み側は必ず
`begin_transfer` → `Transfer::submit/poll` → `finish_transfer` を使い、失敗時は
両モデルを停止・隔離する。模擬統合テストでこの順序を確認した。

QIDの一意性、CQ依存関係、実DMA割当て・barrier・command queueの初期登録は
まだadapter側の責務。新しいモデルを作って同じQIDを再利用する許可はない。
flush/解除成功もDMA停止やバッファ解放の証拠としない。実通信、キャンセル、
ファームウェアqueueのリセット、VHCI列挙は未実装。

## HIDメモリー測定

実行方法（リポジトリで指定するRust環境を使用）:

```sh
python3 tools/measure-hid-memory.py
```

Rust 1.99.0 / LLVM 23.1.1、`x86_64-unknown-uefi`、release最適化、red zoneなし。
出力は `out/hid-memory/report.json` と `input.s` / `probe.s`。
測定専用probeはEFIへリンクしない。

| 対象 | byte |
|---|---:|
| Layout固定領域 | 10,136 |
| Decoder固定領域（Layoutを含む） | 10,392 |
| 両型のalignment | 8 |
| Layout::parse フレーム | 11,816 |
| Decoder::new フレーム | 312 |
| parse→new初期化probeのフレーム | 20,328 |
| update probeのフレーム | 232 |
| release_all probeのフレーム | 136 |

フレームは生成アセンブリのprologueの確保量と保存レジスター分。
初期化probe→parseだけで32,144byteとなる。戻り先アドレス、memcpy/memsetなどの
下位関数、実callback、割込み、最終リンクのLTOによる変化は含めない。
これは実行時の最大使用量や、全呼出し経路の上限を測ったものではない。
ツールは未知のprologueを検出したら停止し、推測の値を出さない。

配置方針:

- 既存runtime stackは128KiB。今の初期化を小さな割込みスタックで呼ばない。
  通常の初期化段階で一度だけ解析し、USB割込み・毎pollで再解析しない。
- 内蔵キーボード1台分のDecoderは、8byte以上に整列した16KiBの長期保持領域を
  起動時のメモリー設計に計上する。これは予算であり、今回は実領域を割り当てない。
  LayoutはDecoderに含まれるので、別のコピーを長期保持しない。
- 記述子の最大4096byteと入力レポートの最大513byteは別のCPU所有bufferとして
  計上する。受信DMA領域を解析器が長期借用する設計にしない。
- 初期化の一時領域は少なくとも64KiBの予算を設けるが、これを実証済み上限とは
  扱わない。組込み後に最終EFIの呼出し経路とガード付きスタックの使用量を再確認する。
- 将来再初期化や複数台対応が必要になった時点で、caller所有領域へ直接構築する
  APIを検討する。現段階では未接続の解析器を大きく書き換えない。

実機でのT2資源採取は引き続き[T2-D1](t2-d1-diagnostics.md)。

## 検証結果

- Rust workspace: 161件成功。新規10件は登録形式の固定byte比較、入力境界、
  状態遷移、期限・観測回数・時計異常、遅延/重複/別QID応答、各段階の失敗、
  ticket枯渇、登録から模擬転送・解除までの統合試験。
- BCE/InputのUEFI target check、通常版・T2-D1 release build、整形・差分検査成功。
- 保存済みFreeBSD参照ソース4ファイルのSHA-256一致。
- HID測定ツールは同じ環境で再実行し、上表の値を取得。CIにも測定と成果物保存を追加。
  CIの遠隔実行成功を主張する記録ではない。
- 今回は起動経路・入力処理を変更していないためQEMUは再実行していない。
  既存のQEMU証拠は[前回の検証](input-prehardware-validation.md)を参照。
- 物理USB更新、実BCE登録、内蔵キーボード動作確認は行っていない。
