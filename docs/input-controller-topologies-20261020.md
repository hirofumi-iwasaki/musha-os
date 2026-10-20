# r2 input controllers: local QEMU checkpoint (2026-10-20)

The live r2 USB session now polls keyboard and relative mouse input when they are on different xHCI controllers. Storage may share either input controller or use a third. Each controller keeps its own Host and DMA slice; nested polls feed the same guest input collector. The storage poll continues while input is live. A mouse-only session with storage on another controller also advances the viewer through an idle input tick. On exit, fault, or keyboard removal, each acquired controller must stop before arena reuse.

`tools/smoke-r2-io.py` creates disposable QEMU layouts with `--input-topology same|split|mouse-storage|keyboard-storage`. The normal cases injected keyboard and mouse events, checked the guest's key down/up, relative motion, buttons and wheel, then checked zero exit, confirmed input stop, viewer frame, and README/FAT32 live file access after return. Split-controller cases also require a distinct `XHCI_QUIESCED BDF=... DMA_DISABLED` record for every live controller between `APP_INPUT_LIVE` and `APP_ARENA_SCRUBBED VIEWER_RESTART`.

| Local case | Result | Live controller stops |
| --- | --- | ---: |
| Same controller, keyboard and mouse | Pass | 1 |
| Keyboard, mouse, storage on three controllers | Pass | 3 |
| Mouse with storage; keyboard separate | Pass | 2 |
| Keyboard with storage; mouse separate | Pass | 2 |
| Mouse only, with storage | Pass | 1 |
| Keyboard only, with storage | Pass | 1 |
| Three controllers, guest fault | Pass; recovered viewer and files | 3 |
| Three controllers, keyboard removed | Pass; disconnected event, recovered viewer and files | 3 |

Evidence is in `/private/tmp/musha-five-offline-20261020/r2-{same-escalated1,split-current1,mouse-storage-current1,keyboard-storage-current1,mouse-only-current1,keyboard-only-current2,split-fault-current1,split-removed-current3}/`, with each case's `report.json` and `debug.log`. The first seven cases used EFI SHA-256 `aa02789a42e3a501853b1d7970b1d57f210579a55280e5365f3dee37915bb316`; the keyboard-removal rerun includes the idle-tick fix and used `d83d9a6fcf3ed5931d821d17f139ddc11e6ef299224ac6f5e718e31fa88539e2`. These are focused QEMU results, not NUC5/Ryzen physical USB acceptance or a full release regression.

## 日本語

r2 の稼働中 USB セッションは、keyboard と相対 mouse が別の xHCI controller にあっても両方を poll します。storage はどちらかの controller と共有しても、三つ目の controller にあっても動作します。各 controller は専用 Host と DMA 領域を持ち、入力は同じ guest collector に渡します。mouse と storage が別 controller で keyboard がない場合も、空の入力 tick で viewer を進めます。終了、fault、keyboard 抜去時には、arena 再利用前に獲得済みの全 controller を停止します。

上表の八つの廃棄可能な QEMU case は合格しました。通常 case は key 押下・解放、mouse 移動・button・wheel、終了後の viewer frame、README/FAT32 の再読込を確認しました。controller 分離 case では `APP_INPUT_LIVE` から arena 消去までの区間で、controller ごとに異なる BDF の DMA 停止記録も確認します。これは実機 NUC5/Ryzen の USB 受入や全 release 回帰の代わりではありません。
