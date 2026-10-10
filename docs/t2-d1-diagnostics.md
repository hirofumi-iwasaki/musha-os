# T2-D1: read-only MacBook Pro evidence

## Purpose / 目的

v0.2.0内蔵キーボード対応の最初の診断ビルド。T2ドライバーはまだ開始しない。
既存の外付けキーボードとUSBストレージ処理に、独立した読取専用診断を追加する。
[調査・実装方針](macbook-t2-keyboard-0.2.0-research.md)の手順1の情報採取に相当する。
BAR範囲の使用許可判定とprotocol/queueモデルは後続段階。

- `t2-diagnostics` featureを指定した場合だけ起動経路に入る。通常ビルドはH19のまま。
- SMBIOS 2/3 entryの署名・checksum・長さ、UEFI memory mapの読取可能RAM範囲を確認する。
  Type 1のproduct nameだけをコピーし、serial・UUIDは表示も保存もしない。
- segment 0のPCIを最大64件、BCE `106b:1801`を最大2件保存する。超過件数を明示する。
  非ゼロsegment、hotplug、新たに出現したBCEの再探索は対象外。
- PCIのID/class、bridgeのbus/memory window、BCEのBAR全6 DWORD、Command/Status、
  PMCSR、MSI/MSI-X controlを表示する。capability listの循環・不正pointer・重複・
  範囲外を検出する。BARの上位DWORDを別のBARとして扱わない。
- BCEのconfigurationをEARLY / PRE EBS / POST EBSの3時点で保存する。
  最終memory mapからExitBootServicesまでにfirmware呼出しや割当てを挟まない。
  その間の新規操作はCF8/CFCによるconfiguration読取だけ。
- T2のPCI configurationへの書込み、BAR sizing、MMIOアクセス、DMA割当て／開始、
  firmware commandは行わない。CF8への書込みはconfiguration読取アドレス選択のみ。
  既存xHCI/NICの動作・所有権管理は従来どおり。
- BARの物理アドレスが表示されてもアクセス許可ではない。サイズ、予約領域との重複、
  上流bridgeとの対応は次段階で検証する。`UNVALIDATED`を明示する。

## Build / ビルド

```sh
python3 tools/build-usb.py --t2-diagnostics --output-dir out/t2-d1-usb
```

`build-manifest.json`のfeaturesは`["t2-diagnostics"]`、diagnostic_revisionは`T2-D1`。
QEMU専用debug/fault featureは実機ファイルへ含めない。
通常ビルドが必要なら`--t2-diagnostics`を省略する。

既存FAT32 USBへのコピー用は`musha-os-fat32-files.zip`。
USB更新時には既存EFIを別の場所へ退避し、ZIPの内容をUSBルートに配置する。
既定64MiBの`musha-os.img`はQEMU用で、32GBの専用USBへraw書込みしない。
詳しくは[USBビルド手順](test-usb-build.md)。このビルド処理は物理USBを書き換えない。

## Hardware capture / 実機で取得するもの

1. MacBook Pro 2018へ専用USBとLenovoキーボードを接続して起動する。
2. 従来どおりA、Shift+A、Escを確認する。USB READ / FAT32 READ / APP FILE READ OKも確認する。
3. 終了後の自動ページ送り（8秒ごと）で、見出しが`T2-D1`であることを確認する。
4. `T2 MODEL`、`T2 SMBIOS`、`T2 PCI`、`T2 BRIDGE`、`T2 BCE COUNT`、
   `T2 EARLY / PRE EBS / POST EBS`、BAR・capability、`T2_DIAGNOSTICS_COMPLETE`
   を含むページを撮影する。`OMIT`や`ERROR`が出たページも含める。

診断は内蔵入力をまだ有効にしない。BCE COUNT 0、SMBIOS UNAVAILABLE、BAR消失などは
診断結果として扱い、独立したUSB処理は続行する。
まずこの実機結果からBCEの位置とUEFI終了時の状態変化を確定する。

## Validation / 検証

2026-10-10のローカル検証:

- Rust host tests: 104件成功。新規4件はPCI capabilityの異常系、SMBIOSの長さ・
  checksum・文字列index・途中切断を含む。
- USB画像／パッケージ検証: 10件成功。
- T2-D1 + qemu-debug: 単一xHCIと複数xHCIでA/Shift/A release/Esc、FAT32読出し、終了を確認。
- QEMUのSMBIOS model取得成功、BCE COUNT 0、診断完了を確認。
  `tools/smoke-qemu.py --expect-t2-diagnostics`でT2診断マーカーも必須にできる。
- ログ: `out/t2-d1/validation/`（生成物、Git管理外）。

QEMUにはApple BCEがないため、T2が存在する実機でのconfiguration取得は未検証。
通信、内蔵キー入力、Touch Barの成功を示す結果ではない。
