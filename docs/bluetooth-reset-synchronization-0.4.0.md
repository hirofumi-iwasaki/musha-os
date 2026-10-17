# Bluetooth initial Reset synchronization — English

Status: implemented locally; physical retry pending. IMG_7691 showed a Reset CC, a Rand command, a second Reset CC and then Rand timeout. The age/origin of either CC is unknown. A residual FIFO is a hypothesis, not an established controller fault.

Normal `configure` calls prepare only when `!host.discovery`; the separate probe feature is independent. Prepare sets endpoint contexts and SET_CONFIGURATION, then runs warm-up continuously. New pre-Reset drain has a 100 ms target/200 ms cap; no received events allows 50 ms quiet exit. Pending interrupt-IN is retained across that exit, so Reset's EP0 transfer uses its own buffer while late event completion keeps exact TRB/slot/DCI identity. No timeout cancels or reclaims DMA. The existing correlated Disable Slot / retired allocation boundary remains the only local cleanup path; inability to establish safety stops the shared session.

Reset `0c03` completion is followed by Read Local Version `1001` (nine return bytes including status, status zero) before Rand. A successful late Reset CC during a parameter-free Version/Rand command permits one retransmission; the original command deadline is restored and also bounds retransmission EP0 waiting. Retry is reset only for a new command. Command status success keeps waiting for CC. Other valid CC does not change pending state or credits. Global event count is 64 including drained events. Unknown envelope-valid events are classified/logged/discarded; malformed known events, zero credits, Hardware Error and flooding fail safely. No response bytes, keys, random values or report payloads are logged.

The 2 s operation plus at most 1 s isolation software budget includes the drain and Version barrier; it has not been enlarged. Slow devices can be disabled as optional Bluetooth under that existing contract. Pure production decision/clock/retry functions and actual mailbox ownership fixtures verify the boundaries; QEMU has no Bluetooth controller and cannot establish physical FIFO flushing, successful firmware behavior, pairing or HOGP compatibility. Existing HOGP diagnostics remain included, without relaxed discovery acceptance. USB delivery must use `SETTINGS/CONFIG.TXT`, never root CONFIG.

# 日本語: Bluetooth初回Reset同期

状態: ローカル実装・実機再試験待ち。IMG_7691ではReset CC→Rand命令→再度Reset CC→Rand timeoutを観測しました。各CCの由来/古さは未確定です。残留FIFOは仮説でありcontroller不具合の確定ではありません。

通常configureのprepareは`!host.discovery`でのみ実行され、別probe featureとは独立です。endpoint設定・SET_CONFIGURATION直後にwarm-upを連続実行します。Reset前drainは目標100ms/最大200ms、無イベントなら50ms quietで終了。quiet終了でもINのpending loanを保持し、EP0独立bufferでResetを送る間も遅着eventのTRB/slot/DCI相関を維持します。timeoutでDMAを解除/再利用しません。Disable Slot相関確認とallocation退役を維持し、安全を確認できなければ共有sessionを停止します。

Reset CCの後、Read Local Version `1001`（status含む9バイト・status0）を確認してからRandへ進みます。引数なしVersion/Rand待ちに妥当な成功Reset CCが来た場合、待機命令を1回だけ再送。元command期限を再送EP0にも適用して復元し、新しい命令の時だけ再送枠をリセットします。CS成功ではCC待ちを継続、他の妥当CCで状態/creditを進めません。drainを含め64events上限です。外枠が正しい未知イベントは分類ログ後に破棄し、既知不正形式・credit0・Hardware Error・floodは停止します。応答本文・鍵・乱数・report payloadは出しません。

drainとVersion追加も既存の操作2秒＋隔離最大1秒に含み、延長していません。遅い機器は従来契約どおり任意BT機能を無効化します。本番共通decision/clock/retryと実mailbox所有のhost fixtureで境界を確認し、QEMUにはBT機器がないため実FIFO・firmware・pairing/HOGP対応は証明しません。HOGP診断も包含し受理条件は緩めていません。USB設定先はSETTINGS/CONFIG.TXTです。
