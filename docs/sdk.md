# Experimental SDK foundation

## English

A separate [C1 fixed assembly probe](ring3-prototype.md) now exercises CPU transitions; it does not load these SDK samples.

Stage B compile/conformance material with an opt-in [C2 SDK C execution probe](ring3-elf.md), **not a supported application runtime**. Do not invoke sample syscalls on the host or current EFI. See [ABI](application-abi.md) and [validation](boot-api-validation.md).

JSON schema generates C/Rust public types. api.h/libmusha wrap query/log/exit. crt0, linker script, minimal memory routines and C Hello produce static ELF with NOTE. musha-sys provides unsafe raw access; musha supplies pure validation/result decoding only. CMake/custom target are starting configurations, not supported toolchain guarantees.

```sh
python3 sdk/tools/generate.py --check
python3 tools/check-sdk.py
python3 sdk/tools/app-check.py out/sdk-check/hello-c.elf
cargo test --manifest-path sdk/rust/Cargo.toml
```

Clang, Python3, Rust1.99.0 and x86_64-unknown-uefi are needed. CC/RUSTC/LD_LLD override tools. --sdk/--output support relocation. Exact versions/command arrays are in out/sdk-check/report.json. No release upload occurs. SDK Cargo.lock is separate from OS workspace.

C uses x86_64-unknown-none-elf, freestanding/static/non-PIC, no stack protector/red zone/AVX, baseline SSE2. Host layout execution and x64 compile assertions are separate. The SDK checker inspects C ELF. C2 independently validates it in the kernel and executes it under QEMU.

Rust UEFI target verifies x64 layout/wrappers, explicit SysV FP IR only; it is not a Rust ELF build. Custom x86_64-musha-user.json removes none target softfloat and supplies SSE2/no red zone/small model. JSON is unstable; pinned rust-src + core/alloc rebuild remains required before Rust ELF linking. References: [none target](https://doc.rust-lang.org/rustc/platform-support/x86_64-unknown-none.html), [custom targets](https://doc.rust-lang.org/nightly/rustc/targets/custom.html).

The [C3 probe](ring3-lifecycle.md) adds repeated C launches and bounded cleanup/return.

C gate: C and Rust guest Hello, query/log/exit, raw-register tests, C↔Rust double/mixed/stack args, FP state across syscall/fault, malformed ELF/pointers, app exit/launch failure. D adds practical handles/wait/cancel/I/O. No successful dummy dispatcher exists.

## 日本語

B段階のSDK雛形・コンパイル／整合検査です。SDK検査とは別の[C2試験](ring3-elf.md)でC ELFのカーネル検査と実行を行います。RustのUEFI型検査と専用user target/core再構築を区別し、後者は未完了です。版とコマンドをreport.jsonへ保存します。公開互換性固定・配布SDK・libc/POSIX対応は未完了です。

[C1試験](ring3-prototype.md)は固定アセンブリを動かす独立構成であり、このSDKのELFを読み込むものではありません。
