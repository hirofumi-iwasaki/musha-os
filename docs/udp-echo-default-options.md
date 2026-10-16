# UDP echo default options / UDP echo既定値の判断案

Status: owner decision, 2026-10-16 JST. Option1 adopted for v0.4.0; implementation/validation is tracked separately. Related: [security policy](security-policy.md), [I218 quickcheck](i218-hardware-quickcheck.md).

## English

### Adopted decision — 2026-10-16 JST

The owner selected **Option1**: v0.4.0 UDP echo is off by default; `/SETTINGS/CONFIG.TXT` on the FAT32 boot device opts in with `lan.echo=on`. DHCP continues without the echo listener. This setting does not enable a gated NIC driver or establish physical acceptance. See [startup configuration](startup-configuration.md), [release notes](release-notes-0.4.0.md) and the [I218 procedure](i218-hardware-quickcheck.md). Old published/development EFIs keep their original behavior; an older parser ignoring this unknown key is not a default-off implementation. No firmware/USB operation or communication test is performed by this document.

### Pre-decision code and options (preserved research)

Current code always allocates/binds a UDP PCB on port12345 in `net/port/shim.c:78–84` after session initialization, then starts DHCP; no CONFIG echo switch exists. `receive` echoes at most1472 bytes to the sender (`shim.c:33–38`), and stop removes the PCB (`shim.c:47–49`). Actual address availability requires a supplied DHCP lease, with no fixed-IP fallback. The native lwIP tests and QEMU network/cooperative/viewer-restart fixtures use that real service; physical quickchecks must not mistake terminal local echo for a reply. This does not establish physical I218 or Realtek acceptance.

**Option1 — v0.4.0 default off, explicit `lan.echo=on`.** Add a bounded config setting (unknown keys currently ignore), carry it through boot/restart to lwIP, and make PCB creation/callback/display conditional while retaining DHCP without a listening service. Quickchecks must explicitly enable echo and record CONFIG; default-off QEMU must prove DHCP plus no echo response/PCB, while opt-in tests retain strict packet, wrap, file/input concurrency and restart assertions. Replace unconditional `UDP echo ... ready` wording with factual disabled/enabled state. Release notes must call out the v0.4.0 default change, new opt-in syntax and unchanged old published EFI behavior. No localhost-only assumption or weakening of existing on-case tests.

**Option2 — v0.4.0 stays on, v0.5.0 defaults off.** Keep current v0.4.0 quickcheck/QEMU service and packet/wrap expectations unchanged, and document the port/payload/exposure clearly in v0.4.0 release notes. Plan the same explicit config/lifecycle and disabled-negative tests for v0.5.0; clarify migration and that future default-off does not describe present binaries. This avoids v0.4.0 behavioral changes but leaves the existing network input service enabled in that release. Neither option enables drivers currently gated or changes DHCP/stop-before-reuse guarantees. Owner selection is pending.

## 日本語

### 採用判断 — 2026-10-16 JST

所有者は**案1**を採用：v0.4.0のUDP echoは既定off、FAT32起動媒体の`/SETTINGS/CONFIG.TXT`へ`lan.echo=on`で明示有効にします。DHCPはecho listener無しで継続。閉じたNIC driverのgateを開かず、実機受入も証明しません。[起動設定](startup-configuration.md)・[公開ノート](release-notes-0.4.0.md)・[I218手順](i218-hardware-quickcheck.md)参照。旧公開/開発EFIの挙動は不変で、未知keyを無視する旧parserを既定off実装としません。この文書でfirmware/USB操作や通信試験は行いません。

### 判断前のコードと選択肢（調査原文保持）

現行は `net/port/shim.c:78–84` で session 初期化後に UDP PCB をport12345へ常時bindしDHCPを開始し、CONFIGのecho switchは無い。`shim.c:33–38` は1472 byte以下を送信元へ返し、stopでPCBを削除（47–49行）。実addressは実DHCP leaseのみで固定IPfallback無し。native lwIPとQEMU network／cooperative／viewer復帰はこのserviceを使う。実機quickcheckでは端末local echoを返信と誤認しない。I218／Realtek実機受入を証明する記述ではない。

**案1 — 0.4.0から既定off、`lan.echo=on`で明示有効。** bounded設定を追加し、boot／再起動からlwIPへ渡してPCB生成・callback・表示を条件化する。DHCPはlisten無しで維持。quickcheckはCONFIG有効化／記録が必須になり、QEMUではoff時DHCP成立とecho応答／PCB無しを追加する。on時の厳密packet・wrap・file／入力並行・再起動条件は維持。無条件のUDP ready表示を実状態へ変更する。リリースノートで0.4既定変更・opt-in書式・旧公開EFIは不変と説明する。localhost前提や既存on試験緩和はしない。

**案2 — 0.4.0はon維持、0.5.0からoff。** 現0.4quickcheck／QEMUのserviceとpacket／wrap期待を維持し、0.4ノートにはport／payload／公開範囲を説明する。同じ設定／寿命接続とoff負例は0.5へ計画し、移行と将来offを現binaryの挙動と混同しないことを明記する。0.4変更を避ける一方、その版には既存network入力serviceが残る。両案とも閉じているdriver gateを開かず、DHCP／停止前再利用禁止を変えない。選択は所有者判断待ちで、本書は決定・実装していない。
