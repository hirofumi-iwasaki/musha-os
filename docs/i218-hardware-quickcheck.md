# NUC5 I218 LAN quickcheck / NUC5 I218 LAN実機1ページ手順

Status: current (procedure only, 2026-10-15). Physical networking remains unverified.

## English

1. Connect NUC5 Ethernet and the Mac to the same IPv4 LAN with a working DHCP server. Record switch/router, cable/ports, Mac interface/IP, NUC model/firmware and link LEDs. Connect boot USB and external keyboard directly; photograph topology. A direct cable without DHCP does not satisfy this test.
2. Identify the approved development EFI by commit/hash and **`i218-network`** feature. Build-only command: `cargo build --locked --release --target x86_64-unknown-uefi -p musha-boot --features i218-network`. Output is `target/x86_64-unknown-uefi/release/musha-boot.efi` unless target-dir overrides it. `tools/build-usb.py` has no arbitrary-features option. Packaging/copying to physical USB is a separate owner operation, not performed here. `i218-phy-probe`/`i218-power-probe` alone do not enable LAN; the normal build does not establish physical I218 support.
3. Use `log=devices`, cold boot ≥10 s and record AC state. Capture actual Ethernet PCI ID/revision/BDF (8086:15A3 is a candidate), MAC/PHY/reset/link diagnostics, supplied DHCP IPv4 and ready/error pages. There is no static-IP fallback; initial/no-address timeout is 30 s. A link LED or detected PCI function alone is not communication.
4. On macOS Terminal run `nc -u -w 3 <NUC_DHCP_IP> 12345`, replacing the placeholder with the observed lease. Type a unique short ASCII token and Return. Record **a second returned copy** of the same token (the first visible line may be terminal local echo). Ctrl-C exits if needed. No `-l` listener: Musha is the UDP server. Source confirms `net/port/shim.c` binds UDP12345 and echoes the same payload to the sender; accepted payload limit1472 bytes. Capture Mac output plus OS UDP count before/after. Repeat a new token after opening/returning from a viewer ELF; record keyboard/file progress.
5. Stop normally and capture failure/stop pages if present. Record exact failing stage, timeout and whether viewer remains usable; do not promote QEMU82574 results to I218 evidence. No router reconfiguration or physical media writing is performed by this procedure preparation.

| Record | Value |
|---|---|
| Date/operator; EFI commit/features/hash; CONFIG/hash | |
| Topology, link LEDs; actual PCI ID/revision/BDF, PHY/MAC | |
| Cold interval/AC; DHCP address/time or error | |
| Mac IP/interface; sent/returned tokens; UDP counts before/after | |
| Viewer/file/input and shutdown result; photos/log paths; PASS/FAIL/BLOCKED | |

See [implementation limits](i218-network-0.4.0-progress.md) and [LAN plan](lan-plan-0.4.0.md).

## 日本語

1. NUC5有線LANとMacをDHCP serverのある同一IPv4 LANへ接続。switch/router、cable/port、Mac interface/IP、NUC型番/firmware、link LEDを記録。起動USB/外付けkeyboardを直結し接続写真を保存。DHCP無し直結は本試験を満たしません。
2. 承認対象の開発EFIをcommit/hashと**`i218-network`** featureで識別。buildのみのcommandは`cargo build --locked --release --target x86_64-unknown-uefi -p musha-boot --features i218-network`。target-dir指定がなければ出力は`target/x86_64-unknown-uefi/release/musha-boot.efi`。`tools/build-usb.py`に任意feature引数はありません。物理USBへのpackage/copyは別途所有者操作で、本作業では実行しません。PHY/power probeだけではLANを有効にせず、通常buildも実I218対応の証拠ではありません。
3. `log=devices`でoff ≥10秒の冷間起動、AC状態も記録。実Ethernet PCI ID/revision/BDF（8086:15A3は候補）、MAC/PHY/reset/link、DHCP IPv4、ready/errorを撮影。static fallbackは無く初回address待ちは30秒。LED/PCI検出だけでは通信成功ではありません。
4. Mac Terminalで`nc -u -w 3 <NUC_DHCP_IP> 12345`（実leaseへ置換）。短い固有ASCII tokenを入力しReturn。同じtokenの**2度目の返り**を確認（最初の表示は端末local echoの場合あり）。必要ならCtrl-Cで終了。`-l`は使いません。Musha側がUDP serverです。`net/port/shim.c`はUDP12345にbindし、受信payloadを送信元へそのまま返します（上限1472 byte）。Mac出力とOS UDP counterの前後を撮影。viewer ELFを開いて戻った後も別tokenで反復し、入力/file進行を記録。
5. 通常停止し失敗/停止ページも保存。止まった段階・timeout・viewer操作可否を明記。QEMU82574成功をI218証拠にしません。本手順準備ではrouter変更・実媒体書込みを行いません。

| 記録欄 | 値 |
|---|---|
| 日時/実施者、EFI commit/features/hash、CONFIG/hash | |
| 接続/LED、実PCI ID/revision/BDF、PHY/MAC | |
| 冷間秒数/AC、DHCP address/時間又はerror | |
| Mac IP/interface、送信/返信token、UDP counter前後 | |
| viewer/file/input/停止、写真/log path、PASS/FAIL/BLOCKED | |

[実装制限](i218-network-0.4.0-progress.md)・[LAN計画](lan-plan-0.4.0.md)参照。
