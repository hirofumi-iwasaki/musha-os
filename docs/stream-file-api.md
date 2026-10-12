# 共通の逐次ファイル読出しAPI

2026-10-11、工程4のホスト実装。`musha_api::stream::Files`はFAT32/exFAT共通の
読出し専用サービス。既存Contextのsnapshot APIとAPI_VERSION=4は変更しない。
新サービスのContextへの組込みとUSB実行は工程5で行う。

## 呼出し契約

ランタイムが検証済みの不変なルートファイル一覧を`mount(format, entries)`で貸し出す。
一覧は最大128件、名前は省略していないUTF-16最大255単位、サイズはu64。
名前は表示用で、実際の選択には`list_begin/list_next`が返す非公開IDを使う。
ID、cursor、handle、requestは作成元サービスだけで使用する。
媒体交換で一覧を差し替え、以前のIDとcursorを拒否する。
64MiBを超えるファイルも一覧に載るが、`open_id`はTooLarge。

`open_id`は同時1handle。`read(handle, out)`は最大4096byteの要求を予約してAgainを返す。
同じreadの再試行では要求を追加しない。Againとエラーでは位置とoutを保持する。
空bufferは位置を進めず0。空でないbufferで0ならEOF。
ランタイムは`take_request`で一度だけ要求を取り出し、index/offset/lengthで処理する。
完了後に`complete(request, result)`へランタイム所有bufferを渡す。サービスがコピーし、
次のアプリreadで初めてoutへコピーして位置を進める。アプリのpointerは保存しない。
完了結果より小さいbufferでの再試行はInvalidで結果を保留する。
要求長と異なる結果はCorrupt。Againは完了扱いにしない。終端エラー後の再試行は
同じ位置でも新しい要求番号を使い、古い完了通知を拒否する。

`close`は予約中・完了済み結果を破棄し、handleを失効させる。
実行中ならキャンセル印を付け、完了通知まで要求slotを隔離する。この間open/mountはAgain。
`disconnect`は即座に全アプリ操作をDisconnectedにするが、実行中slotは同様に隔離する。
ランタイムはDMAが停止した後にだけcompleteを通知する。成功結果でもキャンセル済みなら
コピーせず捨てる。サービス単独ではDMAを止めない。timeout時もこの契約が必要。
媒体・handle・要求番号は周回せず、u64の枯渇時はLimit。

## ファイルシステム側

`musha_fs::fat32::FileSystem::mount`は明示されたpartition領域を検証してルート短名一覧を作る。
既存の`read_root`とGPT/MBR選択は維持する。新mountへの領域選択の接続は工程5。
FAT32長名は扱わず、11byteの短名を保存する。OEM文字は表示側でescapeし、
省略名や変換後の文字列で検索し直さない。削除済み、volume label、subdirectoryは対象外。
最大128件・root走査1MiB・root chain128clusterを超えるとLimit。部分一覧は公開しない。
重複短名、空ファイルの不正cluster、予約上位bit、root循環を拒否する。

FAT32/exFATとも`entries`と`read_at(index, offset, out)`を提供する。
新FAT32 read_atも最大4096byte、64MiB上限、途中失敗時out不変。
毎回ファイルの全chainと宣言サイズに対する正確な終端を確認するため、
短いreadでも大きなファイルでは多くのsector確認が必要。1呼出し最大100万sector操作。
これらは同期ホストbackendであり、アプリstepから呼び出してはいけない。
工程5でブロック処理を分割し、入力・通信への実行機会を保証する。
媒体内容はmount中不変であることを前提とし、変更・切断時はmountを破棄する。

## 検証範囲

共通サービスでは両formatの一覧、分割、空、EOF、上限、buffer変更再試行、
エラー、重複通知、エラー後の旧通知、予約取消し、実行中取消し、切断、媒体交換、
世代枯渇を検証。さらにFAT32の断片化した既知payloadとexFAT生成fixtureの全5ファイルを
要求/完了経由で読んで照合した。FAT32は512/4096byte sector、領域offset、
複数chunk長、循環・短い/長いchain・I/O失敗のatomic性も検証する。

今回はホスト試験とUEFI向けコンパイルを対象とする。
USB上のオンデマンド読出し、応答時間、物理的な切断、実機動作の合格を意味しない。

最終検証: workspaceホスト203試験合格、musha-api/musha-fsのUEFI library check、
通常UEFI boot release build、formatと差分の検査に合格。
サービスはまだContextに組み込まず、既存UEFI実行経路を変更していない。
一覧のAPI用変換で名前を二重保持するとexFAT Workspaceとの合計が初期目標256KiBを
超え得るため、工程5ではメタデータの共有・配置とstack使用量を改めて測定する。


## 2026-10-12 接続試験の更新

ホストのFAT32/exFAT統合試験は`operation::Read`を介し、metadataとデータsectorを
同じ要求/完了経路で進める。QEMU専用`live-file-probe`では共通APIからFAT32の1ファイルを
実USB転送で読み、内容・EOF・待機中の入力/通信・Esc取消しを確認した。
[工程5の接続記録](bot-polling.md)に制約と再現手順を記載。
通常Contextへの組込み、再利用可能なruntime、exFAT USBは未完了。


2026-10-12追記: `planner::Source` により、起動時の同期readerを破棄した後も
API要求ごとにPlannerを作れるようにした。FAT32/exFATの共通API試験で利用し、
exFATは5ファイルの選択と分割読出しを検証。USB診断も7byte単位の複数要求へ移行した。
Sourceの寿命は元の媒体に限る。通常版への公開とUSBでの複数ファイル選択は残作業。
