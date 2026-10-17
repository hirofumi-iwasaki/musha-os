# Pairing entropy and DRBG / ペアリング乱数とDRBG

Status: current implementation, 2026-10-17. Physical NUC5 validation pending.

## English

The owner decision in [security policy, item 9](security-policy.md) replaces the firmware-only 4096-byte pool. Bluetooth SC keys, nonces and random passkey sampling use one BSP-owned HMAC-SHA-256 DRBG. There is no clock/timer/fixed-seed or weak PRNG fallback. Healthy firmware RNG or RDSEED is mandatory; RDRAND/HCI alone cannot instantiate it. This trusts source properties, not a measurement or certification of entropy quantity.

Before GOP selection/final memory map/ExitBootServices, `EFI_RNG_PROTOCOL.GetInfo` is read into a maximum 32-GUID buffer. All returned GUIDs are tried, then NULL/default, RAW and explicit SP800-90 HASH256/HMAC256/CTR256. A larger list is reported and not silently truncated; explicit requests still run. Each attempt requests 64 bytes. Successful samples must pass the same health checks; the first healthy firmware algorithm supplies the firmware input. Other algorithms are still attempted and their statuses recorded, not counted as independent entropy sources. Protocol pointers never survive EBS. Logs include GUID names/first eight GUID bytes, statuses, counts and health decisions, never random bytes.

CPUID leaf 7 EBX bit18 gates RDSEED; carry **set** means success. Eight 64-bit words are collected, at most 64 instruction attempts per word. CPUID leaf1 ECX bit30 gates RDRAND; its per-word limit is ten. Unsupported features never execute their instructions. Failed carry values are ignored. Every source rejects zero/FF eight-byte blocks or adjacent equal eight-byte blocks. CPU retry exhaustion/health failure is visible. The `strong` count is firmware plus RDSEED (maximum two), not the number of successful firmware algorithms. RDRAND is auxiliary only.

After USB HCI endpoint preparation, a standard Reset must complete with status0 and positive credit before four LE Rand (`0x2018`) reads collect 32 bytes. A rejected command makes this auxiliary source unavailable; health rejection also leaves a healthy existing DRBG intact. Timeout, malformed framing or zero credit instead uses the existing controller halt/BME-off boundary because transport state is ambiguous. Thus four successful samples are required for contribution; failure may stop collection earlier. The later authorized scan retains its existing Reset, so these are separate initialisation phases. Every new live HCI session may mix fresh auxiliary data without resetting the DRBG request counter. HCI-only material never enables pairing.

The original HMAC_DRBG follows [NIST SP800-90A Rev1, 10.1.2](https://nvlpubs.nist.gov/nistpubs/SpecialPublications/NIST.SP.800-90Ar1.pdf), using the existing SHA-256 implementation and zeroize dependency; no new dependency is added. Source labels and lengths separate inputs. A successful Generate increments its request counter once, including multiple SHA blocks. The local request cap is 8192 bytes (below NIST's maximum), and reseed interval is 2^48 requests. Auxiliary HCI update does not reset this counter or count as a fresh strong reseed. Runtime does not reacquire firmware after EBS. HMAC K/V, sample storage and scalar/key state are erased; complete elimination of compiler/register/SHA temporary copies is not guaranteed. NIST CAVP known-answer agreement is a test, not NIST validation or an independent security audit.

Early source metadata is buffered in 80 fixed 192-byte rows and published immediately after the mandatory `Musha-OS 0.4.0 starting`/initial settings/display records. Overflow is explicit. Source/strong/DRBG decisions enter the same log/ring; secret bytes, K/V, keys, nonces and passkeys are never logged. Viewer P reports missing healthy firmware/RDSEED, DRBG initialization rejection or a DRBG request error distinctly. The eight-attempt pairing limit remains. Rejected P does not consume random bytes.

Local evidence: `out/entropy-integration/verification.json`, with DRBG vector details in `out/entropy-drbg/core-results.json`. QEMU source tests are emulated capability/provider evidence, not physical NUC5 proof. The previous `secure entropy unavailable` video predates this implementation. Repeat the normal-EFI P/Xbox test only after explicitly approved deployment; record actual source decisions before claiming pairing/HOGP/input success.

## 日本語

[security policy第9項](security-policy.md)の所有者決定により、firmware限定4096byte poolを置換します。SC鍵・nonce・乱数passkey samplingはBSP所有のHMAC-SHA-256 DRBGを使います。時計・timer・固定seed・弱いPRNGへのfallbackはありません。健全なfirmware RNGまたはRDSEEDを必須とし、RDRAND/HCIだけでは初期化しません。源の性質を信頼する方針であり、entropy量の測定・認証ではありません。

GOP選択/最終memory map/EBSより前に、EFI RNGのGetInfoを最大32GUIDの固定bufferで読みます。返された全GUIDに加えNULL/default・RAW・SP800-90 HASH/HMAC/CTR256を各64byteで試します。上限超過は明示し、黙って一覧を切り詰めません。明示algorithmは引き続き試します。最初の健全なfirmware候補をfirmware入力に使い、他algorithmもstatusを記録しますが独立した強源として数えません。pointerはEBS後へ残しません。GUID識別・status・件数・health判定だけを記録し、乱数bytesは記録しません。

RDSEEDはCPUID leaf7 EBX bit18で有効化し、carry **1**だけを成功とします。64bitを8語、各語最大64試行。RDRANDはleaf1 ECX bit30、各語最大10試行です。未対応命令は実行せず、carry失敗値は捨てます。各源で8byte単位のzero/FF・隣接同一値を拒否します。retry上限やhealth失敗を表示します。strong数はfirmwareとRDSEEDの最大2で、firmware algorithmの成功数ではありません。RDRANDは補助です。

USB HCI endpoint準備後、標準Resetのstatus0と正creditを確認してからLE Randを4回読み、32byteを集めます。command拒否やhealth不合格は補助源なしとし、既存の健全なDRBGを壊しません。timeout・形式不正・credit0はtransportが曖昧なので従来のhalt/BME無効境界で停止します。寄与には4成功応答が必要で、失敗なら途中で採取を打ち切る場合があります。後続の明示scanにも既存Resetがあり、初期化段階は別です。live HCIの再作成時も補助mixは要求counterをリセットせず、HCIだけでpairingを有効化しません。

HMAC_DRBGは上記NIST仕様に従う独自のoriginal実装で、既存SHA-256とzeroizeを使用し、新依存はありません。源名と長さで入力を分離し、成功Generateごとにcounterを1増やします。ローカル要求上限8192byte、間隔2^48要求です。HCI補助mixはcounterや強源reseed回数を戻しません。EBS後にfirmwareへ再取得しません。K/V・source・scalar/key stateを消去しますが、全compiler/register/SHA一時値の消去保証ではありません。CAVP既知解一致は試験でありNIST認証や独立監査ではありません。

起動前metadataは80行×192byteの固定bufferへ保持し、必須starting/settings/display行の後に公開します。上限は明示し、同じlog/ringに源・strong・DRBG判定を残します。乱数・K/V・鍵・nonce・passkeyは表示しません。viewer Pの拒否は強源不足・DRBG初期化拒否・要求errorを区別します。1boot8試行は維持し、拒否されたP自体は乱数を消費しません。

証跡は上記JSONです。QEMU能力/providerの観測をNUC5実機対応としません。前回の乱数拒否動画は旧版です。承認済み配備後に通常EFIのP/Xboxを再試験し、実源判定とpairing/HOGP/入力を分けて記録します。
