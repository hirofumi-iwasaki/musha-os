# BOT転送と工程5の接続記録

最新の到達点は末尾の「同一controllerでのlive転送」。以下の各節は段階ごとの記録。

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


## 2026-10-12 続き: ファイル配置metadataの再開

FAT32/exFATの`begin_plan`はI/Oなしで`planner::Planner`を作成する。
`poll`は配置検証を1段階だけ進め、Yield、Pending、Read、Completeを返す。
ReadはsectorのLBA・長さと識別tokenを持つ。FAT chainとexFAT allocation bitmapの
検査は1sectorずつ要求し、待機中は重複要求しない。断片化したbitmap自身のchainも
再開可能で、循環・短いchain・未割当bit・不正sector長・I/O失敗を拒否する。
全配置の検証が完了して初めて最大4096byteのPlanを取り出せる。
既存の同期`plan_read`と`read_at`もこの状態機械を使い、検証・配置計算を共通化した。

cancelは未完了tokenをtransport完了/停止の通知まで保持し、届いたデータを破棄する。
`is_quiescent`はその通知を受けたことだけを示し、DMAを停止する機能ではない。
tokenは元のPlannerインスタンス内だけで照合する。runtime側は媒体世代と要求の所有者を
確認し、破棄したPlannerの通知を新しいPlannerへ渡してはならない。
最大100万sector要求、Planner本体2KiB未満をホストで検査した。
処理全体の時間制限はlive runtimeへの接続時に設ける必要がある。

FAT32/exFATの共通API統合試験を、metadata要求/完了→Plan→データ要求/完了の経路へ変更。
追加試験では保留を繰り返した場合、旧token・二重通知、各metadata待機点でのcancel、
bitmapの断片化と循環、要求上限、部分的な結果を公開しないことを確認した。
ホスト214試験、UEFI向けlibrary check、通常UEFI release buildに合格した。

**現在の残作業:** mount/discoveryとroot catalog構築は同期のまま。
起動時の探索とlive処理の境界を確定し、storage slot/DMAの保持、共通API接続、
同一/別controllerの入力event配送を実装する。その後に実際のUSB待機と入力・通信が
重なるQEMU試験を行う。今回の新経路はホスト検証のみで、工程5全体は未完了。
物理USBの更新は行っていない。


## 2026-10-12 同一controllerでのlive転送

入力セッションを実行するcontrollerでは、最初に検出したstorageのslot、bulk ring、
CBW/DATA/CSWのDMA領域を入力loopの終了まで保持する。探索専用controllerと追加storageは
従来通り探索後に停止する。現在の通常版には共通ファイルAPIからの要求は未接続で、
保持したstorageは待機する。キーボードなしでは保持slotを解放して既存fallbackへ進む。

保持したDeviceはbegin_read/poll_readを提供し、pollは最大1転送投入または1event確認。
CPU出力bufferへのコピーはCSW検証後だけ。Hostはkeyboard/bulkそれぞれ1件の完了を
保存でき、command・EP0・keyboard・bulkの各pollerが相手の正常完了を配送する。
TRBアドレス、slot、endpoint、completion code、残byte数を照合し、重複・未知の通知は
エラーにする。bulkの同時二重投入と異なる所有者からのpollも拒否する。

通常終了ではDisable Slot完了後にpendingと保存通知を消し、親hubを最後に解放する。
途中エラーは外側のcontroller haltとPCI bus master停止へ進み、DMA領域を再利用しない。
この段階ではstorage障害時に同じcontrollerの入力も停止する。ネットワークと既存snapshot
処理はfallbackで継続できる。storageだけを切り離して入力を維持する復旧は未実装。

### QEMU専用の同時転送試験

`live-storage-probe`は入力loop内でLBA 0を反復READ(10)し、起動時のhashと照合する。
DATA TRBの投入後、doorbellを750ms遅らせ、USB待機中の入力・通信を観測できるようにした。
`live-storage-timeout`は探索後のこのDATA doorbellだけを抑止し、コマンド全体2秒の期限を
検証する。どちらもqemu-debugを含む専用機能で、通常版では周期読出しを行わない。
これはsector transportの試験で、共通ファイルAPIやexFATのlive読出し成功とは区別する。

再現手順（toolchainとQEMU firmwareの設定後、出力像は毎回新しい名前を使用）:

```sh
cargo build --locked --release --target x86_64-unknown-uefi -p musha-boot --features live-storage-probe
python3 tools/make-usb-image.py out/live.img --efi target/x86_64-unknown-uefi/release/musha-boot.efi
python3 tools/smoke-cooperative.py --firmware-dir "$MUSHA_FIRMWARE_DIR" --usb-image out/live.img --case traffic --live-storage
```

`--hub-depth 2`でkeyboardを2段hub経由にする。`--live-fault disconnect`でDATA待機中の
storageをQMPから取り外す。timeoutは`--features live-storage-timeout`で別像を生成し、
`--live-fault timeout`を付ける。release-candidate workflowにも同じ4caseを追加した。

今回の検証:

- ホスト215試験合格。完了通知の宛先・成功状態・重複拒否を追加検証。
- QEMU実sector読出し: 内容hash一致、待機中のaキー、ARP/ICMP/257件UDP、snapshot再検査、
  次のDATA待機中のEsc、slot解放、controller DMA停止を確認。
- 最初のUDP応答をホスト側で受信した時点で、読出し完了/失敗がまだ発生していないことも検査。
- 2段hub経由の入力でも合格。EP0状態確認とbulk/keyboardの完了配送を回帰。
- DATA待機中のstorage切断とタイムアウトで、結果を公開せずDMA停止。
  入力ソースを解除し、ネットワーク・snapshot処理の継続と最終終了を確認。
- 周期読出し無効のqemu-debug版でも通常trafficに合格。キーボードなし・2 controllerのfallbackも合格。
- 通常UEFI release buildに合格し、out/espの起動ファイルも通常版へ復帰。CIへの追加はローカル変更であり、リモートCIの実行結果ではない。
- 最終成功像SHA-256: `99aab18295e0e9faecef5ee0b39d78884810cb7a12cdc87a1505fbbe1079e285`。
- 最終timeout像SHA-256: `e10eca1d31d0c4cea007b92579a71173df287041efbf685d754a38b5b544667f`。

### 次の接続範囲

共通stream API→metadata Planner→データExecutor→保持したDeviceの接続、
mount/catalogの配置とメモリ上限、アプリからのキャンセル伝播が残る。
MacBookのC0 keyboard/C2 storageのような別controllerは現在も逐次探索なので、
両Hostを同時保持する実行構造が必要。キーボードなしのlive読出しも接続対象。
工程5全体は未完了であり、物理USB更新・実機合格・exFAT USB合格は今回に含まない。
