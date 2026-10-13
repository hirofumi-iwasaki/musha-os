# Musha experimental SDK skeleton

## English

Stage B compilation/conformance with an opt-in [C2 SDK C guest probe](../docs/ring3-elf.md). The normal EFI is not an external application runtime.
Do not execute the sample on the development host. No public ABI is fixed.
See [SDK instructions](../docs/sdk.md), [ABI ledger](../docs/application-abi.md) and
[validation/gates](../docs/boot-api-validation.md). C ELF is inspected; Rust user ELF,
custom-target core/alloc, general application resource management and practical external I/O remain pending. The [C3 probe](../docs/ring3-lifecycle.md) tests bounded recovery and reuse.

## 日本語

B段階の基礎にC2 SDK C実行試験を追加しました。通常EFIは外部アプリ実行環境ではありません。
サンプルをホストで実行しないでください。公開ABIは未固定です。
詳細・検査手順・残件は上記リンクを参照してください。
