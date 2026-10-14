# Musha-OS experimental C SDK

## English

Documentation: [Writing a C program](../docs/sdk-guide.md).

Build the C Hello with Python 3, Clang and LLD: `python3 tools/check-sdk.py --c-only --sdk . --output build` from the extracted archive. In the source repository use `python3 ../tools/check-sdk.py --c-only --sdk . --output build` from sdk/. Optional CMake: `cmake -S . -B build-cmake` then `cmake --build build-cmake`.

The archive includes local `docs/sdk-guide.md`, `docs/sdk.md`, `docs/external-app.md` and `docs/application-abi.md`; in the repository these documents are in ../docs/. They describe required tools, AUTORUN.ELF placement, 64-byte–16MiB/5-second bounds and reboot after exit/fault. Do not execute the guest ELF on the host. ABI candidate r1 is experimental and may change. Rust remains compile-only in the repository and is excluded from this C archive. Mac external applications are currently unsupported due to unrecognized storage. Candidate hardware acceptance is pending.

## 日本語

Documentation: [Cプログラムの作成ガイド](../docs/sdk-guide.md)。

展開ZIPでは`python3 tools/check-sdk.py --c-only --sdk . --output build`でC Helloを生成します。リポジトリのsdk/内では`../tools/check-sdk.py`を指定します。Python 3、Clang、LLDが必要です。任意のCMake手順は上記を参照してください。

ZIP内のdocs/（リポジトリでは../docs/）にSDK、外部アプリ、ABIの手順を同梱します。AUTORUN.ELF配置、64バイト～16MiB／5秒期限、終了・障害後の再起動を説明しています。ホストでゲストELFを実行しないでください。ABIは変更可能な実験候補r1です。Rustはcompile-onlyでZIPへ含めません。Macはストレージ未認識のため外部アプリ未対応、公開候補の実機受入は未完了です。
