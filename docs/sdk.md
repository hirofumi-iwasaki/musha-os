# C SDK for Musha-OS 0.3.0 candidates

## English

The separate `musha-os-vVERSION-x64-c-sdk.zip` contains C sources, crt0, minimal libmusha/memory routines, linker script, headers, ABI schema, ELF checker, tests, C Hello and a CMake entry point. Extract into any directory, including a path containing spaces. It has no dependency on the OS source checkout. The ABI is **experimental candidate r1**, not frozen compatibility. Rust material in the repository is **compile-only** and is not included in this archive. No libc/POSIX or practical application I/O is supplied.

Required: Python 3, Clang with x86-64 ELF support, and LLVM LLD (`ld.lld` or `rust-lld`). CMake 3.20+ is optional. `CC` selects Clang; `LD_LLD` selects LLD. With a standalone LLD, Rust is not required. If LLD is omitted, the checker can use the installed Rust toolchain's LLD (repository validation uses pinned Rust 1.99.0). Record the exact versions in `build/report.json`; Hello hashes are reproducible for the same compiler/linker versions and options, not promised across toolchain versions.

From the extracted SDK root:

```sh
export CC=clang
export LD_LLD=/absolute/path/to/ld.lld
python3 tools/check-sdk.py --c-only --sdk . --output "build result"
python3 tools/app-check.py "build result/hello-c.elf"
```

The checker compiles and links the freestanding sample, executes only a native host layout test, rejects malformed ELF fixtures and records `hello_sha256`. It never executes guest syscalls on the host. Copy `build result/hello-c.elf` to the selected USB filesystem root as **AUTORUN.ELF**. See [running an application](external-app.md) for boot, read limits and failure handling. Do not execute the ELF on macOS/Linux/Windows.

Optional CMake uses the same canonical checked build, so it produces the same ELF:

```sh
cmake -S . -B "cmake build"
cmake --build "cmake build"
# output: cmake build/checked/hello-c.elf
```

To modify Hello, edit `examples/hello-c/main.c` and repeat. The linker emits a static, non-PIC x86-64 ELF with the MUSHA NOTE; baseline SSE2, no AVX, no red zone and no host libc/crt. The exposed operations remain query, log and exit. The kernel validates the file independently.

Inside the OS repository the equivalent command is `python3 tools/check-sdk.py --c-only --sdk sdk --output out/sdk-c`. Omitting `--c-only` additionally performs repository Rust compile-only checks; it is not a Rust guest build. [ABI ledger](application-abi.md), [validation](boot-api-validation.md).

NUC5/Ryzen development builds have owner-reported Hello success. **Mac external applications are currently unsupported because the development build does not recognize its storage.** Candidate cold-boot acceptance is pending. Published v0.2.0 assets remain unchanged.

## 日本語

独立配布の`musha-os-vVERSION-x64-c-sdk.zip`にCソース、crt0、libmusha／メモリ処理、リンカースクリプト、ヘッダー、ABIスキーマ、ELF検査、試験、Hello、CMake入口を含めます。空白入りを含む任意の場所へ展開でき、OSリポジトリを必要としません。ABIは**実験候補r1**で変更の可能性があります。Rustはリポジトリ内の**compile-only**検査のみで、ZIPには同梱しません。libc／POSIXやアプリ向け実用I/Oはありません。

必要ツールはPython 3、x86-64 ELF対応Clang、LLVM LLDです。任意でCMake 3.20以上を使えます。`CC`と`LD_LLD`でツールを指定します。独立LLDならRust不要、未指定なら導入済みRustのLLDも利用可能です。リポジトリ検証はRust 1.99.0付属LLDを使います。具体的な版・コマンド・Hello SHA-256は`report.json`へ記録します。ハッシュ再現性は同じツール版・オプションで確認し、異なる版での一致は保証しません。

展開したSDKのルートで上記の`check-sdk.py --c-only`と`app-check.py`を実行してください。ホストで実行するのはネイティブの型配置検査のみです。生成された`hello-c.elf`を**AUTORUN.ELF**としてUSBの選択対象ファイルシステムのルートへ配置します。ゲストELFをmacOS等で直接実行しないでください。CMakeも同じ検証ビルドを呼び、`cmake build/checked/hello-c.elf`を生成します。サンプル変更は`examples/hello-c/main.c`を編集して再実行します。

実行方法・64バイト～16MiB・5秒期限・画面出力・終了／例外・再起動による復帰は[外部アプリ手順](external-app.md)を参照してください。NUC5／Ryzenの開発版Hello成功は報告済みですが公開候補の冷間起動確認は残件です。**Macは開発版ストレージ未認識のため外部アプリ未対応**。公開済みv0.2.0配布物は不変です。
