# BOT転送の段階実行（工程5の前半）

2026-10-12。`musha_xhci::storage::transaction::Transaction`で、読出し専用BOTを
CBW送信→必要ならDATA受信→CSW受信に分割した。各pollは要求1件の提示または
待機状態の確認だけを行い、ループ・MMIO・DMAアクセスは持たない。
Ready/Sense/Capacity/READ(10)を扱い、ゼロのtag、不正sector size、
順序違反、短い転送、異なるCSW tag、residue、phase errorを拒否する。
CSWのcommand failedは正常なBOT完了(false)として返し、既存のSense手順を維持する。

起動側の`Bot::begin/poll`へ接続した。beginは所有DMA領域を準備し、pollは
最大1転送の投入または最大1eventの確認を行う。完了までring cursorを進めず、
次の転送の投入は次回pollへ戻す。CBW/data/CSW領域は別々で、アプリbufferは渡さない。
時間制限はコマンド全体2秒、観測100万回。時計の逆行・overflowも終端エラー。
故障注入のReadは20msで期限切れにする。以前の「各転送最大1秒」から変更した。

失敗したTransactionは再投入を許さず、起動側もpendingを維持する。
外側のcontroller停止・PCI bus master停止を経由し、DMA領域を再利用しない。
cancelだけではハードウェアを止めない。停止完了前のbuffer再利用は禁止。

## 今回の到達点と残作業

実際のUSB転送は新状態機械を使うが、起動時の互換`command`はbegin/pollを
繰り返し呼び、同期的に探索を完了させる。ストレージslotも探索後に解放する既存仕様。
従って今回だけでストレージ待機中の入力・通信への実行機会を保証したとは扱わない。
共通stream APIからのオンデマンドUSB読出し、exFATのUSB実行もまだ未接続。

次の実装順:

1. ファイルシステムのsector要求・解析を中断/再開できる形にし、同期parserをapp stepへ入れない。
2. ストレージslotとDMA所有権をlive loopまで保持し、共通APIの要求をBOT pollへ渡す。
3. 同一controller上のkeyboard/storage eventを振り分け、入力・通信を各周回で進める。
4. 実USB読出しと入力・通信が重なるQEMU試験、切断、キャンセル、timeout、停止後の旧完了拒否を追加する。

この順番で工程5を完了させる。媒体metadataの二重保持・stack配置も接続時に検査する。

## 検証

- ホスト全206試験合格。新状態機械は全コマンドの段階遷移、待機中の再投入禁止、
  CSW異常、期限、時計停止・逆行、cancelを試験。
- QEMU通常GPT/FAT32イメージ: USB読出し、FAT32ファイル、アプリsnapshot読出し成功。
- QEMU traffic: キー入力、ARP/ICMP、257件UDP応答、snapshot再読出し、ring wrap、DMA停止に合格。
  これは既存機能の回帰試験で、ストレージ待機との同時実行試験ではない。
- QEMU storage-timeout: ReadのDATA doorbellを抑止しTRANSFER TIMEOUTを検出。
  検証スクリプトがXHCI_QUIESCED DMA_DISABLEDと後続アプリ完了を確認。
  このcaseはスクリプトの仕様でout/esp由来の仮想FAT媒体を使用する。
- QEMU 4096byte sector fixture: 容量・先頭/末尾sectorの内容hashと転送成功を確認。
- 最後に通常UEFI版を再ビルドし、out/espも通常版へ復帰。
- GPT正常試験像SHA-256: `4940d6fdd020c72493bf262f82d676cc1702bceaa262fc80db53152e0ee83bdd`。

物理USBの更新、MacBookでの実機検証は行っていない。

## 2026-10-12 後半の途中経過: データ読出しの再開境界

FAT32/exFATに`plan_read(index, offset, requested)`を追加した。既存のchain・bitmap・
サイズの検証を行い、最大4096byteの範囲について、データsectorとコピー範囲だけを返す。
最大9sector、追加heapなし。exFATのValidDataLength以降はsectorを要求せずゼロで埋める。
既存`read_at`も同じplanを同期実行するため、配置計算を二重実装しない。

`read_plan::Executor`はplanを受け取り、最大1sectorずつ要求する。
要求中の再取得はNone、完了は要求tokenとsectorサイズを照合。結果はCPU所有4096byteへ
蓄積し、全部成功してからfinishで呼出し側へコピーする。途中のエラーやpendingでは
呼出し側bufferを変更しない。cancel後も実行中slotはtransportの停止/完了通知まで保持し、
新しいplanを開始しない。要求番号は周回させず、旧通知を拒否する。
Executorは8KiB未満というホスト検査を追加。これはUEFI全体stackの保証ではない。

FAT32の断片化fixtureとexFATの全5ファイルを
共通API→plan→sector要求/完了→共通APIの順でホスト検証した。
非整列4096byte読出しが9sectorにまたがる場合も検査。

**残作業:** plan_readのmetadata検証・探索はまだ同期I/O。
従ってアプリstepへ直接接続しない。metadataの再開可能化、保持したUSB slotへの接続、
同一/別controllerのkeyboardとstorageのevent配送、同時動作・切断のQEMU検証が必要。
今回のホスト検証を工程5全体の完了、または実USBでの非同期ファイル読出し成功とはしない。

今回の最終検証: ホスト210試験合格、musha-fsのUEFI library checkと通常UEFI release buildに合格。新経路のQEMU同時I/O試験は未実施。
