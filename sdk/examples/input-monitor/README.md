# Live input monitor

## English

This r2 C example observes real keyboard, relative mouse and gamepad records without assuming a device, event count or fixture sequence. It prints kind, device, code, flags, three values, button mask, modifiers and boot milliseconds. Numbers are hexadecimal; negative signed values use 32-bit two's complement. Overflow is displayed as kind5; reset assumptions about held state after it. Q or Esc exits; without input the program ends after five minutes. Each iteration drains at most64 records and waits at most100ms, with a fixed240-byte line buffer. Log/permission errors fail explicitly. This is an observer, not a Bluetooth pairing or hardware-support claim.

From the repository: `python3 tools/check-sdk.py --c-only --abi-revision 2 --source sdk/examples/input-monitor/main.c --output out/input-monitor`. The result is `out/input-monitor/hello-c.elf` (the existing checker output name). An extracted C SDK uses its own `tools/check-sdk.py` with `--sdk` pointing to that SDK. Place the validated ELF as the user-owned `AUTORUN.ELF` or select it from the viewer using the existing manual USB procedure.

## 日本語

r2 Cの観測用exampleです。特定device・件数・fixture順序を要求せず、実keyboard・相対mouse・gamepad recordのkind/device/code/flags/3値/button mask/modifier/boot msを表示します。数値は16進、負のsigned値は32bit二の補数です。overflowはkind5として表示し、発生後は保持状態の前提をリセットします。Q/Escで終了、入力無しでも5分で終了。1反復は最大64 record、待機は最大100ms、行bufferは固定240byteです。log/permission errorは明示的に失敗します。観測用であり、Bluetooth pairingや実機対応成功の主張ではありません。

repoから上記commandでbuildし、既存checkerの出力名 `out/input-monitor/hello-c.elf` を使います。展開済みC SDKでは自身の `tools/check-sdk.py` と当該SDKを指す `--sdk` を使用。既存の手動USB手順で検証済みELFを利用者所有 `AUTORUN.ELF` として配置するかviewerから選択します。
