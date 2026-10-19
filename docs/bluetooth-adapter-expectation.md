# Bluetooth live adapter verification

Every live Bluetooth initialization records `LIVE selected` with the controller BDF, root port, route, slot and USB VID:PID. After Read Local Version succeeds, `LIVE adapter` adds the HCI manufacturer. These records do not use discovery's duplicate suppression; slot numbers alone do not identify a device across discovery/live enumeration.

For a controlled MM-BTUD46 test, opt in through `SETTINGS/CONFIG.TXT`:

```text
log=devices
display=max
bt.expect=0a12:0001
```

`bt.expect` accepts exactly four hexadecimal digits on each side of `:`. It validates the adapter already selected by the existing first-available policy; it does not prefer, select or switch adapters. Omit the key for ordinary operation. Invalid values follow the existing atomic configuration fallback.

If the selected adapter differs, the viewer shows the expected and actual VID:PID and refuses new pairing requests. If no adapter is selected, it reports that the expectation is unmet. USB storage, keyboard and the file viewer continue. The validation state resets for a new USB session; another live controller without Bluetooth cannot overwrite an already selected adapter's result. This is a diagnostic acceptance check, not a replacement for stored reconnect policy.

With the expectation enabled, Devices logs include up to 32 live root-port snapshots and selection/skip records for the bounded enumeration queue. Connected/queued state is observational: it does not prove that a dongle was physically absent, nor that a queued transfer completed. Existing reset/descriptor failures still use the existing USB error path.

The viewer Bluetooth log uses the remaining rows below the file list, with one footer at the screen bottom. A bounded 64-line bootlog tail supplies rows without enlarging Viewer beyond its existing 16 KiB limit. Standalone viewer callers retain their existing small history. The tail sanitizes non-ASCII text, and its lock is released before framebuffer drawing. No keys or packet payloads are added to diagnostics.

## 日本語

LIVE 初期化ごとに BDF・ルートポート・経路・slot・USB VID:PID を記録し、Read Local Version 成功後に HCI manufacturer を追加します。探索時の重複抑制とは独立しています。

`SETTINGS/CONFIG.TXT` の `bt.expect=0a12:0001` は、従来の先着方式で選ばれたアダプターが試験対象かを検証する任意設定です。選択順や自動切替は変更しません。省略時は従来動作です。不一致・未選択時は明示して新規ペアリングを拒否しますが、USB・ビューアーは継続します。

Bluetooth ログはファイル一覧の下から画面下端のフッター直前まで表示します。64行の固定長ログ保持領域を Viewer 外に置き、従来の Viewer メモリー上限を維持しています。設定を含む物理 USB 更新は、このコード変更では行っていません。
